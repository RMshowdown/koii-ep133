//! Colour themes (from the owner's mock-ups in `designs/`), the app fonts,
//! and small drawing helpers that use the current theme.

use eframe::egui::{self, Color32, CornerRadius, Stroke};
use std::sync::atomic::{AtomicUsize, Ordering};

/// One colour theme.
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    /// Background: top-left, middle, bottom-right (same colour 3 times = no gradient).
    pub bg: [Color32; 3],
    /// Boxes ("cards") that hold each part of the screen.
    pub card: Color32,
    /// Pads in the pad grid.
    pub pad: Color32,
    /// Sunken spots: empty beat steps, text boxes, slider rails.
    pub well: Color32,
    /// Normal text.
    pub ink: Color32,
    /// Quiet text (tips, labels).
    pub soft: Color32,
    /// Thin borders.
    pub line: Color32,
    /// Main colour: Play button, picked items, lit KO II buttons.
    pub accent: Color32,
    /// Text on top of the accent colour.
    pub accent_ink: Color32,
    /// Kick, Snare, Hat (same order as `beat::DRUMS`).
    pub drums: [Color32; 3],
    /// "No sound" warnings.
    pub warn: Color32,
    /// Solid colour for pop-up lists and the settings window.
    pub popup: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Colour with see-through (alpha 0.0 to 1.0).
const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied_const(r, g, b, (a * 255.0) as u8)
}

pub const THEMES: [Theme; 7] = [
    Theme {
        name: "Amber Tape",
        dark: true,
        bg: [hex(0x201E1B), hex(0x1C1B18), hex(0x181715)],
        card: hex(0x23211E),
        pad: hex(0x2A2825),
        well: hex(0x151412),
        ink: hex(0xE9E2D3),
        soft: hex(0x8A8377),
        line: hex(0x34312C),
        accent: hex(0xE0A458),
        accent_ink: hex(0x1B1A18),
        drums: [hex(0xE0A458), hex(0xCDBB9C), hex(0x9C9384)],
        warn: hex(0xE8925A),
        popup: hex(0x26241F),
    },
    Theme {
        name: "Sage Paper",
        dark: false,
        bg: [hex(0xF6F2EA), hex(0xF6F2EA), hex(0xF6F2EA)],
        card: hex(0xFFFCF6),
        pad: hex(0xFFFFFF),
        well: hex(0xF0EADF),
        ink: hex(0x33302B),
        soft: hex(0x7A7266),
        line: hex(0xE7E0D2),
        accent: hex(0x5E8F82),
        accent_ink: hex(0xFFFFFF),
        drums: [hex(0xD9A68A), hex(0x9DB7C4), hex(0xB9A98F)],
        warn: hex(0xB8662F),
        popup: hex(0xFFFCF6),
    },
    Theme {
        name: "Morning Mist",
        dark: false,
        bg: [hex(0xF7F3EC), hex(0xE6EFEA), hex(0xDCE7F0)],
        card: rgba(255, 255, 255, 0.62),
        pad: rgba(255, 255, 255, 0.85),
        well: rgba(47, 58, 54, 0.07),
        ink: hex(0x2F3A36),
        soft: hex(0x6B7872),
        line: rgba(47, 58, 54, 0.10),
        accent: hex(0x5E8F82),
        accent_ink: hex(0xFFFFFF),
        drums: [hex(0xD9A68A), hex(0x9DB7C4), hex(0xB9A98F)],
        warn: hex(0xB8662F),
        popup: hex(0xF7F8F5),
    },
    Theme {
        name: "Rainy Window",
        dark: false,
        bg: [hex(0xE9ECF0), hex(0xE9ECF0), hex(0xE9ECF0)],
        card: hex(0xF6F8FA),
        pad: hex(0xFCFDFE),
        well: hex(0xE3E7EC),
        ink: hex(0x2D3440),
        soft: hex(0x6E7887),
        line: hex(0xDCE1E8),
        accent: hex(0x7A8FB0),
        accent_ink: hex(0xFFFFFF),
        drums: [hex(0x7486A6), hex(0x98A9C2), hex(0xB3BFD0)],
        warn: hex(0xB8662F),
        popup: hex(0xF6F8FA),
    },
    Theme {
        name: "Sunday Coffee",
        dark: false,
        bg: [hex(0xFBF4EC), hex(0xFBF4EC), hex(0xFBF4EC)],
        card: hex(0xFFFFFF),
        pad: hex(0xFFFDFA),
        well: hex(0xFAEFE4),
        ink: hex(0x3A2F2A),
        soft: hex(0x8C7B70),
        line: hex(0xF0E2D4),
        accent: hex(0xD9825B),
        accent_ink: hex(0xFFFFFF),
        drums: [hex(0xC98A6F), hex(0xF0B9A0), hex(0xE9CDB0)],
        warn: hex(0xB8562F),
        popup: hex(0xFFFFFF),
    },
    Theme {
        name: "Ocean Night",
        dark: true,
        bg: [hex(0x0B2A5B), hex(0x0E3A7A), hex(0x15508F)],
        card: rgba(255, 255, 255, 0.07),
        pad: rgba(255, 255, 255, 0.08),
        well: rgba(0, 0, 0, 0.20),
        ink: hex(0xE8EEF8),
        soft: hex(0x9AAAC4),
        line: rgba(160, 190, 235, 0.16),
        accent: hex(0x7FB0E8),
        accent_ink: hex(0x0B1A33),
        drums: [hex(0xB59BE0), hex(0x6FC4D6), hex(0x8FA7C9)],
        warn: hex(0xF0B060),
        popup: hex(0x123766),
    },
    Theme {
        name: "Midnight Indigo",
        dark: true,
        bg: [hex(0x161238), hex(0x241C5C), hex(0x2E2470)],
        card: rgba(255, 255, 255, 0.06),
        pad: rgba(255, 255, 255, 0.07),
        well: rgba(0, 0, 0, 0.20),
        ink: hex(0xECEAFA),
        soft: hex(0xA6A3CC),
        line: rgba(190, 185, 255, 0.16),
        accent: hex(0xA89BFF),
        accent_ink: hex(0x1A1640),
        drums: [hex(0xE09BC8), hex(0x7FB8E0), hex(0x8F8AC4)],
        warn: hex(0xF0B060),
        popup: hex(0x221B55),
    },
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// The theme in use now.
pub fn current() -> &'static Theme {
    &THEMES[CURRENT.load(Ordering::Relaxed).min(THEMES.len() - 1)]
}

