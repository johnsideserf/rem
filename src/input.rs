use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode, OpBuffer, OpType, PendingAction, JUMP_KEYS};
use crate::sysmon::SysMon;
use crate::throbber::{Throbber, ThrobberKind};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Dismiss error on any key
    if app.error.is_some() {
        app.error = None;
        return;
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
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            app.should_quit = true;
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.cursor_down();
            app.preview_scroll = 0;
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.cursor_up();
            app.preview_scroll = 0;
        }
        (KeyModifiers::NONE, KeyCode::Char('l'))
        | (KeyModifiers::NONE, KeyCode::Right)
        | (KeyModifiers::NONE, KeyCode::Enter) => {
            app.enter_selected();
        }
        (KeyModifiers::NONE, KeyCode::Char('h'))
        | (KeyModifiers::NONE, KeyCode::Left)
        | (KeyModifiers::NONE, KeyCode::Char('-')) => {
            app.go_parent();
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
        _ => {}
    }
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
                app.error = Some((format!("MARK '{}' NOT SET", c), std::time::Instant::now()));
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
    match key.code {
        KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('q') => {
            app.show_theme_picker = false;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.theme_picker_cursor = (app.theme_picker_cursor + 1)
                % crate::ui::theme_picker::THEME_COUNT;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.theme_picker_cursor = app.theme_picker_cursor
                .checked_sub(1)
                .unwrap_or(crate::ui::theme_picker::THEME_COUNT - 1);
        }
        KeyCode::Enter => {
            let new_palette = crate::ui::theme_picker::palette_for_index(
                app.theme_picker_cursor,
            );
            app.palette = new_palette;
            app.heartbeat = Throbber::new(ThrobberKind::Heartbeat, new_palette.variant);
            if let Some(throb) = &mut app.telemetry_throbber {
                *throb = Throbber::new(ThrobberKind::Processing, new_palette.variant);
            }
            crate::config::save_theme(new_palette.variant);
            app.show_theme_picker = false;
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
