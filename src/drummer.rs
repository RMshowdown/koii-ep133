//! Plays the beat on the KO II: a timer thread that sends MIDI notes on each 16th step.
//! The KO II just plays the notes (like tapping pads), so nothing on the device is changed.

use crate::beat::{self, Beat, STEPS};
use crate::ko2::MidiOut;
use eframe::egui;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Loop lengths the owner can pick (bars). The last bar is the fill bar (when fills are on).
pub const LOOP_CHOICES: [u32; 3] = [4, 8, 16];
const STOPPED: u32 = u32::MAX;

#[derive(Clone)]
pub struct Settings {
    pub playing: bool,
    pub bpm: f32,
    pub swing: f32,
    pub fills: bool,
    /// Bars in one loop (one of LOOP_CHOICES).
    pub bars: u32,
    pub beat: Beat,
    /// MIDI note for Kick, Snare, Hat (None = that drum stays quiet).
    pub notes: [Option<u8>; 3],
}

pub struct Drummer {
    settings: Arc<Mutex<Settings>>,
    /// bar * 16 + step now playing, or STOPPED.
    position: Arc<AtomicU32>,
}

impl Drummer {
    pub fn start(out: MidiOut, ctx: egui::Context, settings: Settings) -> Self {
        let settings = Arc::new(Mutex::new(settings));
        let position = Arc::new(AtomicU32::new(STOPPED));
        let (s, p) = (settings.clone(), position.clone());
        std::thread::Builder::new()
            .name("drummer".into())
            .spawn(move || run(out, ctx, s, p))
            .expect("start drummer thread");
        Self { settings, position }
    }

    /// Change settings; the beat thread picks them up on the next step.
    pub fn update(&self, f: impl FnOnce(&mut Settings)) {
        f(&mut self.settings.lock().unwrap_or_else(|e| e.into_inner()));
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// (bar from 0, step 0-15) now playing.
    pub fn position(&self) -> Option<(u32, usize)> {
        match self.position.load(Ordering::Relaxed) {
            STOPPED => None,
            p => Some((p / STEPS as u32, p as usize % STEPS)),
        }
    }
}

fn run(out: MidiOut, ctx: egui::Context, settings: Arc<Mutex<Settings>>, position: Arc<AtomicU32>) {
    let mut was_playing = false;
    let mut step = 0usize;
    let mut bar = 0u32;
    let mut next = Instant::now();
    let mut sounding: Vec<u8> = Vec::new();

    loop {
        let s = settings.lock().unwrap_or_else(|e| e.into_inner()).clone();

        if !s.playing {
            if was_playing {
                for note in sounding.drain(..) {
                    out.send(&[0x80, note, 0]);
                }
                position.store(STOPPED, Ordering::Relaxed);
                ctx.request_repaint();
                was_playing = false;
            }
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        if !was_playing {
            step = 0;
            bar = 0;
            next = Instant::now();
            was_playing = true;
        }

        let now = Instant::now();
        if next > now {
            // Short naps so Stop and tempo changes react quickly.
            std::thread::sleep((next - now).min(Duration::from_millis(2)));
            continue;
        }
        if now - next > Duration::from_millis(200) {
            next = now; // we fell far behind (PC was busy): don't rush to catch up
        }

        let bars = s.bars.max(1);
        if bar >= bars {
            bar = 0; // loop got shorter while playing
        }
        let use_fill = s.fills && bar == bars - 1;
        let pattern = if use_fill { &s.beat.fill } else { &s.beat.main };
        for (drum, note) in s.notes.iter().enumerate() {
            let (Some(note), velocity) = (*note, pattern[drum][step]) else { continue };
            if velocity == 0 {
                continue;
            }
            if sounding.contains(&note) {
                out.send(&[0x80, note, 0]);
            } else {
                sounding.push(note);
            }
            out.send(&[0x90, note, velocity]);
        }
        position.store(bar * STEPS as u32 + step as u32, Ordering::Relaxed);
        ctx.request_repaint();

        let (on, off) = beat::step_lengths(s.bpm, s.swing);
        next += Duration::from_secs_f64(if step % 2 == 0 { on } else { off });
        step += 1;
        if step == STEPS {
            step = 0;
            bar = (bar + 1) % bars;
        }
    }
}
