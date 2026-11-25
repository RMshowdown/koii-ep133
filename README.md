# KO II Buddy

A friendly Windows app for the Teenage Engineering EP-133 K.O. II.
It plays lo-fi / hip-hop beats on the KO II over USB-C MIDI, follows what you press on the KO II,
and shows step-by-step button help for people new to the device. (It was called "KO II Lo-fi Drummer".) Written in Rust (egui window).

Not affiliated with Teenage Engineering.

The side card with KO II button steps is called **Button Guide** (it was "How do I…?").

## Run it

```powershell
cargo run
```

Tests: `cargo test`

## Code map

- `src/main.rs`: window, KO II-style main panel (group buttons, pads, knobs, Play), Settings window, Button Guide card placement
- `src/ko2.rs`: USB MIDI thread (pad notes both ways, KO II group events, read-only "Read pads")
- `src/sysex.rs`: TE SysEx framing (packed7, request/reply)
- `src/pads.rs`: pad layout, MIDI notes, saved pad map, drum pad picks
- `src/beat.rs`: lo-fi beat maker (styles, ghost notes, fills, swing timing)
- `src/drummer.rs`: timer thread that plays the beat as MIDI notes
- `src/howto.rs`: Button Guide answers (button steps from the TE guide) and search
- `src/help.rs`: Button Guide card screen and the KO II button drawing
- `src/settings.rs`: saved app settings (theme, text size) in `%APPDATA%\koii-ep133\settings.json`
- `src/theme.rs`: 7 colour themes, app fonts, card/button drawing helpers
- `assets/fonts/`: Familjen Grotesk + Noto Sans Thai (free, SIL Open Font License, licence files next to them)
- `designs/`: HTML mock-ups the themes came from (not used by the app)

## Phases

| Phase | What | Status |
|---|---|---|
| 0 | Project setup, empty window, size limits (min 960x620, start 960x730, max 1600x1000), "How do I…?" card shell: a separate borderless window docked to the right edge (left if no room), Winamp-style; main window never resizes | done |
| 1 | Connect to KO II over USB-C, "Read pads from KO II" (read-only), tap a pad to hear it, remember the pad map (`%APPDATA%\koii-ep133\pad-map.json`) | done (tested on KO II, OS 2.5.1) |
| 2 | Lo-fi / hip-hop drummer: Play/Stop, Tempo, Swing, Style, New beat, fill on bar 4, 16-step view; drum pads picked from sound names | done (tested on KO II) |
| 3 | "How do I…?" card: search by goal (Thai words OK), 8 most-used, 23 answers from the TE guide, step-by-step with buttons lit on a KO II drawing laid out like the real device, link to Joey's cheat sheet; Text size setting (4 sizes, card grows with it); beat strip pinned above a bottom status bar | done (MVP) |
| 4 | Bass + melody | on hold: owner picking a way (see TODO.md and "KO II limits" below) |
| 5 | Keep / Skip, song library, export MIDI file | |
| 6 | You play, the app fills in the rest | |
| 7 | Make it pretty; Settings: help panel on left or right | started: Settings window (7 themes, text size), main panel laid out like the KO II, knobs for BPM/Swing, loop length 4/8/16 bars, follows KO II pad and group presses, connection status in the bottom bar, new fonts. Still to do: card side choice, owner's look list |
| 8 | Optional, risky: write songs into a KO II project (backup first) | |

Open tasks and ideas: [TODO.md](TODO.md).

## KO II limits found by testing (owner's KO II, OS 2.5.1, 2026-11-24)

- Incoming MIDI notes 36-83 play pads A-D (official guide). The channel set per pad in sound edit (MIDI page, KNOB X) did NOT route notes: notes on channel 2 still played the normal pad for that note number.
- With KEYS on, incoming notes play the picked pad up and down in pitch, but then every note does that, so the drums stop working.
- No MIDI or known SysEx command presses KEYS (or any other button) for us.
- So drums plus a bass that changes pitch cannot both come from KO II pads over MIDI at the same time.
- Pad reads (METADATA GET `sym`) worked on this firmware: 36 of 48 pads read in project 2.
- To follow the KO II, the app listens to the notes it sends when pads are pressed, and sends FILE INIT (read-only) on connect so the KO II reports the picked group (`active` on the project's groups folder). Not yet tested on the owner's KO II (2026-11-25).

## Rules for this project

- Owner is not a programmer: explain in simple Thai, one step at a time.
- Ask before adding libraries or tools. Current libraries: `eframe` (window), `midir` (MIDI), `rand`, `serde` + `serde_json` (save settings). Fonts are files, not libraries.
- Never guess KO II facts. Check the official guide first: https://teenage.engineering/guides/ep-133
- Talking to the KO II beyond normal MIDI notes uses an undocumented SysEx protocol.
  Read-only messages only (METADATA GET / list). Never write, upload or delete on the device unless the owner agrees for that exact step.
  Community docs (MIT): https://github.com/kmorrill/ep-series-sysex and https://github.com/icherniukh/ep133-krate (PROTOCOL.md). Owner's firmware: 2.5.1.
- Do not copy code from AGPL projects (for example ep133-export-to-daw).
- Button help text comes from the official TE guide. Link to https://ep133.joeyh.org/ for the full cheat sheet; do not copy its content.
- App text is English, short and plain. No Thai on app screens; Thai words still work in the Button Guide search.
- Never commit or push; suggest a commit message instead (`type(scope): summary`, no Co-Authored-By line).
