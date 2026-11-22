# League-eye

League-eye is a video-first League of Legends timeline analyzer. It turns a local MP4 into factual, confidence-scored event evidence and a readable timeline.

It reports what is visible and when it is visible. It does not infer player intent.

## Requirements

- Rust 1.89 or later
- `ffmpeg` and `ffprobe` on `PATH`
- Ollama at `http://localhost:11434`
- `qwen3-vl:2b-instruct-q4_K_M` installed locally

```sh
ollama pull qwen3-vl:2b-instruct-q4_K_M
```

## Analyze a video

```sh
cargo run --release -- analyze \
  --input match.mp4 \
  --output run
```

League-eye samples the video, uses scene changes to propose windows, and uses local vision to classify visible death, recall, objective, and fight evidence.

## Output

- `run.json`: input identity and analysis configuration.
- `evidence.jsonl`: append-only visual evidence records.
- `timeline.jsonl`: ordered, merged event candidates.
- `timeline.md`: readable event timeline.
- `errors.jsonl`: recoverable sampling or vision failures.
- `checkpoint.json`: latest completed evidence record.

Each event includes confidence, confirmation state, and source video timestamps. Frame images are regenerated from the MP4 when needed.

## Resume

```sh
cargo run --release -- analyze \
  --input match.mp4 \
  --output run \
  --resume
```

## Development tools

The other Cargo binaries are experiments for HUD extraction, clock evaluation, portrait retrieval, and diagnostics. They are not part of the MVP analysis path.
