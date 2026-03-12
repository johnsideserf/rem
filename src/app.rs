use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Instant, SystemTime};

use crate::palette::Palette;
use crate::symbols::SymbolSet;
use crate::sysmon::SysMon;
use crate::throbber::{Throbber, ThrobberKind};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum RightPanel {
    Info,
    Preview,
    Hidden,
    DiskUsage,
}

impl RightPanel {
    pub fn cycle(self) -> Self {
        match self {
            Self::Info => Self::Preview,
            Self::Preview => Self::Hidden,
            Self::Hidden => Self::Info,
            Self::DiskUsage => Self::Info,
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
    WaitingForYank,       // first 'y' pressed, awaiting second 'y'
    WaitingForCut,        // first 'd' pressed, awaiting second 'd'
    WaitingForDeleteMark, // 'M' pressed, awaiting key to delete mark
    RecursiveSearch,
    BulkRename,           // editing find/replace, Tab switches fields, Enter applies
    Edit,                 // in-app text editor
    OpsLog,               // operations log viewer (#43)
    Command,              // MU-TH-UR command mode (#41)
    TagInput,             // typing a tag name (#58)
}

#[derive(PartialEq, Eq, Clone)]
#[allow(dead_code)]
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

/// Git repository info for the current directory.
#[derive(Clone)]
pub struct GitInfo {
    pub branch: String,
    pub dirty: bool,
}

impl GitInfo {
    /// Detect git branch and dirty status for a given directory.
    pub fn detect(dir: &std::path::Path) -> Option<Self> {
        // Walk up to find .git
        let mut current = dir.to_path_buf();
        let git_dir = loop {
            let dot_git = current.join(".git");
            if dot_git.is_dir() {
                break dot_git;
            }
            // .git can also be a file (worktrees)
            if dot_git.is_file() {
                if let Ok(content) = std::fs::read_to_string(&dot_git) {
                    if let Some(path) = content.strip_prefix("gitdir: ") {
                        let resolved = current.join(path.trim());
                        if resolved.is_dir() {
                            break resolved;
                        }
                    }
                }
            }
            if !current.pop() {
                return None;
            }
        };

        // Read HEAD
        let head_path = git_dir.join("HEAD");
        let head = std::fs::read_to_string(head_path).ok()?;
        let branch = if let Some(r) = head.strip_prefix("ref: refs/heads/") {
            r.trim().to_string()
        } else {
            // Detached HEAD — show short hash
            head.trim().chars().take(7).collect()
        };

        // Dirty check: run git status --porcelain (fast)
        let dirty = std::process::Command::new("git")
            .args(["status", "--porcelain", "-uno"])
            .current_dir(dir)
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        Some(GitInfo { branch, dirty })
    }
}

/// Snapshot for editor undo.
pub struct EditorSnapshot {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

/// In-app text editor state.
pub struct EditorState {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub dirty: bool,
    pub undo_stack: Vec<EditorSnapshot>,
    pub viewport_rows: usize,
    pub viewport_cols: usize,
    pub confirm_exit: bool,
}

impl EditorState {
    /// Open a file for editing. Rejects binary files and files > 1MB.
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let meta = std::fs::metadata(&path).map_err(|e| format!("ACCESS VIOLATION: {}", e))?;
        if meta.len() > 1_048_576 {
            return Err("ASSET EXCEEDS 1MB CLEARANCE THRESHOLD".to_string());
        }

        let bytes = std::fs::read(&path).map_err(|e| format!("DATA RETRIEVAL FAILURE: {}", e))?;

        // Binary detection: check first 512 bytes for null bytes
        let check_len = bytes.len().min(512);
        if bytes[..check_len].contains(&0) {
            return Err("BINARY ASSET \u{2014} DECODE RESTRICTED".to_string());
        }

        let content = String::from_utf8_lossy(&bytes);
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.replace('\t', "    ")).collect()
        };
        // Ensure at least one line
        let lines = if lines.is_empty() { vec![String::new()] } else { lines };

        Ok(Self {
            path,
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            dirty: false,
            undo_stack: Vec::new(),
            viewport_rows: 20,
            viewport_cols: 80,
            confirm_exit: false,
        })
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(EditorSnapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        // Cap undo stack
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(snap) = self.undo_stack.pop() {
            self.lines = snap.lines;
            self.cursor_row = snap.cursor_row;
            self.cursor_col = snap.cursor_col;
            self.clamp_cursor();
            self.ensure_cursor_visible();
            self.dirty = true;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        let byte_idx = char_to_byte(line, self.cursor_col);
        line.insert(byte_idx, c);
        self.cursor_col += 1;
        self.dirty = true;
        self.ensure_cursor_visible();
    }

    pub fn insert_newline(&mut self) {
        let line = &self.lines[self.cursor_row];
        let byte_idx = char_to_byte(line, self.cursor_col);
        let rest = line[byte_idx..].to_string();
        self.lines[self.cursor_row] = line[..byte_idx].to_string();
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.dirty = true;
        self.ensure_cursor_visible();
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte(line, self.cursor_col - 1);
            let next_byte = char_to_byte(line, self.cursor_col);
            line.drain(byte_idx..next_byte);
            self.cursor_col -= 1;
            self.dirty = true;
        } else if self.cursor_row > 0 {
            // Join with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].chars().count();
            self.lines[self.cursor_row].push_str(&current);
            self.dirty = true;
        }
        self.ensure_cursor_visible();
    }

    pub fn delete_char(&mut self) {
        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte(line, self.cursor_col);
            let next_byte = char_to_byte(line, self.cursor_col + 1);
            line.drain(byte_idx..next_byte);
            self.dirty = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Join with next line
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.dirty = true;
        }
    }

