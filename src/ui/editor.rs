use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::highlight;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let Some(ed) = &app.editor else { return };

    let width = area.width as usize;
    let height = area.height as usize;
    if height < 2 || width < 10 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Title bar (first row)
    let filename = ed.path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string());
    let modified_tag = if ed.dirty { " [MODIFIED]" } else { "" };
    let position = format!("Ln {}, Col {} ", ed.cursor_row + 1, ed.cursor_col + 1);

    let title_left = format!(" EDIT \u{2502} {}{}", filename, modified_tag);
    let pad = width.saturating_sub(title_left.len() + position.len());

    lines.push(Line::from(vec![
        Span::styled(" EDIT ", Style::default().fg(pal.text_hot).bg(pal.surface)),
        Span::styled("\u{2502} ", Style::default().fg(pal.border_dim).bg(pal.surface)),
        Span::styled(
            filename,
            Style::default().fg(pal.text_mid).bg(pal.surface),
        ),
        Span::styled(
            modified_tag,
            Style::default().fg(pal.warn).bg(pal.surface),
        ),
        Span::styled(
            " ".repeat(pad),
            Style::default().bg(pal.surface),
        ),
        Span::styled(
            position,
            Style::default().fg(pal.text_dim).bg(pal.surface),
        ),
    ]));

    // Content area
    let content_height = height - 1; // minus title bar
    let total_lines = ed.lines.len();
    let gutter_width = format!("{}", total_lines).len().max(3) + 1; // digits + separator

    let content_cols = width.saturating_sub(gutter_width + 1); // +1 for separator char

    for vi in 0..content_height {
        let line_idx = ed.scroll_row + vi;

        if line_idx >= total_lines {
            // Empty line below content
            let gutter = format!("{:>width$} ", "~", width = gutter_width);
            lines.push(Line::from(vec![
                Span::styled(gutter, Style::default().fg(pal.border_dim).bg(pal.bg)),
                Span::styled(
                    " ".repeat(content_cols),
                    Style::default().bg(pal.bg),
                ),
            ]));
            continue;
        }

        let is_cursor_row = line_idx == ed.cursor_row;
        let row_bg = if is_cursor_row { pal.surface } else { pal.bg };

        // Gutter (line number)
        let gutter = format!("{:>width$}\u{2502}", line_idx + 1, width = gutter_width);
        let gutter_style = if is_cursor_row {
            Style::default().fg(pal.text_mid).bg(row_bg)
        } else {
            Style::default().fg(pal.border_dim).bg(row_bg)
        };

        let mut row_spans: Vec<Span> = vec![
            Span::styled(gutter, gutter_style),
        ];

        // Line content with syntax highlighting
        let line_text = &ed.lines[line_idx];
        let visible_start = ed.scroll_col;
        let char_count = line_text.chars().count();

        if visible_start >= char_count {
            // Line is scrolled past — show empty with cursor if needed
            if is_cursor_row && ed.cursor_col >= visible_start {
                let cursor_offset = ed.cursor_col - visible_start;
                if cursor_offset < content_cols {
                    row_spans.push(Span::styled(
                        " ".repeat(cursor_offset),
                        Style::default().bg(row_bg),
                    ));
                    if app.blink_on {
                        row_spans.push(Span::styled(
                            "\u{258b}",
                            Style::default().fg(pal.text_hot).bg(row_bg),
                        ));
                        let remaining = content_cols.saturating_sub(cursor_offset + 1);
                        row_spans.push(Span::styled(
                            " ".repeat(remaining),
                            Style::default().bg(row_bg),
                        ));
                    } else {
                        row_spans.push(Span::styled(
                            " ".repeat(content_cols.saturating_sub(cursor_offset)),
                            Style::default().bg(row_bg),
                        ));
                    }
                } else {
                    row_spans.push(Span::styled(
                        " ".repeat(content_cols),
                        Style::default().bg(row_bg),
                    ));
                }
            } else {
                row_spans.push(Span::styled(
                    " ".repeat(content_cols),
                    Style::default().bg(row_bg),
                ));
            }
        } else {
            // Get the visible slice of the line
            let visible_chars: String = line_text.chars()
                .skip(visible_start)
                .take(content_cols)
                .collect();

            if is_cursor_row {
                // Render char-by-char for cursor insertion
                let hl_spans = highlight::highlight_line(&visible_chars, &ed.path, &pal);
                let cursor_vis_col = ed.cursor_col.saturating_sub(visible_start);

                // Flatten highlighted spans into individual characters
                let mut char_styles: Vec<(char, Style)> = Vec::new();
                for span in &hl_spans {
                    for ch in span.content.chars() {
                        char_styles.push((ch, span.style));
                    }
                }

                let mut col = 0;
                let mut cursor_rendered = false;
                for (ch, style) in &char_styles {
                    if col == cursor_vis_col && !cursor_rendered {
                        // Render cursor: inverted char
                        if app.blink_on {
                            row_spans.push(Span::styled(
                                ch.to_string(),
                                Style::default()
                                    .fg(pal.bg)
                                    .bg(pal.text_hot),
                            ));
                        } else {
                            row_spans.push(Span::styled(
                                ch.to_string(),
                                style.bg(row_bg),
                            ));
                        }
                        cursor_rendered = true;
                    } else {
                        row_spans.push(Span::styled(
                            ch.to_string(),
                            style.bg(row_bg),
                        ));
                    }
                    col += 1;
                }

                // Cursor at end of line (past all chars)
                if !cursor_rendered && cursor_vis_col >= char_styles.len() && cursor_vis_col < content_cols {
                    let pad_before = cursor_vis_col.saturating_sub(char_styles.len());
                    if pad_before > 0 {
                        row_spans.push(Span::styled(
                            " ".repeat(pad_before),
                            Style::default().bg(row_bg),
                        ));
                    }
                    if app.blink_on {
                        row_spans.push(Span::styled(
                            "\u{258b}",
                            Style::default().fg(pal.text_hot).bg(row_bg),
                        ));
                    }
                    col = cursor_vis_col + 1;
                }

                // Pad remaining
                let rendered = col.max(if !cursor_rendered { char_styles.len() } else { 0 });
                let remaining = content_cols.saturating_sub(rendered.max(cursor_vis_col + 1));
                if remaining > 0 {
                    row_spans.push(Span::styled(
                        " ".repeat(remaining),
                        Style::default().bg(row_bg),
                    ));
                }
            } else {
                // Non-cursor row: just highlight
                let hl_spans = highlight::highlight_line(&visible_chars, &ed.path, &pal);
                let mut rendered_len = 0;
                for span in hl_spans {
                    rendered_len += span.content.chars().count();
                    row_spans.push(Span::styled(
                        span.content.into_owned(),
                        span.style.bg(row_bg),
                    ));
                }
                // Pad
                if rendered_len < content_cols {
                    row_spans.push(Span::styled(
                        " ".repeat(content_cols - rendered_len),
                        Style::default().bg(row_bg),
                    ));
                }
            }
        }

        lines.push(Line::from(row_spans));
    }

    // Confirm exit overlay on last line
    if ed.confirm_exit {
        let msg = " UNSAVED CHANGES \u{2502} y: discard & exit \u{2502} any key: cancel ";
        let pad = width.saturating_sub(msg.len());
        let confirm_line = Line::from(vec![
            Span::styled(
                msg,
                Style::default().fg(pal.warn).bg(pal.surface).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " ".repeat(pad),
                Style::default().bg(pal.surface),
            ),
        ]);
        if let Some(last) = lines.last_mut() {
            *last = confirm_line;
        }
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);

    // Scrollbar
    if total_lines > content_height && content_height > 0 {
        let track_height = content_height;
        let thumb_size = ((content_height as f64 / total_lines as f64) * track_height as f64)
            .ceil() as usize;
        let thumb_size = thumb_size.max(1);
        let thumb_pos = if total_lines <= content_height {
            0
        } else {
            ((ed.scroll_row as f64 / (total_lines - content_height) as f64)
                * (track_height - thumb_size) as f64) as usize
        };

        let scroll_x = area.x + area.width - 1;
        for row in 0..track_height {
            let y = area.y + 1 + row as u16; // +1 for title bar
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
