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
}
