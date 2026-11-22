use anyhow::{Context, Result};
use clap::Parser;
use image::{DynamicImage, imageops::FilterType};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    labels: PathBuf,
    #[arg(long)]
    frames: PathBuf,
    #[arg(long, default_value_t = 33)]
    x: u32,
    #[arg(long, default_value_t = 2)]
    y: u32,
    #[arg(long, default_value_t = 5)]
    width: u32,
    #[arg(long, default_value_t = 8)]
    height: u32,
    #[arg(long, default_value_t = 7)]
    second_offset: u32,
    #[arg(long, default_value_t = 16)]
    third_offset: u32,
    #[arg(long, default_value_t = 22)]
    fourth_offset: u32,
    #[arg(long, default_value_t = 0.0)]
    threshold: f32,
}

#[derive(Deserialize)]
struct Label {
    video_timestamp_ms: u64,
    game_time_seconds: u32,
}

fn signature(image: DynamicImage, threshold: f32) -> Vec<f32> {
    let mut values: Vec<f32> = image
        .resize_exact(60, 40, FilterType::Lanczos3)
        .to_luma8()
        .pixels()
        .map(|pixel| pixel[0] as f32 / 255.0)
        .collect();
    let minimum = values.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (maximum - minimum).max(f32::EPSILON);
    for value in &mut values {
        *value = (*value - minimum) / span;
        if threshold > 0.0 {
            *value = f32::from(*value >= threshold);
        }
    }
    values
}

fn digits(seconds: u32) -> [u8; 4] {
    let text = format!("{:02}{:02}", seconds / 60, seconds % 60);
    let bytes = text.into_bytes();
    [
        bytes[0] - b'0',
        bytes[1] - b'0',
        bytes[2] - b'0',
        bytes[3] - b'0',
    ]
}

fn digit_signature(image: &DynamicImage, position: usize, args: &Args) -> Vec<f32> {
    let x = args.x + [0, args.second_offset, args.third_offset, args.fourth_offset][position];
    signature(
        image.crop_imm(x, args.y, args.width, args.height),
        args.threshold,
    )
}

fn distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum::<f32>()
        / left.len() as f32
}

fn main() -> Result<()> {
    let args = Args::parse();
    let labels: Vec<Label> = fs::read_to_string(&args.labels)
        .with_context(|| format!("read {}", args.labels.display()))?
        .lines()
        .map(serde_json::from_str)
        .collect::<serde_json::Result<_>>()?;
    let images = labels
        .iter()
        .map(|label| {
            let path = args
                .frames
                .join(format!("{}.png", label.video_timestamp_ms));
            Ok((
                label,
                image::open(&path).with_context(|| format!("open {}", path.display()))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut exact = 0_u32;
    let mut error = 0_u64;
    for (index, (label, image)) in images.iter().enumerate() {
        let mut output = [0_u8; 4];
        for (position, digit) in output.iter_mut().enumerate() {
            let current = digit_signature(image, position, &args);
            let (_, candidate) = images
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .min_by(|(_, left), (_, right)| {
                    distance(&current, &digit_signature(&left.1, position, &args)).total_cmp(
                        &distance(&current, &digit_signature(&right.1, position, &args)),
                    )
                })
                .expect("at least two labels");
            *digit = digits(candidate.0.game_time_seconds)[position];
        }
        let value = u32::from(output[0]) * 600
            + u32::from(output[1]) * 60
            + u32::from(output[2]) * 10
            + u32::from(output[3]);
        exact += u32::from(label.game_time_seconds == value);
        error += label.game_time_seconds.abs_diff(value) as u64;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "labeled_frames": labels.len(),
            "exact_matches": exact,
            "exact_match_rate": exact as f64 / labels.len() as f64,
            "mean_absolute_error_seconds": error as f64 / labels.len() as f64
        }))?
    );
    Ok(())
}
