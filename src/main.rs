mod inference;
mod persistence;
mod schema;
mod video;
use anyhow::Result;
use clap::Parser;
use persistence::Checkpoint;
use reqwest::Client;
use schema::Observation;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "probe-footage.mp4")]
    input: PathBuf,
    #[arg(long, default_value = "results")]
    output: PathBuf,
    #[arg(long, default_value_t = 10)]
    interval: u64,
    #[arg(long, default_value = "qwen3-vl:2b-instruct-q4_K_M")]
    model: String,
    #[arg(long, default_value = "http://localhost:11434")]
    ollama: String,
    #[arg(long)]
    resume: bool,
    #[arg(long)]
    limit: Option<u64>,
    #[arg(long)]
    max_width: Option<u32>,
    #[arg(long, default_value = "lanczos")]
    resize_filter: String,
    #[arg(long)]
    crop: Option<String>,
    #[arg(long, default_value_t = 3)]
    jpeg_quality: u8,
    #[arg(long, default_value_t = 1)]
    burst_frames: u32,
    #[arg(long, default_value_t = 0)]
    burst_radius_ms: u64,
    #[arg(long, default_value = "sharpest")]
    burst_strategy: String,
    #[arg(long, default_value_t = 512)]
    max_tokens: u16,
    #[arg(long, default_value_t = 0)]
    start_seconds: u64,
    #[arg(long, default_value = "full")]
    prompt_profile: String,
    #[arg(long)]
    champion_candidates: Option<String>,
    #[arg(long)]
    clock_candidates: Option<String>,
    #[arg(long, default_value = "full-frame-v1")]
    experiment: String,
}
fn fingerprint(p: &Path) -> Result<String> {
    let mut h = Sha256::new();
    let mut file = File::open(p)?;
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        h.update(&buffer[..count]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
#[tokio::main]
async fn main() -> Result<()> {
    let a = Args::parse();
    anyhow::ensure!(
        a.input.is_file(),
        "input video does not exist: {}",
        a.input.display()
    );
    fs::create_dir_all(&a.output)?;
    anyhow::ensure!(
        a.resume || !a.output.join("observations.jsonl").exists(),
        "output already contains observations; use --resume or choose a new output directory"
    );
    let _run_lock = persistence::acquire_run_lock(&a.output)?;
    let meta = video::metadata(&a.input)?;
    fs::write(
        a.output.join("video.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    let duration = video::duration_seconds(&meta)?;
    let source = fingerprint(&a.input)?;
    anyhow::ensure!(
        a.burst_frames > 0 && a.burst_frames % 2 == 1,
        "burst frames must be a positive odd number"
    );
    let cfg = serde_json::json!({"interval":a.interval,"crop":a.crop,"max_width":a.max_width,"resize_filter":a.resize_filter,"jpeg_quality":a.jpeg_quality,"burst_frames":a.burst_frames,"burst_radius_ms":a.burst_radius_ms,"burst_strategy":a.burst_strategy,"max_tokens":a.max_tokens,"start_seconds":a.start_seconds,"prompt_profile":a.prompt_profile,"champion_candidates":a.champion_candidates,"clock_candidates":a.clock_candidates,"experiment":a.experiment});
    let config = format!("{:x}", Sha256::digest(serde_json::to_vec(&cfg)?));
    let client = Client::new();
    inference::check_model(&client, &a.ollama, &a.model).await?;
    let done = if a.resume {
        if let Some(old) = persistence::load_checkpoint(&a.output)? {
            anyhow::ensure!(
                old.source_fingerprint == source,
                "resume source fingerprint differs from checkpoint"
            );
            anyhow::ensure!(
                old.config_fingerprint == config,
                "resume configuration differs from checkpoint"
            );
            anyhow::ensure!(old.model == a.model, "resume model differs from checkpoint");
        }
        persistence::completed(&a.output, &source, &config)?
    } else {
        Default::default()
    };
    let mut cp = Checkpoint {
        source_fingerprint: source.clone(),
        model: a.model.clone(),
        config_fingerprint: config.clone(),
        ..Default::default()
    };
    let mut attempted = 0;
    let started = Instant::now();
    for sec in (a.start_seconds..duration.ceil() as u64).step_by(a.interval as usize) {
        if a.limit.is_some_and(|n| attempted >= n) {
            break;
        };
        let ms = sec * 1000;
        let id = format!("{}-{}-{}", &source[..16], &config[..16], ms);
        if done.contains(&id) {
            continue;
        };
        attempted += 1;
        let result = async {
            let prep = Instant::now();
            let jpg = video::burst_jpeg(
                &a.input,
                ms,
                a.crop.as_deref(),
                a.max_width,
                &a.resize_filter,
                a.jpeg_quality,
                a.burst_frames,
                a.burst_radius_ms,
                &a.burst_strategy,
            )?;
            let preprocess = prep.elapsed().as_millis();
            let began = Instant::now();
            let x = inference::extract(
                &client,
                &a.ollama,
                &a.model,
                &jpg,
                a.max_tokens,
                &a.prompt_profile,
                a.champion_candidates.as_deref(),
                a.clock_candidates.as_deref(),
            )
            .await?;
            let latency = began.elapsed().as_millis();
            let mut errors = schema::validate(&x);
            if let Some(previous) = persistence::latest_observation(&a.output, &source, &config)? {
                errors.extend(schema::validate_temporal(
                    &previous.extraction,
                    &x,
                    ms.saturating_sub(previous.video_timestamp_ms),
                ));
            }
            let o = Observation {
                video_fingerprint: source.clone(),
                frame_id: id.clone(),
                video_timestamp_ms: ms,
                experiment_id: a.experiment.clone(),
                config_fingerprint: config.clone(),
                model: a.model.clone(),
                extraction: x,
                validation_errors: errors,
                inference_latency_ms: latency,
            };
            persistence::append(&a.output.join("observations.jsonl"), &o)?;
            Ok::<_, anyhow::Error>((preprocess, latency))
        }
        .await;
        match result {
            Ok((_p, _i)) => cp.completed += 1,
            Err(e) => {
                cp.failed += 1;
                persistence::append(
                    &a.output.join("errors.jsonl"),
                    &serde_json::json!({"frame_id":id,"timestamp_ms":ms,"error":format!("{e:#}"),"at_unix_ms":now()}),
                )?;
            }
        };
        cp.last_frame_id = Some(id);
        cp.updated_unix_ms = now();
        persistence::checkpoint(&a.output, &cp)?;
        let elapsed = started.elapsed().as_secs_f64();
        fs::write(
            a.output.join("progress.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"experiment":a.experiment,"attempted":attempted,"completed":cp.completed,"failed":cp.failed,"current_timestamp_ms":ms,"elapsed_seconds":elapsed,"frames_per_minute":if elapsed>0.0{attempted as f64/elapsed*60.0}else{0.0}}),
            )?,
        )?;
    }
    Ok(())
}
