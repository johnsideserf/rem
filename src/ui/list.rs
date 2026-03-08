use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Mode, OpType, JUMP_KEYS, file_type_badge, format_size, icon_for};

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
    render_pane(f, app, app.active_pane, area);
}

pub fn render_pane(f: &mut Frame, app: &App, pane_idx: usize, area: Rect) {
    let pal = app.palette;
    let pane = &app.panes[pane_idx];
    let is_active = pane_idx == app.active_pane;
    let width = area.width as usize;

    // Determine which columns to show
    let show_size = width >= 90;
    let show_type = width >= 80;

    let size_col = if show_size { 9 } else { 0 };
    let type_col = if show_type { 5 } else { 0 };
    let jump_col = 5;
    let sigil_col = 2;
    let right_cols = size_col + type_col;
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

        let row_bg = if is_cursor { pal.surface } else { pal.bg };
        let text_color = if is_cursor {
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

        let mut spans: Vec<Span> = Vec::new();

        // Indicator column: ▶ cursor, ◆ marked, ✂ cut, ⊕ copied
        let indicator = if is_cursor && is_marked {
            "\u{25c6}" // ◆
        } else if is_cursor {
            "\u{25b6}" // ▶
        } else if is_marked {
            "\u{25c6}" // ◆
        } else if is_cut {
            "\u{2702}" // ✂
        } else if is_copied {
            "\u{2295}" // ⊕
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

        // File icon (Nerd Font glyph)
        let icon = icon_for(entry);
        let icon_text = format!("{} ", icon);
        let mut icon_style = Style::default().fg(text_color).bg(row_bg);
        if is_cut {
            icon_style = icon_style.add_modifier(Modifier::ITALIC);
        }
        spans.push(Span::styled(icon_text, icon_style));

        // Name — check for Rename/Create mode inline editing
        let is_rename_row = is_active && is_cursor && app.mode == Mode::Rename;

        if is_rename_row {
            let cursor_char = if app.blink_on { "\u{258b}" } else { " " };
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
                None => "\u{2014}".to_string(),
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
        let icon = if is_dir { "\u{f07b} " } else { "\u{f15b} " };
        let cursor_char = if app.blink_on { "\u{258b}" } else { " " };
        let display = format!("{}{}", app.create_buf, cursor_char);
        let truncated = trunc_pad(&display, name_width, name_width);

        let create_line = Line::from(vec![
            Span::styled("\u{25b6}", Style::default().fg(pal.text_hot).bg(pal.border_mid)),
            Span::styled("    ", Style::default().bg(pal.border_mid)),
            Span::styled(icon, Style::default().fg(pal.text_hot).bg(pal.border_mid)),
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
                ("\u{2588}", pal.text_dim)
            } else {
                ("\u{2502}", pal.border_dim)
            };
            let span = Span::styled(ch, Style::default().fg(color).bg(pal.bg));
            f.render_widget(Paragraph::new(Line::from(span)), Rect::new(scroll_x, y, 1, 1));
        }
    }
}
