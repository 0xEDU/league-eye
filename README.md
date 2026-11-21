# League Vision

League Vision samples a local MP4 with FFmpeg and sends each frame to a local Ollama vision model. It writes one durable JSON record per completed frame.

## Requirements

- Rust 1.89 or later
- `ffmpeg` and `ffprobe` on `PATH`
- Ollama running at `http://localhost:11434`
- The local `qwen3-vl:2b-instruct-q4_K_M` model

Start Ollama, then install the model only when it is absent:

```sh
ollama serve
ollama pull qwen3-vl:2b-instruct-q4_K_M
```

## Run

Place the source video at `probe-footage.mp4`, then run:

```sh
cargo run --release -- --input probe-footage.mp4 --output results
```

The default interval is 10 seconds. Use `--limit 10` for an initial test. Use `--max-width 1280` to benchmark a resized full frame. Use `--crop x:y:width:height` only for a separate crop experiment.

## Resume

```sh
cargo run --release -- --input probe-footage.mp4 --output results --resume
```

`observations.jsonl` is the source of truth. The program flushes each record before atomically replacing `checkpoint.json`.

## Evaluate clock experiments

```sh
cargo run --release --bin evaluate_clock -- \
  --labels labels/clock-smoke.jsonl \
  --observations results/clock-text-x605/observations.jsonl
```

## Select clock candidates

Use a manually verified clock reading as the seed. The selector preserves every crop candidate and returns null when no candidate fits the expected next clock range.

```sh
cargo run --release --bin select_clock_candidates -- \
  --observations results/clock-text-x605-v2/observations.jsonl \
  --output results/clock-ensemble.jsonl \
  --seed-timestamp-ms 90000 \
  --seed-seconds 132
```

## Results

- `video.json`: FFprobe metadata
- `observations.jsonl`: successful typed model observations
- `errors.jsonl`: per-frame failures
- `checkpoint.json`: restart position and counters
- `progress.json`: current run rate and counts

Stop with Ctrl-C. Completed records remain available. The pipeline stores only one JPEG frame in memory at a time and does not retain sampled images.
