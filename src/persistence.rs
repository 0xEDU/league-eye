use crate::schema::Observation;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Default, Serialize, Deserialize)]
pub struct Checkpoint {
    pub source_fingerprint: String,
    pub model: String,
    pub config_fingerprint: String,
    pub last_frame_id: Option<String>,
    pub completed: u64,
    pub failed: u64,
    pub updated_unix_ms: u128,
}

pub struct RunLock {
    path: std::path::PathBuf,
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn acquire_run_lock(dir: &Path) -> Result<RunLock> {
    let path = dir.join("run.lock");
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
pub fn completed(dir: &Path, source: &str, config: &str) -> Result<HashSet<String>> {
    let p = dir.join("observations.jsonl");
    if !p.exists() {
        return Ok(HashSet::new());
    };
    Ok(std::fs::read_to_string(p)?
        .lines()
        .filter_map(|l| {
            serde_json::from_str::<Observation>(l)
                .ok()
                .filter(|x| x.video_fingerprint == source && x.config_fingerprint == config)
                .map(|x| x.frame_id)
        })
        .collect())
}

pub fn load_checkpoint(dir: &Path) -> Result<Option<Checkpoint>> {
    let path = dir.join("checkpoint.json");
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

pub fn latest_observation(dir: &Path, source: &str, config: &str) -> Result<Option<Observation>> {
    let path = dir.join("observations.jsonl");
    if !path.exists() {
        return Ok(None);
    }
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter_map(|line| serde_json::from_str::<Observation>(line).ok())
        .filter(|observation| {
            observation.video_fingerprint == source && observation.config_fingerprint == config
        })
        .max_by_key(|observation| observation.video_timestamp_ms))
}
pub fn append<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut f, value)?;
    f.write_all(b"\n")?;
    f.sync_data()?;
    Ok(())
}
pub fn checkpoint(dir: &Path, cp: &Checkpoint) -> Result<()> {
    let temp = dir.join("checkpoint.json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(cp)?)?;
    fs::rename(temp, dir.join("checkpoint.json")).context("replace checkpoint")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::acquire_run_lock;
    use std::fs;

    #[test]
    fn prevents_concurrent_runs_and_releases_after_drop() {
        let path = std::env::temp_dir().join(format!("league-vision-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        let lock = acquire_run_lock(&path).unwrap();
        assert!(acquire_run_lock(&path).is_err());
        drop(lock);
        assert!(acquire_run_lock(&path).is_ok());
        fs::remove_dir_all(&path).unwrap();
    }
}
