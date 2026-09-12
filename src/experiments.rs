//! Append-only experiment saves. A receipt is published only after its complete
//! body checkpoint; failed writes never replace the last resumable experiment.
use crate::simulation::Simulation;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

// Bound receipt reads, including files that grow during reading.
const RECEIPT_LIMIT: u64 = 64 * 1024 * 1024;
const RECEIPT_VERSION: u32 = 4;
/// Enough recovery points for one half-hour of normal autosaves, without
/// letting a long-running wallpaper consume unbounded storage.
const SNAPSHOTS_PER_EXPERIMENT: usize = 6;
/// The library remains useful across experiments without silently filling a disk.
const LIBRARY_BUDGET_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct Experiment {
    pub directory: PathBuf,
    pub name: String,
    pub origin: String,
    pub total_ticks: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRecord {
    pub version: u32,
    pub model: String,
    pub name: String,
    pub origin: String,
    pub checkpoint: String,
    pub saved_at_ms: u64,
    pub seed: u32,
    pub tick: u64,
    pub living: u32,
    pub world: u64,
    pub total_ticks: u64,
}

#[derive(Clone)]
pub struct SavedExperiment {
    pub directory: PathBuf,
    pub record: SaveRecord,
}

type LibraryResult = Result<(Vec<SavedExperiment>, usize), String>;

/// Disk traversal and receipt parsing never run in the window event handler.
/// Repeated refresh requests coalesce into one follow-up scan.
#[derive(Default)]
pub struct LibraryScan {
    receiver: Option<mpsc::Receiver<LibraryResult>>,
    pending: Option<PathBuf>,
}

impl LibraryScan {
    pub fn request(&mut self, root: PathBuf) {
        if self.receiver.is_some() {
            self.pending = Some(root);
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(list(&root));
        });
    }

    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn poll(&mut self) -> Option<LibraryResult> {
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Save-library scan stopped unexpectedly; refresh to retry".into())
            }
        };
        self.receiver = None;
        if let Some(root) = self.pending.take() {
            self.request(root);
        }
        Some(result)
    }
}

impl SavedExperiment {
    pub fn checkpoint(&self) -> PathBuf {
        self.directory.join(&self.record.checkpoint)
    }
    pub fn world_number(&self) -> u64 {
        self.record.world
    }
    pub fn experiment(&self) -> Experiment {
        Experiment {
            directory: self.directory.clone(),
            name: self.record.name.clone(),
            origin: self.record.origin.clone(),
            total_ticks: self.record.total_ticks,
        }
    }
}

pub fn stamp() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos())
}

pub fn save_root() -> PathBuf {
    if let Some(path) = std::env::var_os("PRIMITIVE_WORLD_SAVES") {
        return path.into();
    }
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("PrimitiveWorld/experiments");
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("primitive-world/experiments");
    }
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path).join(".local/share/primitive-world/experiments");
    }
    PathBuf::from("saves/experiments")
}

pub fn create(name: &str, origin: &str) -> Result<Experiment, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Give your experiment a name of 1–80 characters.".into());
    }
    let root = save_root();
    std::fs::create_dir_all(&root).map_err(|e| format!("Cannot open save folder: {e}"))?;
    // Display names are never used as filesystem paths.
    let directory = root.join(format!("experiment-{}-{}", stamp()?, std::process::id()));
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    Ok(Experiment {
        directory,
        name: name.into(),
        origin: origin.into(),
        total_ticks: 0,
    })
}

impl Experiment {
    pub fn save(
        &self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<PathBuf, String> {
        let stamp = stamp()?;
        let checkpoint = format!("save-{stamp}.checkpoint");
        let pending = self
            .directory
            .join(format!("save-{stamp}.checkpoint.partial"));
        sim.save_checkpoint(d, q, &pending)?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&pending)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        let complete = self.directory.join(&checkpoint);
        std::fs::rename(&pending, &complete).map_err(|e| e.to_string())?;
        let record = SaveRecord {
            version: RECEIPT_VERSION,
            model: crate::model::MODEL_ID.into(),
            name: self.name.clone(),
            origin: self.origin.clone(),
            checkpoint,
            saved_at_ms: (stamp / 1_000_000) as u64,
            seed: sim.seed,
            tick: sim.tick,
            living: sim.metrics(d, q)?.living as u32,
            world: sim.progress.world,
            total_ticks: self.total_ticks,
        };
        publish_record(&self.directory.join(format!("save-{stamp}.json")), &record)?;
        if let Some(root) = self.directory.parent()
            && let Err(error) = prune(root)
        {
            // The just-published save remains valid. A later save or explicit
            // cleanup can retry retention without pausing the simulation.
            eprintln!("Save retention cleanup failed: {error}");
        }
        Ok(complete)
    }
}

