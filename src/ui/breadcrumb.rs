use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    render_pane(f, app, app.active_pane, area, true);
}

pub fn render_pane(f: &mut Frame, app: &App, pane_idx: usize, area: Rect, show_cursor: bool) {
    let pal = app.palette;

    // Archive mode: show archive path with internal directory (#19)
    let archive_path_str;
    let path_str = if let Some(archive) = &app.archive {
        let archive_name = archive.archive_path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        archive_path_str = if archive.internal_dir.is_empty() {
            format!("{}:", archive_name)
        } else {
            format!("{}:/{}", archive_name, archive.internal_dir.trim_end_matches('/'))
        };
        archive_path_str.clone()
    } else {
        app.panes[pane_idx].current_dir.to_string_lossy().into_owned()
    };
    let segments: Vec<&str> = path_str.split(['/', '\\', ':']).filter(|s| !s.is_empty()).collect();

    let mut spans = vec![Span::styled(" ", Style::default())];

    let max_width = area.width as usize - 4; // reserve space for cursor
    let mut built = String::new();

    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let sep = " / ";
        let addition = if built.is_empty() {
            seg.to_string()
        } else {
            format!("{}{}", sep, seg)
        };

        if built.len() + addition.len() > max_width && !built.is_empty() {
            // Truncate from the left
            spans = vec![Span::styled(" ", Style::default())];
            spans.push(Span::styled("\u{2026} / ", Style::default().fg(pal.text_dim)));
            built.clear();
        }

        if !built.is_empty() {
            spans.push(Span::styled(" / ", Style::default().fg(pal.text_dim)));
        }

        let style = if is_last {
            Style::default().fg(pal.text_hot)
        } else {
            Style::default().fg(pal.text_mid)
        };
        spans.push(Span::styled(seg.to_uppercase(), style));
        built.push_str(&addition);
    }

    // Blinking cursor (only on active pane)
    if show_cursor && app.blink_on {
        spans.push(Span::styled(format!(" {}", app.symbols.text_cursor), Style::default().fg(pal.text_hot)));
    } else {
        spans.push(Span::styled("  ", Style::default()));
    }

    // Use pulsed border color for active pane (#18)
    let border_color = if show_cursor { app.pulsed_border() } else { pal.border_dim };
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
