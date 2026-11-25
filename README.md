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

- `src/main.rs`: window, beat "screen" at the top, KO II-style panel with every button (VOL = beat volume, X = BPM, Y = swing, -/+ = tempo, PLAY = play/stop, A-D, pads; other buttons open the Button Guide for that button), Settings window, Button Guide card placement
- `src/ko2.rs`: USB MIDI thread (pad notes both ways, KO II group events, read-only "Read pads"); writes everything the KO II sends to `%APPDATA%\koii-ep133\midi-log.txt`
- `src/sysex.rs`: TE SysEx framing (packed7, request/reply)
- `src/pads.rs`: pad layout, MIDI notes, saved pad map, drum pad picks
- `src/beat.rs`: lo-fi beat maker (styles, ghost notes, fills, swing timing)
- `src/drummer.rs`: timer thread that plays the beat as MIDI notes
- `src/howto.rs`: Button Guide answers (button steps from the TE guide) and search
- `src/help.rs`: Button Guide card screen and the KO II button drawing
- `src/settings.rs`: saved app settings (theme, text size) in `%APPDATA%\koii-ep133\settings.json`
- `src/update.rs`: "Check for update": reads TE's public what's-new page and compares the newest OS with the KO II's OS; links to TE's update tool and EP Sample Tool
- `src/theme.rs`: 7 colour themes, app fonts, card/button drawing helpers
- `assets/fonts/`: Familjen Grotesk + Noto Sans Thai (free, SIL Open Font License, licence files next to them)
- `designs/`: HTML mock-ups the themes came from (not used by the app)

## Phases

| Phase | What | Status |
|---|---|---|
| 0 | Project setup, empty window, size limits (min 960x620, start 960x780, max 1600x1000), "How do I…?" card shell: a separate borderless window docked to the right edge (left if no room), Winamp-style; main window never resizes | done |
| 1 | Connect to KO II over USB-C, "Read pads from KO II" (read-only), tap a pad to hear it, remember the pad map (`%APPDATA%\koii-ep133\pad-map.json`) | done (tested on KO II, OS 2.5.1) |
| 2 | Lo-fi / hip-hop drummer: Play/Stop, Tempo, Swing, Style, New beat, fill on bar 4, 16-step view; drum pads picked from sound names | done (tested on KO II) |
| 3 | "How do I…?" card: search by goal (Thai words OK), 8 most-used, 23 answers from the TE guide, step-by-step with buttons lit on a KO II drawing laid out like the real device, link to Joey's cheat sheet; Text size setting (4 sizes, card grows with it); beat strip pinned above a bottom status bar | done (MVP) |
| 4 | Bass + melody | on hold: owner picking a way (see TODO.md and "KO II limits" below) |
| 5 | Keep / Skip, song library, export MIDI file | |
| 6 | You play, the app fills in the rest | |
| 7 | Make it pretty; Settings: help panel on left or right | started: Settings window (7 themes, text size), beat screen on top, full KO II panel (every button, lit when pressed), knobs for VOL/BPM/Swing, loop length 4/8/16 bars, follows KO II pad and group presses, connection status in the bottom bar, new fonts, Settings "KO II" part (OS version, check for update, links to TE update tool and EP Sample Tool, format steps). Still to do: card side choice, owner's look list |
| 8 | Optional, risky: write songs into a KO II project (backup first) | |

Open tasks and ideas: [TODO.md](TODO.md).

## KO II limits found by testing (owner's KO II, OS 2.5.1, 2026-11-24)

- Incoming MIDI notes 36-83 play pads A-D (official guide). The channel set per pad in sound edit (MIDI page, KNOB X) did NOT route notes: notes on channel 2 still played the normal pad for that note number.
- With KEYS on, incoming notes play the picked pad up and down in pitch, but then every note does that, so the drums stop working.
- No MIDI or known SysEx command presses KEYS (or any other button) for us.
- So drums plus a bass that changes pitch cannot both come from KO II pads over MIDI at the same time.
- Pad reads (METADATA GET `sym`) worked on this firmware: 36 of 48 pads read in project 2.
- What the KO II reports by itself (tested 2026-11-25, after a read-only FILE INIT on connect; log in `midi-log.txt`):
  - Pad press: note on/off on channel 1 (notes 36-83 by group) plus a metadata event `active` = pad file id.
  - Group A-D press: metadata event on the project's groups folder (e.g. file 4100 `active` 4200-4500).
  - Changing the sound on a pad (sound edit): metadata event `sym` = new sample slot, and `sample.end`.
  - VOL / X / Y knobs, SOUND, MAIN, TEMPO, KEYS, FADER, SHIFT, SAMPLE, TIMING, FX, ERASE, -, +, RECORD: nothing is sent, so the app cannot follow them. Twice an "all sound off" (channel 2, CC 120) came in, button unknown.

## Rules for this project

- Owner is not a programmer: explain in simple Thai, one step at a time.
- Ask before adding libraries or tools. Current libraries: `eframe` (window), `midir` (MIDI), `rand`, `serde` + `serde_json` (save settings), `ureq` (reads one TE web page for "Check for update"). Fonts are files, not libraries.
- Never guess KO II facts. Check the official guide first: https://teenage.engineering/guides/ep-133
- Talking to the KO II beyond normal MIDI notes uses an undocumented SysEx protocol.
  Read-only messages only (METADATA GET / list). Never write, upload or delete on the device unless the owner agrees for that exact step.
  Community docs (MIT): https://github.com/kmorrill/ep-series-sysex and https://github.com/icherniukh/ep133-krate (PROTOCOL.md). Owner's firmware: 2.5.1.
- Do not copy code from AGPL projects (for example ep133-export-to-daw).
- Button help text comes from the official TE guide. Link to https://ep133.joeyh.org/ for the full cheat sheet; do not copy its content.
- App text is English, short and plain. No Thai on app screens; Thai words still work in the Button Guide search.
- Internet: only "Check for update" reads https://teenage.engineering/guides/ep-133/whats-new. Firmware update, backup, restore and format are done with TE's own tools or on the device; the app only links to them and shows the steps.
- Never commit or push; suggest a commit message instead (`type(scope): summary`, no Co-Authored-By line).
