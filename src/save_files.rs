use std::{io::Write, path::Path};

pub(crate) fn write_new(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if path.exists() {
        return if std::fs::read(path).map_err(|e| e.to_string())? == bytes {
            Ok(())
        } else {
            Err(format!(
                "{} already contains a different receipt",
                path.display()
            ))
        };
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
