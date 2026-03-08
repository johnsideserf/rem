use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::App;
use crate::palette::Palette;

const THEMES: &[(&str, &str)] = &[
    ("PHOSPHOR GREEN", "Classic CRT terminal"),
    ("AMBER",          "Corporate mainframe"),
    ("DEGRADED CYAN",  "Field unit signal"),
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;

    let box_w: u16 = 38;
    let box_h: u16 = (THEMES.len() as u16) + 4; // border(2) + title(1) + blank(1) + items

    // Center the popup
    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let popup = Rect::new(x, y, box_w.min(area.width), box_h.min(area.height));

    // Clear the area behind popup
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_hot))
        .style(Style::default().bg(pal.surface));

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        " T H E M E  S E L E C T",
        Style::default().fg(pal.text_mid),
    )));
    lines.push(Line::from(Span::raw("")));

    // Theme options
    for (i, (name, desc)) in THEMES.iter().enumerate() {
        let is_selected = i == app.theme_picker_cursor;
        let is_current = match (i, pal.variant) {
            (0, crate::throbber::PaletteVariant::Green) => true,
            (1, crate::throbber::PaletteVariant::Amber) => true,
            (2, crate::throbber::PaletteVariant::Cyan) => true,
            _ => false,
        };

        let marker = if is_selected { "\u{25b6} " } else { "  " };
        let active = if is_current { " \u{2713}" } else { "" };

        let name_style = if is_selected {
            Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(pal.text_mid)
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(pal.text_hot)),
            Span::styled(*name, name_style),
            Span::styled(active, Style::default().fg(pal.text_hot)),
            Span::styled(format!("  {}", desc), Style::default().fg(pal.text_dim)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup);
}

/// Get the palette for a given theme index.
pub fn palette_for_index(idx: usize) -> Palette {
    match idx {
        0 => Palette::phosphor_green(),
        1 => Palette::amber(),
        2 => Palette::degraded_cyan(),
        _ => Palette::phosphor_green(),
    }
}

pub const THEME_COUNT: usize = THEMES.len();
