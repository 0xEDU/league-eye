mod inference;
mod persistence;
mod video;

use anyhow::Result;
use clap::Parser;
use league_vision::{
    domain::{EvidenceKind, EvidenceRecord, FrameReference, TIMELINE_SCHEMA_VERSION},
    report, timeline,
};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(name = "league-eye")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Analyze(AnalyzeArgs),
}

#[derive(Parser)]
struct AnalyzeArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 2)]
    sample_interval_seconds: u64,
    #[arg(long, default_value_t = 10.0)]
    scene_change_threshold: f64,
    #[arg(long)]
    limit: Option<u64>,
    #[arg(long, default_value = "qwen3-vl:2b-instruct-q4_K_M")]
    model: String,
    #[arg(long, default_value = "http://localhost:11434")]
    ollama: String,
    #[arg(long)]
    resume: bool,
}

fn fingerprint(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut source = fs::File::open(path)?;
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn scene_change_score(previous: &[u8], current: &[u8]) -> Result<f64> {
    let previous = image::load_from_memory(previous)?
        .resize_exact(96, 54, image::imageops::FilterType::Triangle)
        .to_luma8();
    let current = image::load_from_memory(current)?
        .resize_exact(96, 54, image::imageops::FilterType::Triangle)
        .to_luma8();
    Ok(previous
        .pixels()
        .zip(current.pixels())
        .map(|(left, right)| left[0].abs_diff(right[0]) as f64)
        .sum::<f64>()
        / (96.0 * 54.0))
}

fn event_kind(kind: &str) -> EvidenceKind {
    match kind {
        "death" => EvidenceKind::Death,
        "recall" => EvidenceKind::Recall,
        "objective" => EvidenceKind::Objective,
        "fight" => EvidenceKind::Fight,
        _ => EvidenceKind::Unknown,
    }
}

async fn analyze(args: AnalyzeArgs) -> Result<()> {
    anyhow::ensure!(
        args.input.is_file(),
        "input video does not exist: {}",
        args.input.display()
    );
    fs::create_dir_all(&args.output)?;
    let evidence_path = args.output.join("evidence.jsonl");
    anyhow::ensure!(
        args.resume || !evidence_path.exists(),
        "output contains evidence; use --resume or choose a new output directory"
    );
    let _run_lock = persistence::acquire_run_lock(&args.output)?;
    let completed_timestamps = if args.resume && evidence_path.exists() {
        fs::read_to_string(&evidence_path)?
            .lines()
            .filter_map(|line| serde_json::from_str::<EvidenceRecord>(line).ok())
            .map(|record| record.frame.video_timestamp_ms)
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };
    let metadata = video::metadata(&args.input)?;
    let duration = video::duration_seconds(&metadata)?;
    let video_fingerprint = fingerprint(&args.input)?;
    let run = serde_json::json!({"schema_version":TIMELINE_SCHEMA_VERSION,"video_fingerprint":video_fingerprint,"input":args.input,"sample_interval_seconds":args.sample_interval_seconds,"scene_change_threshold":args.scene_change_threshold,"model":args.model});
    fs::write(
        args.output.join("run.json"),
        serde_json::to_vec_pretty(&run)?,
    )?;
    let client = Client::new();
    inference::check_model(&client, &args.ollama, &args.model).await?;
    let mut previous_frame = None;
    let mut evidence = Vec::new();
    let errors_path = args.output.join("errors.jsonl");
    let checkpoint_path = args.output.join("checkpoint.json");
    let mut attempted = 0_u64;
    let mut failed = 0_u64;
    for second in (0..duration.ceil() as u64).step_by(args.sample_interval_seconds as usize) {
        if args.limit.is_some_and(|limit| attempted >= limit) {
            break;
        }
        let timestamp_ms = second * 1_000;
        if completed_timestamps.contains(&timestamp_ms) {
            continue;
        }
        attempted += 1;
        let frame = match video::frame_jpeg(
            &args.input,
            timestamp_ms,
            None,
            Some(960),
            "lanczos",
            3,
        ) {
            Ok(frame) => frame,
            Err(error) => {
                failed += 1;
                persistence::append_jsonl(
                    &errors_path,
                    &serde_json::json!({"timestamp_ms":timestamp_ms,"error":format!("{error:#}")}),
                )?;
                continue;
            }
        };
        let Some(previous_frame) = previous_frame.replace(frame.clone()) else {
            continue;
        };
        let score = scene_change_score(&previous_frame, &frame)?;
        if score < args.scene_change_threshold {
            continue;
        }
        let classification = match inference::classify_event(
            &client,
            &args.ollama,
            &args.model,
            &frame,
        )
        .await
        {
            Ok(classification) => classification,
            Err(error) => {
                failed += 1;
                persistence::append_jsonl(
                    &errors_path,
                    &serde_json::json!({"timestamp_ms":timestamp_ms,"error":format!("{error:#}")}),
                )?;
                continue;
            }
        };
        let observed_kind = event_kind(&classification.kind);
        let evidence_id = format!("evidence-{:05}", evidence.len() + 1);
        let mut validation_notes = vec![format!("scene change score: {score:.2}")];
        if observed_kind == EvidenceKind::Unknown {
            validation_notes.push("vision did not prove a target event".into());
        }
        let record = EvidenceRecord {
            schema_version: TIMELINE_SCHEMA_VERSION,
            evidence_id,
            video_fingerprint: video_fingerprint.clone(),
            detector: "scene-change-plus-vision".into(),
            detector_version: "v1".into(),
            frame: FrameReference {
                video_timestamp_ms: timestamp_ms,
                crop: None,
            },
            observed_kind,
            confidence: classification.confidence,
            factual_summary: classification.summary,
            validation_notes,
        };
        persistence::append_jsonl(&evidence_path, &record)?;
        evidence.push(record);
        fs::write(
            &checkpoint_path,
            serde_json::to_vec_pretty(
                &serde_json::json!({"attempted":attempted,"failed":failed,"last_timestamp_ms":timestamp_ms}),
            )?,
        )?;
    }
    let events = timeline::build(&evidence, args.sample_interval_seconds * 1_500);
    let timeline_path = args.output.join("timeline.jsonl");
    for event in &events {
        persistence::append_jsonl(&timeline_path, event)?;
    }
    fs::write(args.output.join("timeline.md"), report::markdown(&events))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Command::Analyze(args) => analyze(args).await,
    }
}
