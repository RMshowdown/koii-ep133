//! "How do I…?" answers: button steps for the KO II.
//!
//! Every step comes from the official TE guide (https://teenage.engineering/guides/ep-133),
//! sections noted in `source`. Do not add steps from memory or from other sites:
//! check the guide first. Wording is ours, kept short.

/// KO II buttons and controls we can light up on the drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    GroupA,
    GroupB,
    GroupC,
    GroupD,
    Shift,
    Erase,
    Record,
    Play,
    Main,
    Sound,
    Tempo,
    Timing,
    Keys,
    Fx,
    Sample,
    Fader,
    /// The fader slider itself (not the FADER button).
    FaderSlider,
    Minus,
    Plus,
    KnobX,
    KnobY,
    /// Any of the 12 pads.
    Pads,
    /// The pad labelled ENTER.
    Enter,
}

pub struct Step {
    /// Big line: what to do now.
    pub title: &'static str,
    /// Small line: extra detail.
    pub detail: &'static str,
    /// Buttons to light up on the drawing.
    pub keys: &'static [Key],
}

pub struct HowTo {
    pub title: &'static str,
    /// Extra search words (English and Thai).
    pub words: &'static str,
    /// Guide section this comes from.
    pub source: &'static str,
    pub steps: &'static [Step],
}

const fn step(title: &'static str, detail: &'static str, keys: &'static [Key]) -> Step {
    Step { title, detail, keys }
}

use Key::*;

/// The first 8 are the "Most used" list.
pub const MOST_USED: usize = 8;

