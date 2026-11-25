// Hide the black console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod beat;
mod drummer;
mod help;
mod howto;
mod ko2;
mod pads;
mod settings;
mod sysex;
mod theme;

use beat::{DRUMS, STEPS, STYLES, Style};
use drummer::{Drummer, LOOP_CHOICES};
use eframe::egui;
use ko2::{Command, Event, Ko2};
use pads::{GROUPS, PAD_LABELS, PadMap, PadRef};

// Window size limits (width, height in points). Owner can ask to change these.
const START_SIZE: [f32; 2] = [960.0, 780.0];
const MIN_SIZE: [f32; 2] = [960.0, 620.0];
const MAX_SIZE: [f32; 2] = [1600.0, 1000.0];

/// Name of the side card with KO II button steps (top bar button, card title).
pub const GUIDE_NAME: &str = "Button Guide";

// Width of the Button Guide card at normal text size. It grows with the Text size setting.
const HELP_CARD_WIDTH: f32 = 400.0;

/// Card width for the current text size.
fn help_card_width(ctx: &egui::Context) -> f32 {
    let now = ctx.global_style().text_styles[&egui::TextStyle::Body].size;
    let normal = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
    HELP_CARD_WIDTH * now / normal
}
// Windows 10/11 adds an invisible border (left, right, bottom) around normal windows, in points.
const WINDOW_INVISIBLE_BORDER: f32 = 8.0;

/// App name: window title and top bar.
pub const APP_NAME: &str = "KO II Buddy";

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size(START_SIZE)
            .with_min_inner_size(MIN_SIZE)
            .with_max_inner_size(MAX_SIZE),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(KoiiApp::new(&cc.egui_ctx)))),
    )
}

/// USB connection state shown in the top bar.
enum Link {
    NotConnected,
    Connected(String),
    Busy(String),
}

/// Which side of the main window the help card sticks to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum CardSide {
    #[default]
    Right,
    Left,
    /// No room outside (full screen): the card lies on top of the main window's right edge.
    Inside,
}

struct KoiiApp {
    /// Is the Button Guide card showing?
    help_open: bool,
    /// Side chosen when the card opened.
    card_side: CardSide,

    ko2: Ko2,
    link: Link,
    pad_map: PadMap,
    /// Group shown in the pad grid (0 = A).
    group: usize,
    /// Pad picked for the details box (index into PAD_LABELS).
    selected: Option<usize>,
    /// Pad being held down: MIDI note playing.
    held: Option<u8>,
    /// Pads held down on the KO II itself (their MIDI notes), shown lit.
    device_held: Vec<u8>,
    /// Last thing the KO II sent, in words (status bar).
    last_from_device: Option<String>,
    /// Sound names by sample slot, from the last "Read pads".
    sound_names: std::collections::HashMap<u16, String>,
    reading: bool,
    /// Last "Read pads" message for the user.
    read_message: String,
    os_version: Option<String>,

    drummer: Drummer,
    style: Style,
    help: help::HelpState,
    settings: settings::Settings,
    /// Is the Settings window showing?
    settings_open: bool,
    /// Frame number when Settings opened (the opening click must not close it again).
    settings_opened_at: u64,
}

impl KoiiApp {
    fn new(ctx: &egui::Context) -> Self {
        ko2::midi_log::clear();
        theme::set_fonts(ctx);
        let settings = settings::Settings::load();
        theme::apply(ctx, settings.theme);
        settings.apply_text_size(ctx);
        let ko2 = Ko2::start(ctx.clone());
        let mut pad_map = PadMap::load();
        pad_map.auto_pick_drums();
        let style = Style::LofiChill;
        let (bpm, swing) = style.defaults();
        let drummer = Drummer::start(
            ko2.out.clone(),
            ctx.clone(),
            drummer::Settings {
                playing: false,
                bpm,
                swing,
                fills: true,
                bars: LOOP_CHOICES[0],
                volume: 100.0,
                beat: beat::new_beat(style),
                notes: pad_map.drums.map(|d| d.map(PadRef::note)),
            },
        );
        Self {
            help_open: false,
            card_side: CardSide::default(),
            ko2,
            drummer,
            style,
            help: help::HelpState::default(),
            settings,
            settings_open: false,
            settings_opened_at: 0,
            link: Link::NotConnected,
            pad_map,
            group: 0,
            selected: None,
            held: None,
            device_held: Vec::new(),
            last_from_device: None,
            sound_names: std::collections::HashMap::new(),
            reading: false,
            read_message: String::new(),
            os_version: None,
        }
    }

    fn save_pad_map(&mut self) {
        if let Err(e) = self.pad_map.save() {
            self.read_message = format!("Could not save the pad map: {e}");
        }
    }

