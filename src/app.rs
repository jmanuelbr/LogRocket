use eframe::egui;
use std::path::PathBuf;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek};
use crate::log_parser::{LogParser, LogEntry, LogLevel};
use crate::file_watcher::FileWatcher;
use crate::config::{AppConfig, ColorPalette};
use crate::search::SearchState;

pub struct LogViewerApp {
    config: AppConfig,
    parser: LogParser,
    file_watcher: FileWatcher,
    search: SearchState,
    
    current_file: Option<PathBuf>,
    entries: Vec<LogEntry>,
    filtered_entries: Vec<usize>, // Indices into entries
    
    tail_log: bool,
    scroll_to_end: bool,
    auto_scroll: bool,
    
    scroll_offset: f32,
    last_file_size: u64,
    
    show_search: bool,
    show_config: bool,
    enabled_levels: std::collections::HashSet<LogLevel>,
    file_path_input: String,
    show_file_dialog: bool,
    current_directory: PathBuf,
    file_dialog_files: Vec<PathBuf>,
}

impl LogViewerApp {
    fn load_file(&mut self, path: PathBuf) -> Result<(), String> {
        // Read file efficiently
        let file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata().map_err(|e| format!("Failed to read metadata: {}", e))?;
        self.last_file_size = metadata.len();
        
        // For large files, use memory-mapped reading
        let content = if metadata.len() > 10_000_000 {
            // For very large files, read only the tail (last 2MB or so)
            let tail_size = 2_000_000.min(metadata.len());
            let mut buffer = vec![0u8; tail_size as usize];
            let mut file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            file.seek(io::SeekFrom::End(-(tail_size as i64)))
                .map_err(|e| format!("Failed to seek: {}", e))?;
            file.read_exact(&mut buffer)
                .map_err(|e| format!("Failed to read: {}", e))?;
            String::from_utf8_lossy(&buffer).to_string()
        } else {
            // For smaller files, read entirely
            fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?
        };
        
        self.entries = self.parser.parse_file(&content);
        self.current_file = Some(path.clone());
        self.auto_scroll = true;
        self.scroll_offset = f32::MAX;
        
        // Start watching the file
        if self.tail_log {
            self.file_watcher.watch_file(path).ok();
        }
        
        // Update search and apply filters to populate filtered_entries
        self.search.update_search(&self.entries);
        self.apply_filters();
        
        Ok(())
    }
    
