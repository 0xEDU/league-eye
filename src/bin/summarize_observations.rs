use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    observations: PathBuf,
    #[arg(long)]
    progress: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Observation {
    inference_latency_ms: u128,
    validation_errors: Vec<String>,
}

#[derive(Deserialize)]
struct Progress {
    frames_per_minute: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut observations: Vec<Observation> = fs::read_to_string(&args.observations)
        .with_context(|| format!("read {}", args.observations.display()))?
        .lines()
        .map(serde_json::from_str)
        .collect::<serde_json::Result<_>>()
        .context("decode observation JSONL")?;
    anyhow::ensure!(!observations.is_empty(), "observations are empty");
    let validation_error_records = observations
        .iter()
        .filter(|observation| !observation.validation_errors.is_empty())
        .count();
    let mut latencies: Vec<_> = observations
        .drain(..)
        .map(|observation| observation.inference_latency_ms)
        .collect();
    latencies.sort_unstable();
    let count = latencies.len();
    let metrics = serde_json::json!({
        "observation_count": count,
        "validation_error_records": validation_error_records,
        "validation_error_rate": validation_error_records as f64 / count as f64,
        "average_inference_latency_ms": latencies.iter().sum::<u128>() as f64 / count as f64,
        "median_inference_latency_ms": latencies[count / 2],
        "p95_inference_latency_ms": latencies[(count * 95).div_ceil(100) - 1],
        "frames_per_minute": args.progress.map(|path| -> Result<f64> {
            let progress: Progress = serde_json::from_slice(&fs::read(&path)?)?;
            Ok(progress.frames_per_minute)
        }).transpose()?
    });
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}
