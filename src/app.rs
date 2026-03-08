use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Instant, SystemTime};

use crate::palette::Palette;
use crate::sysmon::SysMon;
use crate::throbber::{Throbber, ThrobberKind};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum RightPanel {
    Info,
    Preview,
    Hidden,
}

impl RightPanel {
    pub fn cycle(self) -> Self {
        match self {
            Self::Info => Self::Preview,
            Self::Preview => Self::Hidden,
            Self::Hidden => Self::Info,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    FuzzySearch,
    JumpKey,
    Visual,
    Rename,
    Create { is_dir: bool },
    Confirm { action: PendingAction },
    WaitingForG,
    WaitingForMark,
    WaitingForJumpToMark,
    WaitingForYank,   // first 'y' pressed, awaiting second 'y'
    WaitingForCut,    // first 'd' pressed, awaiting second 'd'
}

#[derive(PartialEq, Eq, Clone)]
pub enum PendingAction {
    Delete { paths: Vec<PathBuf> },
    Overwrite { src: PathBuf, dest: PathBuf },
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum OpType {
    Copy,
    Cut,
}

pub struct OpBuffer {
    pub paths: Vec<PathBuf>,
    pub op: OpType,
}

/// Progress message from background operation thread.
pub enum OpMessage {
    Progress { done: u64, total: u64, current_file: String },
    Complete,
    Error(String),
}

/// Tracks an active background operation.
pub struct BgOperation {
    pub label: String,
    pub throbber: Throbber,
    pub done: u64,
    pub total: u64,
    pub current_file: String,
    pub receiver: mpsc::Receiver<OpMessage>,
    pub started: Instant,
}

/// Result feedback shown briefly after an operation completes.
pub struct OpFeedback {
    pub success: bool,
    pub label: String,
    pub timestamp: Instant,
}

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

pub struct PaneState {
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub filtered_indices: Vec<usize>,
    pub fuzzy_match_positions: HashMap<usize, Vec<usize>>,  // entry index -> matched char positions
    pub fuzzy_query: String,
    pub nav_history: Vec<PathBuf>,
    pub nav_history_cursor: usize,
    pub viewport_height: usize,
}

impl PaneState {
    pub fn new(start_dir: PathBuf) -> Self {
        Self {
            current_dir: start_dir.clone(),
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            filtered_indices: Vec::new(),
            fuzzy_match_positions: HashMap::new(),
            fuzzy_query: String::new(),
            nav_history: vec![start_dir],
            nav_history_cursor: 0,
            viewport_height: 20,
        }
    }
}

pub struct App {
    pub panes: [PaneState; 2],
    pub active_pane: usize,
    pub dual_pane: bool,
    pub show_hidden: bool,
    pub right_panel: RightPanel,
    pub preview_scroll: usize,
    pub mode: Mode,
    pub marks: HashMap<char, PathBuf>,
    pub error: Option<(String, Instant)>,
    pub blink_on: bool,
    pub last_blink: Instant,
    pub palette: Palette,
    pub should_quit: bool,
    pub selected_path: Option<PathBuf>,
    pub last_dir_before_jump: Option<PathBuf>,
    pub heartbeat: Throbber,
    pub visual_marks: std::collections::HashSet<usize>,
    pub op_buffer: Option<OpBuffer>,
    pub rename_buf: String,
    pub create_buf: String,
    pub confirm_timer: Option<Instant>,
    pub bg_operation: Option<BgOperation>,
    pub op_feedback: Option<OpFeedback>,
    pub show_telemetry: bool,
    pub sysmon: Option<SysMon>,
    pub telemetry_throbber: Option<Throbber>,
    pub sidebar_pct: u16,
    pub show_theme_picker: bool,
    pub theme_picker_cursor: usize,
}

impl App {
    pub fn new(start_dir: PathBuf, palette: Palette) -> Self {
        let mut app = Self {
            panes: [PaneState::new(start_dir.clone()), PaneState::new(start_dir)],
            active_pane: 0,
            dual_pane: false,
            show_hidden: true,
            right_panel: RightPanel::Info,
            preview_scroll: 0,
            mode: Mode::Normal,
            marks: HashMap::new(),
            error: None,
            blink_on: true,
            last_blink: Instant::now(),
            palette,
            should_quit: false,
            selected_path: None,
            last_dir_before_jump: None,
            heartbeat: Throbber::new(ThrobberKind::Heartbeat, palette.variant),
            visual_marks: std::collections::HashSet::new(),
            op_buffer: None,
            rename_buf: String::new(),
            create_buf: String::new(),
            confirm_timer: None,
            bg_operation: None,
            op_feedback: None,
            show_telemetry: false,
            sysmon: None,
            telemetry_throbber: None,
            sidebar_pct: 22,
            show_theme_picker: false,
            theme_picker_cursor: 0,
        };
        app.load_entries();
        app
    }

    /// Active pane reference.
    pub fn pane(&self) -> &PaneState {
        &self.panes[self.active_pane]
    }

    /// Active pane mutable reference.
    pub fn pane_mut(&mut self) -> &mut PaneState {
        &mut self.panes[self.active_pane]
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
        // Auto-cancel confirm after 10 seconds
        if let Some(ts) = self.confirm_timer {
            if now.duration_since(ts).as_secs() >= 10 {
                self.confirm_timer = None;
                self.mode = Mode::Normal;
            }
        }
        // Poll background operation
        let mut op_finished = false;
        if let Some(bg) = &mut self.bg_operation {
            bg.throbber.tick();
            while let Ok(msg) = bg.receiver.try_recv() {
                match msg {
                    OpMessage::Progress { done, total, current_file } => {
                        bg.done = done;
                        bg.total = total;
                        bg.current_file = current_file;
                    }
                    OpMessage::Complete => {
                        self.op_feedback = Some(OpFeedback {
                            success: true,
                            label: format!("\u{2713} {}", bg.label),
                            timestamp: Instant::now(),
                        });
                        op_finished = true;
                    }
                    OpMessage::Error(e) => {
                        self.op_feedback = Some(OpFeedback {
                            success: false,
                            label: format!("\u{2717} {}", e),
                            timestamp: Instant::now(),
                        });
                        op_finished = true;
                    }
                }
            }
        }
        if op_finished {
            self.bg_operation = None;
            self.load_entries();
            // Refresh other pane too
            if self.dual_pane {
                let old = self.active_pane;
                self.active_pane = 1 - old;
                self.load_entries();
                self.active_pane = old;
            }
        }
        // Clear feedback after 3 seconds
        if let Some(fb) = &self.op_feedback {
            if now.duration_since(fb.timestamp).as_secs() >= 3 {
                self.op_feedback = None;
            }
        }
        // Telemetry
        if let Some(mon) = &mut self.sysmon {
            mon.tick();
        }
        if let Some(throb) = &mut self.telemetry_throbber {
            throb.tick();
        }
        self.heartbeat.tick();
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

/// Nerd Font icon glyph for a file entry.
pub fn icon_for(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        // Check for special directory names
        match entry.name.as_str() {
            ".git" => "\u{e5fb}",        // git branch icon
            "src" => "\u{f121}",          // code icon
            "target" | "build" | "dist" => "\u{f487}",  // package icon
            "node_modules" => "\u{e718}", // nodejs icon
            ".github" => "\u{f408}",     // github icon
            _ => "\u{f07b}",             // folder icon
        }
    } else {
        // Check for special filenames first
        match entry.name.as_str() {
            ".gitignore" | ".gitmodules" | ".gitattributes" => "\u{e5fb}",
            "Cargo.toml" | "Cargo.lock" => "\u{e7a8}",
            "Dockerfile" | "docker-compose.yml" => "\u{f308}",
            "Makefile" | "CMakeLists.txt" => "\u{f085}",
            "LICENSE" | "LICENSE.md" => "\u{f0e3}",
            _ => {
                // Match by extension
                match entry.name.rsplit('.').next() {
                    Some(ext) => match ext.to_lowercase().as_str() {
                        "rs" => "\u{e7a8}",              // rust
                        "py" => "\u{e73c}",              // python
                        "js" | "mjs" | "cjs" => "\u{e781}",  // javascript
                        "ts" | "tsx" => "\u{e628}",      // typescript
                        "jsx" => "\u{e7ba}",             // react
                        "go" => "\u{e626}",              // go
                        "c" => "\u{e61e}",               // c
                        "cpp" | "cc" | "cxx" => "\u{e61d}", // c++
                        "h" | "hpp" => "\u{e61e}",       // c header
                        "cs" => "\u{f81a}",              // c#
                        "java" => "\u{e738}",            // java
                        "rb" => "\u{e739}",              // ruby
                        "php" => "\u{e73d}",             // php
                        "swift" => "\u{e755}",           // swift
                        "kt" | "kts" => "\u{e634}",     // kotlin
                        "lua" => "\u{e620}",             // lua
                        "sh" | "bash" | "zsh" | "fish" => "\u{e795}",  // shell
                        "ps1" => "\u{e70e}",             // powershell
                        "html" | "htm" => "\u{e736}",    // html
                        "css" | "scss" | "sass" | "less" => "\u{e749}", // css
                        "json" => "\u{e60b}",            // json
                        "yaml" | "yml" => "\u{e6a8}",   // yaml
                        "toml" => "\u{e615}",            // config
                        "xml" => "\u{f121}",             // code
                        "md" | "mdx" => "\u{e73e}",     // markdown
                        "txt" => "\u{f15c}",             // text file
                        "pdf" => "\u{f1c1}",             // pdf
                        "doc" | "docx" => "\u{f1c2}",   // word
                        "xls" | "xlsx" => "\u{f1c3}",   // excel
                        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => "\u{f1c5}", // image
                        "svg" => "\u{f1c5}",             // image
                        "mp3" | "wav" | "flac" | "ogg" | "aac" => "\u{f1c7}", // audio
                        "mp4" | "mkv" | "avi" | "mov" | "webm" => "\u{f1c8}", // video
                        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "\u{f1c6}", // archive
                        "exe" | "msi" => "\u{f013}",    // executable
                        "dll" | "so" | "dylib" => "\u{f013}", // library
                        "lock" => "\u{f023}",            // lock
                        "log" => "\u{f18d}",             // log
                        "env" => "\u{f462}",             // environment
                        "sql" | "db" | "sqlite" => "\u{f1c0}", // database
                        _ => "\u{f15b}",                 // generic file
                    },
                    None => "\u{f15b}",
                }
            }
        }
    }
}
