pub mod boot;
mod header;
mod breadcrumb;
mod editor;
mod list;
mod sidebar;
mod preview;
mod statusbar;
mod telemetry;
mod footer;
pub mod theme_picker;
mod disk_usage;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Style};
use ratatui::widgets::{Block};

use crate::app::{App, RightPanel};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let pal = app.palette;

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(pal.bg)),
        area,
    );

    // Update viewport height for scroll calculations
    let telemetry_height = if app.show_telemetry { telemetry_panel_height(app) } else { 0 };
    let footer_h = footer::required_height(app, area.width);
    app.pane_mut().viewport_height = area.height.saturating_sub(4 + footer_h + telemetry_height as u16) as usize;

    if app.dual_pane && area.width >= 100 {
        render_dual(f, app, area);
    } else {
        render_single(f, app, area);
    }
}

/// Calculate telemetry panel height based on number of disks.
fn telemetry_panel_height(app: &App) -> usize {
    let disk_rows = app.sysmon.as_ref()
        .map(|s| s.disk_info.len())
        .unwrap_or(0)
        .max(1);
    // border(1) + disk rows + blank(1) + vitals(1) = disk_rows + 3
    // but also need at least 5 for network side (TX, RX, blank, LINK, pad)
    let left_height = disk_rows + 3;
    let right_height = 4;
    left_height.max(right_height) + 1 // +1 for top border
}

fn render_single(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let show_status = statusbar::should_show(app);
    let status_height: u16 = if show_status { 1 } else { 0 };
    let telem_height: u16 = if app.show_telemetry {
        telemetry_panel_height(app) as u16
    } else {
        0
    };
    let footer_height = footer::required_height(app, area.width);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // header + border
            Constraint::Length(2),             // breadcrumb + border
            Constraint::Min(3),               // body
            Constraint::Length(telem_height),  // telemetry (conditional)
            Constraint::Length(status_height), // status bar (conditional)
            Constraint::Length(footer_height), // footer (wraps)
        ])
        .split(area);

    header::render(f, app, outer[0]);
    breadcrumb::render(f, app, outer[1]);

    // Editor / RecursiveSearch take over the full body area
    if app.mode == crate::app::Mode::Edit {
        // Update editor viewport dimensions
        if let Some(ed) = &mut app.editor {
            let gutter_w = format!("{}", ed.lines.len()).len().max(3) + 2;
            ed.viewport_rows = outer[2].height as usize - 1; // minus title bar
            ed.viewport_cols = (outer[2].width as usize).saturating_sub(gutter_w);
        }
        editor::render(f, app, outer[2]);
    } else if app.mode == crate::app::Mode::RecursiveSearch {
        list::render_rsearch(f, app, outer[2]);
    } else {
        // Right panel visibility: show if wide enough AND not Hidden
        let show_right = area.width >= 100 && !matches!(app.right_panel, RightPanel::Hidden);
        if show_right {
            let right_pct = app.sidebar_pct;
            let left_pct = 100u16.saturating_sub(right_pct);
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(left_pct),
                    Constraint::Percentage(right_pct),
                ])
                .split(outer[2]);
            list::render(f, app, body[0]);
            match app.right_panel {
                RightPanel::Info => sidebar::render(f, app, body[1]),
                RightPanel::Preview => preview::render(f, app, body[1]),
                RightPanel::DiskUsage => disk_usage::render(f, app, body[1]),
                RightPanel::Hidden => unreachable!(),
            }
        } else {
            list::render(f, app, outer[2]);
        }
    }

    if app.show_telemetry {
        telemetry::render(f, app, outer[3]);
    }

    if show_status {
        statusbar::render(f, app, outer[4]);
    }

    footer::render(f, app, outer[5]);

    if app.show_theme_picker {
        theme_picker::render(f, app, area);
    }

    if app.mode == crate::app::Mode::BulkRename {
        render_bulk_rename(f, app, area);
    }

    // Idle overlay (#17)
    if app.idle_active {
        render_idle_overlay(f, app, area);
    }

    // CRT glitch effects for cyan palette (#15)
    if app.glitch_enabled {
        match app.palette.variant {
            crate::throbber::PaletteVariant::Cyan => render_cyan_glitch(f, app, area),
            crate::throbber::PaletteVariant::Green => render_green_effects(f, app, area),
            crate::throbber::PaletteVariant::Amber => render_amber_effects(f, app, area),
        }
    }
}

