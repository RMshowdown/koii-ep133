//! The Button Guide card: search by goal, most-used list, then step by step
//! with the buttons lit up on a simple KO II drawing.

use crate::howto::{HOW_TOS, Key, MOST_USED, search};
use eframe::egui;

const CHEAT_SHEET_URL: &str = "https://ep133.joeyh.org/";
const TE_GUIDE_URL: &str = "https://teenage.engineering/guides/ep-133";

#[derive(Default)]
pub struct HelpState {
    query: String,
    /// Open answer (index into HOW_TOS) and its current step.
    open: Option<(usize, usize)>,
    /// Show only answers that use this KO II button (and its name), picked on the app's KO II panel.
    key: Option<(Key, &'static str)>,
}

impl HelpState {
    /// List the answers that use this button (clicked on the KO II panel in the main window).
    pub fn show_key(&mut self, key: Key, name: &'static str) {
        self.key = Some((key, name));
        self.open = None;
        self.query.clear();
    }

    /// Draws the card contents. Sets `close_clicked` when ✕ is pressed.
    pub fn show(&mut self, ui: &mut egui::Ui, close_clicked: &mut bool) {
        ui.horizontal(|ui| {
            ui.strong(crate::GUIDE_NAME);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    *close_clicked = true;
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            match self.open {
                Some((idx, step)) => self.answer(ui, idx, step),
                None => self.list(ui),
            }
        });
    }

    fn list(&mut self, ui: &mut egui::Ui) {
        if let Some((key, name)) = self.key {
            ui.horizontal(|ui| {
                ui.strong(format!("What {name} does"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Show all").clicked() {
                        self.key = None;
                    }
                });
            });
            ui.add_space(6.0);
            let results: Vec<usize> = (0..HOW_TOS.len())
                .filter(|&i| HOW_TOS[i].steps.iter().any(|s| s.keys.contains(&key)))
                .collect();
            if results.is_empty() {
                ui.weak(format!("No answers use {name} yet. Try the full cheat sheet below."));
            }
            self.result_list(ui, results);
            ui.add_space(12.0);
            ui.hyperlink_to("Full list: Joey's cheat sheet ↗", CHEAT_SHEET_URL);
            return;
        }
        ui.label("What do you want to do?");
        ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("copy, erase, tempo…")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(8.0);

        let results: Vec<usize> = if self.query.trim().is_empty() {
            (0..MOST_USED).collect()
        } else {
            search(&self.query)
        };
        if self.query.trim().is_empty() {
            ui.weak("Most used");
        } else if results.is_empty() {
            ui.weak("Nothing found. Try another word, or the full cheat sheet below.");
        }
        self.result_list(ui, results);

        ui.add_space(12.0);
        ui.hyperlink_to("Full list: Joey's cheat sheet ↗", CHEAT_SHEET_URL);
    }

    fn result_list(&mut self, ui: &mut egui::Ui, results: Vec<usize>) {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            for idx in results {
                let text = egui::RichText::new(format!("  {}", HOW_TOS[idx].title))
                    .size(14.0 * text_scale(ui));
                if ui.selectable_label(false, text).clicked() {
                    self.open = Some((idx, 0));
                }
            }
        });
    }

    fn answer(&mut self, ui: &mut egui::Ui, idx: usize, step: usize) {
        let how = &HOW_TOS[idx];
        let step = step.min(how.steps.len() - 1);
        let back = if self.key.is_some() { "← Back" } else { "← All" };
        if ui.button(back).clicked() {
            self.open = None;
            return;
        }
        ui.add_space(4.0);
        ui.strong(format!("{} · step {} of {}", how.title, step + 1, how.steps.len()));
        ui.add_space(6.0);

        let s = &how.steps[step];
        draw_ko2(ui, s.keys);
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(crate::theme::current().well)
            .corner_radius(10.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(s.title).size(16.0 * text_scale(ui)).strong());
                if !s.detail.is_empty() {
                    ui.label(s.detail);
                }
            });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.add_enabled(step > 0, egui::Button::new("← Back")).clicked() {
                self.open = Some((idx, step - 1));
            }
            ui.weak(dots(step, how.steps.len()));
            let last = step + 1 >= how.steps.len();
            if ui.add_enabled(!last, egui::Button::new("Next →")).clicked() {
                self.open = Some((idx, step + 1));
            }
        });
        if let Some(next) = how.steps.get(step + 1) {
            ui.add_space(4.0);
            ui.weak(format!("Next step: {}", next.title));
        } else {
            ui.add_space(4.0);
            ui.weak("That's it!");
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.weak(format!("From the TE guide, {}.", how.source));
            ui.hyperlink_to("Open guide ↗", TE_GUIDE_URL);
        });
    }
}

fn dots(step: usize, total: usize) -> String {
    (0..total).map(|i| if i == step { '●' } else { '○' }).collect()
}

