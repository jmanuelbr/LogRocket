use serde::{Deserialize, Serialize};
use egui::Color32;

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub info: Color32,
    pub info_bg: Color32,
    pub warn: Color32,
    pub warn_bg: Color32,
    pub error: Color32,
    pub error_bg: Color32,
    pub debug: Color32,
    pub debug_bg: Color32,
    pub trace: Color32,
    pub trace_bg: Color32,
    pub default: Color32,
    pub default_bg: Color32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl ColorPalette {
    pub fn dark() -> Self {
        Self {
            // INFO - keep as is (white text, transparent background)
            info: Color32::from_rgb(239, 246, 246),
            info_bg: Color32::TRANSPARENT,
            
            // WARN - #5E4602 text, #FFE67EE6 background
            warn: Color32::from_rgb(0x5E, 0x46, 0x02),
            warn_bg: Color32::from_rgba_unmultiplied(0xFF, 0xE6, 0x7E, 0xE6),
            
            // ERROR - #721C24 text, #FDBAB5E6 background
            error: Color32::from_rgb(0x72, 0x1C, 0x24),
            error_bg: Color32::from_rgba_unmultiplied(0xFD, 0xBA, 0xB5, 0xE6),
            
            // DEBUG - #155724 text, #D4EDDAE6 background
            debug: Color32::from_rgb(0x15, 0x57, 0x24),
            debug_bg: Color32::from_rgba_unmultiplied(0xD4, 0xED, 0xDA, 0xE6),
            
            trace: Color32::from_rgb(100, 100, 100),
            trace_bg: Color32::TRANSPARENT,
            default: Color32::from_rgb(220, 220, 220),
            default_bg: Color32::TRANSPARENT,
        }
    }

    pub fn light() -> Self {
        Self {
            // INFO - almost black for light mode
            info: Color32::from_rgb(36, 41, 46),
            info_bg: Color32::TRANSPARENT,
            
            // WARN - #5E4602 text, #FFE67EE6 background
            warn: Color32::from_rgb(0x5E, 0x46, 0x02),
            warn_bg: Color32::from_rgba_unmultiplied(0xFF, 0xE6, 0x7E, 0xE6),
            
            // ERROR - #721C24 text, #FDBAB5E6 background
            error: Color32::from_rgb(0x72, 0x1C, 0x24),
            error_bg: Color32::from_rgba_unmultiplied(0xFD, 0xBA, 0xB5, 0xE6),
            
            // DEBUG - #155724 text, #D4EDDAE6 background
            debug: Color32::from_rgb(0x15, 0x57, 0x24),
            debug_bg: Color32::from_rgba_unmultiplied(0xD4, 0xED, 0xDA, 0xE6),
            
            trace: Color32::from_rgb(88, 96, 105),
            trace_bg: Color32::TRANSPARENT,
            default: Color32::from_rgb(36, 41, 46),
            default_bg: Color32::TRANSPARENT,
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

