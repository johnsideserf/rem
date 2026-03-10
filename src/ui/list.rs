use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FsEntry, Mode, OpType, JUMP_KEYS, file_type_badge, format_size, icon_for};

/// Interpolate between two RGB colors. `t` ranges from 0.0 (fully `from`) to 1.0 (fully `to`).
fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => {
            Color::Rgb(
                (fr as f32 + (tr as f32 - fr as f32) * t) as u8,
                (fg as f32 + (tg as f32 - fg as f32) * t) as u8,
                (fb as f32 + (tb as f32 - fb as f32) * t) as u8,
            )
        }
        _ => to,
    }
}

/// Truncate string to `max_chars` characters, with ellipsis if needed, padded to `pad_to`.
fn trunc_pad(s: &str, max_chars: usize, pad_to: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{:\u{2026}<width$}", t, width = pad_to)
    } else {
        format!("{:<width$}", s, width = pad_to)
    }
}


pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.tree_mode && !app.tree_nodes.is_empty() {
        render_tree(f, app, area);
    } else {
        render_pane(f, app, app.active_pane, area);
    }
}

pub fn render_pane(f: &mut Frame, app: &App, pane_idx: usize, area: Rect) {
    let pal = app.palette;
    let pane = &app.panes[pane_idx];
    let is_active = pane_idx == app.active_pane;
    let width = area.width as usize;

    // Animation: compute fade factor and horizontal offset
    let (anim_t, anim_offset) = if is_active && app.anim_frame > 0 {
        let t = app.anim_frame as f32 / 4.0; // 1→0.25, 2→0.50, 3→0.75
        let offset = match app.anim_frame {
            1 => 6usize,
            2 => 3,
            3 => 1,
            _ => 0,
        };
        (t, offset)
    } else {
        (1.0, 0)
    };

    // Determine which columns to show
    let show_size = width >= 90;
    let show_type = width >= 80;
    let show_perms = width >= 110;

    let size_col = if show_size { 9 } else { 0 };
    let type_col = if show_type { 5 } else { 0 };
    let perms_col = if show_perms { 10 } else { 0 };
    let jump_col = 5;
    let sigil_col = 2;
    let right_cols = size_col + type_col + perms_col;
    let name_width = width.saturating_sub(jump_col + sigil_col + right_cols + 2);

    let visible_height = area.height as usize;
    let start = pane.scroll_offset;
    let end = (start + visible_height).min(pane.filtered_indices.len());

    // Determine which entries are in the op buffer
    let cut_paths: std::collections::HashSet<std::path::PathBuf> = app.op_buffer.as_ref()
        .filter(|b| b.op == OpType::Cut)
        .map(|b| b.paths.iter().cloned().collect())
        .unwrap_or_default();
    let copy_paths: std::collections::HashSet<std::path::PathBuf> = app.op_buffer.as_ref()
        .filter(|b| b.op == OpType::Copy)
        .map(|b| b.paths.iter().cloned().collect())
        .unwrap_or_default();

    let mut lines: Vec<Line> = Vec::new();

    for vi in start..end {
        let idx = pane.filtered_indices[vi];
        let entry = &pane.entries[idx];
        let is_cursor = is_active && vi == pane.cursor;
        let is_marked = app.visual_marks.contains(&idx);
        let is_cut = cut_paths.contains(&entry.path);
        let is_copied = copy_paths.contains(&entry.path);
        let is_fuzzy_match = !pane.fuzzy_query.is_empty();

        let row_bg_base = if is_cursor { pal.surface } else { pal.bg };
        // Purge animation (#35) — corrupt characters for entries being deleted
        let is_purging = app.purge_anim.as_ref()
            .map_or(false, |a| a.entries.contains(&entry.name));

        let text_color_base = if is_cursor {
            pal.text_hot
        } else if is_cut {
            pal.text_dim
        } else if is_marked {
            pal.text_mid
        } else if is_fuzzy_match {
            pal.text_mid
        } else {
            pal.text_dim
        };

        // Apply animation fade
        let row_bg = if anim_t < 1.0 { lerp_color(pal.bg, row_bg_base, anim_t) } else { row_bg_base };
        let text_color = if anim_t < 1.0 { lerp_color(pal.bg, text_color_base, anim_t) } else { text_color_base };

        let mut spans: Vec<Span> = Vec::new();

        // Animation: horizontal offset (slide-in from right)
        if anim_offset > 0 {
            spans.push(Span::styled(
                " ".repeat(anim_offset),
                Style::default().bg(pal.bg),
            ));
        }

        // Indicator column
        let sym = &app.symbols;
        let indicator = if is_cursor && is_marked {
            sym.mark
        } else if is_cursor {
            sym.cursor
        } else if is_marked {
            sym.mark
        } else if is_cut {
            sym.cut
        } else if is_copied {
            sym.copy
        } else {
            " "
        };
        spans.push(Span::styled(indicator, Style::default().fg(pal.text_hot).bg(row_bg)));

        // Jump key column
        if is_active && app.mode == Mode::JumpKey {
            if let Some(jk) = JUMP_KEYS.get(vi) {
                spans.push(Span::styled(
                    format!("[{}] ", jk),
                    Style::default()
                        .fg(pal.text_hot)
                        .bg(pal.border_mid)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("    ", Style::default().bg(row_bg)));
            }
        } else {
            spans.push(Span::styled("    ", Style::default().bg(row_bg)));
        }

        // File icon
        let icon = icon_for(entry, &app.symbols);
        let icon_text = format!("{} ", icon);
        let mut icon_style = Style::default().fg(text_color).bg(row_bg);
        if is_cut {
            icon_style = icon_style.add_modifier(Modifier::ITALIC);
        }
        spans.push(Span::styled(icon_text, icon_style));

        // Permissions column (#47)
        if show_perms {
            let perm_str = entry.permissions.as_deref().unwrap_or("---");
            spans.push(Span::styled(
                format!("{:<10}", perm_str),
                Style::default().fg(pal.text_dim).bg(row_bg),
            ));
        }

        // Name — check for Rename/Create mode inline editing
        let is_rename_row = is_active && is_cursor && app.mode == Mode::Rename;

        if is_purging {
            // Purge corruption effect (#35)
            let tick = app.purge_anim.as_ref().map_or(0, |a| a.tick);
            const CORRUPT: &[char] = &['\u{2591}', '\u{2592}', '\u{2593}', ' '];
            let reveal = (tick as usize * name_width / 8).min(name_width);
            let mut corrupted = String::new();
            for (ci, ch) in entry.name.chars().enumerate() {
                if ci < reveal {
                    corrupted.push(' ');
                } else {
                    let idx = (ci * 7 + tick as usize * 3) % CORRUPT.len();
                    corrupted.push(if ch.is_alphanumeric() { CORRUPT[idx] } else { ch });
                }
            }
            let truncated = trunc_pad(&corrupted, name_width, name_width);
            spans.push(Span::styled(truncated, Style::default().fg(pal.warn).bg(row_bg)));
        } else if is_rename_row {
            let cursor_char = if app.blink_on { app.symbols.text_cursor } else { " " };
            let display = format!("{}{}", app.rename_buf, cursor_char);
            let truncated = trunc_pad(&display, name_width, name_width);
            spans.push(Span::styled(truncated, Style::default().fg(pal.text_hot).bg(pal.border_mid)));
        } else {
            let display_name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };

            // Fuzzy match highlighting: render matched chars in text_hot+Bold
            let match_positions = pane.fuzzy_match_positions.get(&idx);
            if !is_cursor && match_positions.is_some() {
                let positions = match_positions.unwrap();
                let chars: Vec<char> = display_name.chars().collect();
                let max_chars = name_width.min(chars.len());
                let truncated = max_chars < chars.len();

                let base_style = Style::default().fg(text_color).bg(row_bg);
                let highlight_style = Style::default()
                    .fg(pal.text_hot).bg(row_bg)
                    .add_modifier(Modifier::BOLD);

                let mut name_spans: Vec<Span> = Vec::new();
                let mut current_run = String::new();
                let mut current_is_match = false;

                let display_len = if truncated { max_chars.saturating_sub(1) } else { max_chars };
                for (ci, &ch) in chars.iter().enumerate().take(display_len) {
                    let is_match = positions.contains(&ci);
                    if is_match != current_is_match && !current_run.is_empty() {
                        let style = if current_is_match { highlight_style } else { base_style };
                        name_spans.push(Span::styled(current_run.clone(), style));
                        current_run.clear();
                    }
                    current_run.push(ch);
                    current_is_match = is_match;
                }
                if !current_run.is_empty() {
                    let style = if current_is_match { highlight_style } else { base_style };
                    name_spans.push(Span::styled(current_run, style));
                }
                if truncated {
                    name_spans.push(Span::styled("\u{2026}", base_style));
                }
                // Pad to name_width
                let rendered: usize = if truncated { display_len + 1 } else { display_len };
                if rendered < name_width {
                    name_spans.push(Span::styled(
                        " ".repeat(name_width - rendered),
                        base_style,
                    ));
                }
                spans.extend(name_spans);
            } else {
                let truncated = trunc_pad(&display_name, name_width, name_width);
                let mut name_style = Style::default().fg(text_color).bg(row_bg);
                if is_cut {
                    name_style = name_style.add_modifier(Modifier::ITALIC);
                }
                spans.push(Span::styled(truncated, name_style));
            }
        }

        // CLASSIFIED badge (#51)
        if entry.is_classified {
            spans.push(Span::styled(
                " [CLASSIFIED]",
                Style::default().fg(pal.warn).bg(row_bg),
            ));
        }

        // Symlink indicator (#42)
        if entry.is_symlink {
            if let Some(target) = &entry.link_target {
                // Check if target exists
                let target_path = std::path::Path::new(target);
                let broken = !target_path.exists() && !entry.path.exists();
                if broken {
                    let suffix = " \u{2192} BROKEN";
                    let avail = name_width.saturating_sub(entry.name.len() + 1);
                    if avail >= suffix.len() {
                        // already rendered name, this gets appended via diff glyph area
                    }
                    spans.push(Span::styled(
                        " BROKEN",
                        Style::default().fg(pal.warn).bg(row_bg),
                    ));
                } else {
                    let arrow_target = format!(" \u{21e2} {}", target);
                    let max_t = 20usize.min(arrow_target.chars().count());
                    let truncated_t: String = arrow_target.chars().take(max_t).collect();
                    spans.push(Span::styled(
                        truncated_t,
                        Style::default().fg(pal.text_dim).bg(row_bg),
                    ));
                }
            }
        }

        // Diff indicator (#45)
        if app.diff_mode && app.dual_pane {
            if let Some((ref left_names, ref right_names)) = app.diff_sets {
                let other_set = if pane_idx == 0 { right_names } else { left_names };
                let glyph = if other_set.contains(&entry.name) {
                    Span::styled("=", Style::default().fg(pal.text_dim).bg(row_bg))
                } else {
                    Span::styled("+", Style::default().fg(pal.text_hot).bg(row_bg))
                };
                spans.push(glyph);
            }
        }

        // Tag badges (#58)
        if let Some(tags) = app.tags.get(&entry.path) {
            for tag in tags.iter().take(3) {
                spans.push(Span::styled(
                    format!(" [{}]", tag.to_uppercase()),
                    Style::default().fg(pal.border_mid).bg(row_bg),
                ));
            }
        }

        // Type badge
        if show_type {
            let badge = file_type_badge(entry);
            spans.push(Span::styled(
                format!("{:>5}", badge),
                Style::default().fg(pal.text_dim).bg(row_bg),
            ));
        }

        // Size
        if show_size {
            let size_str = match entry.size {
                Some(s) => format_size(s),
                None => app.symbols.em_dash.to_string(),
            };
            spans.push(Span::styled(
                format!("{:>9}", size_str),
                Style::default().fg(pal.text_dim).bg(row_bg),
            ));
        }

        lines.push(Line::from(spans));
    }

    // Create mode: insert a new row at the cursor position
    if is_active && matches!(app.mode, Mode::Create { .. }) {
        let is_dir = matches!(app.mode, Mode::Create { is_dir: true });
        let icon = if is_dir { app.symbols.dir_icon } else { app.symbols.file_icon };
        let cursor_char = if app.blink_on { app.symbols.text_cursor } else { " " };
        let display = format!("{}{}", app.create_buf, cursor_char);
        let truncated = trunc_pad(&display, name_width, name_width);

        let create_line = Line::from(vec![
            Span::styled(app.symbols.cursor, Style::default().fg(pal.text_hot).bg(pal.border_mid)),
            Span::styled("    ", Style::default().bg(pal.border_mid)),
            Span::styled(format!("{} ", icon), Style::default().fg(pal.text_hot).bg(pal.border_mid)),
            Span::styled(truncated, Style::default().fg(pal.text_hot).bg(pal.border_mid)),
        ]);

        // Insert after cursor position
        let insert_pos = (pane.cursor - start + 1).min(lines.len());
        lines.insert(insert_pos, create_line);
        if lines.len() > visible_height {
            lines.pop();
        }
    }

    // Pad remaining lines
    while lines.len() < visible_height {
        lines.push(Line::from(Span::styled(
            " ".repeat(width),
            Style::default().bg(pal.bg),
        )));
    }

    // Fuzzy search overlay on last row
    if is_active && app.mode == Mode::FuzzySearch {
        let match_count = pane.filtered_indices.len();
        let input_display = &pane.fuzzy_query;
        let cursor_char = if app.blink_on { "\u{258b}" } else { " " };
        let right_info = format!("[{} matches]", match_count);
        let left = format!(" [/] {}{}", input_display, cursor_char);
        let pad = width.saturating_sub(left.len() + right_info.len());

        let fuzzy_line = Line::from(vec![
            Span::styled(" [", Style::default().fg(pal.border_hot).bg(pal.surface)),
            Span::styled("/", Style::default().fg(pal.text_hot).bg(pal.surface)),
            Span::styled("] ", Style::default().fg(pal.border_hot).bg(pal.surface)),
            Span::styled(input_display.clone(), Style::default().fg(pal.text_hot).bg(pal.surface)),
            Span::styled(cursor_char, Style::default().fg(pal.text_hot).bg(pal.surface)),
            Span::styled(
                " ".repeat(pad),
                Style::default().bg(pal.surface),
            ),
            Span::styled(right_info, Style::default().fg(pal.text_dim).bg(pal.surface)),
        ]);

        if let Some(last) = lines.last_mut() {
            *last = fuzzy_line;
        }
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);

    // Scrollbar
    let total = pane.filtered_indices.len();
    if total > visible_height && visible_height > 0 {
        let track_height = visible_height;
        let thumb_size = ((visible_height as f64 / total as f64) * track_height as f64)
            .ceil() as usize;
        let thumb_size = thumb_size.max(1);
        let thumb_pos = if total <= visible_height {
            0
        } else {
            ((pane.scroll_offset as f64 / (total - visible_height) as f64)
                * (track_height - thumb_size) as f64) as usize
        };

        let scroll_x = area.x + area.width - 1;
        for row in 0..track_height {
            let y = area.y + row as u16;
            let (ch, color) = if row >= thumb_pos && row < thumb_pos + thumb_size {
                (app.symbols.scroll_thumb, pal.text_dim)
            } else {
                (app.symbols.scroll_track, pal.border_dim)
            };
            let span = Span::styled(ch, Style::default().fg(color).bg(pal.bg));
            f.render_widget(Paragraph::new(Line::from(span)), Rect::new(scroll_x, y, 1, 1));
        }
    }
}

