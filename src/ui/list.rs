use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Mode, JUMP_KEYS, file_type_badge, format_size};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;

    // Determine which columns to show
    let show_size = width >= 90;
    let show_type = width >= 80;

    let size_col = if show_size { 9 } else { 0 };
    let type_col = if show_type { 5 } else { 0 };
    let jump_col = 5; // "[a] "
    let sigil_col = 2;
    let right_cols = size_col + type_col;
    let name_width = width.saturating_sub(jump_col + sigil_col + right_cols + 2); // +2 for padding

    let visible_height = area.height as usize;
    let start = app.scroll_offset;
    let end = (start + visible_height).min(app.filtered_indices.len());

    let mut lines: Vec<Line> = Vec::new();

    for vi in start..end {
        let idx = app.filtered_indices[vi];
        let entry = &app.entries[idx];
        let is_cursor = vi == app.cursor;
        let is_fuzzy_match = !app.fuzzy_query.is_empty();

        let row_bg = if is_cursor { pal.surface } else { pal.bg };
        let text_color = if is_cursor {
            pal.text_hot
        } else if is_fuzzy_match {
            pal.text_mid
        } else {
            pal.text_dim
        };

        let mut spans: Vec<Span> = Vec::new();

        // Selected indicator
        if is_cursor {
            spans.push(Span::styled("\u{25b6}", Style::default().fg(pal.text_hot).bg(row_bg)));
        } else {
            spans.push(Span::styled(" ", Style::default().bg(row_bg)));
        }

        // Jump key column
        if app.mode == Mode::JumpKey {
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

        // Sigil
        let sigil = if entry.is_dir { "\u{25a3} " } else { "\u{25fb} " };
        spans.push(Span::styled(sigil, Style::default().fg(text_color).bg(row_bg)));

        // Name
        let display_name = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };
        let truncated = if display_name.len() > name_width {
            format!("{}\u{2026}", &display_name[..name_width.saturating_sub(1)])
        } else {
            format!("{:<width$}", display_name, width = name_width)
        };
        spans.push(Span::styled(truncated, Style::default().fg(text_color).bg(row_bg)));

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

    // Pad remaining lines
    while lines.len() < visible_height {
        lines.push(Line::from(Span::styled(
            " ".repeat(width),
            Style::default().bg(pal.bg),
        )));
    }

    // Fuzzy search overlay on last row
    if app.mode == Mode::FuzzySearch {
        let match_count = app.filtered_indices.len();
        let input_display = &app.fuzzy_query;
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
    let total = app.filtered_indices.len();
    if total > visible_height && visible_height > 0 {
        let track_height = visible_height;
        let thumb_size = ((visible_height as f64 / total as f64) * track_height as f64)
            .ceil() as usize;
        let thumb_size = thumb_size.max(1);
        let thumb_pos = if total <= visible_height {
            0
        } else {
            ((app.scroll_offset as f64 / (total - visible_height) as f64)
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