    fn check_file_updates(&mut self) {
        if !self.tail_log || !self.file_watcher.is_watching() {
            return;
        }
        
        if self.file_watcher.check_for_changes() {
            if let Some(ref path) = self.current_file {
                if let Ok(metadata) = fs::metadata(path) {
                    let new_size = metadata.len();
                    if new_size > self.last_file_size {
                        // Read new content
                        if let Ok(file) = fs::File::open(path) {
                            let mut reader = BufReader::new(file);
                            reader.seek(io::SeekFrom::Start(self.last_file_size))
                                .ok();
                            
                            let mut new_lines = Vec::new();
                            let mut line_buf = String::new();
                            let start_line = self.entries.len();
                            
                            while reader.read_line(&mut line_buf).unwrap_or(0) > 0 {
                                let line = line_buf.trim_end();
                                if !line.is_empty() {
                                    let entry = self.parser.parse_line(line, start_line + new_lines.len() + 1);
                                    new_lines.push(entry);
                                }
                                line_buf.clear();
                            }
                            
                            if !new_lines.is_empty() {
                                self.entries.extend(new_lines);
                                self.filtered_entries = (0..self.entries.len()).collect();
                                self.search.update_search(&self.entries);
                                self.last_file_size = new_size;
                                
                                if self.scroll_to_end {
                                    self.auto_scroll = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn apply_filters(&mut self) {
        // Update search first
        if !self.search.query.is_empty() {
            self.search.update_search(&self.entries);
        }
        
        self.filtered_entries = self.entries
            .iter()
            .enumerate()
            .filter(|(idx, entry)| {
                // Level filter - check if this level is enabled
                if !self.enabled_levels.contains(&entry.level) {
                    return false;
                }
                
                // Search filter - only filter if "show only matches" is enabled
                if self.search.show_only_matches && !self.search.query.is_empty() {
                    if !self.search.is_match(*idx) {
                        return false;
                    }
                }
                
                true
            })
            .map(|(idx, _)| idx)
            .collect();
    }
    
    fn get_color_for_level(&self, level: &LogLevel) -> egui::Color32 {
        match level {
            LogLevel::Info => self.config.color_palette.info,
            LogLevel::Warn => self.config.color_palette.warn,
            LogLevel::Error => self.config.color_palette.error,
            LogLevel::Debug => self.config.color_palette.debug,
            LogLevel::Trace => self.config.color_palette.trace,
            LogLevel::Unknown => self.config.color_palette.default,
        }
    }
}

impl Default for LogViewerApp {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            config: AppConfig::default(),
            parser: LogParser::new(),
            file_watcher: FileWatcher::new(),
            search: SearchState::new(),
            current_file: None,
            entries: Vec::new(),
            filtered_entries: Vec::new(),
            tail_log: true,
            scroll_to_end: true,
            auto_scroll: false,
            scroll_offset: 0.0,
            last_file_size: 0,
            show_search: false,
            show_config: false,
            enabled_levels: {
                let mut set = std::collections::HashSet::new();
                set.insert(LogLevel::Info);
                set.insert(LogLevel::Warn);
                set.insert(LogLevel::Error);
                set.insert(LogLevel::Debug);
                set.insert(LogLevel::Trace);
                set.insert(LogLevel::Unknown);
                set
            },
            file_path_input: String::new(),
            show_file_dialog: false,
            current_directory: current_dir.clone(),
            file_dialog_files: Self::list_files(&current_dir),
        }
    }
}

impl LogViewerApp {
    fn list_files(dir: &PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "log" || ext == "txt" || ext == "LOG" || ext == "TXT" {
                            files.push(path);
                        }
                    }
                }
            }
        }
        files.sort();
        files
    }
}

impl eframe::App for LogViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        ctx.input(|input| {
            // Cmd+F or Ctrl+F to toggle search
            if input.key_pressed(egui::Key::F) && 
               (input.modifiers.command || input.modifiers.ctrl) {
                self.show_search = !self.show_search;
            }
            
            // ESC to close search
            if input.key_pressed(egui::Key::Escape) && self.show_search {
                self.show_search = false;
            }
        });
        
        // Check for file updates
        self.check_file_updates();
        
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📁 Open File...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Log files", &["log", "txt"])
                            .pick_file()
                        {
                            if let Err(e) = self.load_file(path) {
                                eprintln!("Error loading file: {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    
                    if ui.button("🔄 Reload").clicked() {
                        if let Some(ref path) = self.current_file {
                            if let Err(e) = self.load_file(path.clone()) {
                                eprintln!("Error reloading file: {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("Export Filtered...").clicked() {
                        if !self.filtered_entries.is_empty() {
                            let content: String = self.filtered_entries
                                .iter()
                                .map(|&idx| self.entries[idx].raw_line.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            
                            let default_name = self.current_file
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .map(|n| format!("{}_filtered.log", n))
                                .unwrap_or_else(|| "export.log".to_string());
                            
                            let export_path = self.current_directory.join(&default_name);
                            if let Err(e) = fs::write(&export_path, content) {
                                eprintln!("Error exporting: {}", e);
                            } else {
                                eprintln!("Exported to: {}", export_path.display());
                            }
                        }
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_search, "Show Search");
                    ui.checkbox(&mut self.show_config, "Show Configuration");
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍").clicked() {
                        self.show_search = !self.show_search;
                    }
                    if ui.button("⚙️").clicked() {
                        self.show_config = !self.show_config;
                    }
                });
            });
        });
        
        // Control panel
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // File open button with icon
                if ui.button("📁 Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log files", &["log", "txt"])
                        .pick_file()
                    {
                        if let Err(e) = self.load_file(path) {
                            eprintln!("Error loading file: {}", e);
                        }
                    }
                }
                
                ui.separator();
                
                // File path display
                ui.label("File:");
                if let Some(ref path) = self.current_file {
                    ui.label(path.display().to_string());
                } else {
                    ui.label("No file loaded");
                }
                
                ui.separator();
                
                // Tail Log toggle with better styling
                ui.checkbox(&mut self.tail_log, "Tail Log");
                if self.tail_log != self.config.tail_log {
                    self.config.tail_log = self.tail_log;
                    if self.tail_log {
                        if let Some(ref path) = self.current_file {
                            self.file_watcher.watch_file(path.clone()).ok();
                        }
                    } else {
                        self.file_watcher.stop();
                    }
                }
                
                // Scroll to End toggle
                ui.checkbox(&mut self.scroll_to_end, "Scroll to End");
                if self.scroll_to_end != self.config.scroll_to_end {
                    self.config.scroll_to_end = self.scroll_to_end;
                }
                
                ui.separator();
                
