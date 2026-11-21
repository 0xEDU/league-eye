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

For the tested 640×360 broadcast layout, the evaluated timer reader uses a tight top-right crop:

```sh
cargo run --release -- \
  --input probe-footage.mp4 \
  --output results/clock \
  --crop 605:0:35:25 \
  --max-width 1200 \
  --max-tokens 64 \
  --prompt-profile clock-text \
  --experiment clock-text-v1
```

The health and mana experiment uses a separate bottom-HUD crop:

```sh
cargo run --release -- \
  --input probe-footage.mp4 \
  --output results/resources \
  --crop 180:310:260:50 \
  --max-width 1080 \
  --max-tokens 128 \
  --prompt-profile resource-hud \
  --experiment resource-hud-v1
```

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

## Reject inconsistent champion names

Run independent portrait crops. This selector preserves every model candidate and emits a name only when one name receives the required number of votes without a tie. Repeated records from one crop count once. Conflicting retries from one crop do not vote.

```sh
cargo run --release --bin select_champion_candidates -- \
  --observations results/portrait-a/observations.jsonl results/portrait-b/observations.jsonl results/portrait-c/observations.jsonl \
  --output results/champion-consensus.jsonl \
  --min-votes 2
```

## Summarize an experiment

```sh
cargo run --release --bin summarize_observations -- \
  --observations results/clock-text-x605-v2/observations.jsonl \
  --progress results/clock-text-x605-v2/progress.json
```

## Results

- `video.json`: FFprobe metadata
- `observations.jsonl`: successful typed model observations
- `errors.jsonl`: per-frame failures
- `checkpoint.json`: restart position and counters
- `progress.json`: current run rate and counts

Stop with Ctrl-C. Completed records remain available. The pipeline stores only one JPEG frame in memory at a time and does not retain sampled images.
