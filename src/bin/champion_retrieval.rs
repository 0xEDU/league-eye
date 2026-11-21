use anyhow::{Context, Result};
use clap::Parser;
use image::{DynamicImage, imageops::FilterType};
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    query: PathBuf,
    #[arg(long, default_value = "assets/champions")]
    references: PathBuf,
    #[arg(long, default_value_t = 5)]
    top: usize,
}

fn signature(image: DynamicImage) -> Vec<f32> {
    image
        .resize_exact(32, 32, FilterType::Lanczos3)
        .to_rgb8()
        .pixels()
        .flat_map(|pixel| pixel.0.map(|channel| channel as f32 / 255.0))
        .collect()
}

fn distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f32>()
        / left.len() as f32
}

fn main() -> Result<()> {
    let args = Args::parse();
    let query = signature(image::open(&args.query).context("open query image")?);
    let mut matches = Vec::new();
    for entry in fs::read_dir(&args.references).context("read champion references")? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "png") {
            continue;
        }
        let image = image::open(&path).with_context(|| format!("open {}", path.display()))?;
        matches.push((distance(&query, &signature(image)), path));
    }
    matches.sort_by(|left, right| left.0.total_cmp(&right.0));
    for (score, path) in matches.into_iter().take(args.top) {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");
        println!("{name}\t{score:.6}");
    }
    Ok(())
}