pub static HOW_TOS: &[HowTo] = &[
    HowTo {
        title: "Record a beat",
        words: "record live count-in อัด บันทึก อัดเสียง อัดบีท",
        source: "9.1 live record",
        steps: &[
            step("Press RECORD, then PLAY", "You hear a four-beat count-in, then recording starts. KO II must be stopped first.", &[Record, Play]),
            step("Hit the pads", "Play your beat on the pads. Every hit is recorded.", &[Pads]),
            step("Press PLAY to stop", "Or press RECORD to keep the beat going but stop recording.", &[Play]),
        ],
    },
    HowTo {
        title: "Copy a pattern",
        words: "copy bar pattern duplicate คัดลอก ก๊อป ก็อป ทำซ้ำ",
        source: "8.3 main, 9.5 copy / paste",
        steps: &[
            step("Go to MAIN", "Press MAIN if you are in another mode.", &[Main]),
            step("Press SHIFT + C once to copy a bar", "Press it twice to copy the whole pattern.", &[Shift, GroupC]),
            step("Paste with SHIFT + D", "Go where you want it, then press SHIFT + D.", &[Shift, GroupD]),
        ],
    },
    HowTo {
        title: "Paste",
        words: "paste วาง แปะ",
        source: "8.3 main, 9.5 copy / paste",
        steps: &[step("Press SHIFT + D", "Pastes what you copied with SHIFT + C.", &[Shift, GroupD])],
    },
    HowTo {
        title: "Undo a mistake",
        words: "undo mistake back เลิกทำ ย้อน ย้อนกลับ ผิด",
        source: "8.3 main, 9.6 erase / undo",
        steps: &[step(
            "Press SHIFT + B",
            "When an undo is possible, the umbrella icon on the screen lights up.",
            &[Shift, GroupB],
        )],
    },
    HowTo {
        title: "Erase notes on a pad",
        words: "erase delete remove note pad clear ลบ ลบโน้ต ลบเสียง",
        source: "9.2 step sequence, 9.6 erase / undo",
        steps: &[
            step("While playing: hold ERASE, then the pad", "Notes on that pad are removed as the pattern runs.", &[Erase, Pads]),
            step("Stopped: hold ERASE + the pad until TRK blinks", "This erases all notes of that pad in the pattern.", &[Erase, Pads]),
        ],
    },
    HowTo {
        title: "Change pattern length",
        words: "pattern length longer bars bar ยาว ความยาว ห้อง เพิ่มห้อง",
        source: "9.1 live record",
        steps: &[
            step("Press RECORD, then − or +", "Patterns start at 1 bar. Up to 99 bars per group.", &[Record, Minus, Plus]),
            step("While playing: hold RECORD, press − or +", "Changes the length on the fly.", &[Record, Minus, Plus]),
        ],
    },
    HowTo {
        title: "Change tempo",
        words: "tempo bpm speed faster slower เทมโป ความเร็ว ช้า เร็ว",
        source: "8.4 tempo",
        steps: &[
            step("Press TEMPO", "Enters tempo mode. Tap TEMPO in time to tap a tempo.", &[Tempo]),
            step("Turn KNOB X", "The knob goes from 60 to 180 BPM.", &[KnobX]),
            step("Or hold TEMPO and type the BPM on the pads", "Typing allows 40 to 399 BPM. Use the . pad for decimals.", &[Tempo, Pads]),
        ],
    },
    HowTo {
        title: "Edit a sound",
        words: "edit sound pitch volume amp trim envelope แก้เสียง ปรับเสียง",
        source: "8.2 sound edit",
        steps: &[
            step("Pick the pad, then press SHIFT + SOUND", "Opens sound edit for that pad's sound.", &[Shift, Sound, Pads]),
            step("Use − and + to change the edit page", "There are 6 pages: sound, trim, envelope, time, midi, mute group.", &[Minus, Plus]),
            step("Turn KNOB X and KNOB Y", "Each page shows what the knobs change.", &[KnobX, KnobY]),
            step("Hold SHIFT + SOUND for two seconds to save", "Edits stay in this project only.", &[Shift, Sound]),
        ],
    },
    // ---- more (found by search) ----
    HowTo {
        title: "Record without count-in",
        words: "record no count in อัด ไม่นับ",
        source: "9.1 live record",
        steps: &[step("Press RECORD and PLAY together", "Recording starts right away.", &[Record, Play])],
    },
    HowTo {
        title: "Add notes while playing (overdub)",
        words: "overdub add record over เพิ่มโน้ต อัดทับ",
        source: "9.1 live record",
        steps: &[
            step("Press PLAY", "Start the pattern.", &[Play]),
            step("Hold RECORD and hit the pads", "Notes are added to the beat.", &[Record, Pads]),
        ],
    },
    HowTo {
        title: "Put a note on a step",
        words: "step sequence record step โน้ต ช่อง สเต็ป",
        source: "9.2 step sequence",
        steps: &[
            step("Stop, then use − and + to pick a step", "Hold SHIFT + − to go back to step 1.1.1.", &[Minus, Plus, Shift]),
            step("Hold RECORD and press a pad", "That pad is recorded to this step and lights up.", &[Record, Pads]),
        ],
    },
    HowTo {
        title: "Change swing",
        words: "swing groove shuffle สวิง กรู๊ฟ",
        source: "7.2 sequencing a beat, step 10",
        steps: &[step(
            "Press TIMING and turn KNOB Y",
            "Swing applies when you record, not to notes already recorded.",
            &[Timing, KnobY],
        )],
    },
    HowTo {
        title: "Repeat a note (note repeat)",
        words: "note repeat hat roll รัว ซ้ำ",
        source: "7.2 sequencing a beat, step 11",
        steps: &[step(
            "Hold TIMING and press a pad",
            "The pad repeats at the note interval. Great for hi-hats.",
            &[Timing, Pads],
        )],
    },
    HowTo {
        title: "Change project",
        words: "project switch load new project โปรเจกต์ เปลี่ยนโปรเจกต์",
        source: "8.3 main, 7.1 new project",
        steps: &[step(
            "Hold MAIN and press a pad 1-9",
            "Projects 6-9 are empty out of the box.",
            &[Main, Pads],
        )],
    },
    HowTo {
        title: "Save the scene (commit)",
        words: "commit save scene บันทึก เซฟ ซีน",
        source: "8.3 main",
        steps: &[step(
            "Press SHIFT + MAIN",
            "Saves the current patterns as a scene and makes a copy to keep working on.",
            &[Shift, Main],
        )],
    },
    HowTo {
        title: "Start a new empty pattern",
        words: "new pattern empty แพทเทิร์นใหม่",
        source: "8.3 main",
        steps: &[step("Press SHIFT + A", "Finds the next empty pattern.", &[Shift, GroupA])],
    },
    HowTo {
        title: "Change the sound on a pad",
        words: "load sample change sound pad เปลี่ยนเสียง ใส่เสียง",
        source: "8.1 sound",
        steps: &[
            step("Press SOUND, pick a group and a pad", "Sound mode.", &[Sound, Pads]),
            step("Press − or + to change the sound", "SHIFT + − / + jumps by ten. Kicks 1-99, snares 100-199, hats 200-299.", &[Minus, Plus]),
        ],
    },
    HowTo {
        title: "Play a sound like a keyboard",
        words: "keys keyboard melody scale bass คีย์ เมโลดี้ เบส สเกล",
        source: "9.3 keys, 12.2 change the scale",
        steps: &[
            step("Pick a pad, then press KEYS", "The pads become a 12-note keyboard of that sound.", &[Pads, Keys]),
            step("Hold KEYS and press − or + for the octave", "", &[Keys, Minus, Plus]),
            step("Hold KEYS and turn KNOB X / KNOB Y", "KNOB X changes the key, KNOB Y the scale.", &[Keys, KnobX, KnobY]),
        ],
    },
    HowTo {
        title: "Add an effect",
        words: "fx effect reverb delay เอฟเฟกต์ เสียงก้อง",
        source: "7.1 add some fx",
        steps: &[
            step("Press FX", "Each group can be sent to one master effect.", &[Fx]),
            step("Move the fader up, use − and + to pick the effect", "", &[FaderSlider, Minus, Plus]),
        ],
    },
    HowTo {
        title: "Punch-in effects while playing",
        words: "punch in fx perform เอฟเฟกต์ เล่นสด",
        source: "7.1 punch-in fx",
        steps: &[
            step("Press PLAY", "", &[Play]),
            step("Hold FX and press the pads", "Each pad is a different effect.", &[Fx, Pads]),
        ],
    },
    HowTo {
        title: "Erase a pattern",
        words: "erase pattern clear ลบแพทเทิร์น",
        source: "9.6 erase / undo",
        steps: &[step(
            "Hold ERASE + the group button until PTN blinks",
            "Erases the current pattern of that group.",
            &[Erase, GroupA, GroupB, GroupC, GroupD],
        )],
    },
    HowTo {
        title: "Record a sample with the mic",
        words: "sample mic record voice สุ่มเสียง อัดเสียง ไมค์",
        source: "7.1 let's sample",
        steps: &[
            step("Press a group (D is a good empty one), then SAMPLE", "The pads flash: they are ready to record.", &[GroupD, Sample]),
            step("Hold a pad and make a sound", "Recording stops when you let go. Press the pad to hear it.", &[Pads]),
        ],
    },
    HowTo {
        title: "Open system settings",
        words: "settings system midi clock ตั้งค่า",
        source: "9.6 erase / undo (system settings)",
        steps: &[step(
            "Press SHIFT + ERASE",
            "Use − / + and ENTER to move, or type a code on the pads and press ENTER.",
            &[Shift, Erase, Minus, Plus, Enter],
        )],
    },
];

