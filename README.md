# KO II Lo-fi Drummer

A friendly Windows app for the Teenage Engineering EP-133 K.O. II.
It plays beats on the KO II over USB-C MIDI, and shows step-by-step button help
for people new to the device. Written in Rust (egui window).

Not affiliated with Teenage Engineering.

## Run it

```powershell
cargo run
```

Tests: `cargo test`

## Code map

- `src/main.rs`: window, pad grid, "How do I…?" card
- `src/ko2.rs`: USB MIDI thread (pad notes, read-only "Read pads")
- `src/sysex.rs`: TE SysEx framing (packed7, request/reply)
- `src/pads.rs`: pad layout, MIDI notes, saved pad map

## Phases

| Phase | What | Status |
|---|---|---|
| 0 | Project setup, empty window, size limits (min 960x620, start 1200x760, max 1600x1000), "How do I…?" card shell: a separate borderless window docked to the right edge (left if no room), Winamp-style; main window never resizes | done |
| 1 | Connect to KO II over USB-C, "Read pads from KO II" (read-only), tap a pad to hear it, remember the pad map (`%APPDATA%\koii-ep133\pad-map.json`) | done (tested on KO II, OS 2.5.1) |
| 2 | Lo-fi / hip-hop drummer: Play/Stop, Tempo, Swing, New beat, fills, 16-step view | next |
| 3 | "How do I…?" right-side panel: search by goal, step-by-step with buttons lit on a KO II drawing | |
| 4 | Bass + melody locked to one key | |
| 5 | Keep / Skip, song library, export MIDI file | |
| 6 | You play, the app fills in the rest | |
| 7 | Make it pretty; Settings: help panel on left or right | |
| 8 | Optional, risky: write songs into a KO II project (backup first) | |

## Rules for this project

- Owner is not a programmer: explain in simple Thai, one step at a time.
- Ask before adding libraries or tools. Current libraries: `eframe` (window), `midir` (MIDI), `rand`, `serde` + `serde_json` (save settings).
- Never guess KO II facts. Check the official guide first: https://teenage.engineering/guides/ep-133
- Talking to the KO II beyond normal MIDI notes uses an undocumented SysEx protocol.
  Read-only messages only (METADATA GET / list). Never write, upload or delete on the device unless the owner agrees for that exact step.
  Community docs (MIT): https://github.com/kmorrill/ep-series-sysex and https://github.com/icherniukh/ep133-krate (PROTOCOL.md). Owner's firmware: 2.5.1.
- Do not copy code from AGPL projects (for example ep133-export-to-daw).
- Button help text comes from the official TE guide. Link to https://ep133.joeyh.org/ for the full cheat sheet; do not copy its content.
- App text is English, short and plain.
- Never commit or push; suggest a commit message instead (`type(scope): summary`, no Co-Authored-By line).
