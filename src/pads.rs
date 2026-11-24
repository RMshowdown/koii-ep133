//! The KO II pads: layout, MIDI notes, and the saved pad map.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const GROUPS: [&str; 4] = ["A", "B", "C", "D"];

/// Pad labels in screen order, top-left first (same as the KO II keypad).
/// This order is also the SysEx pad index minus 1 (community docs: index 1-12
/// counts from the top left: 7 8 9 / 4 5 6 / 1 2 3 / . 0 ENTER).
pub const PAD_LABELS: [&str; 12] = ["7", "8", "9", "4", "5", "6", "1", "2", "3", ".", "0", "ENTER"];

/// Note offset inside a group for each pad (same order as PAD_LABELS).
/// Notes run from the bottom-left pad "." upward (community docs, live-tested on OS 2.5.1).
const NOTE_OFFSET: [u8; 12] = [9, 10, 11, 6, 7, 8, 3, 4, 5, 0, 1, 2];

/// MIDI note for a pad. Group note ranges come from the official TE guide:
/// A 36-47, B 48-59, C 60-71, D 72-83.
pub fn midi_note(group: usize, pad: usize) -> u8 {
    36 + 12 * group as u8 + NOTE_OFFSET[pad]
}

/// SysEx file id of a pad node: 2000 + 1000*project + 200 + 100*group + index (1-12).
pub fn pad_file_id(project: u8, group: usize, pad: usize) -> u16 {
    2000 + 1000 * project as u16 + 200 + 100 * group as u16 + pad as u16 + 1
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PadInfo {
    /// Sound name shown on the pad. Empty = unknown.
    #[serde(default)]
    pub name: String,
    /// Sample slot (1-999) read from the KO II, if known.
    #[serde(default)]
    pub slot: Option<u16>,
}

/// A pad: group (0 = A) and index in PAD_LABELS order.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PadRef {
    pub group: usize,
    pub pad: usize,
}

impl PadRef {
    pub fn note(self) -> u8 {
        midi_note(self.group, self.pad)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PadMap {
    /// KO II project (1-9) these pads were read from.
    #[serde(default)]
    pub project: Option<u8>,
    /// 4 groups x 12 pads, in PAD_LABELS order.
    #[serde(default)]
    pub groups: Vec<Vec<PadInfo>>,
    /// Pads the drummer uses for Kick, Snare, Hat.
    #[serde(default)]
    pub drums: [Option<PadRef>; 3],
}

impl Default for PadMap {
    fn default() -> Self {
        Self {
            project: None,
            groups: vec![vec![PadInfo::default(); 12]; 4],
            drums: [None; 3],
        }
    }
}

/// Words in sound names for Kick, Snare, Hat (best match first).
const DRUM_WORDS: [&[&str]; 3] = [
    &["kick", "bd", "bass drum"],
    &["snare", "clap", "sd", "rim"],
    &["closed hat", "hat", "hh", "shaker"],
];

/// Factory sample slots per kind (community docs: kick 1-99, snare 100-199, hat 200-299).
const DRUM_SLOTS: [std::ops::RangeInclusive<u16>; 3] = [1..=99, 100..=199, 200..=299];

/// Pads used when we know nothing about the sounds: ".", "2", "5" in group A.
const FALLBACK_PADS: [usize; 3] = [9, 7, 4];

impl PadMap {
    /// Pick Kick / Snare / Hat pads from sound names (then slot numbers),
    /// searching group A first. Keeps picks the user already made.
    pub fn auto_pick_drums(&mut self) {
        for drum in 0..3 {
            if self.drums[drum].is_some() {
                continue;
            }
            self.drums[drum] = self.find_drum(drum).or(Some(PadRef { group: 0, pad: FALLBACK_PADS[drum] }));
        }
    }

    fn find_drum(&self, drum: usize) -> Option<PadRef> {
        let all = || {
            (0..4).flat_map(|group| (0..12).map(move |pad| PadRef { group, pad }))
        };
        for word in DRUM_WORDS[drum] {
            if let Some(found) = all().find(|r| self.pad(r.group, r.pad).name.to_lowercase().contains(word)) {
                return Some(found);
            }
        }
        all().find(|r| {
            self.pad(r.group, r.pad)
                .slot
                .is_some_and(|s| DRUM_SLOTS[drum].contains(&s))
        })
    }

    pub fn pad(&self, group: usize, pad: usize) -> &PadInfo {
        &self.groups[group][pad]
    }

    pub fn pad_mut(&mut self, group: usize, pad: usize) -> &mut PadInfo {
        &mut self.groups[group][pad]
    }

    /// Make sure there are always 4 groups of 12 pads (old or hand-edited files).
    fn fix_shape(&mut self) {
        self.groups.resize(4, vec![PadInfo::default(); 12]);
        for group in &mut self.groups {
            group.resize(12, PadInfo::default());
        }
    }

    /// `%APPDATA%\koii-ep133\pad-map.json`
    fn file_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("koii-ep133").join("pad-map.json"))
    }

