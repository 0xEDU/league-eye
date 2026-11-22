# League-eye Context

## Core Idea

League-eye is descriptive League of Legends analytics.

It reconstructs what happened in a game, when it happened, and the game state around each event.

Its primary output is a factual timeline:

`timestamp → state snapshot → event or action → resulting state`

Examples:

- `04:12 — Qiyana clears raptors; 36 CS; allied bot wave is pushed in.`
- `05:03 — Qiyana moves toward river.`
- `05:18 — Dragon vision is established.`
- `05:42 — First dragon is taken.`

## Scope

League-eye states the what and when. It does not infer hidden player intent or claim why a player acted.

Structured replay or client data is the preferred source of truth. Video is optional context and a fallback source when structured data is unavailable.

Pixel extraction is not the product goal. It is only useful when it supports a factual timeline.
