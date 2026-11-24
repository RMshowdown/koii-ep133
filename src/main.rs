// Hide the black console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

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
        Box::new(|_cc| Ok(Box::new(KoiiApp::default()))),
    )
}

/// Which side of the main window the help card sticks to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum CardSide {
    #[default]
    Right,
    Left,
}

#[derive(Default)]
struct KoiiApp {
    /// Is the "How do I…?" card showing?
    help_open: bool,
    /// Side chosen when the card opened.
    card_side: CardSide,
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
                    // Phase 1 will replace this with the real USB status.
                    ui.label("KO II not connected");
                });
            });
        });
        if toggle_clicked {
            self.toggle_help(&ctx);
        }

        // Main area.
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.label("Pads, controls and the beat view come in Phases 1 and 2.");
        });

        self.show_help_card(&ctx);
    }
}

impl KoiiApp {
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