/// Render tree view (#44).
fn render_tree(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let sym = &app.symbols;
    let width = area.width as usize;
    let visible_height = area.height as usize;
    let total = app.tree_nodes.len();
    let pane = app.pane();
    let cursor = pane.cursor.min(total.saturating_sub(1));

    // Compute scroll offset
    let scroll = if total <= visible_height {
        0
    } else if cursor < visible_height / 2 {
        0
    } else if cursor + visible_height / 2 >= total {
        total.saturating_sub(visible_height)
    } else {
        cursor.saturating_sub(visible_height / 2)
    };

    let end = (scroll + visible_height).min(total);
    let mut lines: Vec<Line> = Vec::new();

    for vi in scroll..end {
        let node = &app.tree_nodes[vi];
        let is_cursor = vi == cursor;
        let row_bg = if is_cursor { pal.surface } else { pal.bg };
        let text_color = if is_cursor { pal.text_hot } else { pal.text_dim };

        let mut spans: Vec<Span> = Vec::new();

        // Indicator
        let indicator = if is_cursor { sym.cursor } else { " " };
        spans.push(Span::styled(indicator, Style::default().fg(pal.text_hot).bg(row_bg)));
        spans.push(Span::styled(" ", Style::default().bg(row_bg)));

        // Indentation with tree glyphs
        let indent_width = node.depth * 2;
        if node.depth > 0 {
            // Leading pipe characters for depth
            for d in 0..node.depth.saturating_sub(1) {
                // Check if a sibling exists below at this depth — simplified: always show pipe
                let _ = d;
                spans.push(Span::styled(
                    sym.tree_pipe,
                    Style::default().fg(pal.border_dim).bg(row_bg),
                ));
            }
            // Branch or last glyph
            let is_last = {
                let mut last = true;
                for later in (vi + 1)..total {
                    if app.tree_nodes[later].depth < node.depth {
                        break;
                    }
                    if app.tree_nodes[later].depth == node.depth {
                        last = false;
                        break;
                    }
                }
                last
            };
            let branch_glyph = if is_last { sym.tree_last } else { sym.tree_branch };
            spans.push(Span::styled(
                branch_glyph,
                Style::default().fg(pal.border_dim).bg(row_bg),
            ));
        }

        // Expand/collapse indicator for dirs
        if node.entry.is_dir {
            let exp = if node.expanded { "-" } else { "+" };
            spans.push(Span::styled(
                exp,
                Style::default().fg(pal.text_hot).bg(row_bg),
            ));
        } else {
            spans.push(Span::styled(" ", Style::default().bg(row_bg)));
        }

        // Icon
        let icon = icon_for(&node.entry, sym);
        spans.push(Span::styled(
            format!("{} ", icon),
            Style::default().fg(text_color).bg(row_bg),
        ));

        // Name
        let name_avail = width.saturating_sub(indent_width + 5 + 3); // indicator + space + expand + icon + space
        let display_name = if node.entry.is_dir {
            format!("{}/", node.entry.name)
        } else {
            node.entry.name.clone()
        };
        let truncated = trunc_pad(&display_name, name_avail, name_avail);
        spans.push(Span::styled(truncated, Style::default().fg(text_color).bg(row_bg)));

        lines.push(Line::from(spans));
    }

    // Pad remaining
    while lines.len() < visible_height {
        lines.push(Line::from(Span::styled(
            " ".repeat(width),
            Style::default().bg(pal.bg),
        )));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Render the recursive search overlay (replaces the body area).
pub fn render_rsearch(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;
    let visible_height = area.height as usize;

    if visible_height < 2 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Search input bar
    let cursor_char = if app.blink_on { "\u{258b}" } else { " " };
    let total_scanned = app.rsearch_paths.len();
    let match_count = app.rsearch_results.len();
    let right_info = format!("[{}/{} matches]", match_count, total_scanned);

    let input_line = Line::from(vec![
        Span::styled(" [", Style::default().fg(pal.border_hot).bg(pal.surface)),
        Span::styled("?", Style::default().fg(pal.text_hot).bg(pal.surface)),
        Span::styled("] ", Style::default().fg(pal.border_hot).bg(pal.surface)),
        Span::styled(app.rsearch_query.clone(), Style::default().fg(pal.text_hot).bg(pal.surface)),
        Span::styled(cursor_char, Style::default().fg(pal.text_hot).bg(pal.surface)),
        Span::styled(
            " ".repeat(width.saturating_sub(5 + app.rsearch_query.len() + 1 + right_info.len())),
            Style::default().bg(pal.surface),
        ),
        Span::styled(right_info, Style::default().fg(pal.text_dim).bg(pal.surface)),
    ]);
    lines.push(input_line);

    // Results
    let results_height = visible_height - 1;
    let start = app.rsearch_scroll;
    let end = (start + results_height).min(app.rsearch_results.len());

    for vi in start..end {
        let (idx, _score) = app.rsearch_results[vi];
        let rel_path = &app.rsearch_paths[idx];
        let is_cursor = vi == app.rsearch_cursor;
        let row_bg = if is_cursor { pal.surface } else { pal.bg };
        let text_color = if is_cursor { pal.text_hot } else { pal.text_dim };

        let display = rel_path.to_string_lossy();

        // Build a fake FsEntry to get the icon
        let is_dir = rel_path.to_string_lossy().ends_with('/') ||
            app.pane().current_dir.join(rel_path).is_dir();
        let name = rel_path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let fake_entry = FsEntry {
            name,
            path: rel_path.to_path_buf(),
            is_dir,
            size: None,
            modified: None,
            is_symlink: false,
            link_target: None,
            permissions: None,
            is_classified: false,
        };
        let icon = icon_for(&fake_entry, &app.symbols);

        let indicator = if is_cursor { app.symbols.cursor } else { " " };
        let max_path = width.saturating_sub(4); // indicator + icon + space + padding

        let path_display = if display.chars().count() > max_path {
            let t: String = display.chars().take(max_path.saturating_sub(1)).collect();
            format!("{}\u{2026}", t)
        } else {
            format!("{:<width$}", display, width = max_path)
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, Style::default().fg(pal.text_hot).bg(row_bg)),
            Span::styled(format!("{} ", icon), Style::default().fg(text_color).bg(row_bg)),
            Span::styled(path_display, Style::default().fg(text_color).bg(row_bg)),
        ]));
    }

    // Pad remaining
    while lines.len() < visible_height {
        lines.push(Line::from(Span::styled(
            " ".repeat(width),
            Style::default().bg(pal.bg),
        )));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
