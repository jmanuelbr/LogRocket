use serde::{Deserialize, Serialize};
use egui::Color32;

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub info: Color32,
    pub warn: Color32,
    pub error: Color32,
    pub debug: Color32,
    pub trace: Color32,
    pub default: Color32,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            info: Color32::from_rgb(216, 237, 250),
            warn: Color32::from_rgb(255, 240, 213),
            error: Color32::from_rgb(250, 202, 202),
            debug: Color32::from_rgb(222, 251, 199),
            trace: Color32::from_rgb(100, 100, 100),
            default: Color32::from_rgb(220, 220, 220),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub color_palette: ColorPalette,
    pub tail_log: bool,
    pub scroll_to_end: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            color_palette: ColorPalette::default(),
            tail_log: true,
            scroll_to_end: true,
        }
    }
}

