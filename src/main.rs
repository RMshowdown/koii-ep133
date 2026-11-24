// Hide the black console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ko2;
mod pads;
mod sysex;

use eframe::egui;
use ko2::{Command, Event, Ko2};
use pads::{GROUPS, PAD_LABELS, PadMap};

// Window size limits (width, height in points). Owner can ask to change these.
const START_SIZE: [f32; 2] = [1200.0, 760.0];
const MIN_SIZE: [f32; 2] = [960.0, 620.0];
const MAX_SIZE: [f32; 2] = [1600.0, 1000.0];

// Width of the "How do I…?" card.
const HELP_CARD_WIDTH: f32 = 300.0;
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
}

impl KoiiApp {
    fn new(ctx: &egui::Context) -> Self {
        Self {
            help_open: false,
            card_side: CardSide::default(),
            ko2: Ko2::start(ctx.clone()),
            link: Link::NotConnected,
            pad_map: PadMap::load(),
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
}

impl KoiiApp {
    fn toggle_help(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            self.card_side = choose_card_side(ctx);
        }
        self.help_open = !self.help_open;
    }
}

/// Right side if the card fits on screen there, otherwise left.
/// (Phase 7 adds a Settings choice for this.)
fn choose_card_side(ctx: &egui::Context) -> CardSide {
    let info = ctx.input(|i| i.viewport().clone());
    match (info.outer_rect, info.monitor_size) {
        (Some(outer), Some(monitor))
            if outer.max.x - WINDOW_INVISIBLE_BORDER + HELP_CARD_WIDTH > monitor.x =>
        {
            CardSide::Left
        }
        _ => CardSide::Right,
    }
}

/// Where the card window goes, glued to the visible edge of the main window.
fn card_rect(main_outer: egui::Rect, side: CardSide) -> egui::Rect {
    let top = main_outer.min.y;
    let height = main_outer.height() - WINDOW_INVISIBLE_BORDER;
    let left = match side {
        CardSide::Right => main_outer.max.x - WINDOW_INVISIBLE_BORDER,
        CardSide::Left => main_outer.min.x + WINDOW_INVISIBLE_BORDER - HELP_CARD_WIDTH,
    };
    egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(HELP_CARD_WIDTH, height))
}

/// What goes inside the help card.
fn help_contents(ui: &mut egui::Ui, close_clicked: &mut bool) {
    ui.horizontal(|ui| {
        ui.strong("How do I…?");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                *close_clicked = true;
            }
        });
    });
    ui.separator();
    ui.label("Step-by-step button help comes in Phase 3.");
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
                });
            });
        });
        if toggle_clicked {
            self.toggle_help(&ctx);
        }

        // Main area.
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| self.pad_grid(ui));
                ui.add_space(24.0);
                ui.vertical(|ui| self.side_box(ui));
            });
        });

        self.show_help_card(&ctx);
    }
}

impl KoiiApp {
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

    /// Group buttons + the 4x3 pads, laid out like the KO II keypad.
    fn pad_grid(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Group");
            for (g, name) in GROUPS.iter().enumerate() {
                ui.selectable_value(&mut self.group, g, *name);
            }
        });
        ui.add_space(8.0);

        let pad_size = egui::vec2(120.0, 84.0);
        let mut down_now: Option<u8> = None;
        egui::Grid::new("pad_grid")
            .spacing(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                for row in 0..4 {
                    for col in 0..3 {
                        let p = row * 3 + col;
                        let info = self.pad_map.pad(self.group, p);
                        let name = if info.name.is_empty() { "—" } else { info.name.as_str() };
                        let text = egui::RichText::new(format!("{}\n{}", PAD_LABELS[p], name));
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
            "Press a pad to hear it on the KO II."
        } else {
            "Plug in the KO II with a USB-C cable to hear the pads."
        });
    }

    /// Right of the pads: read button and the picked pad's details.
    fn side_box(&mut self, ui: &mut egui::Ui) {
        ui.set_width(300.0);
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
    fn show_help_card(&mut self, ctx: &egui::Context) {
        let main_info = ctx.input(|i| i.viewport().clone());
        let Some(main_outer) = main_info.outer_rect else {
            ctx.request_repaint();
            return;
        };
        let minimized = main_info.minimized == Some(true);
        let rect = card_rect(main_outer, self.card_side);

        let builder = egui::ViewportBuilder::default()
            .with_title("How do I…?")
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_visible(self.help_open && !minimized)
            .with_position(rect.min)
            .with_inner_size(rect.size());

        let side = self.card_side;
        let mut close_clicked = false;
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
                        CardSide::Right => area.left() + 0.5,
                        CardSide::Left => area.right() - 0.5,
                    };
                    ui.painter().vline(
                        edge_x,
                        area.y_range(),
                        ui.visuals().widgets.noninteractive.bg_stroke,
                    );
                    help_contents(ui, &mut close_clicked);
                });
            },
        );

        if close_clicked || card_closed_by_os {
            self.help_open = false;
        }
    }
}
