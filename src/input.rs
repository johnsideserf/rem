use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode, OpBuffer, OpType, PendingAction, JUMP_KEYS};
use crate::sysmon::SysMon;
use crate::throbber::{Throbber, ThrobberKind};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Cancel any running animation on input
    if app.anim_frame > 0 {
        app.anim_frame = 0;
    }

    // Dismiss comms on input (#74)
    app.comms.dismiss();

    // Dismiss error/feedback on any key (but don't consume the keypress)
    if app.error.is_some() {
        app.error = None;
    }

    // Theme picker intercepts all input when open
    if app.show_theme_picker {
        handle_theme_picker(app, key);
        return;
    }

    match &app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::FuzzySearch => handle_fuzzy(app, key),
        Mode::JumpKey => handle_jump(app, key),
        Mode::Visual => handle_visual(app, key),
        Mode::Rename => handle_rename(app, key),
        Mode::Create { .. } => handle_create(app, key),
        Mode::Confirm { .. } => handle_confirm(app, key),
        Mode::WaitingForG => handle_waiting_g(app, key),
        Mode::WaitingForMark => handle_set_mark(app, key),
        Mode::WaitingForJumpToMark => handle_jump_mark(app, key),
        Mode::WaitingForYank => handle_waiting_yank(app, key),
        Mode::WaitingForCut => handle_waiting_cut(app, key),
        Mode::WaitingForDeleteMark => handle_delete_mark(app, key),
        Mode::RecursiveSearch => handle_rsearch(app, key),
        Mode::BulkRename => handle_bulk_rename(app, key),
        Mode::Edit => handle_edit(app, key),
        Mode::OpsLog => handle_ops_log(app, key),
        Mode::Command => handle_command(app, key),
        Mode::FileDiff => handle_diff(app, key),
        Mode::TagInput => handle_tag_input(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Archive mode: restrict to navigation-only keys (#19)
    if app.archive.is_some() {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                app.exit_archive();
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                app.cursor_down();
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                app.cursor_up();
            }
            (KeyModifiers::NONE, KeyCode::Char('l'))
            | (KeyModifiers::NONE, KeyCode::Right)
            | (KeyModifiers::NONE, KeyCode::Enter) => {
                app.enter_selected();
            }
            (KeyModifiers::NONE, KeyCode::Char('h'))
            | (KeyModifiers::NONE, KeyCode::Left) => {
                app.go_parent();
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                app.mode = Mode::WaitingForG;
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                app.jump_bottom();
            }
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                app.mode = Mode::FuzzySearch;
                app.pane_mut().fuzzy_query.clear();
                app.rebuild_filtered();
            }
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                app.mode = Mode::JumpKey;
            }
            _ => {}
        }
        return;
    }

    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            app.should_quit = true;
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            if app.tree_mode {
                let max = app.tree_nodes.len();
                let pane = app.pane_mut();
                if pane.cursor + 1 < max { pane.cursor += 1; }
            } else {
                app.cursor_down();
            }
            app.preview_scroll = 0;
            app.declassify_tick = Some(0);
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            if app.tree_mode {
                let pane = app.pane_mut();
                if pane.cursor > 0 { pane.cursor -= 1; }
            } else {
                app.cursor_up();
            }
            app.preview_scroll = 0;
            app.declassify_tick = Some(0);
        }
        (KeyModifiers::NONE, KeyCode::Char('l'))
        | (KeyModifiers::NONE, KeyCode::Right)
        | (KeyModifiers::NONE, KeyCode::Enter) => {
            if app.tree_mode {
                tree_toggle_expand(app);
            } else {
                app.enter_selected();
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('h'))
        | (KeyModifiers::NONE, KeyCode::Left)
        | (KeyModifiers::NONE, KeyCode::Char('-')) => {
            if app.tree_mode {
                tree_collapse_or_parent(app);
            } else {
                app.go_parent();
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            app.mode = Mode::WaitingForG;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.jump_bottom();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.scroll_half_up();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.scroll_half_down();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            app.nav_back();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('i')) => {
            app.nav_forward();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
            app.preview_scroll = app.preview_scroll.saturating_add(3);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            app.preview_scroll = app.preview_scroll.saturating_sub(3);
        }
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            app.mode = Mode::FuzzySearch;
            app.pane_mut().fuzzy_query.clear();
            app.rebuild_filtered();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('?')) => {
            app.rsearch_walk();
            app.mode = Mode::RecursiveSearch;
        }
        (KeyModifiers::NONE, KeyCode::Char(' ')) => {
            app.mode = Mode::JumpKey;
        }
        (KeyModifiers::NONE, KeyCode::Char('m')) => {
            app.mode = Mode::WaitingForMark;
        }
        (KeyModifiers::NONE, KeyCode::Char('\'')) => {
            app.mode = Mode::WaitingForJumpToMark;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('M')) => {
            app.mode = Mode::WaitingForDeleteMark;
        }
        (KeyModifiers::NONE, KeyCode::Tab) => {
            if app.dual_pane {
                // Switch active pane
                app.active_pane = 1 - app.active_pane;
                app.preview_scroll = 0;
            } else {
                app.right_panel = app.right_panel.cycle();
                app.preview_scroll = 0;
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.dual_pane = !app.dual_pane;
            if app.dual_pane {
                // Initialize second pane to same directory if it's still at startup
                let dir = app.panes[0].current_dir.clone();
                if app.panes[1].entries.is_empty() {
                    app.panes[1].current_dir = dir;
                    let old = app.active_pane;
                    app.active_pane = 1;
                    app.load_entries();
                    app.active_pane = old;
                }
            }
        }
        // Multi-tab workspaces (#81)
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            app.new_tab();
        }
        (KeyModifiers::ALT, KeyCode::Left) => {
            app.switch_tab(-1);
        }
        (KeyModifiers::ALT, KeyCode::Right) => {
            app.switch_tab(1);
        }
        // File operations
        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            app.mode = Mode::WaitingForYank;
        }
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            app.mode = Mode::WaitingForCut;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('D')) => {
            // Delete current entry or marked entries
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.mode = Mode::Confirm { action: PendingAction::Delete { paths } };
                app.confirm_timer = Some(Instant::now());
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('p')) => {
            app.paste();
        }
        (KeyModifiers::NONE, KeyCode::Char('v')) => {
            // Toggle mark on current entry and move down
            toggle_mark_at_cursor(app);
            app.cursor_down();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('V')) => {
            app.mode = Mode::Visual;
            toggle_mark_at_cursor(app);
        }
        (KeyModifiers::NONE, KeyCode::Char('u')) => {
            app.visual_marks.clear();
        }
        (KeyModifiers::NONE, KeyCode::Char('r')) => {
            if let Some(entry) = app.current_entry() {
                app.rename_buf = entry.name.clone();
                app.mode = Mode::Rename;
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('o')) => {
            app.create_buf.clear();
            app.mode = Mode::Create { is_dir: false };
        }
        (KeyModifiers::SHIFT, KeyCode::Char('O')) => {
            app.create_buf.clear();
            app.mode = Mode::Create { is_dir: true };
        }
        (KeyModifiers::NONE, KeyCode::Char('e')) => {
            app.edit_selected();
        }
        (KeyModifiers::SHIFT, KeyCode::Char('E')) => {
            app.edit_external();
        }
        (KeyModifiers::NONE, KeyCode::Char('s')) => {
            app.sort_mode = app.sort_mode.next();
            app.load_entries();
            crate::config::save_sort_mode(app.sort_mode);
        }
        (KeyModifiers::SHIFT, KeyCode::Char('S')) => {
            app.sort_mode = app.sort_mode.prev();
            app.load_entries();
            crate::config::save_sort_mode(app.sort_mode);
        }
        (KeyModifiers::SHIFT, KeyCode::Char('H')) => {
            app.show_hidden = !app.show_hidden;
            app.rebuild_filtered();
        }
        (KeyModifiers::NONE, KeyCode::Char('`')) => {
            app.show_telemetry = !app.show_telemetry;
            if app.show_telemetry && app.sysmon.is_none() {
                app.sysmon = Some(SysMon::new());
                app.telemetry_throbber = Some(Throbber::new(
                    ThrobberKind::Processing,
                    app.palette.variant,
                ));
            }
        }
        // Sidebar resize
        (KeyModifiers::NONE, KeyCode::Char('[')) => {
            app.sidebar_pct = app.sidebar_pct.saturating_sub(3).max(10);
        }
        (KeyModifiers::NONE, KeyCode::Char(']')) => {
            app.sidebar_pct = (app.sidebar_pct + 3).min(60);
        }
        // SHA-256 hash (#20)
        (KeyModifiers::SHIFT, KeyCode::Char('#'))
        | (KeyModifiers::NONE, KeyCode::Char('#')) => {
            app.hash_selected();
        }
        // Disk usage scan (#21)
        (KeyModifiers::SHIFT, KeyCode::Char('W')) => {
            app.scan_disk_usage();
        }
        // Lock screen — activate idle screensaver immediately
        (KeyModifiers::SHIFT, KeyCode::Char('L')) => {
            app.idle_active = true;
            app.idle_locked = true;
        }
        // Theme picker
        (KeyModifiers::NONE, KeyCode::Char('t')) => {
            app.show_theme_picker = true;
            // Set cursor to current theme
            app.theme_picker_cursor = match app.palette.variant {
                crate::throbber::PaletteVariant::Green => 0,
                crate::throbber::PaletteVariant::Amber => 1,
                crate::throbber::PaletteVariant::Cyan => 2,
            };
        }
        // Clipboard yank (#39)
        (KeyModifiers::SHIFT, KeyCode::Char('Y')) => {
            clipboard_yank(app);
        }
        // Operations log (#43)
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
            app.ops_log_scroll = 0;
            app.mode = Mode::OpsLog;
        }
        // Undo last file operation (#53)
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            app.undo_last();
        }
        // Command mode (#41)
        (KeyModifiers::SHIFT, KeyCode::Char(':'))
        | (KeyModifiers::NONE, KeyCode::Char(':')) => {
            app.command_state.input.clear();
            app.command_state.cursor = 0;
            app.command_state.history_idx = None;
            app.mode = Mode::Command;
        }
        // File tagging (#58)
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            if app.current_entry().is_some() {
                app.tag_input.clear();
                app.mode = Mode::TagInput;
            }
        }
        // Tree view toggle (#44)
        (KeyModifiers::SHIFT, KeyCode::Char('T')) => {
            app.tree_mode = !app.tree_mode;
            if app.tree_mode {
                build_tree(app);
            }
        }
        // Favorites toggle (#54)
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            let dir = app.pane().current_dir.clone();
            if let Some(pos) = app.favorites.iter().position(|p| *p == dir) {
                app.favorites.remove(pos);
                app.error = Some(("FAVORITE REMOVED".to_string(), Instant::now()));
            } else {
                app.favorites.push(dir);
                app.error = Some(("FAVORITE ADDED".to_string(), Instant::now()));
            }
            crate::favorites::save_favorites(&app.favorites);
        }
        // Quick jump to favorite (#54)
        (KeyModifiers::ALT, KeyCode::Char(c)) if c >= '1' && c <= '9' => {
            let idx = (c as usize) - ('1' as usize);
            if let Some(dir) = app.favorites.get(idx).cloned() {
                if dir.is_dir() {
                    app.navigate_to(dir);
                } else {
                    app.error = Some(("FAVORITE PATH NO LONGER EXISTS".to_string(), Instant::now()));
                }
            }
        }
        // Git stage/unstage (#82)
        (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
            if let Some(entry) = app.current_entry() {
                let name = entry.name.clone();
                let dir = app.pane().current_dir.clone();
                if let Some(status) = app.git_file_statuses.get(&name) {
                    let result = match status {
                        crate::gitstatus::GitFileStatus::Staged => crate::gitstatus::git_unstage(&dir, &name),
                        _ => crate::gitstatus::git_stage(&dir, &name),
                    };
                    match result {
                        Ok(()) => {
                            app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                            app.error = Some(("GIT STATUS UPDATED".to_string(), Instant::now()));
                        }
                        Err(e) => app.error = Some((e, Instant::now())),
                    }
                } else {
                    match crate::gitstatus::git_stage(&dir, &name) {
                        Ok(()) => {
                            app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                            app.error = Some(("STAGED".to_string(), Instant::now()));
                        }
                        Err(e) => app.error = Some((e, Instant::now())),
                    }
                }
            }
        }
        // Preview minimap toggle (#86)
        (KeyModifiers::CONTROL, KeyCode::Char('m')) => {
            app.show_minimap = !app.show_minimap;
        }
        // Dual-pane diff toggle (#45)
        (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
            if app.dual_pane {
                app.diff_mode = !app.diff_mode;
                if app.diff_mode {
                    compute_diff(app);
                } else {
                    app.diff_sets = None;
                }
            }
        }
        _ => {}
    }
}

