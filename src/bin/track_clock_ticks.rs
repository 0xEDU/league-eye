use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Deserialize;
use std::{fs, path::PathBuf, process::Command};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "probe-footage.mp4")]
    input: PathBuf,
    #[arg(long, required = true)]
    start_seconds: u64,
    #[arg(long, required = true)]
    end_seconds: u64,
    #[arg(long, required = true)]
    anchor_seconds: u32,
    #[arg(long, default_value = "605:0:35:12")]
    crop: String,
    #[arg(long, default_value_t = 30)]
    fps: u32,
    #[arg(long, default_value_t = 7.0)]
    threshold: f64,
    #[arg(long, default_value_t = 400)]
    refractory_ms: u64,
    #[arg(long)]
    labels: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Label {
    video_timestamp_ms: u64,
    game_time_seconds: u32,
}

fn dimensions(crop: &str) -> Result<(usize, usize, String)> {
    let parts = crop.split(':').collect::<Vec<_>>();
    anyhow::ensure!(parts.len() == 4, "crop must use x:y:width:height");
    let x = parts[0].parse::<usize>()?;
    let y = parts[1].parse::<usize>()?;
    let width = parts[2].parse::<usize>()? & !1;
    let height = parts[3].parse::<usize>()? & !1;
    anyhow::ensure!(
        width > 0 && height > 0,
        "crop dimensions must be at least two pixels"
    );
    Ok((width, height, format!("crop={width}:{height}:{x}:{y}")))
}

fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        args.end_seconds > args.start_seconds,
        "end must follow start"
    );
    let (width, height, filter) = dimensions(&args.crop)?;
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-ss", &args.start_seconds.to_string(), "-to"])
        .arg(args.end_seconds.to_string())
        .arg("-i")
        .arg(&args.input)
        .args([
            "-vf",
            &format!("{filter},fps={}", args.fps),
            "-pix_fmt",
            "gray",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .output()
        .context("run ffmpeg")?;
    if !output.status.success() {
        bail!("ffmpeg failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let bytes_per_frame = width * height;
    anyhow::ensure!(
        output.stdout.len() % bytes_per_frame == 0,
        "ffmpeg returned incomplete frames"
    );
    let frames = output
        .stdout
        .chunks_exact(bytes_per_frame)
        .collect::<Vec<_>>();
    let mut ticks = Vec::new();
    let mut last_tick_ms = None;
    for (index, pair) in frames.windows(2).enumerate() {
        let score = pair[0]
            .iter()
            .zip(pair[1])
            .map(|(left, right)| left.abs_diff(*right) as u64)
            .sum::<u64>() as f64
            / bytes_per_frame as f64;
        let timestamp_ms =
            args.start_seconds * 1000 + ((index + 1) * 1000 / args.fps as usize) as u64;
        if score >= args.threshold
            && last_tick_ms
                .is_none_or(|last| timestamp_ms.saturating_sub(last) >= args.refractory_ms)
        {
            ticks.push(timestamp_ms);
            last_tick_ms = Some(timestamp_ms);
        }
    }
    let value_at = |timestamp_ms: u64| {
        args.anchor_seconds + ticks.iter().filter(|tick| **tick <= timestamp_ms).count() as u32
    };
    let evaluation = args
        .labels
        .as_ref()
        .map(|path| -> Result<_> {
            let labels = fs::read_to_string(path)
                .with_context(|| format!("read {}", path.display()))?
                .lines()
                .map(serde_json::from_str)
                .collect::<serde_json::Result<Vec<Label>>>()?;
            let mut exact = 0;
            let mut error = 0_u64;
            for label in &labels {
                let value = value_at(label.video_timestamp_ms);
                exact += u32::from(value == label.game_time_seconds);
                error += u64::from(value.abs_diff(label.game_time_seconds));
            }
            Ok(serde_json::json!({
                "labeled_frames": labels.len(),
                "exact_matches": exact,
                "exact_match_rate": exact as f64 / labels.len() as f64,
                "mean_absolute_error_seconds": error as f64 / labels.len() as f64
            }))
        })
        .transpose()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "sampled_frames": frames.len(),
            "detected_ticks": ticks.len(),
            "ticks_ms": ticks,
            "evaluation": evaluation
        }))?
    );
    Ok(())
}
