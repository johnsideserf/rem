use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::logo;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let pane = app.pane();

    let item_count = pane.entries.len();
    let mark_count = app.visual_marks.len();

    // Split header: left (status) | right (logo)
    let badge_width = logo::HEADER_BADGE.chars().count() as u16 + 2;
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(30),
            Constraint::Length(badge_width),
        ])
        .split(area);

    // Left side: status info
    let mut spans = vec![
        Span::styled(" REM", Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)),
        Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)),
        Span::styled("FILE SYSTEM", Style::default().fg(pal.text_mid)),
        Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)),
        Span::styled(format!("ITEMS:{}", item_count), Style::default().fg(pal.text_hot)),
    ];

    if mark_count > 0 {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled(
            format!("MARKED:{}", mark_count),
            Style::default().fg(pal.text_hot),
        ));
    }

    if app.show_telemetry {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled("TELEM:ACTIVE", Style::default().fg(pal.text_hot)));
    }

    spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
    spans.push(Span::styled(app.heartbeat.frame(), Style::default().fg(pal.text_hot)));
    spans.push(Span::styled("  ", Style::default()));
    spans.push(Span::styled("SYS:NOMINAL", Style::default().fg(pal.text_hot)));

    let left_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_mid))
        .style(Style::default().bg(pal.surface));

    let left_para = Paragraph::new(Line::from(spans)).block(left_block);
    f.render_widget(left_para, halves[0]);

    // Right side: corporate badge
    let logo_spans = vec![
        Span::styled(
            "\u{25c6}",
            Style::default().fg(pal.border_hot).bg(pal.surface),
        ),
        Span::styled(
            " WEYLAND-YUTANI ",
            Style::default().fg(pal.text_mid).bg(pal.surface),
        ),
        Span::styled(
            "CORP ",
            Style::default().fg(pal.text_hot).bg(pal.surface)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let right_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_mid))
        .style(Style::default().bg(pal.surface));

    let right_para = Paragraph::new(Line::from(logo_spans))
        .block(right_block)
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(right_para, halves[1]);
}