fn render_dual(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let pal = app.palette;
    let show_status = statusbar::should_show(app);
    let status_height: u16 = if show_status { 1 } else { 0 };
    let telem_height: u16 = if app.show_telemetry {
        telemetry_panel_height(app) as u16
    } else {
        0
    };
    let footer_height = footer::required_height(app, area.width);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // header
            Constraint::Length(2),             // breadcrumbs (both panes)
            Constraint::Min(3),               // body (both panes)
            Constraint::Length(telem_height),  // telemetry
            Constraint::Length(status_height), // status bar
            Constraint::Length(footer_height), // footer (wraps)
        ])
        .split(area);

    header::render(f, app, outer[0]);

    // Split breadcrumb row into two halves
    let breadcrumb_halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(outer[1]);

    breadcrumb::render_pane(f, app, 0, breadcrumb_halves[0], app.active_pane == 0);
    breadcrumb::render_pane(f, app, 1, breadcrumb_halves[1], app.active_pane == 1);

    // Editor takes over full body in dual-pane mode too
    if app.mode == crate::app::Mode::Edit {
        if let Some(ed) = &mut app.editor {
            let gutter_w = format!("{}", ed.lines.len()).len().max(3) + 2;
            ed.viewport_rows = outer[2].height as usize - 1;
            ed.viewport_cols = (outer[2].width as usize).saturating_sub(gutter_w);
        }
        editor::render(f, app, outer[2]);
    } else {

    // Split body into two halves
    let body_halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(outer[2]);

    // Update viewport height for both panes
    let vh = outer[2].height as usize;
    app.panes[0].viewport_height = vh;
    app.panes[1].viewport_height = vh;

    list::render_pane(f, app, 0, body_halves[0]);

    // Render a pulsing border between panes (#18)
    let pulsed = app.pulsed_border();
    let border_area = ratatui::layout::Rect::new(
        body_halves[1].x, body_halves[1].y, 1, body_halves[1].height,
    );
    for row in 0..border_area.height {
        f.render_widget(
            ratatui::widgets::Paragraph::new(ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    "\u{2502}",
                    Style::default().fg(pulsed).bg(pal.bg),
                ),
            )),
            ratatui::layout::Rect::new(border_area.x, border_area.y + row, 1, 1),
        );
    }

    list::render_pane(f, app, 1, body_halves[1]);

    } // end else (non-Edit mode body rendering)

    if app.show_telemetry {
        telemetry::render(f, app, outer[3]);
    }

    if show_status {
        statusbar::render(f, app, outer[4]);
    }

    footer::render(f, app, outer[5]);

    if app.show_theme_picker {
        theme_picker::render(f, app, area);
    }

    if app.mode == crate::app::Mode::BulkRename {
        render_bulk_rename(f, app, area);
    }

    // Idle overlay (#17)
    if app.idle_active {
        render_idle_overlay(f, app, area);
    }

    // CRT glitch effects for cyan palette (#15)
    if app.glitch_enabled {
        match app.palette.variant {
            crate::throbber::PaletteVariant::Cyan => render_cyan_glitch(f, app, area),
            crate::throbber::PaletteVariant::Green => render_green_effects(f, app, area),
            crate::throbber::PaletteVariant::Amber => render_amber_effects(f, app, area),
        }
    }
}

fn render_bulk_rename(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Borders, BorderType, Clear, Paragraph};

    let pal = app.palette;
    let popup_w = 60u16.min(area.width.saturating_sub(4));
    let popup_h = (app.bulk_paths.len() as u16 + 8).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        " BULK RENAME",
        Style::default().fg(pal.text_hot).bg(pal.bg),
    )));
    lines.push(Line::from(Span::raw("")));

    // Find field
    let sym = &app.symbols;
    let find_label = if app.bulk_field == 0 { format!("{} Find:    ", sym.cursor) } else { "  Find:    ".to_string() };
    lines.push(Line::from(vec![
        Span::styled(find_label, Style::default().fg(pal.text_mid).bg(pal.bg)),
        Span::styled(
            format!("{}{}", &app.bulk_find, sym.text_cursor),
            Style::default().fg(if app.bulk_field == 0 { pal.text_hot } else { pal.text_dim }).bg(pal.bg),
        ),
    ]));

    // Replace field
    let repl_label = if app.bulk_field == 1 { format!("{} Replace: ", sym.cursor) } else { "  Replace: ".to_string() };
    lines.push(Line::from(vec![
        Span::styled(repl_label, Style::default().fg(pal.text_mid).bg(pal.bg)),
        Span::styled(
            format!("{}{}", &app.bulk_replace, sym.text_cursor),
            Style::default().fg(if app.bulk_field == 1 { pal.text_hot } else { pal.text_dim }).bg(pal.bg),
        ),
    ]));

    lines.push(Line::from(Span::raw("")));

    // Preview renames
    let inner_w = popup_w.saturating_sub(4) as usize;
    for path in &app.bulk_paths {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let new_name = if !app.bulk_find.is_empty() {
            name.replace(&app.bulk_find, &app.bulk_replace)
        } else {
            name.clone()
        };
        let changed = name != new_name;
        let arrow = if changed { format!(" {} ", sym.arrow_right) } else { " = ".to_string() };
        let display = format!(" {}{}{}", name, arrow, new_name);
        let truncated = if display.chars().count() > inner_w {
            let t: String = display.chars().take(inner_w.saturating_sub(1)).collect();
            format!("{}\u{2026}", t)
        } else {
            display
        };
        lines.push(Line::from(Span::styled(
            truncated,
            Style::default()
                .fg(if changed { pal.text_hot } else { pal.text_dim })
                .bg(pal.bg),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.border_hot))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup);
}