#[derive(Clone)]
struct Snapshot {
    receipt: PathBuf,
    checkpoint: PathBuf,
    saved_at_ms: u64,
    bytes: u64,
}

/// Remove only complete, current-format snapshot pairs. The newest valid save
/// of every experiment is protected even when the global budget is exceeded.
pub fn prune(root: &Path) -> Result<PruneReport, String> {
    if !root.exists() {
        return Ok(PruneReport::default());
    }
    let mut experiments = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let directory = entry.map_err(|e| e.to_string())?.path();
        if !directory.is_dir() {
            continue;
        }
        let mut snapshots = snapshots_in(&directory)?;
        snapshots.sort_unstable_by_key(|snapshot| std::cmp::Reverse(snapshot.saved_at_ms));
        experiments.push(snapshots);
    }

    let mut report = PruneReport::default();
    let mut removable = Vec::new();
    for snapshots in &experiments {
        report.bytes_before += snapshots.iter().map(|snapshot| snapshot.bytes).sum::<u64>();
        removable.extend(snapshots.iter().skip(SNAPSHOTS_PER_EXPERIMENT).cloned());
    }
    removable.sort_unstable_by_key(|snapshot| snapshot.saved_at_ms);
    let mut retained_bytes = report
        .bytes_before
        .saturating_sub(removable.iter().map(|snapshot| snapshot.bytes).sum::<u64>());

    // Once each experiment is down to its six newest snapshots, age out further
    // snapshots across the library. The newest save of every experiment stays.
    if retained_bytes > LIBRARY_BUDGET_BYTES {
        let mut extra: Vec<_> = experiments
            .iter()
            .flat_map(|snapshots| snapshots.iter().skip(1).cloned())
            .filter(|snapshot| !removable.iter().any(|old| old.receipt == snapshot.receipt))
            .collect();
        extra.sort_unstable_by_key(|snapshot| snapshot.saved_at_ms);
        for snapshot in extra {
            if retained_bytes <= LIBRARY_BUDGET_BYTES {
                break;
            }
            retained_bytes = retained_bytes.saturating_sub(snapshot.bytes);
            removable.push(snapshot);
        }
    }
    for snapshot in removable {
        std::fs::remove_file(&snapshot.checkpoint).map_err(|e| e.to_string())?;
        std::fs::remove_file(&snapshot.receipt).map_err(|e| e.to_string())?;
        report.removed_snapshots += 1;
        report.removed_bytes += snapshot.bytes;
    }
    report.bytes_after = report.bytes_before.saturating_sub(report.removed_bytes);
    Ok(report)
}

#[derive(Default)]
pub struct PruneReport {
    pub removed_snapshots: usize,
    pub removed_bytes: u64,
    pub bytes_before: u64,
    pub bytes_after: u64,
}

impl PruneReport {
    pub fn message(&self) -> String {
        format!(
            "Removed {} old snapshots and freed {:.1} GiB; save library is now {:.1} GiB",
            self.removed_snapshots,
            self.removed_bytes as f64 / 1024.0_f64.powi(3),
            self.bytes_after as f64 / 1024.0_f64.powi(3),
        )
    }
}

fn snapshots_in(directory: &Path) -> Result<Vec<Snapshot>, String> {
    let mut snapshots = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let receipt = entry.map_err(|e| e.to_string())?.path();
        if receipt
            .extension()
            .is_none_or(|extension| extension != "json")
            || !receipt
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("save-"))
        {
            continue;
        }
        let Ok(saved) = read_record(&receipt) else {
            continue;
        };
        let checkpoint = saved.checkpoint();
        let bytes = receipt.metadata().map_err(|e| e.to_string())?.len()
            + checkpoint.metadata().map_err(|e| e.to_string())?.len();
        snapshots.push(Snapshot {
            receipt,
            checkpoint,
            saved_at_ms: saved.record.saved_at_ms,
            bytes,
        });
    }
    Ok(snapshots)
}