    /// Handle news from the MIDI thread.
    fn handle_events(&mut self) {
        while let Some(event) = self.ko2.poll() {
            match event {
                Event::Connected(name) => self.link = Link::Connected(name),
                Event::Disconnected => {
                    self.link = Link::NotConnected;
                    self.held = None;
                    self.device_held.clear();
                    if self.reading {
                        self.reading = false;
                        self.read_message = "KO II was unplugged.".into();
                    }
                }
                Event::Busy(msg) => self.link = Link::Busy(msg),
                Event::ReadProgress(msg) => self.read_message = msg,
                Event::ReadFailed(msg) => {
                    self.reading = false;
                    self.read_message = msg;
                }
                Event::ReadDone(result) => {
                    self.reading = false;
                    self.os_version = result.os_version.clone();
                    self.apply_read(result);
                }
                // Pressed on the KO II: follow its group and light the pad.
                Event::DeviceNote { note, on } => {
                    self.device_held.retain(|&n| n != note);
                    if on && let Some(pad) = pads::pad_from_note(note) {
                        self.device_held.push(note);
                        self.group = pad.group;
                        self.selected = Some(pad.pad);
                    }
                }
                Event::DeviceGroup(group) => self.group = group,
                Event::DeviceMessage(text) => self.last_from_device = Some(text),
                Event::DevicePadSound { project, pad, slot } => self.device_pad_sound(project, pad, slot),
            }
        }
    }

    /// The sound on a pad was changed on the KO II: show the new name (if we know it).
    fn device_pad_sound(&mut self, project: u8, pad: PadRef, slot: u16) {
        if self.pad_map.project.is_some_and(|p| p != project) {
            return; // another project than the one we show
        }
        let name = match self.sound_names.get(&slot) {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("Sound {slot}"),
        };
        let info = self.pad_map.pad_mut(pad.group, pad.pad);
        info.slot = Some(slot);
        info.name = name;
        self.save_pad_map();
    }

    fn apply_read(&mut self, result: ko2::ReadResult) {
        let found = result.slots.iter().flatten().filter(|s| s.is_some()).count();
        self.pad_map.project = Some(result.project);
        for (g, group) in result.slots.iter().enumerate() {
            for (p, slot) in group.iter().enumerate() {
                let pad = self.pad_map.pad_mut(g, p);
                pad.slot = *slot;
                if let Some(slot) = slot {
                    pad.name = match result.sound_names.get(slot) {
                        Some(name) if !name.is_empty() => name.clone(),
                        _ => format!("Sound {slot}"),
                    };
                }
            }
        }
        self.sound_names = result.sound_names;
        // Re-pick drums whose pad has no sound in this project.
        for drum in self.pad_map.drums.iter_mut() {
            if drum.is_some_and(|r| self.pad_map.groups[r.group][r.pad].slot.is_none()) {
                *drum = None;
            }
        }
        self.pad_map.auto_pick_drums();
        self.sync_drum_notes();
        self.save_pad_map();
        self.read_message = if found == 0 {
            format!(
                "Read project {}, but the KO II did not say which sound is on each pad. \
                 You can type pad names by hand for now.",
                result.project
            )
        } else {
            format!("Read project {}: {found} of 48 pads have a sound.", result.project)
        };
    }

    fn connected(&self) -> bool {
        matches!(self.link, Link::Connected(_))
    }

    /// Tell the drummer which notes to play for Kick / Snare / Hat.
    fn sync_drum_notes(&self) {
        let notes = self.pad_map.drums.map(|d| d.map(PadRef::note));
        self.drummer.update(|s| s.notes = notes);
    }
}

impl KoiiApp {
    fn toggle_help(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            self.card_side = choose_card_side(ctx);
        }
        self.help_open = !self.help_open;
    }
}

/// Right side if the card fits on screen there, otherwise left,
/// otherwise inside the window (full screen, or no room on either side).
/// (Phase 7 adds a Settings choice for this.)
fn choose_card_side(ctx: &egui::Context) -> CardSide {
    let info = ctx.input(|i| i.viewport().clone());
    if info.maximized == Some(true) || info.fullscreen == Some(true) {
        return CardSide::Inside;
    }
    let (Some(outer), Some(monitor)) = (info.outer_rect, info.monitor_size) else {
        return CardSide::Right;
    };
    let card_w = help_card_width(ctx);
    let right_fits = outer.max.x - WINDOW_INVISIBLE_BORDER + card_w <= monitor.x;
    let left_fits = outer.min.x + WINDOW_INVISIBLE_BORDER - card_w >= 0.0;
    if right_fits {
        CardSide::Right
    } else if left_fits {
        CardSide::Left
    } else {
        CardSide::Inside
    }
}

/// Where the card window goes, glued to the visible edge of the main window.
fn card_rect(main_outer: egui::Rect, side: CardSide, card_w: f32) -> egui::Rect {
    let top = main_outer.min.y;
    let height = main_outer.height() - WINDOW_INVISIBLE_BORDER;
    let left = match side {
        CardSide::Right | CardSide::Inside => main_outer.max.x - WINDOW_INVISIBLE_BORDER,
        CardSide::Left => main_outer.min.x + WINDOW_INVISIBLE_BORDER - card_w,
    };
    egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(card_w, height))
}