/// Switch theme: colours for every widget, both windows.
pub fn apply(ctx: &egui::Context, index: usize) {
    CURRENT.store(index.min(THEMES.len() - 1), Ordering::Relaxed);
    let t = current();
    let (egui_theme, system) = if t.dark {
        (egui::Theme::Dark, egui::SystemTheme::Dark)
    } else {
        (egui::Theme::Light, egui::SystemTheme::Light)
    };
    ctx.set_theme(egui_theme);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(system));
    ctx.set_visuals_of(egui_theme, visuals(t));
}

fn visuals(t: &Theme) -> egui::Visuals {
    let mut v = if t.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let radius = CornerRadius::same(10);
    let soft_fill = t.well;
    // Hover: a little lighter on dark themes, a little darker on light ones.
    let hover_fill = if t.dark { rgba(255, 255, 255, 0.12) } else { rgba(0, 0, 0, 0.06) };

    v.override_text_color = None;
    v.weak_text_color = Some(t.soft);
    v.hyperlink_color = t.accent;
    v.faint_bg_color = t.well;
    v.extreme_bg_color = t.well;
    v.text_edit_bg_color = Some(t.well);
    v.code_bg_color = t.well;
    v.warn_fg_color = t.warn;
    v.panel_fill = Color32::TRANSPARENT; // the background is painted by `paint_background`
    v.window_fill = t.popup;
    v.window_stroke = Stroke::new(1.0, t.line);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, t.accent_ink);
    v.slider_trailing_fill = true;
    v.text_cursor.stroke = Stroke::new(2.0, t.accent);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = t.card;
    w.noninteractive.weak_bg_fill = t.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, t.ink);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = soft_fill;
    w.inactive.weak_bg_fill = soft_fill;
    w.inactive.bg_stroke = Stroke::new(1.0, t.line);
    w.inactive.fg_stroke = Stroke::new(1.0, t.ink);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = hover_fill;
    w.hovered.weak_bg_fill = hover_fill;
    w.hovered.bg_stroke = Stroke::new(1.0, t.accent);
    w.hovered.fg_stroke = Stroke::new(1.5, t.ink);
    w.hovered.corner_radius = radius;

    // Pressed. Its text colour is also egui's "strong" text colour (headings), so keep it ink.
    let pressed = t.accent.gamma_multiply(0.35);
    w.active.bg_fill = pressed;
    w.active.weak_bg_fill = pressed;
    w.active.bg_stroke = Stroke::new(1.0, t.accent);
    w.active.fg_stroke = Stroke::new(1.5, t.ink);
    w.active.corner_radius = radius;

    w.open.bg_fill = soft_fill;
    w.open.weak_bg_fill = soft_fill;
    w.open.bg_stroke = Stroke::new(1.0, t.accent);
    w.open.fg_stroke = Stroke::new(1.0, t.ink);
    w.open.corner_radius = radius;
    v
}

/// Paint the theme background (solid or gradient) behind everything in this window.
pub fn paint_background(ui: &egui::Ui) {
    let t = current();
    let rect = ui.ctx().content_rect();
    let [a, b, c] = t.bg;
    let mut mesh = egui::Mesh::default();
    // Corners: top-left, top-right, bottom-right, bottom-left. Diagonal gradient.
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), b);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.ctx()
        .layer_painter(egui::LayerId::background())
        .add(egui::Shape::mesh(mesh));
}

/// A rounded box that holds one part of the screen.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let t = current();
    egui::Frame::new()
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(14.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, add)
}

/// Main button (accent colour), e.g. Play.
pub fn primary_button(text: &str, on: bool) -> egui::Button<'static> {
    let t = current();
    if on {
        egui::Button::new(egui::RichText::new(text.to_string()).color(t.accent_ink))
            .fill(t.accent)
            .stroke(Stroke::new(1.0, t.accent))
    } else {
        egui::Button::new(text.to_string())
    }
}

/// Fonts: Familjen Grotesk (Latin) + Noto Sans Thai (for Thai search words).
/// Both are free (SIL Open Font License), shipped in `assets/fonts`.
pub fn set_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "grotesk".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/FamiljenGrotesk.ttf"
        ))),
    );
    fonts.font_data.insert(
        "thai".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansThai.ttf"
        ))),
    );
    let prop = fonts.families.entry(egui::FontFamily::Proportional).or_default();
    prop.insert(0, "grotesk".into());
    prop.insert(1, "thai".into());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("thai".into());
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_are_unique() {
        for (i, a) in THEMES.iter().enumerate() {
            for b in &THEMES[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn popups_are_solid() {
        // Pop-up lists sit on top of other things, so they must not be see-through.
        for t in &THEMES {
            assert_eq!(t.popup.a(), 255, "{}", t.name);
        }
    }
}