/// Copy current entry's full path to system clipboard (#39).
fn clipboard_yank(app: &mut App) {
    let path_str = match app.current_entry() {
        Some(e) => e.path.to_string_lossy().into_owned(),
        None => return,
    };

    let result = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", &format!("echo {}| clip", path_str)])
            .output()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("sh")
            .args(["-c", &format!("printf '%s' '{}' | pbcopy", path_str)])
            .output()
    } else {
        std::process::Command::new("sh")
            .args(["-c", &format!("printf '%s' '{}' | xclip -selection clipboard 2>/dev/null || printf '%s' '{}' | xsel --clipboard", path_str, path_str)])
            .output()
    };

    match result {
        Ok(_) => {
            app.error = Some(("PATH COPIED TO CLIPBOARD".to_string(), Instant::now()));
        }
        Err(_) => {
            app.error = Some(("CLIPBOARD ACCESS DENIED".to_string(), Instant::now()));
        }
    }
}

/// Build initial tree from current directory (#44).
fn build_tree(app: &mut App) {
    let nodes: Vec<crate::app::TreeNode> = {
        let pane = app.pane();
        pane.filtered_indices.iter().map(|&i| {
            let entry = &pane.entries[i];
            crate::app::TreeNode {
                entry: crate::app::FsEntry {
                    name: entry.name.clone(),
                    path: entry.path.clone(),
                    is_dir: entry.is_dir,
                    size: entry.size,
                    modified: entry.modified,
                    is_symlink: entry.is_symlink,
                    link_target: entry.link_target.clone(),
                    permissions: entry.permissions.clone(),
                    is_classified: entry.is_classified,
                },
                depth: 0,
                expanded: false,
                children_loaded: false,
            }
        }).collect()
    };
    app.tree_nodes = nodes;
}

