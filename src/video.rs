use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{path::Path, process::Command};

pub fn metadata(path: &Path) -> Result<Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("run ffprobe")?;
    if !output.status.success() {
        bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub fn duration_seconds(meta: &Value) -> Result<f64> {
    meta.pointer("/format/duration")
        .and_then(Value::as_str)
        .context("video has no duration")?
        .parse()
        .context("invalid duration")
}

pub fn frame_jpeg(
    path: &Path,
    timestamp_ms: u64,
    crop: Option<&str>,
    max_width: Option<u32>,
    quality: u8,
) -> Result<Vec<u8>> {
    let mut filters = Vec::new();
    if let Some(region) = crop {
        anyhow::ensure!(
            region.chars().all(|c| c.is_ascii_digit() || c == ':'),
            "crop must use x:y:width:height"
        );
        let parts: Vec<_> = region.split(':').collect();
        anyhow::ensure!(parts.len() == 4, "crop must use x:y:width:height");
        filters.push(format!(
            "crop={}:{}:{}:{}",
            parts[2], parts[3], parts[0], parts[1]
        ));
    }
    if let Some(width) = max_width {
        filters.push(format!("scale={width}:-2:flags=lanczos"));
    }
    let filter = filters.join(",");
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-v",
        "error",
        "-ss",
        &format!("{:.3}", timestamp_ms as f64 / 1000.0),
        "-i",
    ])
    .arg(path)
    .args(["-frames:v", "1"]);
    if !filter.is_empty() {
        cmd.args(["-vf", &filter]);
    }
    let output = cmd
        .args([
            "-q:v",
            &quality.to_string(),
            "-f",
            "image2pipe",
            "-vcodec",
            "mjpeg",
            "pipe:1",
        ])
        .output()
        .context("run ffmpeg")?;
    if !output.status.success() {
        bail!("ffmpeg failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    if output.stdout.is_empty() {
        bail!("ffmpeg returned an empty frame");
    }
    Ok(output.stdout)
}