    pub fn delete_line(&mut self) {
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor_row);
            if self.cursor_row >= self.lines.len() {
                self.cursor_row = self.lines.len() - 1;
            }
        } else {
            self.lines[0].clear();
        }
        self.clamp_cursor();
        self.dirty = true;
        self.ensure_cursor_visible();
    }

    pub fn kill_to_eol(&mut self) {
        let byte_idx = char_to_byte(&self.lines[self.cursor_row], self.cursor_col);
        self.lines[self.cursor_row].truncate(byte_idx);
        self.dirty = true;
    }

    pub fn save(&mut self) -> Result<(), String> {
        let content = self.lines.join("\n");
        // Add trailing newline if the file has content
        let output = if content.is_empty() { content } else { format!("{}\n", content) };
        std::fs::write(&self.path, output).map_err(|e| format!("WRITE SEQUENCE ABORTED: {}", e))?;
        self.dirty = false;
        Ok(())
    }

    pub fn clamp_cursor(&mut self) {
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len().saturating_sub(1);
        }
        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    pub fn ensure_cursor_visible(&mut self) {
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        }
        if self.cursor_row >= self.scroll_row + self.viewport_rows {
            self.scroll_row = self.cursor_row - self.viewport_rows + 1;
        }
        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        }
        if self.cursor_col >= self.scroll_col + self.viewport_cols {
            self.scroll_col = self.cursor_col - self.viewport_cols + 1;
        }
    }
}

/// Convert a char-index to a byte-index in a string.
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Message from SHA-256 hash background thread (#20).
pub enum HashMessage {
    Progress(f64),
    Complete(String),
    Error(String),
}

/// Active hash computation.
pub struct HashOp {
    pub path: PathBuf,
    pub progress: f64,
    pub throbber: Throbber,
    pub receiver: mpsc::Receiver<HashMessage>,
}

/// A single entry in a disk usage scan result (#21).
pub struct DiskUsageEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

/// Completed disk usage scan data.
#[allow(dead_code)]
pub struct DiskUsageData {
    pub path: PathBuf,
    pub entries: Vec<DiskUsageEntry>,
    pub total_size: u64,
    pub total_items: u64,
}

/// Message from disk usage scan background thread.
#[allow(dead_code)]
pub enum DiskScanMessage {
    Progress(u64),
    Complete(DiskUsageData),
    Error(String),
}

/// Active disk scan operation.
#[allow(dead_code)]
pub struct DiskScanOp {
    pub dir_name: String,
    pub nodes: u64,
    pub throbber: Throbber,
    pub receiver: mpsc::Receiver<DiskScanMessage>,
}

/// An entry inside a zip archive (#19).
#[derive(Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// State for browsing inside an archive.
pub struct ArchiveContext {
    pub archive_path: PathBuf,
    pub internal_dir: String,
    pub all_entries: Vec<ArchiveEntry>,
}

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub is_symlink: bool,
    pub link_target: Option<String>,
    pub permissions: Option<String>,
    pub is_classified: bool,
}