/// Toggle expand/collapse on current tree node (#44).
fn tree_toggle_expand(app: &mut App) {
    let cursor = app.pane().cursor;
    if cursor >= app.tree_nodes.len() {
        return;
    }

    if !app.tree_nodes[cursor].entry.is_dir {
        // For files, open as usual
        app.enter_selected();
        return;
    }

    if app.tree_nodes[cursor].expanded {
        // Collapse: remove all children (nodes with depth > current that follow consecutively)
        let depth = app.tree_nodes[cursor].depth;
        let remove_start = cursor + 1;
        let mut remove_end = remove_start;
        while remove_end < app.tree_nodes.len() && app.tree_nodes[remove_end].depth > depth {
            remove_end += 1;
        }
        app.tree_nodes.drain(remove_start..remove_end);
        app.tree_nodes[cursor].expanded = false;
    } else {
        // Expand: load children (max depth 10)
        let depth = app.tree_nodes[cursor].depth;
        if depth >= 10 {
            return;
        }
        let dir_path = app.tree_nodes[cursor].entry.path.clone();
        let show_hidden = app.show_hidden;
        let mut children: Vec<crate::app::TreeNode> = Vec::new();

        if let Ok(rd) = std::fs::read_dir(&dir_path) {
            let mut entries: Vec<crate::app::FsEntry> = Vec::new();
            for de in rd.flatten() {
                let name = de.file_name().to_string_lossy().into_owned();
                if !show_hidden && name.starts_with('.') {
                    continue;
                }
                let meta = std::fs::symlink_metadata(de.path());
                let (is_dir, size, modified, is_symlink, link_target) = match &meta {
                    Ok(m) => {
                        let is_sym = m.file_type().is_symlink();
                        let lt = if is_sym {
                            std::fs::read_link(de.path()).ok().map(|p| p.to_string_lossy().into_owned())
                        } else {
                            None
                        };
                        let real_meta = std::fs::metadata(de.path());
                        let is_d = real_meta.as_ref().map_or(m.is_dir(), |rm| rm.is_dir());
                        let sz = if is_d { None } else { Some(real_meta.as_ref().map_or(m.len(), |rm| rm.len())) };
                        let mt = real_meta.as_ref().ok().and_then(|rm| rm.modified().ok());
                        (is_d, sz, mt, is_sym, lt)
                    }
                    Err(_) => (false, None, None, false, None),
                };
                entries.push(crate::app::FsEntry {
                    name,
                    path: de.path(),
                    is_dir,
                    size,
                    modified,
                    is_symlink,
                    link_target,
                    permissions: None,
                    is_classified: false,
                });
            }
            // Sort: dirs first, then case-insensitive name
            entries.sort_by(|a, b| {
                b.is_dir.cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            for e in entries {
                children.push(crate::app::TreeNode {
                    entry: e,
                    depth: depth + 1,
                    expanded: false,
                    children_loaded: false,
                });
            }
        }

        // Insert children after current node
        let insert_pos = cursor + 1;
        for (i, child) in children.into_iter().enumerate() {
            app.tree_nodes.insert(insert_pos + i, child);
        }
        app.tree_nodes[cursor].expanded = true;
        app.tree_nodes[cursor].children_loaded = true;
    }
}

/// Collapse current node or jump to parent in tree (#44).
fn tree_collapse_or_parent(app: &mut App) {
    let cursor = app.pane().cursor;
    if cursor >= app.tree_nodes.len() {
        return;
    }

    if app.tree_nodes[cursor].expanded {
        // Collapse this node
        tree_toggle_expand(app);
    } else if app.tree_nodes[cursor].depth > 0 {
        // Jump to parent node
        let target_depth = app.tree_nodes[cursor].depth - 1;
        for i in (0..cursor).rev() {
            if app.tree_nodes[i].depth == target_depth {
                app.pane_mut().cursor = i;
                break;
            }
        }
    } else {
        // At root level, exit tree and go parent
        app.tree_mode = false;
        app.tree_nodes.clear();
        app.go_parent();
    }
}

/// Compute diff sets for dual-pane mode (#45).
fn compute_diff(app: &mut App) {
    let left_names: std::collections::HashSet<String> = app.panes[0].entries.iter()
        .map(|e| e.name.clone())
        .collect();
    let right_names: std::collections::HashSet<String> = app.panes[1].entries.iter()
        .map(|e| e.name.clone())
        .collect();
    app.diff_sets = Some((left_names, right_names));
}

fn handle_waiting_g(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('g') => {
            app.jump_top();
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
}

fn handle_set_mark(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() {
            app.set_mark(c);
        }
    }
    app.mode = Mode::Normal;
}

fn handle_jump_mark(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() || c == '\'' {
            app.jump_to_mark(c);
        }
    }
    app.mode = Mode::Normal;
}

fn handle_delete_mark(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() {
            if app.marks.remove(&c).is_some() {
                crate::marks::save_marks(&app.marks);
            } else {
                app.error = Some((format!("MARK '{}' \u{2014} DESIGNATION NOT REGISTERED", c), std::time::Instant::now()));
            }
        }
    }
    app.mode = Mode::Normal;
}

fn handle_waiting_yank(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('y') {
        let paths = collect_operation_paths(app);
        if !paths.is_empty() {
            app.op_buffer = Some(OpBuffer { paths, op: OpType::Copy });
        }
    }
    app.mode = Mode::Normal;
}

fn handle_waiting_cut(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('d') => {
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.op_buffer = Some(OpBuffer { paths, op: OpType::Cut });
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
}

fn handle_fuzzy(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
            {
                let pane = app.pane_mut();
                pane.fuzzy_query.clear();
                pane.cursor = 0;
                pane.scroll_offset = 0;
            }
            app.rebuild_filtered();
        }
        (_, KeyCode::Enter) => {
            app.mode = Mode::Normal;
            if !app.pane().filtered_indices.is_empty() {
                app.enter_selected();
            }
        }
        (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            app.cursor_down();
        }
        (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            app.cursor_up();
        }
        (_, KeyCode::Backspace) => {
            {
                let pane = app.pane_mut();
                pane.fuzzy_query.pop();
                pane.cursor = 0;
                pane.scroll_offset = 0;
            }
            app.rebuild_filtered();
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            {
                let pane = app.pane_mut();
                pane.fuzzy_query.push(c);
                pane.cursor = 0;
                pane.scroll_offset = 0;
            }
            app.rebuild_filtered();
        }
        _ => {}
    }
}

