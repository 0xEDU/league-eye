use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    labels: PathBuf,
    #[arg(long)]
    selections: PathBuf,
}

#[derive(Deserialize)]
struct Label {
    video_timestamp_ms: u64,
    champion: String,
}

#[derive(Deserialize)]
struct Selection {
    video_timestamp_ms: u64,
    selected_champion: Option<String>,
}

fn lines<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<Vec<T>> {
    fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?
        .lines()
        .map(serde_json::from_str)
        .collect::<serde_json::Result<Vec<_>>>()
        .context("decode JSONL")
}

fn main() -> Result<()> {
    let args = Args::parse();
    let labels: Vec<Label> = lines(&args.labels)?;
    let selections: HashMap<_, _> = lines::<Selection>(&args.selections)?
        .into_iter()
        .map(|selection| (selection.video_timestamp_ms, selection.selected_champion))
        .collect();
    let mut predicted = 0_u32;
    let mut exact = 0_u32;
    for label in &labels {
        if let Some(Some(value)) = selections.get(&label.video_timestamp_ms) {
            predicted += 1;
            exact += u32::from(value.eq_ignore_ascii_case(&label.champion));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "labeled_frames": labels.len(),
            "predicted_frames": predicted,
            "exact_matches": exact,
            "exact_match_rate": if labels.is_empty() { 0.0 } else { exact as f64 / labels.len() as f64 }
        }))?
    );
    Ok(())
}
