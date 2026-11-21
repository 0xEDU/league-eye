use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Parser)]
struct Args {
    #[arg(long, required = true, num_args = 1..)]
    observations: Vec<PathBuf>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 15)]
    max_drift_seconds: u32,
    #[arg(long)]
    seed_timestamp_ms: Option<u64>,
    #[arg(long)]
    seed_seconds: Option<u32>,
}

#[derive(Deserialize)]
struct Extraction {
    game_time_seconds: Option<u32>,
}

#[derive(Deserialize)]
struct Observation {
    video_timestamp_ms: u64,
    extraction: Extraction,
}

#[derive(Serialize)]
struct Selection {
    video_timestamp_ms: u64,
    candidates_seconds: Vec<u32>,
    selected_seconds: Option<u32>,
    status: String,
}

fn load(path: &PathBuf) -> Result<Vec<Observation>> {
    fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?
        .lines()
        .map(serde_json::from_str)
        .collect::<serde_json::Result<Vec<_>>>()
        .context("decode observation JSONL")
}

fn source_values(values: Vec<(u64, u32)>) -> BTreeMap<u64, u32> {
    let mut grouped = BTreeMap::<u64, BTreeSet<u32>>::new();
    for (timestamp, value) in values {
        grouped.entry(timestamp).or_default().insert(value);
    }
    grouped
        .into_iter()
        .filter_map(|(timestamp, values)| {
            (values.len() == 1).then(|| (timestamp, values.into_iter().next().expect("one value")))
        })
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        args.seed_timestamp_ms.is_some() && args.seed_seconds.is_some(),
        "a verified seed timestamp and seed seconds are required"
    );
    let mut grouped: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
    for path in &args.observations {
        let values = load(path)?
            .into_iter()
            .filter_map(|observation| {
                observation
                    .extraction
                    .game_time_seconds
                    .map(|value| (observation.video_timestamp_ms, value))
            })
            .collect();
        for (timestamp, value) in source_values(values) {
            grouped.entry(timestamp).or_default().push(value);
        }
    }
    let seed_timestamp_ms = args.seed_timestamp_ms.expect("verified seed is required");
    let seed_seconds = args.seed_seconds.expect("verified seed is required");
    let mut output = format!(
        "{}\n",
        serde_json::to_string(&Selection {
            video_timestamp_ms: seed_timestamp_ms,
            candidates_seconds: Vec::new(),
            selected_seconds: Some(seed_seconds),
            status: "verified_seed".to_string(),
        })?
    );
    let mut previous = args.seed_timestamp_ms.zip(args.seed_seconds);
    for (timestamp, mut candidates) in grouped {
        if args.seed_timestamp_ms.is_some_and(|seed| timestamp <= seed) {
            continue;
        }
        candidates.sort_unstable();
        candidates.dedup();
        let (previous_timestamp, previous_value) = previous.expect("verified seed is required");
        let expected = previous_value + ((timestamp - previous_timestamp) / 1000) as u32;
        let candidate = candidates
            .iter()
            .min_by_key(|candidate| candidate.abs_diff(expected))
            .copied();
        let (selected, status) = match candidate
            .filter(|candidate| candidate.abs_diff(expected) <= args.max_drift_seconds)
        {
            Some(candidate) => (Some(candidate), "selected_near_expected".to_string()),
            None => (None, "no_candidate_near_expected".to_string()),
        };
        if let Some(value) = selected {
            previous = Some((timestamp, value));
        }
        let record = Selection {
            video_timestamp_ms: timestamp,
            candidates_seconds: candidates,
            selected_seconds: selected,
            status,
        };
        output.push_str(&serde_json::to_string(&record)?);
        output.push('\n');
    }
    fs::write(&args.output, output).with_context(|| format!("write {}", args.output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::source_values;

    #[test]
    fn retries_count_once_and_conflicts_are_suppressed() {
        let values = source_values(vec![(10, 120), (10, 120), (20, 130), (20, 131)]);
        assert_eq!(values.get(&10), Some(&120));
        assert!(!values.contains_key(&20));
    }
}