/// Render the idle screen overlay with WY logo burn-in (#17).
fn render_idle_overlay(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Clear, Paragraph};

    let pal = app.palette;
    let logo = crate::logo::logo_for(pal.variant);

    // Center the logo
    let logo_h = logo.len() as u16;
    let logo_w = logo.first().map(|l| l.len()).unwrap_or(0) as u16;
    if area.width < logo_w + 4 || area.height < logo_h + 6 {
        return;
    }

    let x = area.x + (area.width.saturating_sub(logo_w)) / 2;
    let y = area.y + (area.height.saturating_sub(logo_h + 4)) / 2;

    // Dim the entire background
    let dim_area = area;
    f.render_widget(Clear, dim_area);
    f.render_widget(
        Block::default().style(Style::default().bg(pal.bg)),
        dim_area,
    );

    // Render logo with phosphor burn-in effect
    let burn_color = match (pal.border_dim, pal.border_mid) {
        (ratatui::style::Color::Rgb(dr, dg, db), ratatui::style::Color::Rgb(mr, mg, mb)) => {
            // Subtle mid-brightness color
            ratatui::style::Color::Rgb(
                (dr as u16 + mr as u16 / 2).min(255) as u8,
                (dg as u16 + mg as u16 / 2).min(255) as u8,
                (db as u16 + mb as u16 / 2).min(255) as u8,
            )
        }
        _ => pal.border_mid,
    };
    let dot_color = pal.border_dim;

    for (row, line_str) in logo.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line_str.chars() {
            let (c, color) = if ch == '@' {
                ('\u{2588}', burn_color) // solid block
            } else {
                ('\u{00b7}', dot_color) // dot for gaps
            };
            spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(color).bg(pal.bg),
            ));
        }
        let logo_rect = ratatui::layout::Rect::new(x, y + row as u16, logo_w, 1);
        f.render_widget(Paragraph::new(Line::from(spans)), logo_rect);
    }

    // AWAITING INPUT message below logo
    let msg = "AWAITING INPUT...";
    let throbber = app.heartbeat.frame();
    let msg_line = Line::from(vec![
        Span::styled(
            format!("{} {}", throbber, msg),
            Style::default().fg(pal.text_dim).bg(pal.bg),
        ),
    ]);
    let msg_w = (msg.len() + 3) as u16;
    let msg_x = area.x + (area.width.saturating_sub(msg_w)) / 2;
    let msg_y = y + logo_h + 2;
    if msg_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(msg_line),
            ratatui::layout::Rect::new(msg_x, msg_y, msg_w, 1),
        );
    }
}

/// Render CRT signal degradation effects for cyan palette (#15).
fn render_cyan_glitch(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let pal = app.palette;
    let tick = app.glitch_tick;

    const JUNK_CHARS: &[char] = &[
        '\u{2591}', '\u{2592}', '\u{2593}', '\u{2580}', '\u{2584}',
        '\u{2840}', '\u{28ff}', '\u{254c}', '\u{2502}', '\u{2500}',
    ];

    // Character corruption: 1-2 random chars on screen edges, every ~15 ticks
    if tick % 17 == 3 {
        let r1 = app.glitch_rand(0);
        let r2 = app.glitch_rand(1);
        let row = (r1 % area.height as u32) as u16;
        let side = if r2 % 2 == 0 { area.x } else { area.x + area.width - 1 };
        let ch = JUNK_CHARS[(r1 as usize / 7) % JUNK_CHARS.len()];

        if row < area.height && side < area.x + area.width {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    ch.to_string(),
                    Style::default().fg(pal.text_dim).bg(pal.bg),
                ))),
                ratatui::layout::Rect::new(side, area.y + row, 1, 1),
            );
        }
    }

    // Horizontal glitch line: rare, every ~50 ticks, single row shifts
    if tick % 53 == 7 {
        let r = app.glitch_rand(2);
        let row = (r % area.height as u32) as u16;
        let glitch_str: String = (0..area.width)
            .map(|i| {
                let gi = app.glitch_rand(100 + i as u32);
                JUNK_CHARS[(gi as usize) % JUNK_CHARS.len()]
            })
            .collect();

        if row < area.height {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    glitch_str,
                    Style::default().fg(pal.border_dim).bg(pal.bg),
                ))),
                ratatui::layout::Rect::new(area.x, area.y + row, area.width, 1),
            );
        }
    }
}

