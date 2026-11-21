# Detection Backlog

## Current State

- Video sampling, FFmpeg cropping, local Ollama inference, typed JSONL output, resume, and validation work.
- A non-resume run now rejects an output directory with observations, preventing duplicate records after an interrupted run.
- An exclusive output lock prevents two runs from appending to one observation file at the same time.
- The instruction model produces valid JSON reliably.
- Clock readings from the broad top crop are not reliable enough for use.
- Champion, KDA, CS, health, and mana values are not reliable enough for use.
- Validation preserves raw model values and records implausible or temporally inconsistent values.
- The 20-label clock fixture shows crop `605:0:35:25` is exact on 10/20 frames with 69.45 seconds MAE. It is not ready as a standalone reader.
- Anchored selection over four timer crops reaches 14/20 exact with 18/20 coverage and 0.67 seconds MAE on emitted readings.
- Multiple crops can agree on an incorrect clock value, so source votes remain audit data rather than an acceptance rule.
- Lanczos scaling with JPEG quality 1 is exact on the first 7 clean labels. Nearest-neighbor scaling scored 0/7 and is rejected for this reader.
- Five manual portrait labels identify the early player as Aphelios. The multi-crop gate scores 4/5 on this small fixture and remains experimental.
- Skin-aware pixel retrieval still ranks Katarina above the labeled Aphelios portrait, so it is rejected as a champion decision rule.
- KDA, CS, health, and mana accuracy remain unverified.

## P0: Establish Measurement

- [x] Expand the manually verified clock set from 7 to 20 frames.
- [ ] Label 20 representative frames with champion, level, KDA, CS, health, and mana where readable.
- [x] Define exact-match and mean-absolute-error metrics for clock readings.
- [x] Build a reproducible clock evaluation command for labeled frames.

## P1: Clock Detection

- [x] Test tight timer crop candidates at 640×360 resolution.
- [x] Add a clock parser that accepts only `m:ss`, then converts it to seconds.
- [x] Reject backward and implausibly fast clock changes without replacing them.
- [x] Select among multiple crop candidates only when they match a manually verified clock anchor and expected interval.
- [ ] Test upscale factors and JPEG qualities on the expanded 20-frame label set.
- [x] Reject nearest-neighbor scaling on the current verified clock labels.
- [ ] Compare `605:0:35:25` with other crop candidates and anchored selection on the expanded label set.

## P1: Champion Detection

- [x] Locate the player portrait crop for the tested broadcast layout.
- [x] Build a local cache of Riot Data Dragon default champion icons.
- [x] Prototype pixel-distance and native feature-print retrieval.
- [x] Add a local cache path for per-skin round HUD portraits.
- [x] Add a portrait-only VLM profile with a one-field schema.
- [x] Reject portrait names that do not reach a unique multi-crop consensus.
- [x] Verify the apparent Aphelios multi-crop consensus against a manual portrait label.
- [ ] Obtain skin-specific portrait references or another candidate source; default-icon retrieval was not accurate.
- [ ] Ask the vision model to choose only from retrieved candidates or return null.
- [ ] Require a score margin before accepting a champion result.

## P2: Player HUD Fields

- [ ] Label and map the broadcast-layout regions for level, KDA, CS, health, and mana.
- [x] Create a resource-only crop and schema for health and mana.
- [ ] Verify whether resource readings represent current values, maximum values, or concatenated text.
- [ ] Create separate crops and prompts for level, KDA, and CS.
- [ ] Define field-specific numeric ranges and temporal constraints.
- [ ] Compare field-specific crops with full-frame extraction on labeled frames.

## P3: Runtime and Scaling

- [ ] Measure crop-specific latency and correct fields per second.
- [ ] Select the best validated configuration for a 30-frame run.
- [ ] Scale only configurations that pass accuracy and temporal checks.