impl eframe::App for KoiiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_events();
        theme::paint_background(ui);
        let side_margin = 20;

        // Top bar: app name, Settings, Button Guide.
        let mut toggle_clicked = false;
        egui::Panel::top("top_bar")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(side_margin, 12)))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(APP_NAME);
                    if let Some(project) = self.pad_map.project {
                        ui.add_space(6.0);
                        ui.weak(format!("Project {project}"));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(self.help_open, format!("📖 {GUIDE_NAME}")).clicked() {
                            toggle_clicked = true;
                        }
                        if ui.selectable_label(self.settings_open, "⚙ Settings").clicked() {
                            self.settings_open = !self.settings_open;
                            self.settings_opened_at = ui.ctx().cumulative_pass_nr();
                        }
                    });
                });
            });
        if toggle_clicked {
            self.toggle_help(&ctx);
        }

        // Status bar at the very bottom: connection and other news.
        let status_area = egui::Panel::bottom("status_bar")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(side_margin, 6)))
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));

        // Beat "screen" pinned under the top bar, like the display on the KO II.
        let beat_area = egui::Panel::top("beat_screen")
            .frame(egui::Frame::new().inner_margin(egui::Margin { left: side_margin, right: side_margin, top: 0, bottom: 10 }))
            .show_separator_line(false)
            .show(ui, |ui| {
                theme::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.beat_screen(ui);
                });
            });
        let beat_area_rect = beat_area.response.rect.union(status_area.response.rect);

        // Main area: the KO II-style panel, then the picked pad's details. Scrolls if too small.
        let main_area = egui::CentralPanel::no_frame()
            .frame(egui::Frame::new().inner_margin(egui::Margin { left: side_margin, right: side_margin, top: 0, bottom: 4 }))
            .show(ui, |ui| {
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    theme::card(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.device_panel(ui);
                        ui.add_space(4.0);
                        self.pad_details(ui);
                    });
                });
            });

        let below_top_bar = main_area.response.rect.union(beat_area_rect);
        self.show_help_card(&ctx, below_top_bar);
        self.settings_window(&ctx);
    }
}

