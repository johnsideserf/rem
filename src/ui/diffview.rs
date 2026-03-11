use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::diff::DiffKind;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let Some(diff) = &app.file_diff else { return };

    // Split into two halves
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let height = area.height.saturating_sub(2) as usize;
    let scroll = diff.scroll;

    // Left side
    render_side(f, &diff.left_path, &diff.left_lines, scroll, height, halves[0], pal, true);
    // Right side
    render_side(f, &diff.right_path, &diff.right_lines, scroll, height, halves[1], pal, false);
}

fn render_side(
    f: &mut Frame,
    title: &str,
    lines: &[crate::diff::DiffLine],
    scroll: usize,
    height: usize,
    area: Rect,
    pal: crate::palette::Palette,
    is_left: bool,
) {
    let width = area.width.saturating_sub(6) as usize; // 4 for line number + 2 for border
    let mut display_lines: Vec<Line> = Vec::new();

    for (i, line) in lines.iter().enumerate().skip(scroll).take(height) {
        let line_num = i + 1;
        let color = match line.kind {
            DiffKind::Same => pal.text_dim,
            DiffKind::Added => pal.text_hot,
            DiffKind::Removed => pal.warn,
        };
        let prefix = match line.kind {
            DiffKind::Same => " ",
            DiffKind::Added => "+",
            DiffKind::Removed => "-",
        };

        let text = if line.text.chars().count() > width {
            let t: String = line.text.chars().take(width.saturating_sub(1)).collect();
            format!("{}\u{2026}", t)
        } else {
            line.text.clone()
        };

        display_lines.push(Line::from(vec![
            Span::styled(
                format!("{:>3} ", line_num),
                Style::default().fg(pal.border_dim).bg(pal.bg),
            ),
            Span::styled(
                prefix,
                Style::default().fg(color).bg(pal.bg),
            ),
            Span::styled(
                text,
                Style::default().fg(color).bg(pal.bg),
            ),
        ]));
    }

    // Pad
    while display_lines.len() < height {
        display_lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    }

    let truncated_title = if title.chars().count() > 20 {
        let t: String = title.chars().take(19).collect();
        format!("{}\u{2026}", t)
    } else {
        title.to_string()
    };

    let borders = if is_left { Borders::ALL } else { Borders::TOP | Borders::RIGHT | Borders::BOTTOM };
    let block = Block::default()
        .title(format!(" {} ", truncated_title))
        .borders(borders)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_mid))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(display_lines).block(block);
    f.render_widget(paragraph, area);
}
