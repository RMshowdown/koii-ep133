# TODO

**Project paused (2026-11-25).** The owner stopped here: the next phases need a lot of music detail (keys, bass lines, tempo sync) and the owner is not a musician. Everything below is kept so work can start again later. The app works as it is: beats, KO II panel, Button Guide, Settings.

Short list. One line each. Done work lives in README (Phases) and git history.

## Where we stopped

- Found 2026-11-25: the KO II DOES record the app's MIDI notes into a pattern (group A selected, RECORD + PLAY).
- Open issue: the owner said the recorded drums "sound different" when the KO II plays them back. Not solved. Clues from `midi-log.txt`:
  - Playback steps were about 0.43 s apart (about 70 BPM, the KO II's own tempo); the app played at 78 BPM. Likely cause: tempo mismatch.
  - On playback the KO II sent every hit as channel 1 note 60, so the log cannot show which pad each hit landed on.
  - Not yet asked: is it (a) rhythm/feel off, (b) every hit the same sound, or (c) louder/softer?
- Next idea if restarted: the app sends MIDI clock so the KO II follows the app's tempo and Play/Stop (KO II: turn on clock in once in the MIDI settings; check the TE guide for the exact steps first).

## Decide next (owner, when restarting)

- [ ] Phase 4 bass + melody: pick a way
  - A. "Like drums": app taps 1 bass pad + 1 chord pad in rhythm. Real KO II sounds, no setup, no new library. One note per pad; key match is by ear.
  - B. "USB audio": app makes bass + melody sound itself and sends it into the KO II over USB (Windows sees `Speakers (EP-133)`). Real melodies, locked key. Needs new library `cpal`. Sound is made by the app, not KO II samples.
  - C. App writes pad pitch to the KO II (SysEx METADATA SET). Breaks our read-only rule; needs a backup first. Not recommended.
  - D. Owner's idea: user picks a sound and presses KEYS, the app plays a bass/melody pattern, the KO II records it (recording now known to work; needs tempo sync first).
- [ ] Or skip to Phase 5 first (Keep / Skip, library, MIDI file export). No device limits, no new library.

## Later phases

- [ ] Phase 5: Keep / Skip, song library (name, play, delete), export `.mid`
- [ ] Phase 6: you play, the app fills in the rest
- [ ] Phase 7: make it pretty (owner: looks need a lot of work). Done so far: Settings window (7 themes, text size), card layout, fonts. Left: Settings choice for Button Guide side (left/right)
- [ ] Phase 8 (optional, risky): write songs into a KO II project, backup first

## Polish list (owner said: looks and details need a lot more work)

- [ ] Show a short note in the app: "KEYS on the KO II makes the drums play one sound" (offered, owner did not answer)
- [ ] Owner to check: pads light up and A-D follow when pressed on the KO II; pad name changes when its sound is changed on the KO II
- [ ] "Check for update" only sees versions TE lists on its what's-new page (it lists "os 2.5", not 2.5.1): small fix releases may be missed
- [ ] Sound names are kept only until the app closes: save them so "Sound 124" shows a real name after a restart
- [ ] Ask the owner for their list of look and detail changes before Phase 7
- [ ] Hide the black console window (release build already hides it)
- [ ] Help card can fall behind other apps when another window is clicked (needs Windows "owner window"; may need `windows-sys`, ask first)
- [ ] Card may lag a little when the main window is dragged fast
- [ ] Check Large / Extra large text at the smallest window size (pads scroll at Extra large in the start-size window)
- [ ] Owner to try all 7 themes and say which need colour fixes
- [ ] Test on the KO II: Settings "Open EP Sample Tool" lets go, the TE page connects, then close the page and Reconnect
- [ ] Play with no sound at first launch: seen once by the owner, cause unknown (maybe two copies of the app open). Watch for it.