/// Animation for file deletion corruption effect (#35).
#[allow(dead_code)]
pub struct PurgeAnim {
    pub entries: Vec<String>,
    pub tick: u16,
    pub done: bool,
}

/// Undo record for file operations (#53).
#[derive(Clone)]
pub struct UndoRecord {
    pub action: String,
    pub original_paths: Vec<PathBuf>,
    pub result_paths: Vec<PathBuf>,
}

/// Operations log entry (#43).
pub struct LogEntry {
    pub timestamp: String,
    pub action: String,
    pub path: String,
}

/// Operations log (#43).
pub struct OpsLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

impl OpsLog {
    pub fn new() -> Self {
        Self { entries: Vec::new(), max_entries: 100 }
    }

    pub fn push(&mut self, action: &str, path: &str) {
        let now = chrono_hms();
        self.entries.push(LogEntry {
            timestamp: now,
            action: action.to_string(),
            path: path.to_string(),
        });
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
}

/// Get current time as HH:MM:SS string.
fn chrono_hms() -> String {
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Tree node for tree view (#44).
#[allow(dead_code)]
pub struct TreeNode {
    pub entry: FsEntry,
    pub depth: usize,
    pub expanded: bool,
    pub children_loaded: bool,
}

/// Command mode state (#41).
pub struct CommandState {
    pub input: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    /// Tab-completion candidates (#49).
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,
    pub completion_prefix: String,
}

/// Layout hit-test areas for mouse support (#38).
#[derive(Default, Clone)]
#[allow(dead_code)]
pub struct LayoutAreas {
    pub list_area: Option<(u16, u16, u16, u16)>,     // x, y, w, h
    pub breadcrumb_area: Option<(u16, u16, u16, u16)>,
    /// Breadcrumb click targets: (start_x, end_x, path) for each visible segment (#48).
    pub breadcrumb_segments: Vec<(u16, u16, std::path::PathBuf)>,
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

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum SortMode {
    #[default]
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    DateNewest,
    DateOldest,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::NameAsc => Self::NameDesc,
            Self::NameDesc => Self::SizeDesc,
            Self::SizeDesc => Self::SizeAsc,
            Self::SizeAsc => Self::DateNewest,
            Self::DateNewest => Self::DateOldest,
            Self::DateOldest => Self::NameAsc,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::NameAsc => Self::DateOldest,
            Self::NameDesc => Self::NameAsc,
            Self::SizeDesc => Self::NameDesc,
            Self::SizeAsc => Self::SizeDesc,
            Self::DateNewest => Self::SizeAsc,
            Self::DateOldest => Self::DateNewest,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NameAsc => "NAME \u{2191}",
            Self::NameDesc => "NAME \u{2193}",
            Self::SizeAsc => "SIZE \u{2191}",
            Self::SizeDesc => "SIZE \u{2193}",
            Self::DateNewest => "DATE \u{2193}",
            Self::DateOldest => "DATE \u{2191}",
        }
    }
}

/// Request to open a file externally (handled by main loop).
pub enum OpenRequest {
    /// Open in $EDITOR
    Editor(PathBuf),
    /// Open with system default handler
    SystemDefault(PathBuf),
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
    pub symbols: SymbolSet,
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
    pub open_request: Option<OpenRequest>,
    pub sort_mode: SortMode,
    // Bulk rename state
    pub bulk_find: String,
    pub bulk_replace: String,
    pub bulk_field: u8,               // 0 = find, 1 = replace
    pub bulk_paths: Vec<PathBuf>,     // original paths of selected entries
    // Git info
    pub git_info: Option<GitInfo>,
    // In-app editor
    pub editor: Option<EditorState>,
    // Directory transition animation
    pub anim_frame: u8,               // 0 = idle, 1..=3 = active frame
    pub anim_tick: Instant,            // last frame advance time
    pub reduce_motion: bool,           // disable animations
    // Recursive search state
    pub rsearch_query: String,
    pub rsearch_paths: Vec<PathBuf>,          // all walked paths (relative)
    pub rsearch_results: Vec<(usize, i64)>,   // (index into rsearch_paths, score)
    pub rsearch_cursor: usize,
    pub rsearch_scroll: usize,
    // Border pulse (#18)
    pub border_pulse_tick: u32,
    // I/O activity throbber (#16)
    pub io_throbber: Throbber,
    pub io_flash_tick: u8,
    // I/O history for oscilloscope (#77)
    pub io_history: Vec<f32>,
    // Idle screen (#17)
    pub last_input: Instant,
    pub idle_active: bool,
    pub idle_locked: bool,
    // CRT glitch for cyan (#15)
    pub glitch_tick: u32,
    // SHA-256 hash (#20)
    pub last_hash: Option<(PathBuf, String)>,
    pub hash_op: Option<HashOp>,
    // Disk usage (#21)
    pub disk_usage: Option<DiskUsageData>,
    pub disk_scan: Option<DiskScanOp>,
    // Archive browsing (#19)
    pub archive: Option<ArchiveContext>,
    // Glitch effects toggle
    pub glitch_enabled: bool,
    // Phosphor trail for green CRT effect
    pub prev_cursor_pos: usize,
    pub phosphor_trail: Vec<(usize, u8)>, // (cursor index, fade frames remaining)
    // Idle screensaver throbber
    pub idle_throbber: Throbber,
    // Declassification animation (#36)
    pub declassify_tick: Option<u16>,
    // Per-palette blink stutter counter (#37)
    pub blink_stutter_counter: u32,
    // Low disk warning (#34)
    pub disk_warning: Option<String>,
    // Delete corruption animation (#35)
    pub purge_anim: Option<PurgeAnim>,
    // Operations log (#43)
    pub ops_log: OpsLog,
    // Operations log scroll
    pub ops_log_scroll: usize,
    // Symlink glyph display (computed at load time) (#42)
    // Tree view (#44)
    pub tree_mode: bool,
    pub tree_nodes: Vec<TreeNode>,
    // Dual-pane diff (#45)
    pub diff_mode: bool,
    pub diff_sets: Option<(std::collections::HashSet<String>, std::collections::HashSet<String>)>,
    // Mouse support (#38)
    pub mouse_enabled: bool,
    pub layout_areas: LayoutAreas,
    // Command mode (#41)
    pub command_state: CommandState,
    // Favorites / pinned directories (#54)
    pub favorites: Vec<PathBuf>,
    // Shell command output (#57)
    pub shell_output: Option<String>,
    // Undo stack (#53)
    pub undo_stack: Vec<UndoRecord>,
    // File tagging (#58)
    pub tags: crate::tags::TagStore,
    pub tag_input: String,
    // Comms intercept (#74)
    pub comms: crate::comms::CommsState,
}

impl App {
    pub fn new(start_dir: PathBuf, palette: Palette, symbols: SymbolSet) -> Self {
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
            symbols,
            should_quit: false,
            selected_path: None,
            last_dir_before_jump: None,
            heartbeat: Throbber::from_frames(symbols.heartbeat_frames, ThrobberKind::Heartbeat),
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
            open_request: None,
            sort_mode: SortMode::default(),
            bulk_find: String::new(),
            bulk_replace: String::new(),
            bulk_field: 0,
            bulk_paths: Vec::new(),
            git_info: None,
            editor: None,
            anim_frame: 0,
            anim_tick: Instant::now(),
            reduce_motion: false,
            rsearch_query: String::new(),
            rsearch_paths: Vec::new(),
            rsearch_results: Vec::new(),
            rsearch_cursor: 0,
            rsearch_scroll: 0,
            border_pulse_tick: 0,
            io_throbber: Throbber::new(ThrobberKind::DataStream, palette.variant),
            io_flash_tick: 0,
            io_history: vec![0.0; 40],
            last_input: Instant::now(),
            idle_active: false,
            idle_locked: false,
            glitch_tick: 0,
            last_hash: None,
            hash_op: None,
            disk_usage: None,
            disk_scan: None,
            archive: None,
            glitch_enabled: true,
            prev_cursor_pos: 0,
            phosphor_trail: Vec::new(),
            idle_throbber: Throbber::new(ThrobberKind::Idle, palette.variant),
            declassify_tick: None,
            blink_stutter_counter: 0,
            disk_warning: None,
            purge_anim: None,
            ops_log: OpsLog::new(),
            ops_log_scroll: 0,
            tree_mode: false,
            tree_nodes: Vec::new(),
            diff_mode: false,
            diff_sets: None,
            mouse_enabled: true,
            layout_areas: LayoutAreas::default(),
            command_state: CommandState {
                input: String::new(),
                cursor: 0,
                history: Vec::new(),
                history_idx: None,
                completions: Vec::new(),
                completion_idx: None,
                completion_prefix: String::new(),
            },
            favorites: Vec::new(),
            shell_output: None,
            undo_stack: Vec::new(),
            tags: crate::tags::TagStore::new(),
            tag_input: String::new(),
            comms: crate::comms::CommsState::new(),
        };
        app.load_entries();
        app.git_info = GitInfo::detect(&app.panes[0].current_dir);
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
        let blink_interval = self.palette.blink_interval_ms as u128;
        if now.duration_since(self.last_blink).as_millis() >= blink_interval {
            // Amber stutter: skip one toggle every 7th cycle (#37)
            let should_skip = if matches!(self.palette.variant, crate::throbber::PaletteVariant::Amber) {
                self.blink_stutter_counter += 1;
                self.blink_stutter_counter % 7 == 0
            } else {
                false
            };
            if !should_skip {
                self.blink_on = !self.blink_on;
            }
            self.last_blink = now;
        }
        // Declassification animation (#36)
        if let Some(ref mut tick) = self.declassify_tick {
            *tick += 1;
            if *tick > 5 {
                self.declassify_tick = None;
            }
        }
        // Purge animation (#35)
        let mut clear_purge = false;
        if let Some(ref mut anim) = self.purge_anim {
            anim.tick += 1;
            if anim.tick > 8 {
                anim.done = true;
                clear_purge = true;
            }
        }
        if clear_purge {
            self.purge_anim = None;
        }
        // Low disk warning (#34)
        if self.show_telemetry {
            if let Some(mon) = &self.sysmon {
                let mut warning = None;
                for disk in &mon.disk_info {
                    if disk.total > 0 {
                        let pct = (disk.used as f64 / disk.total as f64 * 100.0) as u64;
                        if pct >= 90 {
                            warning = Some(format!(
                                "STORAGE CRITICAL \u{2014} {} AT {}% \u{2014} PURGE NON-ESSENTIAL ASSETS",
                                disk.mount, pct
                            ));
                            break;
                        }
                    }
                }
                self.disk_warning = warning;
            }
        } else {
            self.disk_warning = None;
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
        // CPU-modulated heartbeat: speed up under load when telemetry is active
        if let Some(mon) = &self.sysmon {
            let cpu = mon.cpu_pct;
            if cpu > 80.0 {
                self.heartbeat.extra_ticks(2);
            } else if cpu > 50.0 {
                self.heartbeat.extra_ticks(1);
            }
        }
        // Idle screensaver throbber
        self.idle_throbber.tick();
        // Border pulse (#18)
        self.border_pulse_tick = self.border_pulse_tick.wrapping_add(1);
        // I/O flash countdown (#16)
        if self.io_flash_tick > 0 {
            self.io_throbber.tick();
            self.io_flash_tick = self.io_flash_tick.saturating_sub(1);
        }
        // I/O history for oscilloscope (#77)
        let io_val = (self.io_flash_tick as f32) / 5.0;
        self.io_history.push(io_val);
        if self.io_history.len() > 40 {
            self.io_history.remove(0);
        }
        // Idle detection (#17) — don't override manual lock
        if !self.idle_locked {
            self.idle_active = now.duration_since(self.last_input).as_secs() >= 45;
        }
        // Comms intercept (#74)
        let idle_secs = now.duration_since(self.last_input).as_secs();
        self.comms.tick(idle_secs);
        // CRT glitch (#15)
        self.glitch_tick = self.glitch_tick.wrapping_add(1);
        // Green phosphor trail: track cursor movement, decay ghosts
        let current_cursor = self.pane().cursor;
        if current_cursor != self.prev_cursor_pos {
            if self.prev_cursor_pos < self.pane().filtered_indices.len() {
                self.phosphor_trail.push((self.prev_cursor_pos, 6));
                if self.phosphor_trail.len() > 4 {
                    self.phosphor_trail.remove(0);
                }
            }
            self.prev_cursor_pos = current_cursor;
        }
        for ghost in self.phosphor_trail.iter_mut() {
            ghost.1 = ghost.1.saturating_sub(1);
        }
        self.phosphor_trail.retain(|g| g.1 > 0);
        // Hash progress polling (#20)
        let mut hash_done = false;
        if let Some(hop) = &mut self.hash_op {
            hop.throbber.tick();
            while let Ok(msg) = hop.receiver.try_recv() {
                match msg {
                    HashMessage::Progress(p) => hop.progress = p,
                    HashMessage::Complete(hash) => {
                        self.last_hash = Some((hop.path.clone(), hash));
                        hash_done = true;
                    }
                    HashMessage::Error(e) => {
                        self.error = Some((format!("HASH VERIFICATION FAILURE: {}", e), Instant::now()));
                        hash_done = true;
                    }
                }
            }
        }
        if hash_done { self.hash_op = None; }
        // Disk scan polling (#21)
        let mut scan_done = false;
        if let Some(ds) = &mut self.disk_scan {
            ds.throbber.tick();
            while let Ok(msg) = ds.receiver.try_recv() {
                match msg {
                    DiskScanMessage::Progress(n) => ds.nodes = n,
                    DiskScanMessage::Complete(data) => {
                        self.disk_usage = Some(data);
                        self.right_panel = RightPanel::DiskUsage;
                        scan_done = true;
                    }
                    DiskScanMessage::Error(e) => {
                        self.error = Some((format!("SCAN SEQUENCE FAILURE: {}", e), Instant::now()));
                        scan_done = true;
                    }
                }
            }
        }
        if scan_done { self.disk_scan = None; }
        // Directory transition animation
        if self.anim_frame > 0 && now.duration_since(self.anim_tick).as_millis() >= 70 {
            self.anim_frame += 1;
            self.anim_tick = now;
            if self.anim_frame > 3 {
                self.anim_frame = 0;
            }
        }
    }

    /// Compute the pulsed border color for the active pane (#18).
    pub fn pulsed_border(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        let phase = (self.border_pulse_tick % 20) as f32 / 20.0;
        let t = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        match (self.palette.border_mid, self.palette.border_hot) {
            (Color::Rgb(mr, mg, mb), Color::Rgb(hr, hg, hb)) => Color::Rgb(
                (mr as f32 + (hr as f32 - mr as f32) * t) as u8,
                (mg as f32 + (hg as f32 - mg as f32) * t) as u8,
                (mb as f32 + (hb as f32 - mb as f32) * t) as u8,
            ),
            _ => self.palette.border_hot,
        }
    }

    /// Pseudo-random value for CRT glitch effects (#15).
    pub fn glitch_rand(&self, index: u32) -> u32 {
        let mut h = self.glitch_tick.wrapping_mul(2654435761).wrapping_add(index);
        h ^= h >> 16;
        h = h.wrapping_mul(0x45d9f3b);
        h ^= h >> 16;
        h
    }

    /// Populate pane entries from archive context (#19).
    pub fn populate_archive_entries(&mut self) {
        if let Some(archive) = &self.archive {
            let visible = archive_ls(&archive.all_entries, &archive.internal_dir);
            let pane = self.pane_mut();
            pane.entries = visible.iter().map(|ae| FsEntry {
                name: ae.name.clone(),
                path: PathBuf::from(&ae.full_path),
                is_dir: ae.is_dir,
                size: if ae.is_dir { None } else { Some(ae.size) },
                modified: None,
                is_symlink: false,
                link_target: None,
                permissions: None,
                is_classified: false,
            }).collect();
            pane.cursor = 0;
            pane.scroll_offset = 0;
            pane.fuzzy_query.clear();
        }
        self.rebuild_filtered();
    }
}

/// Extract visible entries for a given directory within an archive.
fn archive_ls(entries: &[ArchiveEntry], dir: &str) -> Vec<ArchiveEntry> {
    let mut result = Vec::new();
    let mut seen_dirs = std::collections::HashSet::new();

    for entry in entries {
        let path = &entry.full_path;
        if !path.starts_with(dir) {
            continue;
        }
        let relative = &path[dir.len()..];
        if relative.is_empty() {
            continue;
        }

        if let Some(slash_pos) = relative.find('/') {
            let subdir_name = &relative[..slash_pos];
            if !subdir_name.is_empty() && seen_dirs.insert(subdir_name.to_string()) {
                result.push(ArchiveEntry {
                    name: subdir_name.to_string(),
                    full_path: format!("{}{}/", dir, subdir_name),
                    is_dir: true,
                    size: 0,
                });
            }
        } else {
            result.push(ArchiveEntry {
                name: relative.to_string(),
                full_path: entry.full_path.clone(),
                is_dir: false,
                size: entry.size,
            });
        }
    }

    result.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    result
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
pub fn icon_for(entry: &FsEntry, symbols: &SymbolSet) -> &'static str {
    if !symbols.use_nerd_fonts {
        return if entry.is_dir { symbols.dir_icon } else { symbols.file_icon };
    }
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
