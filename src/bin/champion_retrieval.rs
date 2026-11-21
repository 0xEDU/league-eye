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

fn png_files(directory: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).with_context(|| format!("read {}", directory.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(png_files(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "png") {
            files.push(path);
        }
    }
    Ok(files)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let query = signature(image::open(&args.query).context("open query image")?);
    let mut matches = Vec::new();
    for path in png_files(&args.references)? {
        let image = image::open(&path).with_context(|| format!("open {}", path.display()))?;
        matches.push((distance(&query, &signature(image)), path));
    }
    matches.sort_by(|left, right| left.0.total_cmp(&right.0));
    for (score, path) in matches.into_iter().take(args.top) {
        let name = path
            .strip_prefix(&args.references)
            .unwrap_or(&path)
            .display();
        println!("{name}\t{score:.6}");
    }
    Ok(())
}