fn handle_jump(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if c.is_ascii_lowercase() => {
            if let Some(pos) = JUMP_KEYS.iter().position(|&k| k == c) {
                if pos < app.pane().filtered_indices.len() {
                    app.pane_mut().cursor = pos;
                    app.enter_selected();
                }
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
}

fn handle_visual(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.cursor_down();
            toggle_mark_at_cursor(app);
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.cursor_up();
            toggle_mark_at_cursor(app);
        }
        (KeyModifiers::NONE, KeyCode::Char('v')) => {
            toggle_mark_at_cursor(app);
        }
        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.op_buffer = Some(OpBuffer { paths, op: OpType::Copy });
            }
            app.mode = Mode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.op_buffer = Some(OpBuffer { paths, op: OpType::Cut });
            }
            app.mode = Mode::Normal;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('D')) => {
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.mode = Mode::Confirm { action: PendingAction::Delete { paths } };
                app.confirm_timer = Some(Instant::now());
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('R')) => {
            let paths = collect_operation_paths(app);
            if !paths.is_empty() {
                app.bulk_paths = paths;
                app.bulk_find.clear();
                app.bulk_replace.clear();
                app.bulk_field = 0;
                app.mode = Mode::BulkRename;
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('u')) => {
            app.visual_marks.clear();
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_rename(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Enter) => {
            app.do_rename();
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Backspace) => {
            app.rename_buf.pop();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.rename_buf.clear();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            // Delete last word
            let trimmed = app.rename_buf.trim_end();
            if let Some(pos) = trimmed.rfind(|c: char| c == ' ' || c == '.' || c == '/') {
                app.rename_buf.truncate(pos);
            } else {
                app.rename_buf.clear();
            }
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            app.rename_buf.push(c);
        }
        _ => {}
    }
}

fn handle_create(app: &mut App, key: KeyEvent) {
    let is_dir = matches!(app.mode, Mode::Create { is_dir: true });
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Enter) => {
            app.do_create(is_dir);
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Backspace) => {
            app.create_buf.pop();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.create_buf.clear();
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            app.create_buf.push(c);
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    if let Mode::Confirm { action } = &app.mode {
        let action = action.clone();
        if key.code == KeyCode::Char('y') {
            // Start purge animation for delete actions (#35)
            if let crate::app::PendingAction::Delete { ref paths } = action {
                let names: Vec<String> = paths.iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect();
                app.purge_anim = Some(crate::app::PurgeAnim {
                    entries: names,
                    tick: 0,
                    done: false,
                });
            }
            app.execute_confirmed(&action);
        }
        // Any key (including 'y' after execution) exits confirm mode
        app.mode = Mode::Normal;
        app.confirm_timer = None;
    }
}

// -- Helpers --

fn toggle_mark_at_cursor(app: &mut App) {
    let pane = app.pane();
    if let Some(&idx) = pane.filtered_indices.get(pane.cursor) {
        if app.visual_marks.contains(&idx) {
            app.visual_marks.remove(&idx);
        } else {
            app.visual_marks.insert(idx);
        }
    }
}

fn handle_theme_picker(app: &mut App, key: KeyEvent) {
    let total = crate::ui::theme_picker::total_picker_items();
    let color_count = crate::ui::theme_picker::THEME_COUNT;
    match key.code {
        KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('q') => {
            app.show_theme_picker = false;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.theme_picker_cursor = (app.theme_picker_cursor + 1) % total;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.theme_picker_cursor = app.theme_picker_cursor
                .checked_sub(1)
                .unwrap_or(total - 1);
        }
        KeyCode::Enter => {
            if app.theme_picker_cursor < color_count {
                // Color theme selection
                let new_palette = crate::ui::theme_picker::palette_for_index(
                    app.theme_picker_cursor,
                );
                app.palette = new_palette;
                app.heartbeat = Throbber::from_frames(
                    app.symbols.heartbeat_frames,
                    ThrobberKind::Heartbeat,
                );
                app.io_throbber = Throbber::new(ThrobberKind::DataStream, new_palette.variant);
                if let Some(throb) = &mut app.telemetry_throbber {
                    *throb = Throbber::from_frames(
                        app.symbols.throbber_frames,
                        ThrobberKind::Processing,
                    );
                }
                crate::config::save_theme(new_palette.variant);
            } else {
                let sym_idx = app.theme_picker_cursor - color_count;
                let sym_count = crate::symbols::SymbolVariant::ALL.len();
                if sym_idx < sym_count {
                    // Symbol set selection
                    if let Some(&variant) = crate::symbols::SymbolVariant::ALL.get(sym_idx) {
                        let new_symbols = crate::symbols::SymbolSet::for_variant(variant);
                        app.symbols = new_symbols;
                        app.heartbeat = Throbber::from_frames(
                            new_symbols.heartbeat_frames,
                            ThrobberKind::Heartbeat,
                        );
                        if let Some(throb) = &mut app.telemetry_throbber {
                            *throb = Throbber::from_frames(
                                new_symbols.throbber_frames,
                                ThrobberKind::Processing,
                            );
                        }
                        crate::config::save_symbols(variant);
                    }
                } else {
                    // Glitch toggle
                    app.glitch_enabled = !app.glitch_enabled;
                    crate::config::save_glitch(app.glitch_enabled);
                }
            }
            app.show_theme_picker = false;
        }
        _ => {}
    }
}

fn handle_rsearch(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Enter) => {
            app.rsearch_confirm();
        }
        (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            if app.rsearch_cursor + 1 < app.rsearch_results.len() {
                app.rsearch_cursor += 1;
                // Scroll
                let vh = app.pane().viewport_height.saturating_sub(2);
                if app.rsearch_cursor >= app.rsearch_scroll + vh {
                    app.rsearch_scroll = app.rsearch_cursor - vh + 1;
                }
            }
        }
        (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            if app.rsearch_cursor > 0 {
                app.rsearch_cursor -= 1;
                if app.rsearch_cursor < app.rsearch_scroll {
                    app.rsearch_scroll = app.rsearch_cursor;
                }
            }
        }
        (_, KeyCode::Backspace) => {
            app.rsearch_query.pop();
            app.rsearch_filter();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.rsearch_query.clear();
            app.rsearch_filter();
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            app.rsearch_query.push(c);
            app.rsearch_filter();
        }
        _ => {}
    }
}

fn handle_bulk_rename(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Tab) => {
            app.bulk_field = 1 - app.bulk_field;
        }
        (_, KeyCode::Enter) => {
            // Execute the rename
            execute_bulk_rename(app);
        }
        (_, KeyCode::Backspace) => {
            if app.bulk_field == 0 {
                app.bulk_find.pop();
            } else {
                app.bulk_replace.pop();
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            if app.bulk_field == 0 {
                app.bulk_find.clear();
            } else {
                app.bulk_replace.clear();
            }
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            if app.bulk_field == 0 {
                app.bulk_find.push(c);
            } else {
                app.bulk_replace.push(c);
            }
        }
        _ => {}
    }
}