impl KoiiApp {
    /// Bottom line: connection (with a dot), project, read news; beat state on the right.
    fn status_bar(&self, ui: &mut egui::Ui) {
        let t = theme::current();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (dot, text, color) = match &self.link {
                Link::Connected(port) => (egui::Color32::from_rgb(80, 190, 120), format!("KO II connected ({port})"), t.soft),
                Link::Busy(_) => (
                    t.warn,
                    "No sound: another app is using the KO II. Close other copies of this app or the TE web tool.".to_string(),
                    t.warn,
                ),
                Link::NotConnected => (t.warn, "KO II not connected: plug it in with USB-C to hear sound".to_string(), t.warn),
            };
            ui.colored_label(dot, "●");
            let resp = ui.colored_label(color, text);
            if let Link::Busy(msg) = &self.link {
                resp.on_hover_text(msg.as_str());
            }
            if let Some(project) = self.pad_map.project {
                ui.weak("·");
                ui.weak(format!("Project {project}"));
            }
            if let Some(v) = &self.os_version {
                ui.weak("·");
                ui.weak(format!("OS {v}"));
            }
            if !self.read_message.is_empty() {
                ui.weak("·");
                ui.weak(&self.read_message);
            }
            if let Some(msg) = &self.last_from_device {
                ui.weak("·");
                ui.weak(format!("From KO II: {msg}"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let s = self.drummer.get();
                let state = if s.playing { "Playing" } else { "Stopped" };
                ui.weak(format!("{state} · {} · {:.0} BPM · swing {:.0}%", self.style.name(), s.bpm, s.swing));
            });
        });
    }

    /// Settings window: theme and text size. Changes show at once and are saved.
    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut done = false;
        let mut picked_theme = self.settings.theme;
        let mut picked_size = self.settings.text_size;
        let t = theme::current();
        let frame = egui::Frame::new()
            .fill(t.popup)
            .stroke(egui::Stroke::new(1.0, t.line))
            .corner_radius(16.0)
            .inner_margin(egui::Margin::same(20));
        let response = egui::Modal::new(egui::Id::unique("settings")).frame(frame).show(ctx, |ui| {
            let scale = help::text_scale(ui);
            ui.set_width(420.0 * scale);
            ui.horizontal(|ui| {
                ui.heading("Settings");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        done = true;
                    }
                });
            });
            ui.add_space(10.0);
            ui.strong("Theme");
            ui.add_space(4.0);
            for (i, t) in theme::THEMES.iter().enumerate() {
                if theme_row(ui, t, i == picked_theme).clicked() {
                    picked_theme = i;
                }
            }
            ui.add_space(12.0);
            ui.strong("Text size");
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (i, (name, _)) in settings::TEXT_SIZES.iter().enumerate() {
                    ui.selectable_value(&mut picked_size, i, *name);
                }
            });
            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let size = egui::vec2(110.0, 32.0) * scale;
                if ui.add(theme::primary_button("Done", true).min_size(size)).clicked() {
                    done = true;
                }
            });
        });
        if picked_theme != self.settings.theme {
            self.settings.theme = picked_theme;
            theme::apply(ctx, picked_theme);
            self.settings.save();
        }
        if picked_size != self.settings.text_size {
            self.settings.text_size = picked_size;
            self.settings.apply_text_size(ctx);
            self.settings.save();
        }
        // Ignore "clicked outside" for a moment: it is the same click that opened Settings.
        let just_opened = ctx.cumulative_pass_nr() <= self.settings_opened_at + 2;
        if done || (!just_opened && response.should_close()) {
            self.settings_open = false;
        }
    }

    /// The main panel, laid out like the real KO II top panel (7 columns x 5 rows):
    ///
    /// ```text
    ///  VOL    SOUND  MAIN   TEMPO  .      X(BPM) Y(SWING)
    ///  KEYS   A      7      8      9      SAMPLE TIMING
    ///  FADER  B      4      5      6      FX     ERASE
    ///  fader  C      1      2      3      -      +
    ///  SHIFT  D      .      0      ENTER  RECORD PLAY
    /// ```
    /// App jobs: VOL = beat volume, X = tempo, Y = swing, - / + = tempo by 1, PLAY = play / stop,
    /// A-D = pad group, pads = play the pad. Every other button opens the Button Guide for it.
    fn device_panel(&mut self, ui: &mut egui::Ui) {
        let t = theme::current();
        let scale = help::text_scale(ui);
        let mut s = self.drummer.get();
        let mut changed = false;

        let gap = 8.0;
        let left_w = 84.0 * scale;
        let group_w = 64.0 * scale;
        let side_w = 104.0 * scale;
        let pad_w = ((ui.available_width() - left_w - group_w - 2.0 * side_w - 6.0 * gap) / 3.0).max(96.0 * scale);
        let widths = [left_w, group_w, pad_w, pad_w, pad_w, side_w, side_w];
        let top_h = 78.0 * scale;
        let row_h = 66.0 * scale;
        let total = egui::vec2(widths.iter().sum::<f32>() + 6.0 * gap, top_h + 4.0 * row_h + 4.0 * gap);
        let (area, _) = ui.allocate_exact_size(total, egui::Sense::hover());
        // Rect of one cell: column, row (0 = top row).
        let cell = |col: usize, row: usize| -> egui::Rect {
            let x = area.min.x + widths[..col].iter().sum::<f32>() + col as f32 * gap;
            let (y, h) = if row == 0 {
                (area.min.y, top_h)
            } else {
                (area.min.y + top_h + gap + (row - 1) as f32 * (row_h + gap), row_h)
            };
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(widths[col], h))
        };
        // Top-row buttons are shorter, like on the device.
        let top_button = |rect: egui::Rect| rect.shrink2(egui::vec2(0.0, 12.0 * scale));

        // Knobs: VOL, X, Y.
        changed |= knob(ui, cell(0, 0), "vol", "VOL", &mut s.volume, 0.0..=100.0, "%");
        changed |= knob(ui, cell(5, 0), "bpm", "X · BPM", &mut s.bpm, 60.0..=140.0, " BPM");
        changed |= knob(ui, cell(6, 0), "swing", "Y · SWING", &mut s.swing, 50.0..=75.0, "%");

        // Buttons that only explain themselves (open the Button Guide).
        use howto::Key;
        let guide_buttons: [(Key, &'static str, &'static str, egui::Rect); 11] = [
            (Key::Sound, "SOUND", "EDIT", top_button(cell(1, 0))),
            (Key::Main, "MAIN", "COMMIT", top_button(cell(2, 0))),
            (Key::Tempo, "TEMPO", "LOOP", top_button(cell(3, 0))),
            (Key::Keys, "KEYS", "", cell(0, 1)),
            (Key::Fader, "FADER", "", cell(0, 2)),
            (Key::Shift, "SHIFT", "", cell(0, 4)),
            (Key::Sample, "SAMPLE", "CHOP", cell(5, 1)),
            (Key::Timing, "TIMING", "CORRECT", cell(6, 1)),
            (Key::Fx, "FX", "OUTPUT", cell(5, 2)),
            (Key::Erase, "ERASE", "SYSTEM", cell(6, 2)),
            (Key::Record, "RECORD", "", cell(5, 4)),
        ];
        for (key, main, sub, rect) in guide_buttons {
            let resp = panel_button(ui, rect, main, sub, false, scale)
                .on_hover_text(format!("{main} on the KO II. Click to see what it does."));
            if resp.clicked() {
                self.open_guide_for(ui.ctx(), key, main);
            }
        }
        // Fader slider (between FADER and SHIFT).
        let fader = cell(0, 3);
        let resp = ui
            .interact(fader, ui.scope_id().with("fader_slider"), egui::Sense::click())
            .on_hover_text("The fader on the KO II. Click to see what it does.");
        let track = egui::Rect::from_center_size(fader.center(), egui::vec2(6.0 * scale, fader.height() - 8.0));
        ui.painter().rect_filled(track, 3.0, t.well);
        let handle_color = if resp.is_pointer_button_down_on() { t.accent } else { t.soft };
        ui.painter().circle(fader.center(), 9.0 * scale, t.pad, egui::Stroke::new(2.0, handle_color));
        if resp.clicked() {
            self.open_guide_for(ui.ctx(), Key::FaderSlider, "the fader");
        }

        // Group buttons A-D.
        for (g, name) in GROUPS.iter().enumerate() {
            if panel_button(ui, cell(1, g + 1), name, "", self.group == g, scale)
                .on_hover_text(format!("Show group {name}"))
                .clicked()
            {
                self.group = g;
            }
        }

        // Pads.
        let mut down_now: Option<u8> = None;
        for row in 0..4 {
            for col in 0..3 {
                let p = row * 3 + col;
                let rect = cell(col + 2, row + 1);
                let resp = ui.interact(rect, ui.scope_id().with(("pad", p)), egui::Sense::click_and_drag());
                let mut pressed = resp.is_pointer_button_down_on();
                if pressed {
                    down_now = Some(pads::midi_note(self.group, p));
                    self.selected = Some(p);
                }
                // Lit while held down on the KO II itself.
                pressed |= self.device_held.contains(&pads::midi_note(self.group, p));
                self.paint_pad(ui, rect, p, pressed, resp.hovered());
            }
        }
        // Like a real pad: sound starts on press, stops on release.
        if down_now != self.held {
            if let Some(note) = self.held {
                self.ko2.send(Command::PadUp(note));
            }
            if let Some(note) = down_now {
                self.ko2.send(Command::PadDown(note));
            }
            self.held = down_now;
        }

        // - / + : tempo by 1. PLAY: play / stop.
        if panel_button(ui, cell(5, 3), "−", "", false, scale).on_hover_text("Tempo down 1 BPM").clicked() {
            s.bpm = (s.bpm.round() - 1.0).max(60.0);
            changed = true;
        }
        if panel_button(ui, cell(6, 3), "+", "", false, scale).on_hover_text("Tempo up 1 BPM").clicked() {
            s.bpm = (s.bpm.round() + 1.0).min(140.0);
            changed = true;
        }
        let (play_main, play_sub) = if s.playing { ("PLAY", "■ stop") } else { ("PLAY", "▶ start") };
        if panel_button(ui, cell(6, 4), play_main, play_sub, s.playing, scale)
            .on_hover_text("Start or stop the beat, like PLAY on the KO II.")
            .clicked()
        {
            s.playing = !s.playing;
            changed = true;
        }

        if changed {
            self.push_beat_settings(&s);
        }
    }

    /// Send changed beat settings to the drummer thread.
    fn push_beat_settings(&self, s: &drummer::Settings) {
        self.drummer.update(|d| {
            d.playing = s.playing;
            d.bpm = s.bpm;
            d.swing = s.swing;
            d.fills = s.fills;
            d.bars = s.bars;
            d.volume = s.volume;
            d.beat = s.beat.clone();
        });
    }

    /// Open the Button Guide card on the answers that use this KO II button.
    fn open_guide_for(&mut self, ctx: &egui::Context, key: howto::Key, name: &'static str) {
        self.help.show_key(key, name);
        if !self.help_open {
            self.toggle_help(ctx);
        }
    }

    /// The beat "screen" at the top, like the KO II display: app controls, then the 16 steps.
    fn beat_screen(&mut self, ui: &mut egui::Ui) {
        let scale = help::text_scale(ui);
        let mut s = self.drummer.get();
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.weak("Style");
            let mut style = self.style;
            egui::ComboBox::from_id_salt("style")
                .width(120.0 * scale)
                .selected_text(style.name())
                .show_ui(ui, |ui| {
                    for st in STYLES {
                        ui.selectable_value(&mut style, st, st.name());
                    }
                });
            if style != self.style {
                // New style: its own tempo, swing and a fresh beat.
                self.style = style;
                (s.bpm, s.swing) = style.defaults();
                s.beat = beat::new_beat(style);
                changed = true;
            }
            ui.add_space(6.0);
            ui.weak("Loop");
            egui::ComboBox::from_id_salt("loop_bars")
                .width(80.0 * scale)
                .selected_text(format!("{} bars", s.bars))
                .show_ui(ui, |ui| {
                    for bars in LOOP_CHOICES {
                        changed |= ui.selectable_value(&mut s.bars, bars, format!("{bars} bars")).changed();
                    }
                });
            ui.add_space(6.0);
            changed |= ui
                .checkbox(&mut s.fills, "Fill on last bar")
                .on_hover_text("Play a drum fill on the last bar of the loop.")
                .changed();
            ui.add_space(6.0);
            if ui.button("🎲  New beat").on_hover_text("Make a new beat in this style.").clicked() {
                s.beat = beat::new_beat(self.style);
                changed = true;
            }
            let can_read = self.connected() && !self.reading;
            if ui
                .add_enabled(can_read, egui::Button::new("Read pads from KO II"))
                .on_hover_text("Gets the sound names from the KO II. Only reads, nothing is changed.\nTip: close the TE Sample Tool web page first.")
                .on_disabled_hover_text("Plug in the KO II first. Tip: close the TE Sample Tool web page.")
                .clicked()
            {
                self.reading = true;
                self.read_message = "Starting…".into();
                self.ko2.send(Command::ReadPads);
            }
            if self.reading {
                ui.spinner();
            }
        });
        if changed {
            self.push_beat_settings(&s);
        }
        ui.add_space(8.0);
        self.beat_view(ui);
    }

    /// "Now playing": 16 steps (one bar) x 3 drums, the step now playing outlined,
    /// and one dot per bar of the loop on the right.
    fn beat_view(&mut self, ui: &mut egui::Ui) {
        let t = theme::current();
        let s = self.drummer.get();
        let pos = self.drummer.position();
        let bar = pos.map(|(b, _)| b).unwrap_or(0);
        let showing_fill = s.fills && bar + 1 == s.bars;
        let scale = help::text_scale(ui);
        ui.horizontal(|ui| {
            match pos {
                Some((b, _)) => {
                    ui.strong("Now playing");
                    ui.weak(format!(
                        "· bar {} of {}{}",
                        b + 1,
                        s.bars,
                        if showing_fill { " · fill" } else { "" }
                    ));
                }
                None => {
                    ui.strong("Beat");
                    ui.weak(format!("· 16 steps = 1 bar · loop of {} bars · press Play to start", s.bars));
                }
            }
            // One dot per bar: filled = now playing, ring = fill bar.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = 4.0 * scale;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(s.bars as f32 * (r * 2.0 + 4.0), r * 2.0), egui::Sense::hover());
                for b in 0..s.bars {
                    let c = egui::pos2(rect.min.x + r + b as f32 * (r * 2.0 + 4.0), rect.center().y);
                    if pos.is_some() && b == bar {
                        ui.painter().circle_filled(c, r, t.accent);
                    } else {
                        ui.painter().circle_filled(c, r, t.well);
                    }
                    if s.fills && b + 1 == s.bars {
                        ui.painter().circle_stroke(c, r, egui::Stroke::new(1.0, t.soft));
                    }
                }
            });
        });
        ui.add_space(6.0);
        let pattern = if showing_fill { &s.beat.fill } else { &s.beat.main };
        // Drawn by hand inside the width we have, so all 16 steps always fit.
        let label_w = 60.0 * scale;
        let gap = 5.0; // between steps
        let beat_gap = 10.0; // extra space every 4 steps
        let row_h = 20.0 * scale;
        let width = ui.available_width();
        let cell_w = ((width - label_w - gap * 15.0 - beat_gap * 3.0) / 16.0).clamp(8.0, 64.0);
        let (area, _) = ui.allocate_exact_size(egui::vec2(width, row_h * 3.0 + gap * 2.0), egui::Sense::hover());
        let painter = ui.painter_at(area);
        let font = egui::TextStyle::Body.resolve(ui.style());
        // Hat on top, Kick at the bottom (like the wireframe).
        for (row, &drum) in DRUMS.iter().rev().enumerate() {
            let y = area.min.y + row as f32 * (row_h + gap);
            painter.text(
                egui::pos2(area.min.x, y + row_h / 2.0),
                egui::Align2::LEFT_CENTER,
                drum.name(),
                font.clone(),
                t.soft,
            );
            for step in 0..STEPS {
                let x = area.min.x + label_w + step as f32 * (cell_w + gap) + (step / 4) as f32 * beat_gap;
                let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, row_h));
                let v = pattern[drum as usize][step];
                let fill = if v == 0 {
                    t.well
                } else {
                    // Quiet hits (ghost notes) look lighter.
                    t.drums[drum as usize].gamma_multiply(0.35 + 0.65 * v as f32 / 127.0)
                };
                painter.rect_filled(rect, 6.0, fill);
                if pos.map(|(_, st)| st) == Some(step) {
                    painter.rect_stroke(rect, 6.0, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
                }
            }
        }
    }

    /// One pad, everything centred: key number, sound name, and the drum it plays in the beat.
    fn paint_pad(&self, ui: &egui::Ui, rect: egui::Rect, p: usize, pressed: bool, hovered: bool) {
        let t = theme::current();
        let scale = help::text_scale(ui);
        let painter = ui.painter();
        let selected = self.selected == Some(p);
        let radius = 12.0;

        painter.rect_filled(rect, radius, t.pad);
        if pressed {
            painter.rect_filled(rect, radius, t.accent.gamma_multiply(0.45));
        } else if selected {
            painter.rect_filled(rect, radius, t.accent.gamma_multiply(0.16));
        }
        let stroke = if selected {
            egui::Stroke::new(2.0, t.accent)
        } else if hovered {
            egui::Stroke::new(1.0, t.accent.gamma_multiply(0.7))
        } else {
            egui::Stroke::new(1.0, t.line)
        };
        painter.rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);

        let wrap = rect.width() - 16.0;
        let key = painter.layout_no_wrap(PAD_LABELS[p].to_string(), egui::FontId::proportional(11.0 * scale), t.soft);
        let info = self.pad_map.pad(self.group, p);
        let (name, color) = if info.name.is_empty() { ("—", t.soft) } else { (info.name.as_str(), t.ink) };
        let name = painter.layout(name.to_string(), egui::FontId::proportional(15.0 * scale), color, wrap);
        let here = PadRef { group: self.group, pad: p };
        let drum = DRUMS.iter().find(|d| self.pad_map.drums[**d as usize] == Some(here));
        let tag = drum.map(|d| painter.layout_no_wrap(d.name().to_string(), egui::FontId::proportional(11.0 * scale), t.soft));

        let line_gap = 2.0 * scale;
        let height = key.size().y + line_gap + name.size().y + tag.as_ref().map_or(0.0, |g| line_gap + g.size().y);
        let mut y = rect.center().y - height / 2.0;
        let cx = rect.center().x;
        painter.galley(egui::pos2(cx - key.size().x / 2.0, y), key.clone(), t.soft);
        y += key.size().y + line_gap;
        painter.galley(egui::pos2(cx - name.size().x / 2.0, y), name.clone(), color);
        y += name.size().y + line_gap;
        if let (Some(tag), Some(drum)) = (tag, drum) {
            // Coloured dot + drum name, centred together.
            let dot_r = 4.0 * scale;
            let w = dot_r * 2.0 + 4.0 + tag.size().x;
            let left = cx - w / 2.0;
            painter.circle_filled(egui::pos2(left + dot_r, y + tag.size().y / 2.0), dot_r, t.drums[*drum as usize]);
            painter.galley(egui::pos2(left + dot_r * 2.0 + 4.0, y), tag, t.soft);
        }
    }

    /// Under the pads: the picked pad's name, beat role, slot and MIDI note in one row.
    fn pad_details(&mut self, ui: &mut egui::Ui) {
        let Some(p) = self.selected else {
            ui.horizontal(|ui| {
                ui.strong("Pad details");
                ui.weak("· pick a pad to name it or use it in the beat");
            });
            return;
        };
        let note = pads::midi_note(self.group, p);
        let group = self.group;
        let scale = help::text_scale(ui);
        let mut save = false;
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Pad {} · Group {}", PAD_LABELS[p], GROUPS[group]));
            ui.add_space(10.0);
            ui.weak("Name");
            let pad = self.pad_map.pad_mut(group, p);
            let edit = ui.add(egui::TextEdit::singleline(&mut pad.name).desired_width(150.0 * scale));
            save = edit.lost_focus();
            ui.add_space(10.0);

            let here = PadRef { group, pad: p };
            let current = DRUMS.iter().copied().find(|d| self.pad_map.drums[*d as usize] == Some(here));
            let mut choice = current;
            ui.weak("Use as");
            egui::ComboBox::from_id_salt("pad_role")
                .width(100.0 * scale)
                .selected_text(choice.map(|d| d.name()).unwrap_or("Not used"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, None, "Not used");
                    for d in DRUMS {
                        ui.selectable_value(&mut choice, Some(d), d.name());
                    }
                });
            if choice != current {
                if let Some(old) = current {
                    self.pad_map.drums[old as usize] = None;
                }
                if let Some(new) = choice {
                    self.pad_map.drums[new as usize] = Some(here);
                }
                self.sync_drum_notes();
                save = true;
            }
            ui.add_space(10.0);
            let slot = match self.pad_map.pad(group, p).slot {
                Some(slot) => format!("slot {slot}"),
                None => "slot unknown".to_string(),
            };
            ui.weak(format!("{slot} · MIDI note {note}"));
        });
        if save {
            self.save_pad_map();
        }
    }

    /// The Button Guide card: a small borderless window stuck to the main window,
    /// like the extra windows in old Winamp. The main window never changes size.
    ///
    /// The card window is created (hidden) when the app starts and is drawn every frame,
    /// so showing it is instant: no black "empty window" flash. Windows fades it in and out.
    fn show_help_card(&mut self, ctx: &egui::Context, main_area: egui::Rect) {
        let main_info = ctx.input(|i| i.viewport().clone());
        let Some(main_outer) = main_info.outer_rect else {
            ctx.request_repaint();
            return;
        };
        let minimized = main_info.minimized == Some(true);

        // While open: move between "outside" and "inside" when the room changes
        // (e.g. full screen on/off). Left/right stays as picked, so it doesn't jump around.
        if self.help_open {
            let wanted = choose_card_side(ctx);
            if (wanted == CardSide::Inside) != (self.card_side == CardSide::Inside) {
                self.card_side = wanted;
            }
        }
        let inside = self.card_side == CardSide::Inside;
        let rect = card_rect(main_outer, self.card_side, help_card_width(ctx));

        let mut close_clicked = false;
        if self.help_open && inside {
            self.show_inside_card(ctx, main_area, &mut close_clicked);
        }

        let builder = egui::ViewportBuilder::default()
            .with_title(GUIDE_NAME)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_visible(self.help_open && !inside && !minimized)
            .with_position(rect.min)
            .with_inner_size(rect.size());

        let side = self.card_side;
        let help = &mut self.help;
        let mut card_closed_by_os = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("help_card"),
            builder,
            |ui, _class| {
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    // Never really close it (we reuse it); just hide it.
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    card_closed_by_os = true;
                }
                theme::paint_background(ui);
                egui::CentralPanel::no_frame()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(14)))
                    .show(ui, |ui| {
                        // Thin line on the side that touches the main window.
                        let area = ui.max_rect().expand(14.0);
                        let edge_x = match side {
                            CardSide::Right | CardSide::Inside => area.left() + 0.5,
                            CardSide::Left => area.right() - 0.5,
                        };
                        ui.painter().vline(
                            edge_x,
                            area.y_range(),
                            egui::Stroke::new(1.0, theme::current().line),
                        );
                        help.show(ui, &mut close_clicked);
                    });
            },
        );

        if close_clicked || card_closed_by_os {
            self.help_open = false;
        }
    }

    /// Full screen: the card lies on top of the right edge of the main area.
    /// Nothing underneath moves; it is just covered while the card is open.
    fn show_inside_card(&mut self, ctx: &egui::Context, main_area: egui::Rect, close_clicked: &mut bool) {
        let rect = egui::Rect::from_min_max(
            egui::pos2(main_area.max.x - help_card_width(ctx), main_area.min.y),
            main_area.max,
        );
        let t = theme::current();
        egui::Area::new(egui::Id::unique("help_card_inside"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.popup)
                    .stroke(egui::Stroke::new(1.0, t.line))
                    .corner_radius(14.0)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_size(rect.size() - egui::vec2(28.0, 28.0));
                        ui.set_max_width(rect.width() - 28.0);
                        ui.set_max_height(rect.height() - 28.0);
                        self.help.show(ui, close_clicked);
                    });
            });
    }
}

