//! Append-only experiment saves. A receipt is published only after its complete
//! body checkpoint; failed writes never replace the last resumable experiment.
use crate::{simulation::Simulation, visible_trial::LoopSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Experiment {
    pub directory: PathBuf,
    pub name: String,
    pub origin: String,
    pub total_ticks: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveRecord {
    pub version: u32,
    pub model: String,
    pub name: String,
    pub origin: String,
    pub checkpoint: String,
    pub saved_at_ms: u64,
    pub seed: u32,
    pub tick: u32,
    pub living: u32,
    pub total_ticks: u64,
    pub evolution: Option<LoopSnapshot>,
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
        self.record.evolution.as_ref().map_or(1, |x| x.world_number)
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
    pub fn next_session(&self) -> Result<PathBuf, String> {
        Ok(self.directory.join(format!("session-{}", stamp()?)))
    }

    pub fn save(
        &self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
        evolution: Option<LoopSnapshot>,
    ) -> Result<PathBuf, String> {
        if let Some(snapshot) = &evolution {
            snapshot.validate()?;
            if snapshot.latest.bank.source_seed != sim.seed
                || snapshot.latest.bank.source_tick > sim.tick
            {
                return Err("Survivor archive does not belong to this checkpoint".into());
            }
        }
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
            version: 1,
            model: crate::model::MODEL_ID.into(),
            name: self.name.clone(),
            origin: self.origin.clone(),
            checkpoint,
            saved_at_ms: (stamp / 1_000_000) as u64,
            seed: sim.seed,
            tick: sim.tick,
            living: sim.metrics(d, q)?.living as u32,
            total_ticks: self.total_ticks,
            evolution,
        };
        publish_record(&self.directory.join(format!("save-{stamp}.json")), &record)?;
        Ok(complete)
    }
}

fn publish_record(path: &Path, record: &SaveRecord) -> Result<(), String> {
    let pending = path.with_extension("json.partial");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(record).map_err(|e| e.to_string())?)
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
    if file.metadata().map_err(|e| e.to_string())?.len() > 16_777_216 {
        return Err("Experiment receipt is too large".into());
    }
    let record = read_record_data(file)?;
    if record.version != 1 || record.model != crate::model::MODEL_ID {
        return Err("Unsupported experiment save".into());
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
    if let Some(snapshot) = &record.evolution {
        snapshot.validate()?;
        if snapshot.latest.bank.source_seed != record.seed
            || snapshot.latest.bank.source_tick > record.tick
        {
            return Err("Survivor archive does not belong to this experiment".into());
        }
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
    const LIMIT: u64 = 16_777_216;
    let mut bytes = Vec::new();
    // Bound the actual read too, in case a file grows after the metadata check.
    reader
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
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
            "version": 1, "model": crate::model::MODEL_ID, "name": "Fixture",
            "origin": "Test", "checkpoint": "save-1.checkpoint", "saved_at_ms": 1,
            "seed": 42, "tick": 128, "living": 1, "total_ticks": 128, "evolution": null
        });
        // Unknown fields remain permitted; a large field isolates I/O behavior.
        value["padding"] = serde_json::Value::String("x".repeat(1_000_000));
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
            read_record_data(std::io::repeat(b' ').take(16_777_217))
                .err()
                .unwrap()
                .contains("too large")
        );
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
    #[ignore = "read-only local diagnostic; requires PRIMITIVE_RECEIPT_PROBE"]
    fn receipt_io_throughput_probe() {
        let path = PathBuf::from(std::env::var_os("PRIMITIVE_RECEIPT_PROBE").unwrap());
        let start = std::time::Instant::now();
        let legacy: SaveRecord =
            serde_json::from_reader(std::fs::File::open(&path).unwrap()).unwrap();
        let old = start.elapsed();
        let start = std::time::Instant::now();
        let new = read_record_data(std::fs::File::open(&path).unwrap()).unwrap();
        let current = start.elapsed();
        assert_eq!(
            serde_json::to_vec(&legacy).unwrap(),
            serde_json::to_vec(&new).unwrap()
        );
        eprintln!(
            "Same {}-byte receipt: legacy unbuffered {:.3}s; bounded memory parse {:.3}s",
            std::fs::metadata(path).unwrap().len(),
            old.as_secs_f64(),
            current.as_secs_f64()
        );
        let start = std::time::Instant::now();
        let (saves, skipped) = list(&save_root()).unwrap();
        eprintln!(
            "Current local library: {:.3}s; {} compatible experiments, {} skipped receipts",
            start.elapsed().as_secs_f64(),
            saves.len(),
            skipped
        );
    }
    #[test]
    fn incomplete_save_keeps_previous_receipt_and_path_traversal_is_rejected() {
        let root = std::env::temp_dir().join(format!("primitive-save-test-{}", stamp().unwrap()));
        let folder = root.join("experiment");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("save-100.checkpoint"), b"fixture").unwrap();
        let mut record = SaveRecord {
            version: 1,
            model: crate::model::MODEL_ID.into(),
            name: "Line A".into(),
            origin: "Random".into(),
            checkpoint: "save-100.checkpoint".into(),
            saved_at_ms: 100,
            seed: 42,
            tick: 300,
            living: 10,
            total_ticks: 900,
            evolution: None,
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
