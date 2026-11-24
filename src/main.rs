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

use beat::{DRUMS, STEPS, STYLES, Style};
use drummer::{Drummer, LOOP_BARS};
use eframe::egui;
use ko2::{Command, Event, Ko2};
use pads::{GROUPS, PAD_LABELS, PadMap, PadRef};

// Window size limits (width, height in points). Owner can ask to change these.
const START_SIZE: [f32; 2] = [1200.0, 760.0];
const MIN_SIZE: [f32; 2] = [960.0, 620.0];
const MAX_SIZE: [f32; 2] = [1600.0, 1000.0];

// Width of the "How do I…?" card at normal text size. It grows with the Text size setting.
const HELP_CARD_WIDTH: f32 = 400.0;

/// Card width for the current text size.
fn help_card_width(ctx: &egui::Context) -> f32 {
    let now = ctx.global_style().text_styles[&egui::TextStyle::Body].size;
    let normal = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
    HELP_CARD_WIDTH * now / normal
}
// Windows 10/11 adds an invisible border (left, right, bottom) around normal windows, in points.
const WINDOW_INVISIBLE_BORDER: f32 = 8.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KO II Lo-fi Drummer")
            .with_inner_size(START_SIZE)
            .with_min_inner_size(MIN_SIZE)
            .with_max_inner_size(MAX_SIZE),
        ..Default::default()
    };

    eframe::run_native(
        "KO II Lo-fi Drummer",
        options,
        Box::new(|cc| Ok(Box::new(KoiiApp::new(&cc.egui_ctx)))),
    )
}

/// egui's built-in fonts have no Thai letters, so Thai search words showed as boxes.
/// Use Windows' own Thai font (Leelawadee UI) as a fallback. No font file is shipped.
fn add_thai_font(ctx: &egui::Context) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let Ok(bytes) = std::fs::read(format!("{windir}\\Fonts\\LeelawUI.ttf")) else {
        return; // no Thai font: Thai shows as boxes, everything else still works
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("thai".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("thai".into());
    }
    ctx.set_fonts(fonts);
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
    /// Is the "How do I…?" card showing?
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
    reading: bool,
    /// Last "Read pads" message for the user.
    read_message: String,
    os_version: Option<String>,

    drummer: Drummer,
    style: Style,
    help: help::HelpState,
    settings: settings::Settings,
}

impl KoiiApp {
    fn new(ctx: &egui::Context) -> Self {
        ctx.set_theme(egui::Theme::Dark);
        add_thai_font(ctx);
        let settings = settings::Settings::load();
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
            link: Link::NotConnected,
            pad_map,
            group: 0,
            selected: None,
            held: None,
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
            }
        }
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

        // Top bar: app name, connection status, "How do I…?" button.
        let mut toggle_clicked = false;
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("KO II Lo-fi Drummer");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .selectable_label(self.help_open, "How do I…?")
                        .clicked()
                    {
                        toggle_clicked = true;
                    }
                    ui.separator();
                    self.link_status(ui);
                    ui.separator();
                    self.text_size_picker(ui);
                });
            });
        });
        if toggle_clicked {
            self.toggle_help(&ctx);
        }

        // Status bar at the very bottom (room for more status later).
        let status_area = egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.add_space(2.0);
            self.status_bar(ui);
            ui.add_space(2.0);
        });

        // Beat strip pinned above the status bar, so it never hides when the window is small.
        let beat_area = egui::Panel::bottom("beat_strip").show(ui, |ui| {
            ui.add_space(8.0);
            self.beat_view(ui);
            ui.add_space(14.0);
        });
        let beat_area_rect = beat_area.response.rect.union(status_area.response.rect);

        // Main area: scrolls if the window is too small for it.
        let main_area = egui::CentralPanel::default_margins().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| self.pad_grid(ui));
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        self.controls(ui);
                        ui.add_space(14.0);
                        self.side_box(ui);
                    });
                });
            });
        });

        let below_top_bar = main_area.response.rect.union(beat_area_rect);
        self.show_help_card(&ctx, below_top_bar);
    }
}