fn execute_bulk_rename(app: &mut App) {
    if app.bulk_find.is_empty() && !app.bulk_replace.contains("{n}") {
        app.error = Some(("SEARCH PATTERN UNDEFINED".to_string(), Instant::now()));
        return;
    }

    // Build the rename plan
    let mut renames: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let mut new_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut has_conflict = false;

    for (i, path) in app.bulk_paths.iter().enumerate() {
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => continue,
        };

        let new_name = if app.bulk_find.is_empty() {
            // Pure replacement mode with {n}
            app.bulk_replace.replace("{n}", &(i + 1).to_string())
        } else {
            let replaced = name.replace(&app.bulk_find, &app.bulk_replace);
            replaced.replace("{n}", &(i + 1).to_string())
        };

        if new_name.is_empty() || new_name == name {
            continue;
        }

        // Check for conflicts
        if !new_names.insert(new_name.clone()) {
            has_conflict = true;
            break;
        }

        let new_path = path.parent()
            .map(|p| p.join(&new_name))
            .unwrap_or_else(|| std::path::PathBuf::from(&new_name));

        // Check if target already exists (and isn't one of the files being renamed)
        if new_path.exists() && !app.bulk_paths.contains(&new_path) {
            has_conflict = true;
            break;
        }

        renames.push((path.clone(), new_path));
    }

    if has_conflict {
        app.error = Some(("REDESIGNATION CONFLICT \u{2014} DUPLICATE NAMES DETECTED".to_string(), Instant::now()));
        return;
    }

    if renames.is_empty() {
        app.error = Some(("NO MODIFICATIONS DETECTED \u{2014} SEQUENCE CANCELLED".to_string(), Instant::now()));
        return;
    }

    // Execute renames
    let mut success = 0;
    let mut errors = 0;
    for (from, to) in &renames {
        match std::fs::rename(from, to) {
            Ok(_) => success += 1,
            Err(_) => errors += 1,
        }
    }

    if errors > 0 {
        app.error = Some((
            format!("REDESIGNATION: {} PROCESSED, {} ABORTED", success, errors),
            Instant::now(),
        ));
    }

    app.visual_marks.clear();
    app.mode = Mode::Normal;
    app.load_entries();
}

fn handle_edit(app: &mut App, key: KeyEvent) {
    let Some(ed) = &mut app.editor else { return };

    // Unsaved changes prompt
    if ed.confirm_exit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.editor = None;
                app.mode = Mode::Normal;
                app.load_entries();
            }
            _ => {
                ed.confirm_exit = false;
            }
        }
        return;
    }

    match (key.modifiers, key.code) {
        // Exit
        (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
            if ed.dirty {
                ed.confirm_exit = true;
            } else {
                app.editor = None;
                app.mode = Mode::Normal;
                app.load_entries();
            }
        }
        // Save
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            if let Err(msg) = ed.save() {
                app.error = Some((msg, Instant::now()));
            }
        }
        // Undo
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            ed.undo();
        }
        // Delete line
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            ed.push_undo();
            ed.delete_line();
        }
        // Kill to end of line
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            ed.push_undo();
            ed.kill_to_eol();
        }
        // Cursor movement
        (_, KeyCode::Up) => {
            if ed.cursor_row > 0 {
                ed.cursor_row -= 1;
                ed.clamp_cursor();
                ed.ensure_cursor_visible();
            }
        }
        (_, KeyCode::Down) => {
            if ed.cursor_row + 1 < ed.lines.len() {
                ed.cursor_row += 1;
                ed.clamp_cursor();
                ed.ensure_cursor_visible();
            }
        }
        (_, KeyCode::Left) => {
            if ed.cursor_col > 0 {
                ed.cursor_col -= 1;
            } else if ed.cursor_row > 0 {
                ed.cursor_row -= 1;
                ed.cursor_col = ed.lines[ed.cursor_row].chars().count();
            }
            ed.ensure_cursor_visible();
        }
        (_, KeyCode::Right) => {
            let line_len = ed.lines[ed.cursor_row].chars().count();
            if ed.cursor_col < line_len {
                ed.cursor_col += 1;
            } else if ed.cursor_row + 1 < ed.lines.len() {
                ed.cursor_row += 1;
                ed.cursor_col = 0;
            }
            ed.ensure_cursor_visible();
        }
        (_, KeyCode::Home) => {
            ed.cursor_col = 0;
            ed.ensure_cursor_visible();
        }
        (_, KeyCode::End) => {
            ed.cursor_col = ed.lines[ed.cursor_row].chars().count();
            ed.ensure_cursor_visible();
        }
        (_, KeyCode::PageUp) => {
            ed.cursor_row = ed.cursor_row.saturating_sub(ed.viewport_rows);
            ed.clamp_cursor();
            ed.ensure_cursor_visible();
        }
        (_, KeyCode::PageDown) => {
            ed.cursor_row = (ed.cursor_row + ed.viewport_rows).min(ed.lines.len().saturating_sub(1));
            ed.clamp_cursor();
            ed.ensure_cursor_visible();
        }
        // Editing
        (_, KeyCode::Enter) => {
            ed.push_undo();
            ed.insert_newline();
        }
        (_, KeyCode::Backspace) => {
            ed.push_undo();
            ed.backspace();
        }
        (_, KeyCode::Delete) => {
            ed.push_undo();
            ed.delete_char();
        }
        (_, KeyCode::Tab) => {
            ed.push_undo();
            for _ in 0..4 {
                ed.insert_char(' ');
            }
        }
        // Character insertion
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            ed.push_undo();
            ed.insert_char(c);
        }
        _ => {}
    }
}

