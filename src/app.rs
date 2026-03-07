use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use crate::palette::Palette;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    FuzzySearch,
    JumpKey,
    WaitingForG,
    WaitingForMark,
    WaitingForJumpToMark,
}

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

pub struct App {
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub mode: Mode,
    pub nav_history: Vec<PathBuf>,
    pub nav_history_cursor: usize,
    pub marks: HashMap<char, PathBuf>,
    pub fuzzy_query: String,
    pub filtered_indices: Vec<usize>,
    pub error: Option<(String, Instant)>,
    pub blink_on: bool,
    pub last_blink: Instant,
    pub palette: Palette,
    pub should_quit: bool,
    pub selected_path: Option<PathBuf>,
    pub viewport_height: usize,
    pub last_dir_before_jump: Option<PathBuf>,
}

impl App {
    pub fn new(start_dir: PathBuf, palette: Palette) -> Self {
        let mut app = Self {
            current_dir: start_dir.clone(),
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            mode: Mode::Normal,
            nav_history: vec![start_dir],
            nav_history_cursor: 0,
            marks: HashMap::new(),
            fuzzy_query: String::new(),
            filtered_indices: Vec::new(),
            error: None,
            blink_on: true,
            last_blink: Instant::now(),
            palette,
            should_quit: false,
            selected_path: None,
            viewport_height: 20,
            last_dir_before_jump: None,
        };
        app.load_entries();
        app
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_blink).as_millis() >= 550 {
            self.blink_on = !self.blink_on;
            self.last_blink = now;
        }
        if let Some((_, ts)) = &self.error {
            if now.duration_since(*ts).as_secs() >= 3 {
                self.error = None;
            }
        }
    }

    pub fn load_entries(&mut self) {
        self.entries.clear();
        match std::fs::read_dir(&self.current_dir) {
            Ok(rd) => {
                for entry in rd.flatten() {
                    let meta = entry.metadata().ok();
                    let is_dir = meta.as_ref().map_or(false, |m| m.is_dir());
                    let size = meta.as_ref().and_then(|m| if !m.is_dir() { Some(m.len()) } else { None });
                    let modified = meta.as_ref().and_then(|m| m.modified().ok());
                    self.entries.push(FsEntry {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: entry.path(),
                        is_dir,
                        size,
                        modified,
                    });
                }
                // Sort: dirs first, then alphabetical case-insensitive
                self.entries.sort_by(|a, b| {
                    b.is_dir.cmp(&a.is_dir)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(e) => {
                self.error = Some((format!("CANNOT READ: {}", e), Instant::now()));
            }
        }
        self.rebuild_filtered();
    }

    pub fn rebuild_filtered(&mut self) {
        if self.fuzzy_query.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            use fuzzy_matcher::FuzzyMatcher;
            use fuzzy_matcher::skim::SkimMatcherV2;
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64)> = self.entries.iter().enumerate()
                .filter_map(|(i, e)| {
                    matcher.fuzzy_match(&e.name, &self.fuzzy_query).map(|s| (i, s))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        }
    }

    pub fn navigate_to(&mut self, dir: PathBuf) {
        self.current_dir = dir.clone();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.fuzzy_query.clear();
        self.load_entries();
        // Push to nav history
        if self.nav_history_cursor + 1 < self.nav_history.len() {
            self.nav_history.truncate(self.nav_history_cursor + 1);
        }
        self.nav_history.push(dir);
        self.nav_history_cursor = self.nav_history.len() - 1;
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(parent);
        }
    }

    pub fn nav_back(&mut self) {
        if self.nav_history_cursor > 0 {
            self.nav_history_cursor -= 1;
            let dir = self.nav_history[self.nav_history_cursor].clone();
            self.current_dir = dir;
            self.cursor = 0;
            self.scroll_offset = 0;
            self.fuzzy_query.clear();
            self.load_entries();
        }
    }

    pub fn nav_forward(&mut self) {
        if self.nav_history_cursor + 1 < self.nav_history.len() {
            self.nav_history_cursor += 1;
            let dir = self.nav_history[self.nav_history_cursor].clone();
            self.current_dir = dir;
            self.cursor = 0;
            self.scroll_offset = 0;
            self.fuzzy_query.clear();
            self.load_entries();
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(&idx) = self.filtered_indices.get(self.cursor) {
            let entry = &self.entries[idx];
            if entry.is_dir {
                let path = entry.path.clone();
                self.navigate_to(path);
            } else {
                self.selected_path = Some(entry.path.clone());
            }
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor + 1 < self.filtered_indices.len() {
            self.cursor += 1;
            self.ensure_visible();
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.ensure_visible();
        }
    }

    pub fn jump_top(&mut self) {
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    pub fn jump_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.cursor = self.filtered_indices.len() - 1;
            self.ensure_visible();
        }
    }

    pub fn scroll_half_up(&mut self) {
        let half = self.viewport_height / 2;
        self.cursor = self.cursor.saturating_sub(half);
        self.ensure_visible();
    }

    pub fn scroll_half_down(&mut self) {
        let half = self.viewport_height / 2;
        self.cursor = (self.cursor + half).min(self.filtered_indices.len().saturating_sub(1));
        self.ensure_visible();
    }

    fn ensure_visible(&mut self) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        }
        if self.cursor >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor - self.viewport_height + 1;
        }
    }

    pub fn current_entry(&self) -> Option<&FsEntry> {
        self.filtered_indices.get(self.cursor).map(|&i| &self.entries[i])
    }

    pub fn set_mark(&mut self, c: char) {
        self.marks.insert(c, self.current_dir.clone());
    }

    pub fn jump_to_mark(&mut self, c: char) {
        if c == '\'' {
            // Jump to last position before last jump
            if let Some(dir) = self.last_dir_before_jump.clone() {
                let old = self.current_dir.clone();
                self.navigate_to(dir);
                self.last_dir_before_jump = Some(old);
            }
        } else if let Some(dir) = self.marks.get(&c).cloned() {
            self.last_dir_before_jump = Some(self.current_dir.clone());
            self.navigate_to(dir);
        } else {
            self.error = Some((format!("MARK '{}' NOT SET", c), Instant::now()));
        }
    }
}

pub const JUMP_KEYS: &[char] = &[
    'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l',
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',
    'z', 'x', 'c', 'v', 'b', 'n', 'm',
];

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{} KB", bytes / 1024)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn file_type_badge(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        "DIR"
    } else {
        match entry.name.rsplit('.').next() {
            Some(ext) => match ext.to_lowercase().as_str() {
                "rs" => "RS",
                "toml" => "TOML",
                "md" => "MD",
                "txt" => "TXT",
                "json" => "JSON",
                "yaml" | "yml" => "YAML",
                "py" => "PY",
                "js" => "JS",
                "ts" => "TS",
                "c" => "C",
                "cpp" | "cc" => "CPP",
                "h" => "H",
                "go" => "GO",
                "sh" => "SH",
                "css" => "CSS",
                "html" => "HTML",
                "xml" => "XML",
                "lock" => "LOCK",
                "log" => "LOG",
                "png" => "PNG",
                "jpg" | "jpeg" => "JPG",
                "gif" => "GIF",
                "svg" => "SVG",
                "zip" => "ZIP",
                "tar" => "TAR",
                "gz" => "GZ",
                "exe" => "EXE",
                "dll" => "DLL",
                "so" => "SO",
                _ => "FILE",
            },
            None => "FILE",
        }
    }
}
