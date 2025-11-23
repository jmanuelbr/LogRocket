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
    scroll_to_top: bool,
    scroll_target_line: Option<usize>, // Line to scroll to
    target_scroll_offset: Option<f32>, // Calculated Y offset to scroll to
    wrap_text: bool, // Whether to wrap long lines
}

impl LogViewerApp {
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), String> {
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
    
    fn get_bg_color_for_level(&self, level: &LogLevel) -> egui::Color32 {
        match level {
            LogLevel::Info => self.config.color_palette.info_bg,
            LogLevel::Warn => self.config.color_palette.warn_bg,
            LogLevel::Error => self.config.color_palette.error_bg,
            LogLevel::Debug => self.config.color_palette.debug_bg,
            LogLevel::Trace => self.config.color_palette.trace_bg,
            LogLevel::Unknown => self.config.color_palette.default_bg,
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
            show_sidebar: false, // Closed by default
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
            scroll_to_top: false,
            scroll_target_line: None,
            target_scroll_offset: None,
            wrap_text: false, // Default: no wrapping, allow horizontal scroll
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
            
            // Cmd+S to toggle sidebar
            if input.key_pressed(egui::Key::S) && 
               (input.modifiers.command || input.modifiers.ctrl) {
                self.show_sidebar = !self.show_sidebar;
            }
            
            // ESC to close search
            if input.key_pressed(egui::Key::Escape) && self.show_search {
                self.show_search = false;
            }
            
            // Navigation shortcuts: Cmd+ArrowUp/Down to jump to top/bottom
            if input.modifiers.command || input.modifiers.ctrl {
                if input.key_pressed(egui::Key::ArrowUp) {
                    // Jump to top
                    self.scroll_to_top = true;
                }
                if input.key_pressed(egui::Key::ArrowDown) {
                    // Jump to bottom
                    self.auto_scroll_frames = 3;
                }
            }

            // Font size shortcuts: Cmd+= to increase, Cmd+- to decrease (like VS Code/Sublime)
            if input.modifiers.command || input.modifiers.ctrl {
                // Decrease with Cmd+-
                if input.key_pressed(egui::Key::Minus) {
                    self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                }
                
                // Increase with Cmd+= or Cmd++
                // Try multiple approaches to catch the equals key
                let mut should_increase = false;
                
                // Check key events
                for event in &input.events {
                    match event {
                        egui::Event::Key { key, pressed: true, .. } => {
                            // Some keyboards report equals as a specific key
                            if format!("{:?}", key).contains("Num0") || 
                               format!("{:?}", key).contains("Equals") {
                                should_increase = true;
                            }
                        }
                        egui::Event::Text(text) => {
                            if text == "=" || text == "+" {
                                should_increase = true;
                            }
                        }
                        _ => {}
                    }
                }
                
                if should_increase {
                    self.config.font_size = (self.config.font_size + 1.0).min(30.0);
                }
            }
        });
        
        // Apply theme
        match self.config.theme {
            Theme::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = egui::Color32::from_rgb(0x2e, 0x2e, 0x2e);
                visuals.extreme_bg_color = egui::Color32::from_rgb(0x2e, 0x2e, 0x2e);
                ctx.set_visuals(visuals);
            }
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
        
        // Check for file updates
        self.check_file_updates();
        
        // Handle Drag & Drop (and macOS File Open events)
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped_files.first() {
                if let Some(path) = &file.path {
                    if path.exists() {
                        if let Err(e) = self.load_file(path.clone()) {
                            eprintln!("Error loading dropped file: {}", e);
                        }
                    }
                }
            }
        }
        
        // Modern UI Layout
        
        // 1. Top Header
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Log Viewer");
                
                ui.add_space(20.0);
                
                // File Controls
                let icon_size = 20.0;
                if ui.add_sized([icon_size, icon_size], egui::Button::new("📁")).on_hover_text("Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log files", &["log", "txt"])
                        .pick_file()
                    {
                        if let Err(e) = self.load_file(path) {
                            eprintln!("Error loading file: {}", e);
                        }
                    }
                }
                
                if ui.add_sized([icon_size, icon_size], egui::Button::new("🔄")).on_hover_text("Reload").clicked() {
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
                    let sidebar_icon = if self.show_sidebar { "⏵" } else { "⏴" };
                    let sidebar_btn = ui.add_sized([icon_size, icon_size], egui::Button::new(sidebar_icon)).on_hover_text("Toggle Sidebar");
                    if sidebar_btn.clicked() {
                        self.show_sidebar = !self.show_sidebar;
                    }
                    
                    ui.add_space(10.0);
                    
                    // Search Toggle
                    let search_btn = ui.add_sized([icon_size, icon_size], egui::Button::new("🔍").selected(self.show_search)).on_hover_text("Toggle Search");
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
                        if let Some(line_idx) = self.search.get_current_match_index() {
                            self.scroll_target_line = Some(line_idx);
                        }
                        response.request_focus(); // Keep focus
                    }
                    
                    if response.changed() {
                        self.search.update_search(&self.entries);
                        // Navigate to first match when typing
                        if self.search.matches.len() > 0 {
                            self.search.current_match = Some(0);
                            if let Some(line_idx) = self.search.get_current_match_index() {
                                self.scroll_target_line = Some(line_idx);
                            }
                        }
                    }
                    
                    if ui.button("⬆").on_hover_text("Previous Match").clicked() {
                        self.search.prev_match();
                        if let Some(line_idx) = self.search.get_current_match_index() {
                            self.scroll_target_line = Some(line_idx);
                        }
                    }
                    
                    if ui.button("⬇").on_hover_text("Next Match").clicked() {
                        self.search.next_match();
                        if let Some(line_idx) = self.search.get_current_match_index() {
                            self.scroll_target_line = Some(line_idx);
                        }
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
                        egui::CollapsingHeader::new("Filters")
                            .default_open(true)
                            .show(ui, |ui| {
                            ui.label(egui::RichText::new("Log Levels:").size(15.0));
                            let mut filter_changed = false;
                            
                            let levels = [
                                (LogLevel::Info, "Info", self.config.color_palette.info),
                                (LogLevel::Warn, "Warn", self.config.color_palette.warn),
                                (LogLevel::Error, "Error", self.config.color_palette.error),
                                (LogLevel::Debug, "Debug", self.config.color_palette.debug),
                            ];
                            
                            for (level, label, color) in levels {
                                let mut enabled = self.enabled_levels.contains(&level);
                                if ui.checkbox(&mut enabled, egui::RichText::new(label).color(color).size(15.0)).changed() {
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
                            ui.label(egui::RichText::new(format!("Showing: {} / {} lines", self.filtered_entries.len(), self.entries.len())).size(13.0));
                        });
                        
                        ui.separator();
                        
                        // Section: View Options
                        egui::CollapsingHeader::new("View Options")
                            .default_open(true)
                            .show(ui, |ui| {
                            // Tail Log
                            ui.checkbox(&mut self.tail_log, egui::RichText::new("Tail Log (Auto-refresh)").size(15.0));
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
                            ui.checkbox(&mut self.scroll_to_end, egui::RichText::new("Auto-scroll to End").size(15.0));
                            
                            // Wrap Text
                            ui.checkbox(&mut self.wrap_text, egui::RichText::new("Wrap Text").size(15.0));
                            if self.scroll_to_end != self.config.scroll_to_end {
                                self.config.scroll_to_end = self.scroll_to_end;
                            }
                        });
                        
                        ui.separator();
                        
                        // Section: Appearance
                        egui::CollapsingHeader::new("Appearance")
                            .default_open(true)
                            .show(ui, |ui| {
                            ui.label(egui::RichText::new("Theme:").size(15.0));
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
            // Use both scrolls when wrapping is disabled, vertical only when wrapping
            let mut scroll_area = if self.wrap_text {
                ScrollArea::vertical()
            } else {
                ScrollArea::both()
            };
            
            scroll_area = scroll_area
                .auto_shrink([false; 2])
                .id_source("log_scroll_area");
            
            // Handle scroll to top
            if self.scroll_to_top {
                scroll_area = scroll_area.vertical_scroll_offset(0.0);
                self.scroll_to_top = false;
            }
            
            // Apply calculated scroll offset if available
            if let Some(offset) = self.target_scroll_offset {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
                self.target_scroll_offset = None;
                self.scroll_target_line = None; // Clear the target after scroll is applied
            }
            
            scroll_area.show(ui, |ui| {
                // Track Y position as we render
                let mut current_y = 0.0;
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0); // Zero spacing between all items
                    
                    if self.entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No log file loaded. Use 'Open' in the top bar to load a log file.");
                        });
                    } else if self.filtered_entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No entries match the current filters.");
                        });
                    } else {
                        // Render all filtered entries as a single TextEdit (allows multi-line selection)
                        let mut all_text = String::new();
                        let mut job = egui::text::LayoutJob::default();
                        
                        // Track character count to find the exact position of the target line
                        let mut current_char_count = 0;
                        let mut target_char_index = None;
                        
                        for (_entry_idx_in_filtered, &entry_idx) in self.filtered_entries.iter().enumerate() {
                            let entry = &self.entries[entry_idx];
                            let color = self.get_color_for_level(&entry.level);
                            
                            let is_search_match = self.search.is_match(entry_idx);
                            let is_current_match = self.search.is_current_match(entry_idx);
                            
                            // Check if this is the scroll target
                            if let Some(target) = self.scroll_target_line {
                                if entry_idx == target && target_char_index.is_none() {
                                    target_char_index = Some(current_char_count);
                                }
                            }
                            
                            for (line_idx, line) in entry.raw_line.lines().enumerate() {
                                if line_idx == 0 {
                                    // Line number
                                    let line_num_text = format!("{:6}   ", entry.line_number);
                                    let text_color = if is_current_match {
                                        Color32::from_rgb(255, 200, 0)
                                    } else {
                                        color
                                    };
                                    job.append(
                                        &line_num_text,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::monospace(self.config.font_size * 0.85),
                                            color: text_color,
                                            ..Default::default()
                                        },
                                    );
                                    all_text.push_str(&line_num_text);
                                    current_char_count += line_num_text.chars().count();
                                } else {
                                    // Indentation for continuation lines
                                    let indent = "         ";
                                    job.append(
                                        indent,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::monospace(self.config.font_size),
                                            color: Color32::TRANSPARENT,
                                            ..Default::default()
                                        },
                                    );
                                    all_text.push_str(indent);
                                    current_char_count += indent.chars().count();
                                }
                                
                                // Log content with search highlighting
                                if is_search_match {
                                    if let Some(positions) = self.search.get_match_positions(entry_idx) {
                                        let mut last_end = 0;
                                        
                                        for &(start, end) in positions {
                                            if start > line.len() || end > line.len() || start > end {
                                                continue;
                                            }
                                            
                                            if start > last_end && last_end < line.len() {
                                                let safe_start = last_end.min(line.len());
                                                let safe_end = start.min(line.len());
                                                if safe_start < safe_end {
                                                    job.append(
                                                        &line[safe_start..safe_end],
                                                        0.0,
                                                        egui::TextFormat {
                                                            font_id: egui::FontId::monospace(self.config.font_size),
                                                            color,
                                                            background: self.get_bg_color_for_level(&entry.level),
                                                            ..Default::default()
                                                        },
                                                    );
                                                }
                                            }
                                            
                                            let highlight_color = if is_current_match {
                                                Color32::from_rgb(255, 200, 0)
                                            } else {
                                                Color32::from_rgb(255, 255, 150)
                                            };
                                            
                                            if start < line.len() && end <= line.len() {
                                                job.append(
                                                    &line[start..end],
                                                    0.0,
                                                    egui::TextFormat {
                                                        font_id: egui::FontId::monospace(self.config.font_size),
                                                        color: Color32::BLACK,
                                                        background: highlight_color,
                                                        underline: egui::Stroke::new(1.0, Color32::from_rgb(200, 150, 0)),
                                                        ..Default::default()
                                                    },
                                                );
                                            }
                                            
                                            last_end = end;
                                        }
                                        
                                        if last_end < line.len() {
                                            job.append(
                                                &line[last_end..],
                                                0.0,
                                                egui::TextFormat {
                                                    font_id: egui::FontId::monospace(self.config.font_size),
                                                    color,
                                                    background: self.get_bg_color_for_level(&entry.level),
                                                    ..Default::default()
                                                },
                                            );
                                        }
                                    } else {
                                        job.append(
                                            line,
                                            0.0,
                                            egui::TextFormat {
                                                font_id: egui::FontId::monospace(self.config.font_size),
                                                color,
                                                background: self.get_bg_color_for_level(&entry.level),
                                                ..Default::default()
                                            },
                                        );
                                    }
                                } else {
                                    job.append(
                                        line,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: egui::FontId::monospace(self.config.font_size),
                                            color,
                                            background: self.get_bg_color_for_level(&entry.level),
                                            ..Default::default()
                                        },
                                    );
                                }
                                all_text.push_str(line);
                                current_char_count += line.chars().count();
                                
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
                                all_text.push('\n');
                                current_char_count += 1; // Count newline char
                            }
                        }
                        
                        // Configure layout job wrapping
                        let wrap_enabled = self.wrap_text;
                        if wrap_enabled {
                            job.wrap.max_width = ui.available_width();
                        } else {
                            job.wrap.max_width = f32::INFINITY;
                        }
                        
                        // Calculate Galley to find exact scroll position
                        let galley = ui.fonts(|f| f.layout_job(job));
                        
                        // If we have a target, calculate exact offset from Galley
                        if let Some(char_idx) = target_char_index {
                            if self.target_scroll_offset.is_none() {
                                // Find the row containing the target character index
                                let mut accumulated_chars = 0;
                                let mut y_offset = 0.0;
                                for row in &galley.rows {
                                    let row_char_count = row.char_count_excluding_newline() + if row.ends_with_newline { 1 } else { 0 };
                                    if accumulated_chars + row_char_count > char_idx {
                                        // Found the row containing the character
                                        y_offset = row.rect.min.y;
                                        break;
                                    }
                                    accumulated_chars += row_char_count;
                                }
                                
                                // Center the target line in viewport
                                let viewport_height = ui.available_height();
                                let centered_offset = (y_offset - viewport_height / 2.0).max(0.0);
                                self.target_scroll_offset = Some(centered_offset);
                            }
                        }
                        
                        // Render using the pre-calculated Galley
                        ui.add(
                            egui::TextEdit::multiline(&mut all_text)
                                .layouter(&mut |ui, _string, _wrap_width| {
                                    // Return the pre-calculated galley (cloned because layouter might be called multiple times)
                                    // Note: we ignore the passed wrap_width because we already used the correct one
                                    galley.clone() 
                                })
                                .frame(false)
                                .margin(egui::vec2(0.0, 0.0))
                                .desired_width(f32::INFINITY)
                        );
                        
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