impl KoiiApp {
    /// Bottom line: what the app knows right now. Later phases can add more here.
    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let link = match &self.link {
                Link::Connected(port) => format!("KO II connected ({port})"),
                Link::Busy(_) => "KO II busy: another app is using it".to_string(),
                Link::NotConnected => "KO II not connected".to_string(),
            };
            ui.weak(link);
            if let Some(project) = self.pad_map.project {
                ui.separator();
                ui.weak(format!("Project {project}"));
            }
            if let Some(v) = &self.os_version {
                ui.separator();
                ui.weak(format!("OS {v}"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let s = self.drummer.get();
                let state = if s.playing { "Playing" } else { "Stopped" };
                ui.weak(format!("{state} · {} · {:.0} BPM · swing {:.0}%", self.style.name(), s.bpm, s.swing));
            });
        });
    }

    /// "Text size" choice in the top bar (4 sizes, saved).
    fn text_size_picker(&mut self, ui: &mut egui::Ui) {
        let mut size = self.settings.text_size;
        egui::ComboBox::from_id_salt("text_size")
            .selected_text(format!("Text: {}", settings::TEXT_SIZES[size].0))
            .show_ui(ui, |ui| {
                for (i, (name, _)) in settings::TEXT_SIZES.iter().enumerate() {
                    ui.selectable_value(&mut size, i, *name);
                }
            });
        if size != self.settings.text_size {
            self.settings.text_size = size;
            self.settings.apply_text_size(ui.ctx());
            self.settings.save();
        }
    }

    fn link_status(&self, ui: &mut egui::Ui) {
        let (dot, text) = match &self.link {
            Link::Connected(_) => (egui::Color32::from_rgb(80, 200, 120), "KO II connected"),
            Link::Busy(_) => (egui::Color32::from_rgb(230, 170, 60), "KO II busy"),
            Link::NotConnected => (ui.visuals().weak_text_color(), "KO II not connected"),
        };
        let resp = ui.label(text);
        if let Link::Busy(msg) | Link::Connected(msg) = &self.link {
            resp.on_hover_text(msg.as_str());
        }
        ui.colored_label(dot, "●");
    }

    /// Play / Stop, Tempo, Swing, Style, fills, New beat.
    fn controls(&mut self, ui: &mut egui::Ui) {
        let mut s = self.drummer.get();
        let mut changed = false;
        ui.strong("Controls");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let size = egui::vec2(140.0, 34.0) * help::text_scale(ui);
            if ui
                .add_sized(size, egui::Button::new("▶  Play").selected(s.playing))
                .clicked()
            {
                s.playing = true;
                changed = true;
            }
            if ui.add_sized(size, egui::Button::new("■  Stop")).clicked() {
                s.playing = false;
                changed = true;
            }
        });
        // Say clearly why there is no sound, right under Play.
        let warn = egui::Color32::from_rgb(230, 170, 60);
        match &self.link {
            Link::Connected(_) => {}
            Link::NotConnected => {
                ui.colored_label(warn, "No sound: KO II not connected. Plug it in with USB-C.");
            }
            Link::Busy(_) => {
                ui.colored_label(
                    warn,
                    "No sound: another app is using the KO II. Close other copies of this app or the TE web tool.",
                );
            }
        }
        ui.add_space(6.0);
        egui::Grid::new("controls_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 8.0))
            .show(ui, |ui| {
                ui.label("Tempo");
                changed |= ui
                    .add(egui::Slider::new(&mut s.bpm, 60.0..=140.0).step_by(1.0).fixed_decimals(0).suffix(" BPM"))
                    .changed();
                ui.end_row();
                ui.label("Swing");
                changed |= ui
                    .add(egui::Slider::new(&mut s.swing, 50.0..=75.0).step_by(1.0).fixed_decimals(0).suffix("%"))
                    .on_hover_text("50% = straight. Higher = lazier, bouncier feel.")
                    .changed();
                ui.end_row();
                ui.label("Style");
                let mut style = self.style;
                egui::ComboBox::from_id_salt("style")
                    .width(180.0)
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
                ui.end_row();
                ui.label("Fills");
                changed |= ui.checkbox(&mut s.fills, "Fill on bar 4").changed();
                ui.end_row();
            });
        ui.add_space(8.0);
        if ui
            .add_sized(egui::vec2(290.0, 34.0) * help::text_scale(ui), egui::Button::new("🎲  New beat"))
            .clicked()
        {
            s.beat = beat::new_beat(self.style);
            changed = true;
        }
        if changed {
            self.drummer.update(|d| {
                d.playing = s.playing;
                d.bpm = s.bpm;
                d.swing = s.swing;
                d.fills = s.fills;
                d.beat = s.beat.clone();
            });
        }
    }

    /// "Now playing": 16 steps x 3 drums, with the step now playing outlined.
    fn beat_view(&mut self, ui: &mut egui::Ui) {
        let s = self.drummer.get();
        let pos = self.drummer.position();
        let bar = pos.map(|(b, _)| b).unwrap_or(0);
        let showing_fill = s.fills && bar == LOOP_BARS - 1;
        let title = match pos {
            Some((b, _)) => format!(
                "Now playing · bar {} of {LOOP_BARS}{}",
                b + 1,
                if s.fills { format!(" · fill on bar {LOOP_BARS}") } else { String::new() }
            ),
            None => "Beat · press Play".to_string(),
        };
        ui.strong(title);
        ui.add_space(4.0);
        let pattern = if showing_fill { &s.beat.fill } else { &s.beat.main };
        // Drawn by hand inside the width we have, so all 16 steps always fit.
        let scale = help::text_scale(ui);
        let label_w = 60.0 * scale;
        let gap = 4.0; // between steps
        let beat_gap = 8.0; // extra space every 4 steps
        let row_h = 22.0 * scale;
        let width = ui.available_width();
        let cell_w = ((width - label_w - gap * 15.0 - beat_gap * 3.0) / 16.0).clamp(8.0, 56.0);
        let (area, _) = ui.allocate_exact_size(
            egui::vec2(width, row_h * 3.0 + gap * 2.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(area);
        let visuals = ui.visuals().clone();
        let hit_color = egui::Color32::from_rgb(200, 200, 190);
        let play_color = egui::Color32::from_rgb(90, 150, 230);
        let font = egui::TextStyle::Body.resolve(ui.style());
        // Hat on top, Kick at the bottom (like the wireframe).
        for (row, &drum) in DRUMS.iter().rev().enumerate() {
            let y = area.min.y + row as f32 * (row_h + gap);
            painter.text(
                egui::pos2(area.min.x + label_w - 8.0, y + row_h / 2.0),
                egui::Align2::RIGHT_CENTER,
                drum.name(),
                font.clone(),
                visuals.text_color(),
            );
            for step in 0..STEPS {
                let x = area.min.x + label_w + step as f32 * (cell_w + gap) + (step / 4) as f32 * beat_gap;
                let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, row_h));
                let v = pattern[drum as usize][step];
                let fill = if v == 0 {
                    visuals.extreme_bg_color
                } else {
                    // Quiet hits (ghost notes) look dimmer.
                    hit_color.gamma_multiply(0.35 + 0.65 * v as f32 / 127.0)
                };
                painter.rect_filled(rect, 3.0, fill);
                if pos.map(|(_, st)| st) == Some(step) {
                    painter.rect_stroke(rect, 3.0, egui::Stroke::new(2.0, play_color), egui::StrokeKind::Inside);
                }
            }
        }
    }

    /// Group buttons + the 4x3 pads, laid out like the KO II keypad.
    fn pad_grid(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Pads");
            ui.add_space(8.0);
            ui.label("Group");
            for (g, name) in GROUPS.iter().enumerate() {
                ui.selectable_value(&mut self.group, g, *name);
            }
        });
        ui.add_space(8.0);

        let pad_size = egui::vec2(120.0, 66.0) * help::text_scale(ui);
        let mut down_now: Option<u8> = None;
        egui::Grid::new("pad_grid")
            .spacing(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                for row in 0..4 {
                    for col in 0..3 {
                        let p = row * 3 + col;
                        let info = self.pad_map.pad(self.group, p);
                        let name = if info.name.is_empty() { "—" } else { info.name.as_str() };
                        let here = PadRef { group: self.group, pad: p };
                        let role = DRUMS
                            .iter()
                            .find(|d| self.pad_map.drums[**d as usize] == Some(here))
                            .map(|d| format!("\n[{}]", d.name()))
                            .unwrap_or_default();
                        let text = egui::RichText::new(format!("{}\n{}{role}", PAD_LABELS[p], name));
                        let resp = ui.add_sized(
                            pad_size,
                            egui::Button::new(text)
                                .selected(self.selected == Some(p))
                                .sense(egui::Sense::click_and_drag()),
                        );
                        if resp.is_pointer_button_down_on() {
                            down_now = Some(pads::midi_note(self.group, p));
                            self.selected = Some(p);
                        }
                    }
                    ui.end_row();
                }
            });

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

        ui.add_space(6.0);
        ui.weak(if self.connected() {
            "Press a pad to hear it. Pick a pad to use it in the beat."
        } else {
            "Plug in the KO II with a USB-C cable to hear the pads."
        });
    }

    /// Right of the pads: read button and the picked pad's details.
    fn side_box(&mut self, ui: &mut egui::Ui) {
        ui.set_width(300.0 * help::text_scale(ui));
        ui.strong("Pad sounds");
        ui.add_space(4.0);
        let can_read = self.connected() && !self.reading;
        if ui
            .add_enabled(can_read, egui::Button::new("Read pads from KO II"))
            .on_hover_text("Only reads. Nothing on the KO II is changed.")
            .clicked()
        {
            self.reading = true;
            self.read_message = "Starting…".into();
            self.ko2.send(Command::ReadPads);
        }
        if self.reading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(&self.read_message);
            });
        } else if !self.read_message.is_empty() {
            ui.label(&self.read_message);
        }
        ui.weak("Tip: close the TE Sample Tool web page first.");
        if let Some(project) = self.pad_map.project {
            ui.weak(format!("Last read: project {project}"));
        }
        if let Some(v) = &self.os_version {
            ui.weak(format!("KO II OS {v}"));
        }

        ui.add_space(16.0);
        ui.separator();
        let Some(p) = self.selected else {
            ui.weak("Pick a pad to see its details.");
            return;
        };
        ui.strong(format!("Pad {} · Group {}", PAD_LABELS[p], GROUPS[self.group]));
        let note = pads::midi_note(self.group, p);
        let group = self.group;
        let pad = self.pad_map.pad_mut(group, p);
        let mut save = false;
        ui.horizontal(|ui| {
            ui.label("Name");
            let edit = ui.add(egui::TextEdit::singleline(&mut pad.name).desired_width(200.0));
            save = edit.lost_focus();
        });
        if save {
            self.save_pad_map();
        }
        let here = PadRef { group, pad: p };
        let current = DRUMS.iter().copied().find(|d| self.pad_map.drums[*d as usize] == Some(here));
        let mut choice = current;
        ui.horizontal(|ui| {
            ui.label("Use in beat as");
            egui::ComboBox::from_id_salt("pad_role")
                .selected_text(choice.map(|d| d.name()).unwrap_or("Not used"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, None, "Not used");
                    for d in DRUMS {
                        ui.selectable_value(&mut choice, Some(d), d.name());
                    }
                });
        });
        if choice != current {
            if let Some(old) = current {
                self.pad_map.drums[old as usize] = None;
            }
            if let Some(new) = choice {
                self.pad_map.drums[new as usize] = Some(here);
            }
            self.sync_drum_notes();
            self.save_pad_map();
        }
        let pad = self.pad_map.pad(group, p);
        match pad.slot {
            Some(slot) => ui.label(format!("Sound slot {slot}")),
            None => ui.weak("Sound slot unknown"),
        };
        ui.weak(format!("MIDI note {note}"));
    }

    /// The "How do I…?" card: a small borderless window stuck to the main window,
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
            .with_title("How do I…?")
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
                egui::CentralPanel::default_margins().show(ui, |ui| {
                    // Thin line on the side that touches the main window.
                    let area = ui.max_rect().expand(8.0);
                    let edge_x = match side {
                        CardSide::Right | CardSide::Inside => area.left() + 0.5,
                        CardSide::Left => area.right() - 0.5,
                    };
                    ui.painter().vline(
                        edge_x,
                        area.y_range(),
                        ui.visuals().widgets.noninteractive.bg_stroke,
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
        egui::Area::new(egui::Id::unique("help_card_inside"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let visuals = ui.visuals().clone();
                egui::Frame::new()
                    .fill(visuals.panel_fill)
                    .stroke(visuals.widgets.noninteractive.bg_stroke)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_min_size(rect.size() - egui::vec2(16.0, 16.0));
                        ui.set_max_width(rect.width() - 16.0);
                        ui.set_max_height(rect.height() - 16.0);
                        self.help.show(ui, close_clicked);
                    });
            });
    }
}
