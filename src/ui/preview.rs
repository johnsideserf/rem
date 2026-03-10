use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::highlight;
use crate::preview::{PreviewContent, load_preview};

/// Truncate a string to at most `max_chars` characters, appending `…` if truncated.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}\u{2026}", truncated)
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let height = area.height.saturating_sub(2) as usize; // account for border + label
    let width = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Label
    lines.push(Line::from(Span::styled(
        " P R E V I E W",
        Style::default().fg(pal.text_dim).bg(pal.bg),
    )));

    // Declassification overlay (#36)
    if let Some(tick) = app.declassify_tick {
        if tick <= 5 {
            lines.push(Line::from(Span::styled(
                " DECLASSIFYING...",
                Style::default().fg(pal.text_hot).bg(pal.bg),
            )));
            let reveal_pct = tick as f32 / 5.0;
            const SCRAMBLE_CHARS: &[char] = &['\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}', '\u{2580}', '\u{2584}'];
            // Fill remaining lines with scrambled/revealing content
            for row in 0..height.saturating_sub(2) {
                let mut row_str = String::new();
                for col in 0..width.saturating_sub(1) {
                    let reveal_col = (reveal_pct * width as f32) as usize;
                    if col < reveal_col {
                        row_str.push(' ');
                    } else {
                        let idx = ((row * 7 + col * 13 + tick as usize * 3) % SCRAMBLE_CHARS.len()) as usize;
                        row_str.push(SCRAMBLE_CHARS[idx]);
                    }
                }
                lines.push(Line::from(Span::styled(
                    format!(" {}", row_str),
                    Style::default().fg(pal.text_dim).bg(pal.bg),
                )));
            }

            // Pad and render
            let full_height = area.height as usize;
            while lines.len() < full_height {
                lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
            }
            let block = Block::default()
                .borders(Borders::LEFT)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(pal.border_dim))
                .style(Style::default().bg(pal.bg));
            let paragraph = Paragraph::new(lines).block(block);
            f.render_widget(paragraph, area);
            return;
        }
    }

    // Load preview for current entry
    if let Some(entry) = app.current_entry() {
        if entry.is_dir {
            // Show directory listing
            match std::fs::read_dir(&entry.path) {
                Ok(rd) => {
                    let mut items: Vec<String> = rd
                        .flatten()
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            let is_dir = e.file_type().map_or(false, |t| t.is_dir());
                            if is_dir { format!("{}/", name) } else { name }
                        })
                        .collect();
                    items.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

                    let count_line = format!(" {} items", items.len());
                    lines.push(Line::from(Span::styled(
                        count_line,
                        Style::default().fg(pal.text_mid).bg(pal.bg),
                    )));
                    lines.push(Line::from(Span::raw("")));

                    for item in items.iter().skip(app.preview_scroll).take(height.saturating_sub(3)) {
                        let display = format!(" {}", truncate_chars(item, width.saturating_sub(1)));
                        lines.push(Line::from(Span::styled(
                            display,
                            Style::default().fg(pal.text_dim).bg(pal.bg),
                        )));
                    }
                }
                Err(e) => {
                    lines.push(Line::from(Span::styled(
                        format!(" ERR: {}", e),
                        Style::default().fg(pal.warn).bg(pal.bg),
                    )));
                }
            }
        } else {
            let content = load_preview(&entry.path);
            match content {
                PreviewContent::Text(text_lines) => {
                    let total = text_lines.len();
                    let scroll = app.preview_scroll.min(total.saturating_sub(1));
                    for line in text_lines.iter().skip(scroll).take(height.saturating_sub(1)) {
                        let truncated = truncate_chars(line, width.saturating_sub(1));
                        let mut hl_spans = vec![
                            Span::styled(" ", Style::default().bg(pal.bg)),
                        ];
                        hl_spans.extend(highlight::highlight_line(&truncated, &entry.path, &pal));
                        lines.push(Line::from(hl_spans));
                    }
                }
                PreviewContent::Binary => {
                    lines.push(Line::from(Span::styled(
                        " [BINARY FILE]",
                        Style::default().fg(pal.text_mid).bg(pal.bg),
                    )));
                }
                PreviewContent::TooLarge => {
                    lines.push(Line::from(Span::styled(
                        " [FILE > 1MB]",
                        Style::default().fg(pal.text_mid).bg(pal.bg),
                    )));
                }
                PreviewContent::Empty => {
                    lines.push(Line::from(Span::styled(
                        " [EMPTY]",
                        Style::default().fg(pal.text_dim).bg(pal.bg),
                    )));
                }
                PreviewContent::Error(e) => {
                    lines.push(Line::from(Span::styled(
                        format!(" ERR: {}", e),
                        Style::default().fg(pal.warn).bg(pal.bg),
                    )));
                }
                PreviewContent::Image { width: img_w, height: img_h, format: fmt, braille: braille_lines } => {
                    lines.push(Line::from(Span::styled(
                        format!(" IMAGE: {}x{} {}", img_w, img_h, fmt),
                        Style::default().fg(pal.text_mid).bg(pal.bg),
                    )));
                    lines.push(Line::from(Span::raw("")));
                    for line in braille_lines.iter().skip(app.preview_scroll).take(height.saturating_sub(3)) {
                        let display = format!(" {}", truncate_chars(line, width.saturating_sub(1)));
                        lines.push(Line::from(Span::styled(
                            display,
                            Style::default().fg(pal.text_mid).bg(pal.bg),
                        )));
                    }
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            " NO SELECTION",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
    }

    // Pad
    let full_height = area.height as usize;
    while lines.len() < full_height {
        lines.push(Line::from(Span::styled(
            "",
            Style::default().bg(pal.bg),
        )));
    }

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_dim))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
