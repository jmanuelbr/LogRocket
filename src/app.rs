use eframe::egui;
use std::path::PathBuf;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek};
use crate::log_parser::{LogParser, LogEntry, LogLevel};
use crate::file_watcher::FileWatcher;
use crate::config::{AppConfig, ColorPalette, Theme};
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
    auto_scroll_frames: usize,
    
    scroll_offset: f32,
    last_file_size: u64,
    
    show_search: bool,
    show_sidebar: bool,
    enabled_levels: std::collections::HashSet<LogLevel>,
    
    // New state fields
    focus_search: bool,
    scroll_to_match: bool,
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
        self.current_file = Some(path.clone());
        self.auto_scroll_frames = 5; // Force scroll for 5 frames to ensure layout settles
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
                                    self.auto_scroll_frames = 3;
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
            auto_scroll_frames: 0,
            scroll_offset: 0.0,
            last_file_size: 0,
            show_search: false,
            show_sidebar: true, // Open by default for visibility
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
            focus_search: false,
            scroll_to_match: false,
        }
    }
}

impl LogViewerApp {

}

impl eframe::App for LogViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use egui::*;
        // Handle keyboard shortcuts
        ctx.input(|input| {
            // Cmd+F or Ctrl+F to toggle search
            if input.key_pressed(egui::Key::F) && 
               (input.modifiers.command || input.modifiers.ctrl) {
                self.show_search = !self.show_search;
                if self.show_search {
                    self.focus_search = true;
                }
            }
            
            // ESC to close search
            if input.key_pressed(egui::Key::Escape) && self.show_search {
                self.show_search = false;
            }

            // Font size shortcuts
            if input.modifiers.command || input.modifiers.ctrl {
                if input.key_pressed(egui::Key::ArrowUp) {
                    self.config.font_size = (self.config.font_size + 1.0).min(30.0);
                }
                if input.key_pressed(egui::Key::ArrowDown) {
                    self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                }
            }
        });
        
        // Apply theme
        match self.config.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
        
        // Check for file updates
        self.check_file_updates();
        
        // Modern UI Layout
        
        // 1. Top Header
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Log Viewer");
                
                ui.add_space(20.0);
                
                // File Controls
                if ui.button("📁 Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log files", &["log", "txt"])
                        .pick_file()
                    {
                        if let Err(e) = self.load_file(path) {
                            eprintln!("Error loading file: {}", e);
                        }
                    }
                }
                
                if ui.button("🔄 Reload").clicked() {
                    if let Some(ref path) = self.current_file {
                        if let Err(e) = self.load_file(path.clone()) {
                            eprintln!("Error reloading file: {}", e);
                        }
                    }
                }
                
                // Breadcrumb / File Info
                ui.add_space(20.0);
                if let Some(ref path) = self.current_file {
                    ui.label(egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy()).strong());
                    
                    // File Size
                    if let Ok(metadata) = fs::metadata(path) {
                        let size_mb = metadata.len() as f64 / 1_000_000.0;
                        ui.label(format!("({:.2} MB)", size_mb));
                    }
                } else {
                    ui.label("No file loaded");
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Sidebar Toggle
                    let sidebar_btn = ui.button(if self.show_sidebar { "Sidebar ⏵" } else { "Sidebar ⏴" });
                    if sidebar_btn.clicked() {
                        self.show_sidebar = !self.show_sidebar;
                    }
                    
                    ui.add_space(10.0);
                    
                    // Search Toggle
                    let search_btn = ui.add(egui::Button::new("🔍 Search").selected(self.show_search));
                    if search_btn.clicked() {
                        self.show_search = !self.show_search;
                        if self.show_search {
                            self.focus_search = true;
                        }
                    }
                });
            });
            ui.add_space(4.0);
        });

        // 2. Search Bar (Floating / Top)
        if self.show_search {
            egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let response = ui.add(egui::TextEdit::singleline(&mut self.search.query).desired_width(300.0));
                    
                    // Handle focus request
                    if self.focus_search {
                        response.request_focus();
                        self.focus_search = false;
                    }
                    
                    // Handle Enter/Shift+Enter shortcuts
                    if (response.has_focus() || response.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if ui.input(|i| i.modifiers.shift) {
                            self.search.prev_match();
                        } else {
                            self.search.next_match();
                        }
                        self.scroll_to_match = true;
                        response.request_focus(); // Keep focus
                    }
                    
                    if response.changed() {
                        self.search.update_search(&self.entries);
                        // Navigate to first match when typing
                        if self.search.matches.len() > 0 {
                            self.search.current_match = Some(0);
                            self.scroll_to_match = true;
                        }
                    }
                    
                    if ui.button("Prev").clicked() {
                        self.search.prev_match();
                        self.scroll_to_match = true;
                    }
                    
                    if ui.button("Next").clicked() {
                        self.search.next_match();
                        self.scroll_to_match = true;
                    }
                    
                    if !self.search.matches.is_empty() {
                        if let Some(idx) = self.search.current_match {
                            ui.label(format!("{}/{}", idx + 1, self.search.matches.len()));
                        } else {
                            ui.label(format!("{} matches", self.search.matches.len()));
                        }
                    } else if !self.search.query.is_empty() {
                        ui.label("No matches");
                    }
                    
                    ui.separator();
                    
                    ui.checkbox(&mut self.search.case_sensitive, "Aa").on_hover_text("Case Sensitive");
                    ui.checkbox(&mut self.search.use_regex, ".*").on_hover_text("Regex");
                });
                ui.add_space(4.0);
            });
        }

        // 3. Right Sidebar (Control Center)
        if self.show_sidebar {
            egui::SidePanel::right("sidebar")
                .resizable(true)
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.heading("Control Center");
                    ui.add_space(10.0);
                    
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Section: Filters
                        ui.collapsing("Filters", |ui| {
                            ui.label("Log Levels:");
                            let mut filter_changed = false;
                            
                            let levels = [
                                (LogLevel::Info, "Info", self.config.color_palette.info),
                                (LogLevel::Warn, "Warn", self.config.color_palette.warn),
                                (LogLevel::Error, "Error", self.config.color_palette.error),
                                (LogLevel::Debug, "Debug", self.config.color_palette.debug),
                            ];
                            
                            for (level, label, color) in levels {
                                let mut enabled = self.enabled_levels.contains(&level);
                                if ui.checkbox(&mut enabled, egui::RichText::new(label).color(color)).changed() {
                                    if enabled {
                                        self.enabled_levels.insert(level);
                                    } else {
                                        self.enabled_levels.remove(&level);
                                    }
                                    filter_changed = true;
                                }
                            }
                            
                            if filter_changed {
                                self.apply_filters();
                            }
                            
                            ui.add_space(5.0);
                            ui.label(format!("Showing: {} / {} lines", self.filtered_entries.len(), self.entries.len()));
                        });
                        
                        ui.separator();
                        
                        // Section: View Options
                        ui.collapsing("View Options", |ui| {
                            // Tail Log
                            ui.checkbox(&mut self.tail_log, "Tail Log (Auto-refresh)");
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
                            
                            // Scroll to End
                            ui.checkbox(&mut self.scroll_to_end, "Auto-scroll to End");
                            if self.scroll_to_end != self.config.scroll_to_end {
                                self.config.scroll_to_end = self.scroll_to_end;
                            }
                        });
                        
                        ui.separator();
                        
                        // Section: Appearance
                        ui.collapsing("Appearance", |ui| {
                            ui.label("Theme:");
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.config.theme == Theme::Dark, "Dark").clicked() {
                                    self.config.theme = Theme::Dark;
                                    self.config.color_palette = ColorPalette::dark();
                                }
                                if ui.selectable_label(self.config.theme == Theme::Light, "Light").clicked() {
                                    self.config.theme = Theme::Light;
                                    self.config.color_palette = ColorPalette::light();
                                }
                            });
                            
                            ui.add_space(5.0);
                            ui.label("Font Size:");
                            ui.add(egui::DragValue::new(&mut self.config.font_size).speed(0.5).clamp_range(8.0..=30.0));
                            
                            ui.add_space(5.0);
                            if ui.button("Export Filtered Logs").clicked() {
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
                                    
                                    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                                    let export_path = current_dir.join(&default_name);
                                    if let Err(e) = fs::write(&export_path, content) {
                                        eprintln!("Error exporting: {}", e);
                                    } else {
                                        eprintln!("Exported to: {}", export_path.display());
                                    }
                                }
                            }
                        });
                    });
                });
        }

        // 4. Central Panel (Log View)
        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0; // Zero spacing between lines
                    
                    if self.entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No log file loaded. Use 'Open' in the top bar to load a log file.");
                        });
                    } else if self.filtered_entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No entries match the current filters.");
                        });
                    } else {
                        // Chunk size for rendering
                        const CHUNK_SIZE: usize = 100;
                        
                        // Render filtered entries in chunks
                        for chunk in self.filtered_entries.chunks(CHUNK_SIZE) {
                            let mut chunk_text = String::new();
                            let mut job = egui::text::LayoutJob::default();
                            let mut chunk_contains_match = false;
                            
                            for &entry_idx in chunk {
                                let entry = &self.entries[entry_idx];
                                let color = self.get_color_for_level(&entry.level);
                                
                                let is_search_match = self.search.is_match(entry_idx);
                                let is_current_match = self.search.is_current_match(entry_idx);
                                
                                if is_current_match && self.scroll_to_match {
                                    self.scroll_to_match = false;
                                    chunk_contains_match = true;
                                }
                                
                                let lines: Vec<&str> = entry.raw_line.lines().collect();
                                for (line_idx, line) in lines.iter().enumerate() {
                                    // Line number and match icon
                                    if line_idx == 0 {
                                        let (line_num_text, text_color) = if is_current_match {
                                            (format!("{:6} 🔍 ", entry.line_number), Color32::YELLOW)
                                        } else if is_search_match {
                                            (format!("{:6} 🔍 ", entry.line_number), Color32::GRAY)
                                        } else {
                                            (format!("{:6}   ", entry.line_number), Color32::GRAY)
                                        };
                                        
                                        job.append(
                                            &line_num_text,
                                            0.0,
                                            egui::TextFormat {
                                                font_id: egui::FontId::monospace(self.config.font_size),
                                                color: text_color,
                                                ..Default::default()
                                            },
                                        );
                                        chunk_text.push_str(&line_num_text);
                                    } else {
                                        // Indentation for continuation lines
                                        let indent = "         "; // 6 chars + 3 spaces
                                        job.append(
                                            indent,
                                            0.0,
                                            egui::TextFormat {
                                                font_id: egui::FontId::monospace(self.config.font_size),
                                                color: Color32::TRANSPARENT, // Invisible but takes space
                                                ..Default::default()
                                            },
                                        );
                                        chunk_text.push_str(indent);
                                    }
                                    
                                    // Log content
                                    job.append(
                                        line,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::monospace(self.config.font_size),
                                            color,
                                            ..Default::default()
                                        },
                                    );
                                    chunk_text.push_str(line);
                                    
                                    // Newline
                                    job.append(
                                        "\n",
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::monospace(self.config.font_size),
                                            color: Color32::TRANSPARENT,
                                            ..Default::default()
                                        },
                                    );
                                    chunk_text.push('\n');
                                }
                            }
                            
                            // Render the chunk as a single TextEdit
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut chunk_text)
                                    .layouter(&mut |ui, _string, _wrap_width| {
                                        let mut layout_job = job.clone();
                                        layout_job.wrap.max_width = ui.available_width();
                                        ui.fonts(|f| f.layout_job(layout_job))
                                    })
                                    .frame(false)
                                    .desired_width(f32::INFINITY)
                            );
                            
                            // Scroll to this chunk if it contains the match
                            if chunk_contains_match {
                                response.scroll_to_me(Some(Align::Center));
                            }
                        }
                        
                        // Add a spacer at the bottom to ensure we can scroll to the very end
                        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                        
                        // Auto-scroll to end on first load or refresh - must be after all content is rendered
                        if self.auto_scroll_frames > 0 && self.scroll_to_end && !self.filtered_entries.is_empty() {
                            // Scroll to the very bottom
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                            self.auto_scroll_frames -= 1;
                            ui.ctx().request_repaint(); // Ensure we keep repainting until scroll settles
                        }
                    }
                });
        });
        

        
        ctx.request_repaint();
    }
}