fn handle_ops_log(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.ops_log_scroll + 1 < app.ops_log.entries.len() {
                app.ops_log_scroll += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.ops_log_scroll = app.ops_log_scroll.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            app.ops_log_scroll = 0;
        }
        KeyCode::Char('G') => {
            app.ops_log_scroll = app.ops_log.entries.len().saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_command(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Enter) => {
            let cmd = app.command_state.input.clone();
            if !cmd.is_empty() {
                app.command_state.history.push(cmd.clone());
            }
            execute_command(app, &cmd);
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Backspace) => {
            if app.command_state.cursor > 0 {
                app.command_state.cursor -= 1;
                let byte_idx = app.command_state.input.char_indices()
                    .nth(app.command_state.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.command_state.input.len());
                let next_byte = app.command_state.input.char_indices()
                    .nth(app.command_state.cursor + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.command_state.input.len());
                app.command_state.input.drain(byte_idx..next_byte);
            }
            app.command_state.completions.clear();
            app.command_state.completion_idx = None;
        }
        (_, KeyCode::Left) => {
            if app.command_state.cursor > 0 {
                app.command_state.cursor -= 1;
            }
        }
        (_, KeyCode::Right) => {
            let len = app.command_state.input.chars().count();
            if app.command_state.cursor < len {
                app.command_state.cursor += 1;
            }
        }
        // Tab completion (#49)
        (_, KeyCode::Tab) => {
            if app.command_state.completions.is_empty()
                || app.command_state.completion_prefix != app.command_state.input
            {
                // Generate completions
                app.command_state.completion_prefix = app.command_state.input.clone();
                app.command_state.completions.clear();
                app.command_state.completion_idx = None;

                let input = &app.command_state.input;
                if input.starts_with("cd ") {
                    // Path completion
                    let partial = &input[3..];
                    let (dir, prefix) = if let Some(pos) = partial.rfind('/').or_else(|| partial.rfind('\\')) {
                        let base = &partial[..=pos];
                        let pfx = &partial[pos + 1..];
                        (app.pane().current_dir.join(base), pfx.to_string())
                    } else {
                        (app.pane().current_dir.clone(), partial.to_string())
                    };
                    if let Ok(rd) = std::fs::read_dir(&dir) {
                        for entry in rd.flatten() {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                                let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
                                let suffix = if is_dir { "/" } else { "" };
                                let base_path = if let Some(pos) = partial.rfind('/').or_else(|| partial.rfind('\\')) {
                                    format!("{}{}{}", &partial[..=pos], name, suffix)
                                } else {
                                    format!("{}{}", name, suffix)
                                };
                                app.command_state.completions.push(format!("cd {}", base_path));
                            }
                        }
                    }
                } else {
                    // Command name completion
                    let commands = [
                        "q", "quit", "cd", "set", "sort", "theme", "symbols", "close", "git", "diff", "help",
                        "rm", "cp", "mv",
                        "|", "|clear", ">",
                    ];
                    for cmd in &commands {
                        if cmd.starts_with(input.as_str()) {
                            app.command_state.completions.push(cmd.to_string());
                        }
                    }
                }
            }
            // Cycle through completions
            if !app.command_state.completions.is_empty() {
                let idx = match app.command_state.completion_idx {
                    Some(i) => (i + 1) % app.command_state.completions.len(),
                    None => 0,
                };
                app.command_state.completion_idx = Some(idx);
                app.command_state.input = app.command_state.completions[idx].clone();
                app.command_state.cursor = app.command_state.input.chars().count();
            }
        }
        (_, KeyCode::Up) => {
            // Command history navigation
            let hist_len = app.command_state.history.len();
            if hist_len > 0 {
                let idx = match app.command_state.history_idx {
                    Some(0) => 0,
                    Some(i) => i - 1,
                    None => hist_len - 1,
                };
                app.command_state.history_idx = Some(idx);
                app.command_state.input = app.command_state.history[idx].clone();
                app.command_state.cursor = app.command_state.input.chars().count();
            }
        }
        (_, KeyCode::Down) => {
            if let Some(idx) = app.command_state.history_idx {
                let hist_len = app.command_state.history.len();
                if idx + 1 < hist_len {
                    let new_idx = idx + 1;
                    app.command_state.history_idx = Some(new_idx);
                    app.command_state.input = app.command_state.history[new_idx].clone();
                    app.command_state.cursor = app.command_state.input.chars().count();
                } else {
                    app.command_state.history_idx = None;
                    app.command_state.input.clear();
                    app.command_state.cursor = 0;
                }
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.command_state.input.clear();
            app.command_state.cursor = 0;
            app.command_state.completions.clear();
            app.command_state.completion_idx = None;
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            let byte_idx = app.command_state.input.char_indices()
                .nth(app.command_state.cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.command_state.input.len());
            app.command_state.input.insert(byte_idx, c);
            app.command_state.cursor += 1;
            app.command_state.completions.clear();
            app.command_state.completion_idx = None;
        }
        _ => {}
    }
}

/// Expand a simple glob pattern against entries. Supports * and ? wildcards.
fn expand_glob(pattern: &str, entries: &[crate::app::FsEntry]) -> Vec<std::path::PathBuf> {
    entries.iter()
        .filter(|e| glob_match(pattern, &e.name))
        .map(|e| e.path.clone())
        .collect()
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let nam: Vec<char> = name.chars().collect();
    glob_match_inner(&pat, &nam, 0, 0)
}

