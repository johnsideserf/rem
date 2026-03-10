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
        Mode::RecursiveSearch => handle_rsearch(app, key),
        Mode::BulkRename => handle_bulk_rename(app, key),
        Mode::Edit => handle_edit(app, key),
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
        app.error = Some(("FIND PATTERN EMPTY".to_string(), Instant::now()));
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
        app.error = Some(("RENAME CONFLICT: DUPLICATE NAMES".to_string(), Instant::now()));
        return;
    }

    if renames.is_empty() {
        app.error = Some(("NO CHANGES TO APPLY".to_string(), Instant::now()));
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
            format!("RENAMED {}, FAILED {}", success, errors),
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
