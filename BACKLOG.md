# Detection Backlog

## Current State

- Video sampling, FFmpeg cropping, local Ollama inference, typed JSONL output, resume, and validation work.
- The instruction model produces valid JSON reliably.
- Clock readings from the broad top crop are not reliable enough for use.
- Champion, KDA, CS, health, and mana values are not reliable enough for use.
- Validation preserves raw model values and records implausible or temporally inconsistent values.
- No independently verified labels exist yet, so field accuracy is unknown.

## P0: Establish Measurement

- [ ] Expand the manually verified clock set from 3 to 20 frames.
- [ ] Label 20 representative frames with champion, level, KDA, CS, health, and mana where readable.
- [x] Define exact-match and mean-absolute-error metrics for clock readings.
- [x] Build a reproducible clock evaluation command for labeled frames.

## P1: Clock Detection

- [x] Test tight timer crop candidates at 640×360 resolution.
- [x] Add a clock parser that accepts only `m:ss`, then converts it to seconds.
- [x] Reject backward and implausibly fast clock changes without replacing them.
- [x] Select among multiple crop candidates only when they match a manually verified clock anchor and expected interval.
- [ ] Test upscale factors and JPEG qualities on the expanded 20-frame label set.
- [ ] Compare `605:0:35:25` with other crop candidates on the expanded label set.

## P1: Champion Detection

- [x] Locate the player portrait crop for the tested broadcast layout.
- [x] Build a local cache of Riot Data Dragon default champion icons.
- [x] Prototype pixel-distance and native feature-print retrieval.
- [ ] Obtain skin-specific portrait references or another candidate source; default-icon retrieval was not accurate.
- [ ] Ask the vision model to choose only from retrieved candidates or return null.
- [ ] Require a score margin before accepting a champion result.

## P2: Player HUD Fields

- [ ] Label and map the broadcast-layout regions for level, KDA, CS, health, and mana.
- [ ] Create separate crops and prompts for the mapped regions.
- [ ] Define field-specific numeric ranges and temporal constraints.
- [ ] Compare field-specific crops with full-frame extraction on labeled frames.

## P3: Runtime and Scaling

- [ ] Measure crop-specific latency and correct fields per second.
- [ ] Select the best validated configuration for a 30-frame run.
- [ ] Scale only configurations that pass accuracy and temporal checks.