fn glob_match_inner(pat: &[char], name: &[char], pi: usize, ni: usize) -> bool {
    if pi == pat.len() && ni == name.len() { return true; }
    if pi == pat.len() { return false; }
    match pat[pi] {
        '*' => {
            // Try matching zero or more characters
            for skip in 0..=(name.len() - ni) {
                if glob_match_inner(pat, name, pi + 1, ni + skip) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if ni < name.len() {
                glob_match_inner(pat, name, pi + 1, ni + 1)
            } else {
                false
            }
        }
        c => {
            if ni < name.len() && name[ni] == c {
                glob_match_inner(pat, name, pi + 1, ni + 1)
            } else {
                false
            }
        }
    }
}

fn execute_command(app: &mut App, cmd: &str) {
    let trimmed = cmd.trim();

    // Shell command: !<command>
    if let Some(shell_cmd) = trimmed.strip_prefix('!') {
        let shell_cmd = shell_cmd.trim();
        if shell_cmd.is_empty() {
            app.error = Some(("SHELL COMMAND REQUIRED AFTER !".to_string(), Instant::now()));
            return;
        }
        app.shell_output = None;
        let output = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", shell_cmd])
                .current_dir(&app.pane().current_dir)
                .output()
        } else {
            std::process::Command::new("sh")
                .args(["-c", shell_cmd])
                .current_dir(&app.pane().current_dir)
                .output()
        };
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                let truncated = if combined.len() > 200 {
                    format!("{}…", &combined[..200])
                } else {
                    combined
                };
                let msg = truncated.lines().next().unwrap_or("").to_string();
                app.error = Some((
                    if msg.is_empty() { "COMMAND EXECUTED".to_string() } else { msg },
                    Instant::now(),
                ));
                app.shell_output = Some(truncated);
                app.ops_log.push("SHELL", shell_cmd);
            }
            Err(e) => {
                app.error = Some((format!("SHELL FAILURE: {}", e), Instant::now()));
            }
        }
        app.load_entries();
        return;
    }

    // Pipe to external tool: | <command> (#83)
    if let Some(pipe_cmd) = trimmed.strip_prefix('|') {
        let pipe_cmd = pipe_cmd.trim();
        if pipe_cmd == "clear" {
            app.pipe_filtered = None;
            app.rebuild_filtered();
            app.error = Some(("PIPE FILTER CLEARED".to_string(), Instant::now()));
            return;
        }
        if pipe_cmd.is_empty() {
            app.error = Some(("PIPE COMMAND REQUIRED AFTER |".to_string(), Instant::now()));
            return;
        }
        // Build file list as stdin
        let pane = app.pane();
        let file_list: String = pane.entries.iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        let output = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", pipe_cmd])
                .current_dir(&pane.current_dir)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    use std::io::Write;
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(file_list.as_bytes());
                    }
                    child.wait_with_output()
                })
        } else {
            std::process::Command::new("sh")
                .args(["-c", pipe_cmd])
                .current_dir(&pane.current_dir)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    use std::io::Write;
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(file_list.as_bytes());
                    }
                    child.wait_with_output()
                })
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let filtered_names: Vec<String> = stdout.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                let count = filtered_names.len();
                app.pipe_filtered = Some(filtered_names.clone());
                // Apply filter: only show entries whose names appear in the output
                let pane = app.pane_mut();
                pane.filtered_indices = pane.entries.iter().enumerate()
                    .filter(|(_, e)| filtered_names.contains(&e.name))
                    .map(|(i, _)| i)
                    .collect();
                pane.cursor = 0;
                pane.scroll_offset = 0;
                app.error = Some((format!("PIPE FILTER: {} RESULTS", count), Instant::now()));
            }
            Err(e) => {
                app.error = Some((format!("PIPE FAILURE: {}", e), Instant::now()));
            }
        }
        return;
    }

    // Write to file: > <path> (#83)
    if let Some(out_path) = trimmed.strip_prefix('>') {
        let out_path = out_path.trim();
        if out_path.is_empty() {
            app.error = Some(("OUTPUT PATH REQUIRED AFTER >".to_string(), Instant::now()));
            return;
        }
        let pane = app.pane();
        let resolved = if std::path::Path::new(out_path).is_absolute() {
            std::path::PathBuf::from(out_path)
        } else {
            pane.current_dir.join(out_path)
        };
        let file_list: String = pane.filtered_indices.iter()
            .filter_map(|&i| pane.entries.get(i))
            .map(|e| e.name.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        match std::fs::write(&resolved, file_list) {
            Ok(_) => {
                app.error = Some((format!("WRITTEN TO {}", out_path), Instant::now()));
                app.ops_log.push("WRITE", out_path);
            }
            Err(e) => {
                app.error = Some((format!("WRITE FAILURE: {}", e), Instant::now()));
            }
        }
        app.load_entries();
        return;
    }

    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    match parts.first().copied() {
        Some("q") | Some("quit") => {
            app.should_quit = true;
        }
        Some("cd") => {
            if let Some(path_str) = parts.get(1) {
                let path = std::path::PathBuf::from(path_str.trim());
                let resolved = if path.is_absolute() {
                    path
                } else {
                    app.pane().current_dir.join(path)
                };
                if resolved.is_dir() {
                    app.navigate_to(resolved);
                } else {
                    app.error = Some(("ASSET NOT LOCATED IN MANIFEST".to_string(), Instant::now()));
                }
            }
        }
        Some("set") => {
            match parts.get(1).map(|s| s.trim()) {
                Some("hidden") => { app.show_hidden = true; app.rebuild_filtered(); }
                Some("nohidden") => { app.show_hidden = false; app.rebuild_filtered(); }
                Some("minimap") => { app.show_minimap = true; }
                Some("nominimap") => { app.show_minimap = false; }
                _ => {
                    app.error = Some(("UNKNOWN PARAMETER".to_string(), Instant::now()));
                }
            }
        }
        Some("sort") => {
            match parts.get(1).map(|s| s.trim()) {
                Some("name") => { app.sort_mode = crate::app::SortMode::NameAsc; app.load_entries(); }
                Some("size") => { app.sort_mode = crate::app::SortMode::SizeDesc; app.load_entries(); }
                Some("date") => { app.sort_mode = crate::app::SortMode::DateNewest; app.load_entries(); }
                _ => {
                    app.error = Some(("VALID: sort name|size|date".to_string(), Instant::now()));
                }
            }
        }
        Some("theme") => {
            match parts.get(1).map(|s| s.trim()) {
                Some("green") => {
                    app.palette = crate::palette::Palette::phosphor_green();
                    crate::config::save_theme(app.palette.variant);
                }
                Some("amber") => {
                    app.palette = crate::palette::Palette::amber();
                    crate::config::save_theme(app.palette.variant);
                }
                Some("cyan") => {
                    app.palette = crate::palette::Palette::degraded_cyan();
                    crate::config::save_theme(app.palette.variant);
                }
                _ => {
                    app.error = Some(("VALID: theme green|amber|cyan".to_string(), Instant::now()));
                }
            }
        }
        Some("symbols") => {
            if let Some(name) = parts.get(1).map(|s| s.trim()) {
                let variant = crate::symbols::SymbolVariant::from_config(name);
                app.symbols = crate::symbols::SymbolSet::for_variant(variant);
                crate::config::save_symbols(variant);
            }
        }
        Some("shell") => {
            app.error = Some(("USE :!<command> TO EXECUTE SHELL COMMANDS".to_string(), Instant::now()));
        }
        Some("tag") => {
            if let Some(tag) = parts.get(1).map(|s| s.trim().to_string()) {
                if !tag.is_empty() {
                    if let Some(entry) = app.current_entry() {
                        let path = entry.path.clone();
                        app.tags.add_tag(path, tag);
                        crate::tags::save_tags(&app.tags);
                        app.error = Some(("TAG ADDED".to_string(), Instant::now()));
                    }
                }
            } else {
                app.error = Some(("USAGE: tag <name>".to_string(), Instant::now()));
            }
        }
        Some("untag") => {
            if let Some(tag) = parts.get(1).map(|s| s.trim()) {
                if !tag.is_empty() {
                    if let Some(entry) = app.current_entry() {
                        let path = entry.path.clone();
                        app.tags.remove_tag(&path, tag);
                        crate::tags::save_tags(&app.tags);
                        app.error = Some(("TAG REMOVED".to_string(), Instant::now()));
                    }
                }
            } else {
                app.error = Some(("USAGE: untag <name>".to_string(), Instant::now()));
            }
        }
        Some("rm") => {
            if let Some(pattern) = parts.get(1).map(|s| s.trim()) {
                let pane = app.pane();
                let matches = expand_glob(pattern, &pane.entries);
                if matches.is_empty() {
                    app.error = Some(("NO MATCHING ASSETS".to_string(), Instant::now()));
                } else {
                    app.mode = Mode::Confirm {
                        action: PendingAction::Delete { paths: matches },
                    };
                    app.confirm_timer = Some(Instant::now());
                }
            } else {
                app.error = Some(("USAGE: rm <glob>".to_string(), Instant::now()));
            }
        }
        Some("cp") => {
            if let Some(args) = parts.get(1) {
                let cmd_parts: Vec<&str> = args.trim().splitn(2, ' ').collect();
                if cmd_parts.len() == 2 {
                    let pattern = cmd_parts[0];
                    let dest_str = cmd_parts[1];
                    let pane = app.pane();
                    let matches = expand_glob(pattern, &pane.entries);
                    if matches.is_empty() {
                        app.error = Some(("NO MATCHING ASSETS".to_string(), Instant::now()));
                    } else {
                        let dest = if std::path::Path::new(dest_str).is_absolute() {
                            std::path::PathBuf::from(dest_str)
                        } else {
                            pane.current_dir.join(dest_str)
                        };
                        for src in &matches {
                            if let Some(name) = src.file_name() {
                                let target = dest.join(name);
                                let _ = std::fs::copy(src, &target);
                                app.ops_log.push("COPY", &src.to_string_lossy());
                            }
                        }
                        app.error = Some((format!("COPIED {} ASSETS", matches.len()), Instant::now()));
                        app.load_entries();
                    }
                } else {
                    app.error = Some(("USAGE: cp <glob> <dest>".to_string(), Instant::now()));
                }
            }
        }
        Some("mv") => {
            if let Some(args) = parts.get(1) {
                let cmd_parts: Vec<&str> = args.trim().splitn(2, ' ').collect();
                if cmd_parts.len() == 2 {
                    let pattern = cmd_parts[0];
                    let dest_str = cmd_parts[1];
                    let pane = app.pane();
                    let matches = expand_glob(pattern, &pane.entries);
                    if matches.is_empty() {
                        app.error = Some(("NO MATCHING ASSETS".to_string(), Instant::now()));
                    } else {
                        let dest = if std::path::Path::new(dest_str).is_absolute() {
                            std::path::PathBuf::from(dest_str)
                        } else {
                            pane.current_dir.join(dest_str)
                        };
                        for src in &matches {
                            if let Some(name) = src.file_name() {
                                let target = dest.join(name);
                                let _ = std::fs::rename(src, &target);
                                app.ops_log.push("MOVE", &src.to_string_lossy());
                            }
                        }
                        app.error = Some((format!("RELOCATED {} ASSETS", matches.len()), Instant::now()));
                        app.load_entries();
                    }
                } else {
                    app.error = Some(("USAGE: mv <glob> <dest>".to_string(), Instant::now()));
                }
            }
        }
        Some("close") => {
            app.close_tab();
        }
        Some("git") => {
            if let Some(subcmd) = parts.get(1) {
                let git_parts: Vec<&str> = subcmd.trim().splitn(2, ' ').collect();
                match git_parts.first().copied() {
                    Some("status") => {
                        let dir = app.pane().current_dir.clone();
                        app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                        let count = app.git_file_statuses.len();
                        app.error = Some((format!("GIT: {} CHANGES DETECTED", count), Instant::now()));
                    }
                    Some("add") => {
                        let dir = app.pane().current_dir.clone();
                        match crate::gitstatus::git_stage(&dir, ".") {
                            Ok(()) => app.error = Some(("GIT: ALL STAGED".to_string(), Instant::now())),
                            Err(e) => app.error = Some((e, Instant::now())),
                        }
                        app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                    }
                    Some("reset") => {
                        let dir = app.pane().current_dir.clone();
                        match crate::gitstatus::git_unstage(&dir, ".") {
                            Ok(()) => app.error = Some(("GIT: ALL UNSTAGED".to_string(), Instant::now())),
                            Err(e) => app.error = Some((e, Instant::now())),
                        }
                        app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                    }
                    Some("commit") => {
                        if let Some(msg) = git_parts.get(1) {
                            let dir = app.pane().current_dir.clone();
                            match crate::gitstatus::git_commit(&dir, msg) {
                                Ok(out) => {
                                    let display = if out.len() > 60 { format!("{}...", &out[..59]) } else { out };
                                    app.error = Some((format!("COMMITTED: {}", display), Instant::now()));
                                }
                                Err(e) => app.error = Some((e, Instant::now())),
                            }
                            app.git_file_statuses = crate::gitstatus::parse_git_status(&dir);
                        } else {
                            app.error = Some(("USAGE: git commit <message>".to_string(), Instant::now()));
                        }
                    }
                    _ => {
                        app.error = Some(("GIT: status|add|reset|commit".to_string(), Instant::now()));
                    }
                }
            } else {
                app.error = Some(("USAGE: git <status|add|reset|commit>".to_string(), Instant::now()));
            }
        }
        Some("diff") => {
            if let Some(args) = parts.get(1) {
                let files: Vec<&str> = args.trim().splitn(2, ' ').collect();
                if files.len() == 2 {
                    let dir = app.pane().current_dir.clone();
                    let p1 = if std::path::Path::new(files[0]).is_absolute() {
                        std::path::PathBuf::from(files[0])
                    } else {
                        dir.join(files[0])
                    };
                    let p2 = if std::path::Path::new(files[1]).is_absolute() {
                        std::path::PathBuf::from(files[1])
                    } else {
                        dir.join(files[1])
                    };
                    match crate::diff::DiffView::from_files(&p1, &p2) {
                        Ok(dv) => {
                            app.file_diff = Some(dv);
                            app.mode = Mode::FileDiff;
                        }
                        Err(e) => app.error = Some((e, Instant::now())),
                    }
                } else {
                    app.error = Some(("USAGE: diff <file1> <file2>".to_string(), Instant::now()));
                }
            } else {
                app.error = Some(("USAGE: diff <file1> <file2>".to_string(), Instant::now()));
            }
        }
        Some("help") => {
            app.error = Some(("COMMANDS: q cd set sort theme symbols shell tag untag rm cp mv |<cmd> >file close git diff help".to_string(), Instant::now()));
        }
        _ => {
            app.error = Some(("UNKNOWN COMMAND \u{2014} TYPE :help".to_string(), Instant::now()));
        }
    }
}

