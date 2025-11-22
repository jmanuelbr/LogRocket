mod app;
mod log_parser;
mod file_watcher;
mod config;
mod search;

use eframe::egui;
use app::LogViewerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "Log Viewer",
        options,
        Box::new(|_cc| Box::new(LogViewerApp::default())),
    )
}