/// Search titles and words. Every word typed must match (any order). Empty = nothing.
pub fn search(query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(|w| w.to_lowercase()).collect();
    if words.is_empty() {
        return Vec::new();
    }
    HOW_TOS
        .iter()
        .enumerate()
        .filter(|(_, h)| {
            let hay = format!("{} {}", h.title, h.words).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_english_and_thai() {
        let copy = HOW_TOS.iter().position(|h| h.title == "Copy a pattern").unwrap();
        assert!(search("copy").contains(&copy));
        assert!(search("ก๊อป").contains(&copy));
        let erase = HOW_TOS.iter().position(|h| h.title == "Erase notes on a pad").unwrap();
        assert!(search("ลบ").contains(&erase));
        assert!(search("TEMPO")[0] == HOW_TOS.iter().position(|h| h.title == "Change tempo").unwrap());
        assert!(search("   ").is_empty());
        assert!(search("zzzz").is_empty());
    }

    #[test]
    fn every_how_to_has_steps_and_source() {
        assert!(HOW_TOS.len() >= MOST_USED);
        for h in HOW_TOS {
            assert!(!h.steps.is_empty(), "{}", h.title);
            assert!(!h.source.is_empty(), "{}", h.title);
            for s in h.steps {
                assert!(!s.keys.is_empty(), "{}: {}", h.title, s.title);
            }
        }
    }
}