fn publish_record(path: &Path, record: &SaveRecord) -> Result<(), String> {
    let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > RECEIPT_LIMIT {
        return Err("Experiment receipt is too large".into());
    }
    let pending = path.with_extension("json.partial");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    if path.exists() {
        return Err("Save receipt already exists".into());
    }
    std::fs::rename(pending, path).map_err(|e| e.to_string())
}

pub fn read_record(path: &Path) -> Result<SavedExperiment, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > RECEIPT_LIMIT {
        return Err("Experiment receipt is too large".into());
    }
    let record = read_record_data(file)?;
    if record.version != RECEIPT_VERSION
        || record.model != crate::model::MODEL_ID
        || record.world == 0
    {
        return Err("Incompatible save: this simulator requires population-search world saves. Only current-format data can be loaded.".into());
    }
    let checkpoint = Path::new(&record.checkpoint);
    if checkpoint.components().count() != 1
        || !matches!(
            checkpoint.components().next(),
            Some(std::path::Component::Normal(_))
        )
        || checkpoint.extension().is_none_or(|x| x != "checkpoint")
        || record.checkpoint.contains(['/', '\\', ':'])
    {
        return Err("Invalid experiment checkpoint path".into());
    }
    let directory = path
        .parent()
        .ok_or("Save has no parent folder")?
        .to_path_buf();
    if !directory.join(checkpoint).is_file() {
        return Err("Experiment checkpoint is missing".into());
    }
    Ok(SavedExperiment { directory, record })
}

