use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, format_size};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;
    let height = area.height as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        " D I S K  A L L O C A T I O N",
        Style::default().fg(pal.text_dim).bg(pal.bg),
    )));

    if let Some(data) = &app.disk_usage {
        let max_size = data.entries.first().map(|e| e.size).unwrap_or(1).max(1);
        let bar_width = (width.saturating_sub(28)).min(12).max(4);

        for entry in data.entries.iter().take(height.saturating_sub(4)) {
            let sym = &app.symbols;
            let icon = if entry.is_dir { sym.dir_icon } else { sym.file_icon };
            let size_str = format_size(entry.size);
            let pct = if data.total_size > 0 {
                (entry.size as f64 / data.total_size as f64 * 100.0) as u64
            } else {
                0
            };

            // Mini bar
            let filled = (entry.size as f64 / max_size as f64 * bar_width as f64) as usize;
            let empty = bar_width.saturating_sub(filled);
            let bar = format!(
                "{}{}",
                sym.bar_fill.repeat(filled),
                sym.bar_empty.repeat(empty),
            );

            // Truncate name
            let name_width = width.saturating_sub(bar_width + 20);
            let name_display = if entry.name.chars().count() > name_width {
                let t: String = entry.name.chars().take(name_width.saturating_sub(1)).collect();
                format!("{}\u{2026}", t)
            } else {
                format!("{:<width$}", entry.name, width = name_width)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", icon),
                    Style::default().fg(pal.text_dim).bg(pal.bg),
                ),
                Span::styled(
                    name_display,
                    Style::default().fg(pal.text_mid).bg(pal.bg),
                ),
                Span::styled(
                    format!("{:>9}", size_str),
                    Style::default().fg(pal.text_hot).bg(pal.bg),
                ),
                Span::styled(
                    format!(" {} ", bar),
                    Style::default().fg(pal.text_hot).bg(pal.bg),
                ),
                Span::styled(
                    format!("{:>3}%", pct),
                    Style::default().fg(pal.text_dim).bg(pal.bg),
                ),
            ]));
        }

        // Total line
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            format!(
                " TOTAL: {} \u{00b7} {} ITEMS",
                format_size(data.total_size),
                data.total_items,
            ),
            Style::default().fg(pal.text_hot).bg(pal.bg),
        )));
    } else if let Some(scan) = &app.disk_scan {
        // Active scan in progress
        let throbber = scan.throbber.frame();
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} SCANNING ALLOCATION... ", throbber),
                Style::default().fg(pal.text_hot).bg(pal.bg),
            ),
            Span::styled(
                format!("{} NODES", scan.nodes),
                Style::default().fg(pal.text_mid).bg(pal.bg),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            " NO DATA",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
    }

    // Pad remaining
    while lines.len() < height {
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
