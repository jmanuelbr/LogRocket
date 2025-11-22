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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl ColorPalette {
    pub fn dark() -> Self {
        Self {
            info: Color32::from_rgb(239, 246, 246),
            warn: Color32::from_rgb(242, 246, 190),
            error: Color32::from_rgb(251, 212, 212),
            debug: Color32::from_rgb(222, 251, 199),
            trace: Color32::from_rgb(100, 100, 100),
            default: Color32::from_rgb(220, 220, 220),
        }
    }

    pub fn light() -> Self {
        Self {
            info: Color32::from_rgb(0, 92, 197), // Blue
            warn: Color32::from_rgb(176, 136, 0), // Dark Yellow/Orange
            error: Color32::from_rgb(215, 58, 73), // Red
            debug: Color32::from_rgb(34, 134, 58), // Green
            trace: Color32::from_rgb(88, 96, 105), // Gray
            default: Color32::from_rgb(36, 41, 46), // Almost Black
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}


#[derive(Debug, Clone)]
pub struct AppConfig {
    pub color_palette: ColorPalette,
    pub tail_log: bool,
    pub scroll_to_end: bool,
    pub theme: Theme,
    pub font_size: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            color_palette: ColorPalette::default(),
            tail_log: true,
            scroll_to_end: true,
            theme: Theme::Dark,
            font_size: 14.0,
        }
    }
}