/// One spot on the KO II drawing.
#[derive(Clone, Copy)]
enum Cell {
    /// Button: key, main label, second label printed under it (shift function).
    Button(Key, &'static str, &'static str),
    Pad(&'static str),
    Knob(Key, &'static str),
    Volume,
    Slider,
    Empty,
}

/// The KO II top panel, 7 columns x 5 rows, as on the real device
/// (checked against a photo of the owner's KO II).
const LAYOUT: [[Cell; 7]; 5] = {
    use Cell::*;
    use Key::*;
    [
        [Volume, Button(Sound, "SOUND", "EDIT"), Button(Main, "MAIN", "COMMIT"), Button(Tempo, "TEMPO", "LOOP"), Empty, Knob(KnobX, "X"), Knob(KnobY, "Y")],
        [Button(Keys, "KEYS", ""), Button(GroupA, "A", ""), Pad("7"), Pad("8"), Pad("9"), Button(Sample, "SAMPLE", "CHOP"), Button(Timing, "TIMING", "CORRECT")],
        [Button(Fader, "FADER", ""), Button(GroupB, "B", ""), Pad("4"), Pad("5"), Pad("6"), Button(Fx, "FX", "OUTPUT"), Button(Erase, "ERASE", "SYSTEM")],
        [Slider, Button(GroupC, "C", ""), Pad("1"), Pad("2"), Pad("3"), Button(Minus, "−", ""), Button(Plus, "+", "")],
        [Button(Shift, "SHIFT", ""), Button(GroupD, "D", ""), Pad("."), Pad("0"), Pad("ENTER"), Button(Record, "RECORD", ""), Button(Play, "PLAY", "")],
    ]
};

/// How much bigger text is than normal (from the Text size setting).
pub fn text_scale(ui: &egui::Ui) -> f32 {
    let now = ui.style().text_styles[&egui::TextStyle::Body].size;
    let normal = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
    now / normal
}

/// KO II drawing with the buttons for this step lit up.
fn draw_ko2(ui: &mut egui::Ui, lit: &[Key]) {
    let t = crate::theme::current();
    let scale = text_scale(ui);
    let stroke = egui::Stroke::new(1.0, t.line);

    egui::Frame::new()
        .stroke(stroke)
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            let gap = 4.0;
            let width = ui.available_width();
            let cell_w = (width - gap * 6.0) / 7.0;
            let cell_h = 30.0 * scale;
            let (area, _) = ui.allocate_exact_size(
                egui::vec2(width, cell_h * 5.0 + gap * 4.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter();
            let main_font = egui::FontId::proportional(9.5 * scale);
            let sub_font = egui::FontId::proportional(7.5 * scale);

            for (r, row) in LAYOUT.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    let rect = egui::Rect::from_min_size(
                        area.min + egui::vec2(c as f32 * (cell_w + gap), r as f32 * (cell_h + gap)),
                        egui::vec2(cell_w, cell_h),
                    );
                    let on = match cell {
                        Cell::Button(k, ..) | Cell::Knob(k, _) => lit.contains(k),
                        Cell::Pad(label) => {
                            lit.contains(&Key::Pads) || (*label == "ENTER" && lit.contains(&Key::Enter))
                        }
                        Cell::Slider => lit.contains(&Key::FaderSlider),
                        Cell::Volume | Cell::Empty => false,
                    };
                    let (fill, text) = if on {
                        (t.accent, t.accent_ink)
                    } else {
                        (t.well, t.soft)
                    };
                    match cell {
                        Cell::Empty => {}
                        Cell::Knob(_, label) => {
                            let radius = cell_w.min(cell_h) * 0.45;
                            painter.circle(rect.center(), radius, fill, stroke);
                            painter.text(rect.center(), egui::Align2::CENTER_CENTER, *label, main_font.clone(), text);
                        }
                        Cell::Volume => {
                            painter.circle(rect.center(), cell_w.min(cell_h) * 0.35, fill, stroke);
                            painter.text(
                                rect.center_bottom(),
                                egui::Align2::CENTER_BOTTOM,
                                "VOL",
                                sub_font.clone(),
                                text,
                            );
                        }
                        Cell::Slider => {
                            let track = egui::Rect::from_center_size(rect.center(), egui::vec2(6.0, cell_h * 1.6));
                            painter.rect_filled(track, 3.0, fill);
                            painter.text(rect.center_top(), egui::Align2::CENTER_TOP, "fader", sub_font.clone(), text);
                        }
                        Cell::Pad(label) => {
                            painter.rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
                            painter.text(rect.center(), egui::Align2::CENTER_CENTER, *label, main_font.clone(), text);
                        }
                        Cell::Button(_, label, sub) => {
                            painter.rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
                            if sub.is_empty() {
                                painter.text(rect.center(), egui::Align2::CENTER_CENTER, *label, main_font.clone(), text);
                            } else {
                                let mid = rect.center();
                                painter.text(mid - egui::vec2(0.0, cell_h * 0.17), egui::Align2::CENTER_CENTER, *label, main_font.clone(), text);
                                painter.text(mid + egui::vec2(0.0, cell_h * 0.24), egui::Align2::CENTER_CENTER, *sub, sub_font.clone(), text);
                            }
                        }
                    }
                }
            }
        });
}