/// One choice in the Settings theme list: colour preview + name.
fn theme_row(ui: &mut egui::Ui, t: &theme::Theme, selected: bool) -> egui::Response {
    let now = theme::current();
    let scale = help::text_scale(ui);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0 * scale), egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 10.0, now.accent.gamma_multiply(0.16));
        painter.rect_stroke(rect, 10.0, egui::Stroke::new(2.0, now.accent), egui::StrokeKind::Inside);
    } else if resp.hovered() {
        painter.rect_filled(rect, 10.0, now.well);
    }
    // Small preview: background, a card, the accent colour and the 3 drum colours.
    let swatch = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 6.0), egui::vec2(72.0 * scale, rect.height() - 12.0));
    let [a, b, c] = t.bg;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(swatch.left_top(), a);
    mesh.colored_vertex(swatch.right_top(), b);
    mesh.colored_vertex(swatch.right_bottom(), c);
    mesh.colored_vertex(swatch.left_bottom(), b);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
    painter.rect_stroke(swatch, 0.0, egui::Stroke::new(1.0, now.line), egui::StrokeKind::Outside);
    let inner = swatch.shrink(5.0);
    painter.rect_filled(inner, 4.0, t.card);
    let dot_r = (inner.height() / 2.0 - 3.0).clamp(2.0, 6.0);
    let mut x = inner.left() + dot_r + 4.0;
    for color in [t.accent, t.drums[0], t.drums[1], t.drums[2]] {
        painter.circle_filled(egui::pos2(x, inner.center().y), dot_r, color);
        x += dot_r * 2.0 + 3.0;
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    painter.text(
        egui::pos2(swatch.right() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        t.name,
        font.clone(),
        now.ink,
    );
    painter.text(
        egui::pos2(rect.right() - 12.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        if t.dark { "Dark" } else { "Light" },
        font,
        now.soft,
    );
    resp
}

/// A turn knob like KNOB X / Y on the KO II. Drag up (or right) to turn it up.
fn knob(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    unit: &str,
) -> bool {
    use std::f32::consts::PI;
    let t = theme::current();
    let scale = help::text_scale(ui);
    let resp = ui
        .interact(rect, ui.scope_id().with(("knob", id)), egui::Sense::click_and_drag())
        .on_hover_text("Drag up or down to turn");
    let (lo, hi) = (*range.start(), *range.end());
    let before = *value;
    if resp.dragged() {
        let d = resp.drag_delta();
        *value = (*value + (d.x - d.y) * (hi - lo) / (160.0 * scale)).clamp(lo, hi);
    }
    if resp.drag_stopped() {
        *value = value.round();
    }

    let painter = ui.painter();
    painter.text(
        rect.center_top() + egui::vec2(0.0, 2.0),
        egui::Align2::CENTER_TOP,
        label,
        egui::FontId::proportional(11.0 * scale),
        t.soft,
    );
    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 2.0),
        egui::Align2::CENTER_BOTTOM,
        format!("{:.0}{unit}", value),
        egui::FontId::proportional(13.0 * scale),
        t.ink,
    );
    let top = rect.top() + 18.0 * scale;
    let bottom = rect.bottom() - 20.0 * scale;
    let r = ((bottom - top) / 2.0).min(rect.width() / 2.0 - 4.0).max(8.0);
    let c = egui::pos2(rect.center().x, (top + bottom) / 2.0);
    let arc = |from: f32, to: f32| -> Vec<egui::Pos2> {
        (0..=40)
            .map(|i| {
                let a = from + (to - from) * i as f32 / 40.0;
                c + r * egui::vec2(a.cos(), a.sin())
            })
            .collect()
    };
    // 270 degree ring from bottom-left to bottom-right, filled up to the value.
    let start = 0.75 * PI;
    let sweep = 1.5 * PI;
    let frac = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let ring = 4.0 * scale;
    painter.add(egui::Shape::line(arc(start, start + sweep), egui::Stroke::new(ring, t.well)));
    if frac > 0.001 {
        painter.add(egui::Shape::line(arc(start, start + sweep * frac), egui::Stroke::new(ring, t.accent)));
    }
    let inner = r - 6.0 * scale;
    let edge = if resp.hovered() || resp.dragged() { t.accent } else { t.line };
    painter.circle(c, inner, t.pad, egui::Stroke::new(1.0, edge));
    let a = start + sweep * frac;
    let dir = egui::vec2(a.cos(), a.sin());
    painter.line_segment([c + dir * inner * 0.35, c + dir * inner * 0.85], egui::Stroke::new(2.5 * scale, t.ink));
    *value != before
}

