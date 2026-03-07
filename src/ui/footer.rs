use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Mode};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;

    // Error state overrides footer
    if let Some((msg, _)) = &app.error {
        let error_line = Line::from(vec![
            Span::styled(
                format!(" \u{26a0} {}", msg),
                Style::default().fg(pal.warn).bg(pal.surface),
            ),
        ]);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.border_dim))
            .style(Style::default().bg(pal.surface));
        let paragraph = Paragraph::new(error_line).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let hints = match app.mode {
        Mode::Normal | Mode::WaitingForG | Mode::WaitingForMark | Mode::WaitingForJumpToMark => {
            vec![
                ("hjkl", "move"),
                ("enter", "open"),
                ("/", "fuzzy"),
                ("space", "jump"),
                ("mx", "mark"),
                ("'x", "goto"),
                ("q", "quit"),
            ]
        }
        Mode::FuzzySearch => {
            vec![
                ("type", "filter"),
                ("enter", "confirm"),
                ("esc", "cancel"),
            ]
        }
        Mode::JumpKey => {
            vec![
                ("a-z", "jump to"),
                ("esc", "cancel"),
            ]
        }
    };

    let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default().bg(pal.surface))];
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  \u{00b7}  ",
                Style::default().fg(pal.border_mid).bg(pal.surface),
            ));
        }
        spans.push(Span::styled(
            *key,
            Style::default().fg(pal.text_mid).bg(pal.surface),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(pal.text_dim).bg(pal.surface),
        ));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
