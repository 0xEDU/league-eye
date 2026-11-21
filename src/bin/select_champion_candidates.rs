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
    #[arg(long, default_value_t = 2)]
    min_votes: usize,
}

#[derive(Deserialize)]
struct Extraction {
    champion: Option<String>,
}

#[derive(Deserialize)]
struct Observation {
    video_timestamp_ms: u64,
    extraction: Extraction,
}

#[derive(Serialize)]
struct Selection {
    video_timestamp_ms: u64,
    candidates: Vec<String>,
    selected_champion: Option<String>,
    status: String,
}

fn normalize(value: String) -> Option<String> {
    let filtered: String = value
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .collect();
    (!filtered.is_empty()).then(|| {
        let mut characters = filtered.chars();
        let first = characters.next().expect("nonempty value");
        format!(
            "{}{}",
            first.to_ascii_uppercase(),
            characters.as_str().to_ascii_lowercase()
        )
    })
}

fn choose(candidates: &[String], min_votes: usize) -> Option<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for candidate in candidates {
        *counts.entry(candidate).or_default() += 1;
    }
    let mut winners = counts
        .into_iter()
        .filter(|(_, count)| *count >= min_votes)
        .collect::<Vec<_>>();
    winners.sort_by_key(|(value, count)| (std::cmp::Reverse(*count), *value));
    match winners.as_slice() {
        [(value, _)] => Some((*value).to_string()),
        _ => None,
    }
}

fn source_votes(values: Vec<(u64, String)>) -> BTreeMap<u64, String> {
    let mut source = BTreeMap::<u64, BTreeSet<String>>::new();
    for (timestamp, champion) in values {
        source.entry(timestamp).or_default().insert(champion);
    }
    source
        .into_iter()
        .filter_map(|(timestamp, candidates)| {
            (candidates.len() == 1).then(|| {
                (
                    timestamp,
                    candidates.into_iter().next().expect("one candidate"),
                )
            })
        })
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(args.min_votes > 0, "min votes must be positive");
    let mut grouped = BTreeMap::<u64, Vec<String>>::new();
    for path in &args.observations {
        let input = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut values = Vec::new();
        for line in input.lines() {
            let observation: Observation =
                serde_json::from_str(line).context("decode observation JSONL")?;
            if let Some(champion) = observation.extraction.champion.and_then(normalize) {
                values.push((observation.video_timestamp_ms, champion));
            }
        }
        for (timestamp, champion) in source_votes(values) {
            grouped.entry(timestamp).or_default().push(champion);
        }
    }
    let mut output = String::new();
    for (timestamp, mut candidates) in grouped {
        candidates.sort();
        let selected_champion = choose(&candidates, args.min_votes);
        let status = if selected_champion.is_some() {
            "consensus"
        } else {
            "no_consensus"
        };
        output.push_str(&serde_json::to_string(&Selection {
            video_timestamp_ms: timestamp,
            candidates,
            selected_champion,
            status: status.to_string(),
        })?);
        output.push('\n');
    }
    fs::write(&args.output, output).with_context(|| format!("write {}", args.output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{choose, normalize, source_votes};

    #[test]
    fn accepts_a_unique_consensus() {
        let values = vec!["Sona".to_string(), "Sona".to_string(), "Lux".to_string()];
        assert_eq!(choose(&values, 2), Some("Sona".to_string()));
    }

    #[test]
    fn rejects_a_tied_consensus() {
        let values = vec!["Sona".to_string(), "Lux".to_string()];
        assert_eq!(choose(&values, 1), None);
    }

    #[test]
    fn normalizes_model_punctuation() {
        assert_eq!(normalize("  sona! ".to_string()), Some("Sona".to_string()));
    }

    #[test]
    fn retries_count_once_and_conflicts_are_suppressed() {
        let votes = source_votes(vec![
            (10, "Sona".to_string()),
            (10, "Sona".to_string()),
            (20, "Sona".to_string()),
            (20, "Lux".to_string()),
        ]);
        assert_eq!(votes.get(&10), Some(&"Sona".to_string()));
        assert!(!votes.contains_key(&20));
    }
}
