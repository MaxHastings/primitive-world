use std::{
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn export_history(seed: u32, tick: u32, bytes: &[u8]) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let path = PathBuf::from("reports/history")
        .join(format!("world-{seed}-tick{tick}-{}.json", stamp.as_nanos()));
    write_new_export(&path, bytes)?;
    Ok(path)
}

fn write_new_export(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

pub fn new_checkpoint_path(seed: u32, tick: u32) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(checkpoint_path(seed, tick, stamp.as_nanos()))
}

fn checkpoint_path(seed: u32, tick: u32, stamp: u128) -> PathBuf {
    // The writer uses create_new: even a clock/name collision cannot overwrite.
    PathBuf::from("reports")
        .join("checkpoints")
        .join(format!("world-{seed}-tick{tick}-{stamp}.checkpoint"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_export_preserves_existing_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("primitive-history-{}-{unique}", std::process::id()));
        let path = root.join("history.json");
        write_new_export(&path, b"[1]").unwrap();
        assert!(write_new_export(&path, b"[2]").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"[1]");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn snapshots_have_distinct_paths_even_at_the_same_paused_tick() {
        let first = checkpoint_path(17, 2000, 100);
        let second = checkpoint_path(17, 2000, 101);
        assert_ne!(first, second);
        assert_eq!(
            first.parent().unwrap(),
            std::path::Path::new("reports/checkpoints")
        );
        assert_eq!(
            first.file_name().unwrap(),
            "world-17-tick2000-100.checkpoint"
        );
    }
}
