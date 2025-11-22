use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Trace,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: LogLevel,
    pub thread: Option<String>,
    pub class: Option<String>,
    pub message: String,
    pub raw_line: String,
    pub is_error_log: bool,
}

pub struct LogParser {
    error_log_regex: Regex,
    access_log_regex: Regex,
}

impl LogParser {
    pub fn new() -> Self {
        // Error log format: DD.MM.YYYY HH:MM:SS.mmm *LEVEL* [thread] class message
        let error_log_pattern = r"^(\d{2}\.\d{2}\.\d{4}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+\*(\w+)\*\s+\[([^\]]+)\]\s+(.+)$";
        
        // Access log format: IP - user DD/MMM/YYYY:HH:MM:SS +TZ "METHOD PATH HTTP/VERSION" STATUS SIZE "referer" "user-agent"
        let access_log_pattern = r"^([^\s]+)\s+-\s+(\S+)\s+(\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}\s+[+-]\d{4})\s+(.+)$";
        
        Self {
            error_log_regex: Regex::new(error_log_pattern).unwrap(),
            access_log_regex: Regex::new(access_log_pattern).unwrap(),
        }
    }

    pub fn parse_line(&self, line: &str, line_number: usize) -> LogEntry {
        // Try error log format first
        if let Some(caps) = self.error_log_regex.captures(line) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string());
            let level_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let thread = caps.get(3).map(|m| m.as_str().to_string());
            let rest = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            
            // Extract class and message
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let class = parts.get(0).map(|s| s.to_string());
            let message = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| rest.to_string());
            
            let level = match level_str.to_uppercase().as_str() {
                "INFO" => LogLevel::Info,
                "WARN" => LogLevel::Warn,
                "ERROR" => LogLevel::Error,
                "DEBUG" => LogLevel::Debug,
                "TRACE" => LogLevel::Trace,
                _ => LogLevel::Unknown,
            };
            
            return LogEntry {
                line_number,
                timestamp,
                level,
                thread,
                class,
                message,
                raw_line: line.to_string(),
                is_error_log: true,
            };
        }
        
        // Try access log format
        if let Some(caps) = self.access_log_regex.captures(line) {
            let ip = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let user = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let timestamp = caps.get(3).map(|m| m.as_str().to_string());
            let rest = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            
            let message = format!("{} - {} - {}", ip, user, rest);
            
            return LogEntry {
                line_number,
                timestamp,
                level: LogLevel::Info, // Access logs are typically INFO level
                thread: None,
                class: None,
                message,
                raw_line: line.to_string(),
                is_error_log: false,
            };
        }
        
        // Default: unparsed line
        LogEntry {
            line_number,
            timestamp: None,
            level: LogLevel::Unknown,
            thread: None,
            class: None,
            message: line.to_string(),
            raw_line: line.to_string(),
            is_error_log: false,
        }
    }

    pub fn parse_file(&self, content: &str) -> Vec<LogEntry> {
        let lines: Vec<&str> = content.lines().collect();
        let mut entries = Vec::new();
        let mut i = 0;
        
        // Pattern to detect if a line starts with a timestamp (DD.MM.YYYY or DD/MMM/YYYY)
        let timestamp_start_pattern = Regex::new(r"^\d{2}[./]").unwrap();
        
        while i < lines.len() {
            let line = lines[i];
            let line_number = i + 1;
            
            // Check if this line starts a new log entry (has timestamp pattern or matches regex)
            let starts_new_entry = self.error_log_regex.is_match(line) || 
                                   self.access_log_regex.is_match(line) ||
                                   timestamp_start_pattern.is_match(line);
            
            if starts_new_entry {
                // Parse the main entry
                let mut entry = self.parse_line(line, line_number);
                let mut full_text = line.to_string();
                i += 1;
                
                // Collect continuation lines (lines that don't start with a timestamp)
                while i < lines.len() {
                    let next_line = lines[i];
                    // Check if next line is a continuation
                    // It's a continuation if it doesn't match entry patterns and doesn't start with timestamp
                    let is_continuation = !self.error_log_regex.is_match(next_line) && 
                                         !self.access_log_regex.is_match(next_line) &&
                                         !timestamp_start_pattern.is_match(next_line) &&
                                         !next_line.trim().is_empty();
                    
                    if is_continuation {
                        full_text.push('\n');
                        full_text.push_str(next_line);
                        i += 1;
                    } else {
                        break;
                    }
                }
                
                // Update the entry with the full multi-line text
                entry.raw_line = full_text;
                entries.push(entry);
            } else {
                // Skip empty lines or unparseable lines
                i += 1;
            }
        }
        
        entries
    }
}

impl Default for LogParser {
    fn default() -> Self {
        Self::new()
    }
}

