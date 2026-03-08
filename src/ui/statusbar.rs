use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn should_show(app: &App) -> bool {
    app.bg_operation.is_some() || app.op_feedback.is_some()
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;

    let spans = if let Some(bg) = &app.bg_operation {
        // Active background operation: throbber + label + file + progress bar
        let throbber_char = bg.throbber.frame();
        let pct = if bg.total > 0 {
            (bg.done as f64 / bg.total as f64 * 100.0) as u64
        } else {
            0
        };

        let progress_str = format!("{}/{}", bg.done, bg.total);
        let file_display = if bg.current_file.len() > 20 {
            format!("{}\u{2026}", &bg.current_file[..19])
        } else {
            bg.current_file.clone()
        };

        // Progress bar: determinate
        let bar_width = 20usize.min(width.saturating_sub(40));
        let filled = (pct as usize * bar_width / 100).min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));

        let file_display_len = file_display.len();
        let label_len = bg.label.len();
        let progress_str_len = progress_str.len();

        let mut s = vec![
            Span::styled(
                format!(" {} ", throbber_char),
                Style::default().fg(pal.text_hot).bg(pal.surface),
            ),
            Span::styled(
                bg.label.clone(),
                Style::default().fg(pal.text_mid).bg(pal.surface),
            ),
            Span::styled(
                "  ",
                Style::default().bg(pal.surface),
            ),
            Span::styled(
                file_display,
                Style::default().fg(pal.text_dim).bg(pal.surface),
            ),
            Span::styled(
                "  ",
                Style::default().bg(pal.surface),
            ),
            Span::styled(
                bar,
                Style::default().fg(pal.text_hot).bg(pal.surface),
            ),
            Span::styled(
                format!(" {}%", pct),
                Style::default().fg(pal.text_mid).bg(pal.surface),
            ),
            Span::styled(
                format!("  {}", progress_str),
                Style::default().fg(pal.text_dim).bg(pal.surface),
            ),
        ];

        // Pad remainder
        let used: usize = 3 + label_len + 2 + file_display_len + 2 + bar_width + 5 + progress_str_len + 2;
        if used < width {
            s.push(Span::styled(
                " ".repeat(width - used),
                Style::default().bg(pal.surface),
            ));
        }
        s
    } else if let Some(fb) = &app.op_feedback {
        // Feedback: success/error message
        let color = if fb.success { pal.text_hot } else { pal.warn };
        vec![
            Span::styled(
                format!(" {}", fb.label),
                Style::default().fg(color).bg(pal.surface),
            ),
        ]
    } else {
        vec![]
    };

    if spans.is_empty() {
        return;
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
