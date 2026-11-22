use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub struct RunLock {
    path: std::path::PathBuf,
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn acquire_run_lock(directory: &Path) -> Result<RunLock> {
    let path = directory.join("run.lock");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| {
            format!(
                "output has an active or stale lock at {}; verify no run is active, then remove this lock",
                path.display()
            )
        })?;
    writeln!(file, "pid={}", std::process::id())?;
    file.sync_data()?;
    Ok(RunLock { path })
}

pub fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::acquire_run_lock;
    use std::fs;

    #[test]
    fn prevents_concurrent_runs_and_releases_after_drop() {
        let path = std::env::temp_dir().join(format!("league-eye-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        let lock = acquire_run_lock(&path).unwrap();
        assert!(acquire_run_lock(&path).is_err());
        drop(lock);
        assert!(acquire_run_lock(&path).is_ok());
        fs::remove_dir_all(&path).unwrap();
    }
}
