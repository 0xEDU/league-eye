# League-eye Context

## Core Idea

League-eye is video-first, descriptive League of Legends analytics.

It reconstructs what happened in a game, when it happened, and the game state around each event.

Its primary output is a factual timeline:

`timestamp → state snapshot → event or action → resulting state`

Examples:

- `04:12 — Qiyana clears raptors; 36 CS; allied bot wave is pushed in.`
- `05:03 — Qiyana moves toward river.`
- `05:18 — Dragon vision is established.`
- `05:42 — First dragon is taken.`

## Scope

League-eye analyzes footage to state the what and when. It does not infer hidden player intent or claim why a player acted.

Video is the primary source. It provides the state, events, and timing used to reconstruct the factual timeline.

Structured replay or client data can validate, enrich, or compare against the video timeline when available. It is not required for the core product.

Pixel extraction is a core capability because it turns video evidence into timeline data. It should be designed around the factual timeline rather than treated as a general-purpose HUD-reading exercise.