/// Green phosphor CRT effects: scan line shimmer + phosphor persistence.
fn render_green_effects(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::Color;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let pal = app.palette;
    let tick = app.glitch_tick;

    // Body content starts after header(2) + breadcrumb(2)
    let body_top = area.y + 4;

    // Scan line shimmer: subtle edge glow sweeping downward
    // Moves 1 row per tick for smooth motion; 3-row band with graduated fade
    let center_row = (tick % area.height as u32) as u16;
    for offset in 0u16..3 {
        let row = (center_row + offset) % area.height;
        if row >= area.height {
            continue;
        }
        // Graduated intensity: center=0.35, edges=0.15
        let intensity = match offset {
            0 => 0.15_f32,
            1 => 0.35,
            2 => 0.15,
            _ => 0.0,
        };
        let glow_color = match pal.border_dim {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f32 * intensity) as u8,
                (g as f32 * intensity) as u8,
                (b as f32 * intensity) as u8,
            ),
            c => c,
        };
        // Only render a subtle pip on left and right edges, not replacing content
        let y = area.y + row;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "\u{2595}", // ▕ right one eighth block
                Style::default().fg(glow_color),
            ))),
            ratatui::layout::Rect::new(area.x, y, 1, 1),
        );
        if area.width > 1 {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "\u{258f}", // ▏ left one eighth block
                    Style::default().fg(glow_color),
                ))),
                ratatui::layout::Rect::new(area.x + area.width - 1, y, 1, 1),
            );
        }
    }

    // Phosphor persistence: faint ghost marks at previous cursor positions
    let pane = app.pane();
    for &(cursor_idx, fade) in &app.phosphor_trail {
        if cursor_idx < pane.scroll_offset || cursor_idx >= pane.scroll_offset + pane.viewport_height {
            continue;
        }
        let visual_row = (cursor_idx - pane.scroll_offset) as u16;
        let screen_row = body_top + visual_row;
        if screen_row >= area.y + area.height {
            continue;
        }

        // Fade: 6→bright, 1→almost invisible
        let intensity = fade as f32 / 6.0;
        let ghost_color = match pal.text_dim {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f32 * intensity * 0.6) as u8,
                (g as f32 * intensity * 0.6) as u8,
                (b as f32 * intensity * 0.6) as u8,
            ),
            c => c,
        };

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "\u{25b8}", // ▸ small right triangle
                Style::default().fg(ghost_color),
            ))),
            ratatui::layout::Rect::new(area.x, screen_row, 1, 1),
        );
    }
}

/// Amber CRT effects: rare thermal flicker.
fn render_amber_effects(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let pal = app.palette;
    let tick = app.glitch_tick;

    // Thermal flicker: very rare (every ~120 ticks), a single row
    // briefly shows a faint voltage-spike artifact at the edge
    if tick % 127 < 2 {
        let r = app.glitch_rand(3);
        let row = (r % area.height as u32) as u16;
        if row < area.height {
            // Subtle bright pip on the right edge
            let x = area.x + area.width.saturating_sub(1);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "\u{2502}", // │
                    Style::default().fg(pal.text_hot),
                ))),
                ratatui::layout::Rect::new(x, area.y + row, 1, 1),
            );
        }
    }

    // Cursor bloom: when cursor just moved, briefly brighten the right edge
    // of the new row with a warm glow that fades over 3 frames
    // Body content starts after header(2) + breadcrumb(2)
    let body_top = area.y + 4;
    if !app.phosphor_trail.is_empty() {
        let pane = app.pane();
        let cursor = pane.cursor;
        if cursor >= pane.scroll_offset && cursor < pane.scroll_offset + pane.viewport_height {
            let visual_row = (cursor - pane.scroll_offset) as u16;
            let screen_row = body_top + visual_row;
            let most_recent_fade = app.phosphor_trail.last().map(|t| t.1).unwrap_or(0);
            if most_recent_fade >= 4 && screen_row < area.y + area.height {
                let x = area.x + area.width.saturating_sub(2);
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "\u{2590}", // ▐ right half block
                        Style::default().fg(pal.border_mid),
                    ))),
                    ratatui::layout::Rect::new(x, screen_row, 1, 1),
                );
            }
        }
    }
}