    pub fn load() -> Self {
        let mut map = Self::file_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<PadMap>(&text).ok())
            .unwrap_or_default();
        map.fix_shape();
        map
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path().ok_or("APPDATA folder not found")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_match_docs() {
        // Group A: "." = 36, "0" = 37, "ENTER" = 38, "1" = 39, "9" = 47.
        assert_eq!(midi_note(0, 9), 36);
        assert_eq!(midi_note(0, 10), 37);
        assert_eq!(midi_note(0, 11), 38);
        assert_eq!(midi_note(0, 6), 39);
        assert_eq!(midi_note(0, 2), 47);
        // Group D ends at 83.
        assert_eq!(midi_note(3, 2), 83);
    }

    #[test]
    fn pad_file_ids_match_docs() {
        // Project 1, group A, pad "." = 3210 (kmorrill midi-reference.md).
        assert_eq!(pad_file_id(1, 0, 9), 3210);
        // Project 1, group A, top-left pad "7" = 3201.
        assert_eq!(pad_file_id(1, 0, 0), 3201);
        // Project 8, group D, pad "6" = 9506 (krate protocol-evidence.md).
        assert_eq!(pad_file_id(8, 3, 5), 10506);
        assert_eq!(pad_file_id(7, 3, 5), 9506);
    }

    #[test]
    fn auto_pick_uses_names_then_fallback() {
        let mut map = PadMap::default();
        map.pad_mut(0, 9).name = "kick grit".into();
        map.pad_mut(0, 11).name = "clap real".into();
        map.pad_mut(0, 7).name = "snare rim".into();
        map.pad_mut(0, 4).name = "closed hat lo".into();
        map.pad_mut(0, 5).name = "open hat real".into();
        map.auto_pick_drums();
        assert_eq!(map.drums[0], Some(PadRef { group: 0, pad: 9 }));
        assert_eq!(map.drums[1], Some(PadRef { group: 0, pad: 7 })); // "snare" beats "clap"
        assert_eq!(map.drums[2], Some(PadRef { group: 0, pad: 4 })); // closed hat first

        let mut empty = PadMap::default();
        empty.auto_pick_drums();
        assert_eq!(empty.drums[0], Some(PadRef { group: 0, pad: 9 }));
    }

    #[test]
    fn json_round_trip_and_shape_fix() {
        let mut map = PadMap::default();
        map.project = Some(1);
        map.pad_mut(0, 9).name = "kick".into();
        map.pad_mut(0, 9).slot = Some(1);
        let text = serde_json::to_string(&map).unwrap();
        let back: PadMap = serde_json::from_str(&text).unwrap();
        assert_eq!(back, map);

        let mut short: PadMap = serde_json::from_str(r#"{"groups":[[{"name":"x"}]]}"#).unwrap();
        short.fix_shape();
        assert_eq!(short.groups.len(), 4);
        assert!(short.groups.iter().all(|g| g.len() == 12));
        assert_eq!(short.pad(0, 0).name, "x");
    }
}