                // Multi-select filter checkboxes
                ui.label("Filter:");
                let mut filter_changed = false;
                
                let mut info_enabled = self.enabled_levels.contains(&LogLevel::Info);
                if ui.checkbox(&mut info_enabled, "Info").changed() {
                    if info_enabled {
                        self.enabled_levels.insert(LogLevel::Info);
                    } else {
                        self.enabled_levels.remove(&LogLevel::Info);
                    }
                    filter_changed = true;
                }
                
                let mut warn_enabled = self.enabled_levels.contains(&LogLevel::Warn);
                if ui.checkbox(&mut warn_enabled, "Warn").changed() {
                    if warn_enabled {
                        self.enabled_levels.insert(LogLevel::Warn);
                    } else {
                        self.enabled_levels.remove(&LogLevel::Warn);
                    }
                    filter_changed = true;
                }
                
                let mut error_enabled = self.enabled_levels.contains(&LogLevel::Error);
                if ui.checkbox(&mut error_enabled, "Error").changed() {
                    if error_enabled {
                        self.enabled_levels.insert(LogLevel::Error);
                    } else {
                        self.enabled_levels.remove(&LogLevel::Error);
                    }
                    filter_changed = true;
                }
                
                let mut debug_enabled = self.enabled_levels.contains(&LogLevel::Debug);
                if ui.checkbox(&mut debug_enabled, "Debug").changed() {
                    if debug_enabled {
                        self.enabled_levels.insert(LogLevel::Debug);
                    } else {
                        self.enabled_levels.remove(&LogLevel::Debug);
                    }
                    filter_changed = true;
                }
                
                if filter_changed {
                    self.apply_filters();
                }
                
                ui.separator();
                
