# TODO

Short list. One line each. Done work lives in README (Phases) and git history.

## Decide next (owner)

- [ ] Phase 4 bass + melody: pick a way
  - A. "Like drums": app taps 1 bass pad + 1 chord pad in rhythm. Real KO II sounds, no setup, no new library. One note per pad; key match is by ear.
  - B. "USB audio": app makes bass + melody sound itself and sends it into the KO II over USB (Windows sees `Speakers (EP-133)`). Real melodies, locked key. Needs new library `cpal`. Sound is made by the app, not KO II samples.
  - C. App writes pad pitch to the KO II (SysEx METADATA SET). Breaks our read-only rule; needs a backup first. Not recommended.
- [ ] Or skip to Phase 5 first (Keep / Skip, library, MIDI file export). No device limits, no new library.

## Later phases

- [ ] Phase 5: Keep / Skip, song library (name, play, delete), export `.mid`
- [ ] Phase 6: you play, the app fills in the rest
- [ ] Phase 7: make it pretty (owner: looks need a lot of work). Done so far: Settings window (7 themes, text size), card layout, fonts. Left: Settings choice for Button Guide side (left/right)
- [ ] Phase 8 (optional, risky): write songs into a KO II project, backup first

## Polish list (owner said: looks and details need a lot more work)

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
