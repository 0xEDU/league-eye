# Detection Backlog

## Current State

- Video sampling, FFmpeg cropping, local Ollama inference, typed JSONL output, resume, and validation work.
- The instruction model produces valid JSON reliably.
- Clock readings from the broad top crop are not reliable enough for use.
- Champion, KDA, CS, health, and mana values are not reliable enough for use.
- Validation preserves raw model values and records implausible or temporally inconsistent values.
- No independently verified labels exist yet, so field accuracy is unknown.

## P0: Establish Measurement

- [ ] Label 20 representative frames with game clock, champion, level, KDA, CS, health, and mana where readable.
- [ ] Define field-level accuracy, null correctness, and hallucination metrics.
- [ ] Build a reproducible evaluation command that runs the same labeled frames for every experiment.

## P1: Clock Detection

- [ ] Test the tight timer crop `590:0:50:30` at the native 640×360 resolution.
- [ ] Test modest upscale factors and JPEG qualities on the same labeled frames.
- [ ] Add a clock parser that accepts only `m:ss`, then converts it to seconds.
- [ ] Reject impossible or temporally inconsistent clock readings without replacing them.

## P1: Champion Detection

- [ ] Locate and validate the player portrait crop across representative frames.
- [ ] Build a local champion portrait reference set.
- [ ] Retrieve the closest champion candidates from the portrait crop.
- [ ] Ask the vision model to choose only from the retrieved candidates or return null.
- [ ] Require a score margin before accepting a champion result.

## P2: Player HUD Fields

- [ ] Create separate crops and prompts for level, KDA, CS, health, and mana.
- [ ] Define field-specific numeric ranges and temporal constraints.
- [ ] Compare field-specific crops with full-frame extraction on labeled frames.

## P3: Runtime and Scaling

- [ ] Measure crop-specific latency and correct fields per second.
- [ ] Select the best validated configuration for a 30-frame run.
- [ ] Scale only configurations that pass accuracy and temporal checks.