                ui.label(format!("Lines: {}", self.filtered_entries.len()));
                if let Some(ref path) = self.current_file {
                    if let Ok(metadata) = fs::metadata(path) {
                        let size_mb = metadata.len() as f64 / 1_000_000.0;
                        ui.label(format!("Size: {:.2} MB", size_mb));
                    }
                }
            });
        });
        
        
        // Configuration panel
        if self.show_config {
            egui::SidePanel::left("config_panel").show(ctx, |ui| {
                ui.heading("Color Configuration");
                ui.separator();
                
                ui.label("Log Level Colors:");
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.info.r() as f32 / 255.0,
                        self.config.color_palette.info.g() as f32 / 255.0,
                        self.config.color_palette.info.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.info = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Info");
                });
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.warn.r() as f32 / 255.0,
                        self.config.color_palette.warn.g() as f32 / 255.0,
                        self.config.color_palette.warn.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.warn = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Warn");
                });
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.error.r() as f32 / 255.0,
                        self.config.color_palette.error.g() as f32 / 255.0,
                        self.config.color_palette.error.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.error = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Error");
                });
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.debug.r() as f32 / 255.0,
                        self.config.color_palette.debug.g() as f32 / 255.0,
                        self.config.color_palette.debug.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.debug = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Debug");
                });
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.trace.r() as f32 / 255.0,
                        self.config.color_palette.trace.g() as f32 / 255.0,
                        self.config.color_palette.trace.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.trace = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Trace");
                });
                
                ui.horizontal(|ui| {
                    let mut rgb = [
                        self.config.color_palette.default.r() as f32 / 255.0,
                        self.config.color_palette.default.g() as f32 / 255.0,
                        self.config.color_palette.default.b() as f32 / 255.0,
                    ];
                    ui.color_edit_button_rgb(&mut rgb);
                    self.config.color_palette.default = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    ui.label("Default");
                });
            });
        }
        
        // File dialog
        if self.show_file_dialog {
            egui::Window::new("Open Log File")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Directory:");
                        ui.label(self.current_directory.display().to_string());
                        if ui.button("⬆").clicked() {
                            if let Some(parent) = self.current_directory.parent() {
                                self.current_directory = parent.to_path_buf();
                                self.file_dialog_files = Self::list_files(&self.current_directory);
                            }
                        }
                    });
                    
                    ui.separator();
                    
                    ui.label("Or enter file path:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.file_path_input);
                        if ui.button("Load").clicked() {
                            let path = PathBuf::from(&self.file_path_input);
                            if path.exists() {
                                if let Err(e) = self.load_file(path.clone()) {
                                    eprintln!("Error loading file: {}", e);
                                } else {
                                    self.show_file_dialog = false;
                                }
                            }
                        }
                    });
                    
                    ui.separator();
                    
                    ui.label("Files in current directory:");
                    let mut file_to_load: Option<PathBuf> = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for file in &self.file_dialog_files {
                            let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                            if ui.button(file_name).clicked() {
                                file_to_load = Some(file.clone());
                            }
                        }
                    });
                    if let Some(path) = file_to_load {
                        if let Err(e) = self.load_file(path) {
                            eprintln!("Error loading file: {}", e);
                        } else {
                            self.show_file_dialog = false;
                        }
                    }
                    
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        self.show_file_dialog = false;
                    }
                });
        }
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            use egui::*;
            
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if self.entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No log file loaded. Use File > Open File to load a log file.");
                        });
                    } else if self.filtered_entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No entries match the current filters.");
                        });
                    } else {
                        // Track if we need to scroll to a match
                        let mut should_scroll_to_match = false;
                        
                        // Render all filtered entries - egui's ScrollArea handles virtual scrolling efficiently
                        for &entry_idx in &self.filtered_entries {
                            let entry = &self.entries[entry_idx];
                            let color = self.get_color_for_level(&entry.level);
                            
                            let is_search_match = self.search.is_match(entry_idx);
                            let is_current_match = self.search.is_current_match(entry_idx);
                            
                            // Track if we need to scroll to this match
                            if is_current_match && !self.search.query.is_empty() {
                                should_scroll_to_match = true;
                            }
                            
                            // Render multi-line entries with proper color for all lines
                            let lines: Vec<&str> = entry.raw_line.lines().collect();
                            for (line_idx, line) in lines.iter().enumerate() {
                                ui.horizontal_wrapped(|ui| {
                                    // Show line number only on first line
                                    if line_idx == 0 {
                                        let line_num_text = if is_search_match {
                                            format!("{:6} 🔍", entry.line_number)
                                        } else {
                                            format!("{:6}", entry.line_number)
                                        };
                                        ui.label(
                                            RichText::new(line_num_text)
                                                .color(Color32::GRAY)
                                        );
                                    } else {
                                        // Indent continuation lines to align with content
                                        ui.add_space(70.0); // Match line number width
                                    }
                                    
                                    // Use selectable text for copy functionality (Cmd+C)
                                    // Render line with proper color (all lines in entry get same color)
                                    let text = RichText::new(*line).color(color);
                                    ui.selectable_label(false, text);
                                });
                            }
                        }
                        
                        // Auto-scroll to current match when typing (after all content is rendered)
                        if should_scroll_to_match && !self.search.query.is_empty() {
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                        }
                        
                        // Auto-scroll to end on first load or refresh - must be after all content is rendered
                        if self.auto_scroll && self.scroll_to_end && !self.filtered_entries.is_empty() {
                            // Scroll to the very bottom
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                            self.auto_scroll = false;
                        }
                    }
                });
        });
        
        // Search panel at the bottom - this will automatically resize the content area
        if self.show_search {
            egui::TopBottomPanel::bottom("search_panel")
                .resizable(true)
                .default_height(150.0)
                .show(ctx, |ui| {
                ui.heading("Search");
                ui.separator();
                
                ui.horizontal(|ui| {
                    let response = ui.text_edit_singleline(&mut self.search.query);
                    if response.changed() {
                        self.search.update_search(&self.entries);
                        // Navigate to first match when typing
                        if !self.search.matches.is_empty() {
                            self.search.current_match = Some(0);
                        } else {
                            self.search.current_match = None;
                        }
                        self.apply_filters();
                    }
                    
                    // Styled buttons with icons
                    if ui.button("⬆ Prev").clicked() {
                        self.search.prev_match();
                    }
                    if ui.button("⬇ Next").clicked() {
                        self.search.next_match();
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.search.case_sensitive, "Case Sensitive").changed() {
                        self.search.update_search(&self.entries);
                        if !self.search.matches.is_empty() {
                            self.search.current_match = Some(0);
                        }
                        self.apply_filters();
                    }
                    if ui.checkbox(&mut self.search.use_regex, "Use Regex").changed() {
                        self.search.update_search(&self.entries);
                        if !self.search.matches.is_empty() {
                            self.search.current_match = Some(0);
                        }
                        self.apply_filters();
                    }
                    if ui.checkbox(&mut self.search.show_only_matches, "Show only matched lines").changed() {
                        self.apply_filters();
                    }
                });
                
                if !self.search.matches.is_empty() {
                    ui.label(format!("Found {} matches", self.search.matches.len()));
                    if let Some(current) = self.search.current_match {
                        ui.label(format!("Match {} of {}", current + 1, self.search.matches.len()));
                    }
                }
            });
        }
        
        ctx.request_repaint();
    }
}

