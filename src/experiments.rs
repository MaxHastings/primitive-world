//! Append-only experiment saves. A receipt is published only after its complete
//! body checkpoint; failed writes never replace the last resumable experiment.
use crate::{simulation::Simulation, visible_trial::LoopSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
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
    let record: SaveRecord = serde_json::from_reader(file).map_err(|e| e.to_string())?;
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
