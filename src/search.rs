use regex::Regex;
use crate::log_parser::LogEntry;

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub show_only_matches: bool,
    pub matches: Vec<usize>,
    pub current_match: Option<usize>,
    pub regex: Option<Regex>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            use_regex: false,
            show_only_matches: false,
            matches: Vec::new(),
            current_match: None,
            regex: None,
        }
    }

    pub fn update_search(&mut self, entries: &[LogEntry]) {
        self.matches.clear();
        self.current_match = None;
        self.regex = None;

        if self.query.is_empty() {
            return;
        }

        let pattern = if self.use_regex {
            let pattern_str = if self.case_sensitive {
                self.query.clone()
            } else {
                format!("(?i){}", self.query)
            };
            match Regex::new(&pattern_str) {
                Ok(re) => {
                    self.regex = Some(re.clone());
                    Some(re)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        for (idx, entry) in entries.iter().enumerate() {
            let text = if self.case_sensitive {
                &entry.raw_line
            } else {
                // For case-insensitive, we'll check in the search function
                &entry.raw_line
            };

            let matches = if let Some(ref regex) = pattern {
                regex.is_match(text)
            } else if self.case_sensitive {
                text.contains(&self.query)
            } else {
                text.to_lowercase().contains(&self.query.to_lowercase())
            };

            if matches {
                self.matches.push(idx);
            }
        }

        if !self.matches.is_empty() {
            self.current_match = Some(0);
        }
    }

    pub fn next_match(&mut self) {
        if let Some(current) = self.current_match {
            let next = (current + 1) % self.matches.len();
            self.current_match = Some(next);
        } else if !self.matches.is_empty() {
            self.current_match = Some(0);
        }
    }

    pub fn prev_match(&mut self) {
        if let Some(current) = self.current_match {
            let prev = if current == 0 {
                self.matches.len() - 1
            } else {
                current - 1
            };
            self.current_match = Some(prev);
        } else if !self.matches.is_empty() {
            self.current_match = Some(self.matches.len() - 1);
        }
    }

    pub fn get_current_match_index(&self) -> Option<usize> {
        self.current_match.and_then(|idx| self.matches.get(idx).copied())
    }

    pub fn is_match(&self, line_index: usize) -> bool {
        self.matches.contains(&line_index)
    }

    pub fn is_current_match(&self, line_index: usize) -> bool {
        self.get_current_match_index() == Some(line_index)
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

