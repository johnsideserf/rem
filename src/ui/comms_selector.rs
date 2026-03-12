use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::App;
use crate::comms::Channel;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let channel_count = Channel::ALL_CHANNELS.len();

    // Height: border(2) + title(1) + blank(1) + channels + blank(1) + feed info(1) + refresh(1)
    let box_w: u16 = 46;
    let box_h: u16 = channel_count as u16 + 8;

    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let popup = Rect::new(x, y, box_w.min(area.width), box_h.min(area.height));

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_hot))
        .style(Style::default().bg(pal.surface));

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        " C O M M S  F R E Q U E N C Y",
        Style::default().fg(pal.text_dim),
    )));
    lines.push(Line::from(Span::raw("")));

    // Channel rows
    for (i, channel) in Channel::ALL_CHANNELS.iter().enumerate() {
        let is_selected = i == app.comms.selector_cursor;
        let is_active = *channel == app.comms.active_channel;

        let marker = if is_selected {
            format!("{} ", app.symbols.cursor)
        } else {
            "  ".to_string()
        };
        let active = if is_active {
            format!(" {}", app.symbols.checkmark)
        } else {
            String::new()
        };

        let name_style = if is_selected {
            Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(pal.text_mid)
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(pal.text_hot)),
            Span::styled(channel.label(), name_style),
            Span::styled(active, Style::default().fg(pal.text_hot)),
            Span::styled(format!("  [{}]", channel.code()), Style::default().fg(pal.text_dim)),
        ]));
    }

    // Blank separator
    lines.push(Line::from(Span::raw("")));

    // Feed info
    let feed_count = app.comms.feeds.len();
    let rss_count = app.comms.rss_items.len();
    lines.push(Line::from(Span::styled(
        format!(" FEEDS: {}  ITEMS: {}", feed_count, rss_count),
        Style::default().fg(pal.text_dim),
    )));

    // Last refresh
    let refresh_info = if let Some(last) = app.comms.last_fetch {
        let elapsed = last.elapsed().as_secs();
        if elapsed < 60 {
            format!(" LAST REFRESH: {}s AGO", elapsed)
        } else {
            format!(" LAST REFRESH: {}m AGO", elapsed / 60)
        }
    } else {
        " LAST REFRESH: NEVER".to_string()
    };
    lines.push(Line::from(Span::styled(
        refresh_info,
        Style::default().fg(pal.text_dim),
    )));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup);
}
