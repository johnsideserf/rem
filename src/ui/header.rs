use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;

    let item_count = app.entries.len();

    let spans = vec![
        Span::styled(" REM", Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)),
        Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)),
        Span::styled("FILE SYSTEM", Style::default().fg(pal.text_mid)),
        Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)),
        Span::styled(format!("ITEMS:{}", item_count), Style::default().fg(pal.text_hot)),
        Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)),
        Span::styled("SYS:NOMINAL", Style::default().fg(pal.text_hot)),
    ];

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_mid))
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
