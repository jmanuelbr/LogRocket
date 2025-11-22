mod app;
mod log_parser;
mod file_watcher;
mod config;
mod search;

use eframe::egui;
use app::LogViewerApp;

fn load_icon() -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon_bytes = include_bytes!("icons/logo.png");
        let image = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        icon_data: Some(load_icon()),
        ..Default::default()
    };
    
    // Check for command line arguments (file to open)
    let args: Vec<String> = std::env::args().collect();
    let file_to_open = if args.len() > 1 {
        Some(std::path::PathBuf::from(&args[1]))
    } else {
        None
    };
    
    eframe::run_native(
        "Log Rocket",
        options,
        Box::new(move |cc| {
            let mut app = LogViewerApp::default();
            
            // If a file was provided via CLI, load it
            if let Some(path) = file_to_open {
                if path.exists() {
                    if let Err(e) = app.load_file(path) {
                        eprintln!("Error loading file from CLI: {}", e);
                    }
                }
            }
            
            Box::new(app)
        }),
    )
}

