use anyhow::{Context, Result, bail};
use image::{ColorType, codecs::jpeg::JpegEncoder};
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
    resize_filter: &str,
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
        anyhow::ensure!(
            matches!(resize_filter, "lanczos" | "neighbor"),
            "resize filter must be lanczos or neighbor"
        );
        filters.push(format!("scale={width}:-2:flags={resize_filter}"));
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

pub fn burst_jpeg(
    path: &Path,
    timestamp_ms: u64,
    crop: Option<&str>,
    max_width: Option<u32>,
    resize_filter: &str,
    quality: u8,
    frames: u32,
    radius_ms: u64,
    strategy: &str,
) -> Result<Vec<u8>> {
    anyhow::ensure!(frames > 0, "burst frames must be positive");
    anyhow::ensure!(frames % 2 == 1, "burst frames must be odd");
    if frames == 1 || radius_ms == 0 {
        return frame_jpeg(path, timestamp_ms, crop, max_width, resize_filter, quality);
    }
    anyhow::ensure!(
        matches!(strategy, "average" | "sharpest"),
        "burst strategy must be average or sharpest"
    );
    let center = frames / 2;
    let mut pixels = Vec::<u64>::new();
    let mut width = 0;
    let mut height = 0;
    let mut sharpest = None;
    for index in 0..frames {
        let offset = radius_ms * u64::from(index.abs_diff(center)) / u64::from(center);
        let sample_ms = if index < center {
            timestamp_ms.saturating_sub(offset)
        } else {
            timestamp_ms.saturating_add(offset)
        };
        let jpeg = frame_jpeg(path, sample_ms, crop, max_width, resize_filter, quality)?;
        let image = image::load_from_memory(&jpeg)?.to_rgb8();
        if strategy == "sharpest" {
            let luma = image::DynamicImage::ImageRgb8(image.clone()).to_luma8();
            let mut sum = 0_f64;
            let mut squared_sum = 0_f64;
            let mut count = 0_u64;
            for y in 1..luma.height().saturating_sub(1) {
                for x in 1..luma.width().saturating_sub(1) {
                    let value = f64::from(luma.get_pixel(x, y)[0]) * -4.0
                        + f64::from(luma.get_pixel(x - 1, y)[0])
                        + f64::from(luma.get_pixel(x + 1, y)[0])
                        + f64::from(luma.get_pixel(x, y - 1)[0])
                        + f64::from(luma.get_pixel(x, y + 1)[0]);
                    sum += value;
                    squared_sum += value * value;
                    count += 1;
                }
            }
            let sharpness = if count == 0 {
                0.0
            } else {
                squared_sum / count as f64 - (sum / count as f64).powi(2)
            };
            if sharpest
                .as_ref()
                .is_none_or(|(best, _): &(f64, Vec<u8>)| sharpness > *best)
            {
                sharpest = Some((sharpness, jpeg));
            }
            continue;
        }
        if pixels.is_empty() {
            width = image.width();
            height = image.height();
            pixels.resize((width * height * 3) as usize, 0);
        }
        anyhow::ensure!(
            image.width() == width && image.height() == height,
            "burst frames have inconsistent dimensions"
        );
        for (sum, value) in pixels.iter_mut().zip(image.as_raw()) {
            *sum += u64::from(*value);
        }
    }
    if let Some((_, jpeg)) = sharpest {
        return Ok(jpeg);
    }
    let averaged = pixels
        .into_iter()
        .map(|value| (value / u64::from(frames)) as u8)
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    JpegEncoder::new_with_quality(&mut output, quality).encode(
        &averaged,
        width,
        height,
        ColorType::Rgb8.into(),
    )?;
    Ok(output)
}