fn read_record_data(reader: impl Read) -> Result<SaveRecord, String> {
    let mut bytes = Vec::new();
    // Bound the actual read too, in case a file grows after the metadata check.
    reader
        .take(RECEIPT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > RECEIPT_LIMIT {
        return Err("Experiment receipt is too large".into());
    }
    // serde_json::from_reader(File) performs tiny unbuffered OS reads. Parsing
    // the bounded memory slice avoids millions of disk calls per archive.
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// One latest valid receipt per experiment. Interrupted saves are skipped, with
/// a visible warning; a previous complete receipt remains available.
pub fn list(root: &Path) -> Result<(Vec<SavedExperiment>, usize), String> {
    if !root.exists() {
        return Ok((Vec::new(), 0));
    }
    let mut saves = Vec::new();
    let mut invalid = 0;
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let directory = entry.map_err(|e| e.to_string())?.path();
        if !directory.is_dir() {
            continue;
        }
        let mut receipts: Vec<_> = std::fs::read_dir(&directory)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|x| x == "json")
                    && p.file_name()
                        .is_some_and(|x| x.to_string_lossy().starts_with("save-"))
            })
            .collect();
        receipts.sort_unstable_by(|a, b| b.cmp(a));
        for path in receipts {
            match read_record(&path) {
                Ok(saved) => {
                    saves.push(saved);
                    break;
                }
                Err(_) => invalid += 1,
            }
        }
    }
    saves.sort_by_key(|x| std::cmp::Reverse(x.record.saved_at_ms));
    Ok((saves, invalid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_parsing_batches_reads_and_keeps_the_size_limit() {
        struct CountedReader {
            data: std::io::Cursor<Vec<u8>>,
            calls: usize,
        }
        impl Read for CountedReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.calls += 1;
                self.data.read(buf)
            }
        }
        let mut value = serde_json::json!({
            "version": RECEIPT_VERSION, "model": crate::model::MODEL_ID, "name": "Fixture",
            "origin": "Test", "checkpoint": "save-1.checkpoint", "saved_at_ms": 1,
            "seed": 42, "tick": 128, "living": 1, "world":1, "total_ticks": 128
        });
        // A large current-schema field isolates I/O behavior without extra fields.
        value["origin"] = serde_json::Value::String("x".repeat(1_000_000));
        let mut reader = CountedReader {
            data: std::io::Cursor::new(serde_json::to_vec(&value).unwrap()),
            calls: 0,
        };
        assert_eq!(read_record_data(&mut reader).unwrap().tick, 128);
        assert!(
            reader.calls < 100,
            "Receipt caused {} underlying reads",
            reader.calls
        );
        assert!(
            read_record_data(std::io::repeat(b' ').take(RECEIPT_LIMIT + 1))
                .err()
                .unwrap()
                .contains("too large")
        );
    }

    #[test]
    fn retention_keeps_the_six_newest_valid_snapshots_per_experiment() {
        let root = std::env::temp_dir().join(format!("primitive-retention-{}", stamp().unwrap()));
        let directory = root.join("experiment-fixture");
        std::fs::create_dir_all(&directory).unwrap();
        for value in 1..=8_u64 {
            let checkpoint = format!("save-{value}.checkpoint");
            std::fs::write(directory.join(&checkpoint), vec![0_u8; value as usize]).unwrap();
            publish_record(
                &directory.join(format!("save-{value}.json")),
                &SaveRecord {
                    version: RECEIPT_VERSION,
                    model: crate::model::MODEL_ID.into(),
                    name: "Fixture".into(),
                    origin: "Test".into(),
                    checkpoint,
                    saved_at_ms: value,
                    seed: 1,
                    tick: value,
                    living: 1,
                    world: 1,
                    total_ticks: value,
                },
            )
            .unwrap();
        }
        let report = prune(&root).unwrap();
        assert_eq!(report.removed_snapshots, 2);
        for value in 1..=2 {
            assert!(!directory.join(format!("save-{value}.json")).exists());
        }
        for value in 3..=8 {
            assert!(directory.join(format!("save-{value}.checkpoint")).exists());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_poll_never_waits_for_slow_work_and_reports_worker_failure() {
        let (sender, receiver) = mpsc::channel();
        let mut scan = LibraryScan {
            receiver: Some(receiver),
            pending: None,
        };
        assert!(scan.busy());
        assert!(scan.poll().is_none()); // sender has intentionally not replied
        sender.send(Ok((Vec::new(), 3))).unwrap();
        assert_eq!(scan.poll().unwrap().unwrap().1, 3);
        assert!(!scan.busy());
        let (sender, receiver) = mpsc::channel();
        scan.receiver = Some(receiver);
        drop(sender);
        assert!(scan.poll().unwrap().is_err());
        assert!(!scan.busy());
    }

    #[test]
    fn refreshes_coalesce_and_a_new_scan_follows_inflight_work() {
        let (sender, receiver) = mpsc::channel();
        let mut scan = LibraryScan {
            receiver: Some(receiver),
            pending: None,
        };
        let missing = std::env::temp_dir().join(format!("missing-library-{}", stamp().unwrap()));
        scan.request(missing.join("superseded"));
        scan.request(missing.clone());
        assert_eq!(scan.pending.as_ref(), Some(&missing));
        assert!(scan.poll().is_none());
        sender.send(Ok((Vec::new(), 2))).unwrap();
        assert_eq!(scan.poll().unwrap().unwrap().1, 2);
        assert!(
            scan.busy(),
            "The queued refresh must start after the old result"
        );
        assert!(scan.pending.is_none());
        // Waiting is test-only: the actual UI exclusively uses non-blocking poll.
        let result = scan
            .receiver
            .take()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert!(result.0.is_empty());
        assert_eq!(result.1, 0);
    }

    #[test]
    fn incomplete_save_keeps_previous_receipt_and_path_traversal_is_rejected() {
        let root = std::env::temp_dir().join(format!("primitive-save-test-{}", stamp().unwrap()));
        let folder = root.join("experiment");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("save-100.checkpoint"), b"fixture").unwrap();
        let mut record = SaveRecord {
            version: 4,
            model: crate::model::MODEL_ID.into(),
            name: "Line A".into(),
            origin: "Random".into(),
            checkpoint: "save-100.checkpoint".into(),
            saved_at_ms: 100,
            seed: 42,
            tick: 300,
            living: 10,
            world: 1,
            total_ticks: 900,
        };
        publish_record(&folder.join("save-100.json"), &record).unwrap();
        std::fs::write(folder.join("save-200.json"), b"broken").unwrap();
        std::fs::write(folder.join("save-300.json.partial"), b"pending").unwrap();
        let (saves, invalid) = list(&root).unwrap();
        assert_eq!(saves.len(), 1);
        assert_eq!(invalid, 1);
        assert_eq!(saves[0].record.total_ticks, 900);
        assert_eq!(saves[0].record.name, "Line A");
        for bad in [
            "../save-100.checkpoint",
            "..\\save-100.checkpoint",
            "C:\\save.checkpoint",
            "save.checkpoint:stream",
        ] {
            record.checkpoint = bad.into();
            let path = folder.join("hostile.json");
            std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            assert!(read_record(&path).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
