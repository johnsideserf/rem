use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::App;
use crate::palette::Palette;
use crate::symbols::SymbolVariant;

const THEMES: &[(&str, &str)] = &[
    ("PHOSPHOR GREEN", "Ship terminal"),
    ("AMBER",          "Colony terminal"),
    ("CORPORATE CYAN", "Executive terminal"),
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let sym_variants = SymbolVariant::ALL;

    // Height: border(2) + color title(1) + blank(1) + 3 themes + blank(1) + sym title(1) + sym items + blank(1) + effects title(1) + glitch toggle(1)
    let box_w: u16 = 42;
    let box_h: u16 = (THEMES.len() + sym_variants.len()) as u16 + 11;

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

    // Color section title
    lines.push(Line::from(Span::styled(
        " C O L O R  P R O F I L E",
        Style::default().fg(pal.text_mid),
    )));
    lines.push(Line::from(Span::raw("")));

    // Color theme options (indices 0..3)
    for (i, (name, desc)) in THEMES.iter().enumerate() {
        let is_selected = i == app.theme_picker_cursor;
        let is_current = match (i, pal.variant) {
            (0, crate::throbber::PaletteVariant::Green) => true,
            (1, crate::throbber::PaletteVariant::Amber) => true,
            (2, crate::throbber::PaletteVariant::Cyan) => true,
            _ => false,
        };

        let marker = if is_selected { format!("{} ", app.symbols.cursor) } else { "  ".to_string() };
        let active = if is_current { format!(" {}", app.symbols.checkmark) } else { String::new() };

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

    // Blank separator
    lines.push(Line::from(Span::raw("")));

    // Symbol section title
    lines.push(Line::from(Span::styled(
        " S Y M B O L  S E T",
        Style::default().fg(pal.text_mid),
    )));

    // Symbol set options (indices 3..3+N)
    for (si, variant) in sym_variants.iter().enumerate() {
        let idx = THEMES.len() + si;
        let is_selected = idx == app.theme_picker_cursor;
        let is_current = app.symbols.variant == *variant;

        let marker = if is_selected { format!("{} ", app.symbols.cursor) } else { "  ".to_string() };
        let active = if is_current { format!(" {}", app.symbols.checkmark) } else { String::new() };

        let name_style = if is_selected {
            Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(pal.text_mid)
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(pal.text_hot)),
            Span::styled(variant.label(), name_style),
            Span::styled(active, Style::default().fg(pal.text_hot)),
            Span::styled(format!("  {}", variant.desc()), Style::default().fg(pal.text_dim)),
        ]));
    }

    // Blank separator
    lines.push(Line::from(Span::raw("")));

    // Effects section title
    lines.push(Line::from(Span::styled(
        " E F F E C T S",
        Style::default().fg(pal.text_mid),
    )));

    // Glitch toggle
    {
        let idx = THEMES.len() + sym_variants.len();
        let is_selected = idx == app.theme_picker_cursor;
        let marker = if is_selected { format!("{} ", app.symbols.cursor) } else { "  ".to_string() };
        let status = if app.glitch_enabled { "ON" } else { "OFF" };
        let status_style = if app.glitch_enabled { pal.text_hot } else { pal.text_dim };

        let name_style = if is_selected {
            Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(pal.text_mid)
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(pal.text_hot)),
            Span::styled("CRT GLITCH", name_style),
            Span::styled(format!("  [{}]", status), Style::default().fg(status_style)),
            Span::styled("  Signal degradation", Style::default().fg(pal.text_dim)),
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

/// Total number of selectable items in the picker (color themes + symbol sets).
pub const THEME_COUNT: usize = THEMES.len();

pub fn total_picker_items() -> usize {
    THEMES.len() + SymbolVariant::ALL.len() + 1 // +1 for glitch toggle
}
