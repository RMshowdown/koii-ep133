//! Lo-fi / hip-hop beat maker. Pure logic: no MIDI here.
//!
//! A beat is 16 steps (one bar of 16th notes) for 3 drums, plus a "fill" bar
//! that replaces the last bar of every 4-bar loop.
//! Patterns are built from hand-picked boom-bap building blocks so they always
//! sound like a real groove, then lightly varied (ghost notes, hat accents).

pub const STEPS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drum {
    Kick = 0,
    Snare = 1,
    Hat = 2,
}

pub const DRUMS: [Drum; 3] = [Drum::Kick, Drum::Snare, Drum::Hat];

impl Drum {
    pub fn name(self) -> &'static str {
        match self {
            Drum::Kick => "Kick",
            Drum::Snare => "Snare",
            Drum::Hat => "Hat",
        }
    }
}

/// Velocity per step for each drum (0 = no hit). Index: [drum][step].
pub type Bar = [[u8; STEPS]; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct Beat {
    pub main: Bar,
    pub fill: Bar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    LofiChill,
    BoomBap,
    Jazzy,
}

pub const STYLES: [Style; 3] = [Style::LofiChill, Style::BoomBap, Style::Jazzy];

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::LofiChill => "Lo-fi chill",
            Style::BoomBap => "Boom bap",
            Style::Jazzy => "Jazzy",
        }
    }

    /// Starting tempo (BPM) and swing (%) when this style is picked.
    pub fn defaults(self) -> (f32, f32) {
        match self {
            Style::LofiChill => (78.0, 58.0),
            Style::BoomBap => (90.0, 55.0),
            Style::Jazzy => (84.0, 62.0),
        }
    }
}

/// Turn "x...X..." into velocities: x = normal hit, X = accent, g = ghost.
fn parse(pattern: &str, normal: u8) -> [u8; STEPS] {
    let mut out = [0; STEPS];
    for (i, c) in pattern.chars().take(STEPS).enumerate() {
        out[i] = match c {
            'X' => 120,
            'x' => normal,
            'g' => 45,
            _ => 0,
        };
    }
    out
}

fn pick<'a>(options: &[&'a str]) -> &'a str {
    options[rand::random_range(0..options.len())]
}

/// Small random change in loudness so it feels played by a person.
fn humanize(v: u8) -> u8 {
    if v == 0 {
        return 0;
    }
    let delta: i16 = rand::random_range(-8..=8);
    (v as i16 + delta).clamp(20, 127) as u8
}

pub fn new_beat(style: Style) -> Beat {
    // Kick shapes: beat 1 always, then classic boom-bap pushes.
    let kick = match style {
        Style::LofiChill => pick(&[
            "x.........x.....",
            "x......x..x.....",
            "x.........x..x..",
            "x.....x...x.....",
        ]),
        Style::BoomBap => pick(&[
            "x.....x...x.....",
            "x..x......x..x..",
            "x......xx.x.....",
            "x.x.......x..x..",
        ]),
        Style::Jazzy => pick(&[
            "x.........x.....",
            "x.......x.......",
            "x.....x.....x...",
        ]),
    };
    // Snare on 2 and 4 (steps 4 and 12). Ghost notes come later.
    let snare = "....x.......x...";
    let hat = match style {
        Style::LofiChill => pick(&["x.x.x.x.x.x.x.x.", "x.x.x.x.x.x.x.xx", "X.x.X.x.X.x.X.x."]),
        Style::BoomBap => pick(&["x.x.x.x.x.x.x.x.", "xxx.x.xxx.x.x.xx", "x.xxx.x.x.xxx.x."]),
        Style::Jazzy => pick(&["x..x.xx..x.xx..x", "x.xxx.xxx.xxx.xx"]),
    };

    let mut main: Bar = [parse(kick, 105), parse(snare, 110), parse(hat, 70)];

    // Ghost snares: quiet hits just before or after the backbeat.
    let ghost_chance = match style {
        Style::LofiChill => 0.35,
        Style::BoomBap => 0.45,
        Style::Jazzy => 0.6,
    };
    for step in [7, 9, 15] {
        if main[Drum::Snare as usize][step] == 0 && rand::random_bool(ghost_chance) {
            main[Drum::Snare as usize][step] = 45;
        }
    }

    for row in main.iter_mut() {
        for v in row.iter_mut() {
            *v = humanize(*v);
        }
    }

    Beat { fill: make_fill(&main), main }
}

/// Same groove, but the last beat turns into a snare roll and the hat drops out.
fn make_fill(main: &Bar) -> Bar {
    let mut fill = *main;
    let roll = pick(&["............gxxX", "............x.xX", "............xgxx", "...........g.xXx"]);
    let roll = parse(roll, 95);
    for step in 12..STEPS {
        fill[Drum::Snare as usize][step] = roll[step];
        if step >= 13 {
            fill[Drum::Hat as usize][step] = 0;
        }
    }
    // Keep the backbeat on step 12 if the roll does not hit there.
    if fill[Drum::Snare as usize][12] == 0 {
        fill[Drum::Snare as usize][12] = main[Drum::Snare as usize][12];
    }
    fill
}

/// When each step starts inside a pair of 16ths, as a share of 2 steps.
/// swing 50 = straight, 66 = triplet feel. Returns (on-beat length, off-beat length) in seconds.
pub fn step_lengths(bpm: f32, swing_percent: f32) -> (f64, f64) {
    let sixteenth = 60.0 / bpm.max(1.0) as f64 / 4.0;
    let s = (swing_percent.clamp(50.0, 75.0) / 100.0) as f64;
    (2.0 * sixteenth * s, 2.0 * sixteenth * (1.0 - s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_beat_has_kick_on_one_and_backbeat() {
        for style in STYLES {
            for _ in 0..50 {
                let b = new_beat(style);
                assert!(b.main[Drum::Kick as usize][0] > 0);
                assert!(b.main[Drum::Snare as usize][4] > 0);
                assert!(b.main[Drum::Snare as usize][12] > 0);
                assert!(b.fill[Drum::Snare as usize][12] > 0);
                // Fill keeps the first 3 beats.
                for d in 0..3 {
                    assert_eq!(b.main[d][..12], b.fill[d][..12]);
                }
            }
        }
    }

    #[test]
    fn swing_keeps_the_tempo() {
        let (on, off) = step_lengths(90.0, 60.0);
        let pair = 2.0 * 60.0 / 90.0 / 4.0;
        assert!((on + off - pair).abs() < 1e-9);
        assert!(on > off);
        let (on, off) = step_lengths(90.0, 50.0);
        assert!((on - off).abs() < 1e-9);
    }
}
