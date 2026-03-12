use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::{App, RightPanel, SortMode};

#[derive(Serialize, Deserialize)]
struct SessionFile {
    left_dir: String,
    right_dir: String,
    dual_pane: bool,
    active_pane: usize,
    right_panel: String,
    sort_mode: String,
    show_hidden: bool,
    show_telemetry: bool,
}

pub struct Session {
    pub left_dir: PathBuf,
    pub right_dir: PathBuf,
    pub dual_pane: bool,
    pub active_pane: usize,
    pub right_panel: RightPanel,
    pub sort_mode: SortMode,
    pub show_hidden: bool,
    pub show_telemetry: bool,
}

fn session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("session.toml"))
}

fn right_panel_to_str(rp: RightPanel) -> &'static str {
    match rp {
        RightPanel::Info => "info",
        RightPanel::Preview => "preview",
        RightPanel::Hidden => "hidden",
        RightPanel::DiskUsage => "disk_usage",
    }
}

fn str_to_right_panel(s: &str) -> RightPanel {
    match s {
        "preview" => RightPanel::Preview,
        "hidden" => RightPanel::Hidden,
        "disk_usage" => RightPanel::DiskUsage,
        _ => RightPanel::Info,
    }
}

fn sort_mode_to_str(sm: SortMode) -> &'static str {
    match sm {
        SortMode::NameAsc => "name_asc",
        SortMode::NameDesc => "name_desc",
        SortMode::SizeAsc => "size_asc",
        SortMode::SizeDesc => "size_desc",
        SortMode::DateNewest => "date_newest",
        SortMode::DateOldest => "date_oldest",
    }
}

fn str_to_sort_mode(s: &str) -> SortMode {
    match s {
        "name_desc" => SortMode::NameDesc,
        "size_asc" => SortMode::SizeAsc,
        "size_desc" => SortMode::SizeDesc,
        "date_newest" => SortMode::DateNewest,
        "date_oldest" => SortMode::DateOldest,
        _ => SortMode::NameAsc,
    }
}

pub fn load_session() -> Option<Session> {
    let path = session_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let file: SessionFile = toml::from_str(&content).ok()?;
    Some(Session {
        left_dir: PathBuf::from(&file.left_dir),
        right_dir: PathBuf::from(&file.right_dir),
        dual_pane: file.dual_pane,
        active_pane: file.active_pane,
        right_panel: str_to_right_panel(&file.right_panel),
        sort_mode: str_to_sort_mode(&file.sort_mode),
        show_hidden: file.show_hidden,
        show_telemetry: file.show_telemetry,
    })
}

pub fn save_session(app: &App) {
    let Some(path) = session_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = SessionFile {
        left_dir: app.panes[0].current_dir.to_string_lossy().into_owned(),
        right_dir: app.panes[1].current_dir.to_string_lossy().into_owned(),
        dual_pane: app.dual_pane,
        active_pane: app.active_pane,
        right_panel: right_panel_to_str(app.right_panel).to_string(),
        sort_mode: sort_mode_to_str(app.sort_mode).to_string(),
        show_hidden: app.show_hidden,
        show_telemetry: app.show_telemetry,
    };
    if let Ok(content) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, content);
    }
}

pub fn apply_session(app: &mut App, session: Session) {
    // Only restore directories that still exist
    if session.left_dir.is_dir() {
        app.panes[0].current_dir = session.left_dir;
    }
    if session.right_dir.is_dir() {
        app.panes[1].current_dir = session.right_dir;
    }
    app.dual_pane = session.dual_pane;
    app.active_pane = if session.active_pane <= 1 { session.active_pane } else { 0 };
    app.right_panel = session.right_panel;
    app.sort_mode = session.sort_mode;
    app.show_hidden = session.show_hidden;
    app.show_telemetry = session.show_telemetry;

    // Reload entries with restored state
    app.load_entries();
}
