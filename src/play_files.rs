use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn new_checkpoint_path(seed: u32, tick: u32) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(checkpoint_path(seed, tick, stamp.as_nanos()))
}

fn checkpoint_path(seed: u32, tick: u32, stamp: u128) -> PathBuf {
    // The writer still uses create_new: even a clock/name collision cannot overwrite.
    PathBuf::from("reports")
        .join("checkpoints")
        .join(format!("world-{seed}-tick{tick}-{stamp}.checkpoint"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
