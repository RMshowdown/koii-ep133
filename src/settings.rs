//! App settings saved in `%APPDATA%\koii-ep133\settings.json`.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Text sizes the owner can pick (name, how much bigger than egui's normal size).
pub const TEXT_SIZES: [(&str, f32); 4] = [("Small", 1.0), ("Medium", 1.15), ("Large", 1.3), ("Extra large", 1.5)];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    /// Index into TEXT_SIZES.
    #[serde(default = "default_text_size")]
    pub text_size: usize,
    /// Index into theme::THEMES.
    #[serde(default)]
    pub theme: usize,
}

fn default_text_size() -> usize {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Self { text_size: default_text_size(), theme: 0 }
    }
}

impl Settings {
    fn file_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("koii-ep133").join("settings.json"))
    }

    pub fn load() -> Self {
        let mut s: Settings = Self::file_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        s.text_size = s.text_size.min(TEXT_SIZES.len() - 1);
        s.theme = s.theme.min(crate::theme::THEMES.len() - 1);
        s
    }

    pub fn save(&self) {
        let Some(path) = Self::file_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    /// Make every text style bigger or smaller (both themes, all windows).
    pub fn apply_text_size(&self, ctx: &egui::Context) {
        let scale = TEXT_SIZES[self.text_size].1;
        let normal = egui::Style::default().text_styles;
        ctx.all_styles_mut(|style| {
            for (text_style, font) in normal.iter() {
                style
                    .text_styles
                    .insert(text_style.clone(), egui::FontId::new(font.size * scale, font.family.clone()));
            }
        });
    }
}