/// Handle tag input mode (#58).
fn handle_tag_input(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Enter) => {
            if !app.tag_input.is_empty() {
                if let Some(entry) = app.current_entry() {
                    let path = entry.path.clone();
                    let tag = app.tag_input.clone();
                    app.tags.add_tag(path, tag);
                    crate::tags::save_tags(&app.tags);
                    app.error = Some(("TAG ADDED".to_string(), Instant::now()));
                }
            }
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Backspace) => {
            app.tag_input.pop();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.tag_input.push(c);
        }
        _ => {}
    }
}

/// Handle file diff view (#85).
fn handle_diff(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
            app.file_diff = None;
            app.mode = Mode::Normal;
        }
        (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
            if let Some(diff) = &mut app.file_diff {
                if diff.scroll + 1 < diff.max_lines {
                    diff.scroll += 1;
                }
            }
        }
        (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
            if let Some(diff) = &mut app.file_diff {
                if diff.scroll > 0 {
                    diff.scroll -= 1;
                }
            }
        }
        (_, KeyCode::Char('g')) => {
            if let Some(diff) = &mut app.file_diff {
                diff.scroll = 0;
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            if let Some(diff) = &mut app.file_diff {
                diff.scroll = diff.max_lines.saturating_sub(1);
            }
        }
        _ => {}
    }
}

/// Collect paths for operations: marked entries if any, otherwise current entry.
fn collect_operation_paths(app: &App) -> Vec<std::path::PathBuf> {
    let pane = app.pane();
    if !app.visual_marks.is_empty() {
        app.visual_marks.iter()
            .filter_map(|&idx| pane.entries.get(idx).map(|e| e.path.clone()))
            .collect()
    } else if let Some(entry) = app.current_entry() {
        vec![entry.path.clone()]
    } else {
        vec![]
    }
}