/// One KO II-style button: main label, and the second label (its SHIFT job) under it.
/// Lights up while pressed, and stays lit when `on`.
fn panel_button(ui: &mut egui::Ui, rect: egui::Rect, main: &str, sub: &str, on: bool, scale: f32) -> egui::Response {
    let t = theme::current();
    let resp = ui.interact(rect, ui.scope_id().with(("panel_button", main)), egui::Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let painter = ui.painter();
    let radius = 12.0;
    let (fill, text, soft) = if on { (t.accent, t.accent_ink, t.accent_ink) } else { (t.pad, t.ink, t.soft) };
    painter.rect_filled(rect, radius, fill);
    if pressed {
        painter.rect_filled(rect, radius, t.accent.gamma_multiply(0.45));
    }
    let stroke = if resp.hovered() || pressed { t.accent } else { t.line };
    painter.rect_stroke(rect, radius, egui::Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
    // One-letter labels (A-D, -, +) are big, like on the device.
    let size = if main.chars().count() == 1 { 20.0 } else { 13.0 };
    let main_font = egui::FontId::proportional(size * scale);
    if sub.is_empty() {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, main, main_font, text);
    } else {
        let sub_font = egui::FontId::proportional(10.5 * scale);
        painter.text(rect.center() - egui::vec2(0.0, 7.0 * scale), egui::Align2::CENTER_CENTER, main, main_font, text);
        painter.text(rect.center() + egui::vec2(0.0, 9.0 * scale), egui::Align2::CENTER_CENTER, sub, sub_font, soft);
    }
    resp
}
