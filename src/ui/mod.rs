pub mod boot;
mod header;
mod breadcrumb;
mod diffview;
mod editor;
mod list;
mod sidebar;
mod preview;
mod statusbar;
mod telemetry;
mod footer;
pub mod theme_picker;
mod comms_selector;
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
    left_height.max(right_height) + 1 + 4 // +1 for top border, +4 for oscilloscope
}

fn render_single(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let show_status = statusbar::should_show(app);
    let status_height: u16 = if show_status { 1 } else { 0 };
    let telem_height: u16 = if app.show_telemetry {
        telemetry_panel_height(app) as u16
    } else {
        0
    };
    let comms_height: u16 = if app.comms.current.is_some() { 1 } else { 0 };
    let footer_height = footer::required_height(app, area.width);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // [0] header + border
            Constraint::Length(2),             // [1] breadcrumb + border
            Constraint::Min(3),               // [2] body
            Constraint::Length(comms_height),  // [3] comms intercept (conditional)
            Constraint::Length(telem_height),  // [4] telemetry (conditional)
            Constraint::Length(status_height), // [5] status bar (conditional)
            Constraint::Length(footer_height), // [6] footer (wraps)
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
    } else if app.mode == crate::app::Mode::FileDiff {
        diffview::render(f, app, outer[2]);
    } else if app.mode == crate::app::Mode::RecursiveSearch {
        list::render_rsearch(f, app, outer[2]);
    } else {
        // Store list area for mouse hit-testing (#38)
        app.layout_areas.list_area = Some((outer[2].x, outer[2].y, outer[2].width, outer[2].height));

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

    statusbar::render_comms_overlay(f, app, outer[3]);

    if app.show_telemetry {
        telemetry::render(f, app, outer[4]);
    }

    if show_status {
        statusbar::render(f, app, outer[5]);
    }

    footer::render(f, app, outer[6]);

    if app.show_theme_picker {
        theme_picker::render(f, app, area);
    }

    if app.comms.show_selector {
        comms_selector::render(f, app, area);
    }

    if app.mode == crate::app::Mode::BulkRename {
        render_bulk_rename(f, app, area);
    }

    // Operations log overlay (#43)
    if app.mode == crate::app::Mode::OpsLog {
        render_ops_log(f, app, area);
    }

    // Idle overlay (#17)
    if app.idle_active {
        render_idle_overlay(f, app, area);
    }

    // CRT effects per palette
    if app.glitch_enabled {
        match app.palette.variant {
            crate::throbber::PaletteVariant::Green => render_green_effects(f, app, area),
            crate::throbber::PaletteVariant::Amber => render_colony_glitch(f, app, area),
            crate::throbber::PaletteVariant::Cyan => render_corporate_effects(f, app, area),
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
    let comms_height: u16 = if app.comms.current.is_some() { 1 } else { 0 };
    let footer_height = footer::required_height(app, area.width);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // [0] header
            Constraint::Length(2),             // [1] breadcrumbs (both panes)
            Constraint::Min(3),               // [2] body (both panes)
            Constraint::Length(comms_height),  // [3] comms intercept (conditional)
            Constraint::Length(telem_height),  // [4] telemetry
            Constraint::Length(status_height), // [5] status bar
            Constraint::Length(footer_height), // [6] footer (wraps)
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

    let active = app.active_pane;
    breadcrumb::render_pane(f, app, 0, breadcrumb_halves[0], active == 0);
    breadcrumb::render_pane(f, app, 1, breadcrumb_halves[1], active == 1);

    // Editor / FileDiff takes over full body in dual-pane mode too
    if app.mode == crate::app::Mode::Edit {
        if let Some(ed) = &mut app.editor {
            let gutter_w = format!("{}", ed.lines.len()).len().max(3) + 2;
            ed.viewport_rows = outer[2].height as usize - 1;
            ed.viewport_cols = (outer[2].width as usize).saturating_sub(gutter_w);
        }
        editor::render(f, app, outer[2]);
    } else if app.mode == crate::app::Mode::FileDiff {
        diffview::render(f, app, outer[2]);
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

    statusbar::render_comms_overlay(f, app, outer[3]);

    if app.show_telemetry {
        telemetry::render(f, app, outer[4]);
    }

    if show_status {
        statusbar::render(f, app, outer[5]);
    }

    footer::render(f, app, outer[6]);

    if app.show_theme_picker {
        theme_picker::render(f, app, area);
    }

    if app.comms.show_selector {
        comms_selector::render(f, app, area);
    }

    if app.mode == crate::app::Mode::BulkRename {
        render_bulk_rename(f, app, area);
    }

    // Operations log overlay (#43)
    if app.mode == crate::app::Mode::OpsLog {
        render_ops_log(f, app, area);
    }

    // Idle overlay (#17)
    if app.idle_active {
        render_idle_overlay(f, app, area);
    }

    // CRT effects per palette
    if app.glitch_enabled {
        match app.palette.variant {
            crate::throbber::PaletteVariant::Green => render_green_effects(f, app, area),
            crate::throbber::PaletteVariant::Amber => render_colony_glitch(f, app, area),
            crate::throbber::PaletteVariant::Cyan => render_corporate_effects(f, app, area),
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

/// Render operations log popup (#43).
fn render_ops_log(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Borders, BorderType, Clear, Paragraph};

    let pal = app.palette;
    let popup_w = 70u16.min(area.width.saturating_sub(4));
    let popup_h = 20u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        " O P E R A T I O N S   L O G",
        Style::default().fg(pal.text_hot).bg(pal.bg),
    )));
    lines.push(Line::from(Span::raw("")));

    let inner_h = popup_h.saturating_sub(4) as usize;
    if app.ops_log.entries.is_empty() {
        lines.push(Line::from(Span::styled(
            " NO OPERATIONS RECORDED",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
    } else {
        let start = app.ops_log_scroll;
        let end = (start + inner_h).min(app.ops_log.entries.len());
        for entry in &app.ops_log.entries[start..end] {
            let inner_w = popup_w.saturating_sub(4) as usize;
            let line_str = format!(
                " [{}] {:>6}  {}",
                entry.timestamp, entry.action, entry.path
            );
            let truncated = if line_str.chars().count() > inner_w {
                let t: String = line_str.chars().take(inner_w.saturating_sub(1)).collect();
                format!("{}\u{2026}", t)
            } else {
                line_str
            };
            lines.push(Line::from(Span::styled(
                truncated,
                Style::default().fg(pal.text_mid).bg(pal.bg),
            )));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
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

    // Distress signal override (#75)
    if app.distress_active {
        render_distress(f, app, area);
        return;
    }

    // Center the logo
    let logo_h = logo.len() as u16;
    let logo_w = logo.first().map(|l| l.len()).unwrap_or(0) as u16;
    if area.width < logo_w + 4 || area.height < logo_h + 22 {
        return;
    }

    let x = area.x + (area.width.saturating_sub(logo_w)) / 2;
    let y = area.y + (area.height.saturating_sub(logo_h + 20)) / 2;

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

    // Title above logo
    let title = "W E Y L A N D - Y U T A N I   C O R P O R A T I O N";
    let title_w = title.len() as u16;
    let title_x = area.x + (area.width.saturating_sub(title_w)) / 2;
    let title_y = y;
    if title_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                title,
                Style::default().fg(pal.text_dim).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(title_x, title_y, title_w, 1),
        );
    }

    let logo_y = y + 2;
    for (row, line_str) in logo.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line_str.chars() {
            let color = if ch == '@' { burn_color } else { dot_color };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(color).bg(pal.bg),
            ));
        }
        let logo_rect = ratatui::layout::Rect::new(x, logo_y + row as u16, logo_w, 1);
        f.render_widget(Paragraph::new(Line::from(spans)), logo_rect);
    }

    // Tagline below logo
    let tagline = "B U I L D I N G   B E T T E R   W O R L D S";
    let tag_w = tagline.len() as u16;
    let tag_x = area.x + (area.width.saturating_sub(tag_w)) / 2;
    let tag_y = logo_y + logo_h + 1;
    if tag_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                tagline,
                Style::default().fg(pal.text_dim).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(tag_x, tag_y, tag_w, 1),
        );
    }

    // Animated braille art per palette
    let remaining = (area.y + area.height).saturating_sub(tag_y + 4); // space left below tagline
    let art_h: u16 = remaining.min(18).max(8); // use up to 18 rows, minimum 8
    let art_w: u16 = (area.width).min(80).max(40);
    let art_y = tag_y + 2;
    let art_x = area.x + (area.width.saturating_sub(art_w)) / 2;
    if art_y + art_h < area.y + area.height {
        let art_rect = ratatui::layout::Rect::new(art_x, art_y, art_w, art_h);
        match pal.variant {
            crate::throbber::PaletteVariant::Green => render_idle_orbits(f, app, art_rect),
            crate::throbber::PaletteVariant::Amber => render_idle_tracker(f, app, art_rect),
            crate::throbber::PaletteVariant::Cyan => render_idle_helix(f, app, art_rect),
        }
    }

    // AWAITING INPUT message with per-palette idle throbber
    let msg = "AWAITING INPUT...";
    let throbber = app.idle_throbber.frame();
    let msg_line = Line::from(vec![
        Span::styled(
            format!("{} {}", throbber, msg),
            Style::default().fg(pal.text_dim).bg(pal.bg),
        ),
    ]);
    let msg_w = (msg.len() + 3) as u16;
    let msg_x = area.x + (area.width.saturating_sub(msg_w)) / 2;
    let msg_y = art_y + art_h + 1;
    if msg_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(msg_line),
            ratatui::layout::Rect::new(msg_x, msg_y, msg_w, 1),
        );
    }
}

/// Distress signal screensaver — pulsing SOS in braille (#75).
fn render_distress(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Clear, Paragraph};

    let pal = app.palette;
    let tick = app.glitch_tick;

    // Dim background
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().style(Style::default().bg(pal.bg)),
        area,
    );

    // SOS in large block letters (7 rows tall)
    let sos_art = [
        " ███  ████  ███ ",
        "█     █  █ █    ",
        "█     █  █ █    ",
        " ███  █  █  ███ ",
        "    █ █  █     █",
        "    █ █  █     █",
        " ███  ████  ███ ",
    ];

    // Pulse between text_hot and text_dim
    let pulse = if (tick / 5) % 2 == 0 { pal.text_hot } else { pal.text_dim };

    let sos_w = sos_art[0].chars().count() as u16;
    let sos_h = sos_art.len() as u16;
    let cx = area.x + (area.width.saturating_sub(sos_w)) / 2;
    let cy = area.y + (area.height.saturating_sub(sos_h + 10)) / 2;

    for (row, line_str) in sos_art.iter().enumerate() {
        let y = cy + row as u16;
        if y >= area.y + area.height { break; }
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                *line_str,
                Style::default().fg(pulse).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(cx, y, sos_w, 1),
        );
    }

    // Coordinates
    let coords = "26.18N  39.47W  SECTOR 7G";
    let coords_w = coords.len() as u16;
    let coords_x = area.x + (area.width.saturating_sub(coords_w)) / 2;
    let coords_y = cy + sos_h + 2;
    if coords_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                coords,
                Style::default().fg(pal.text_mid).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(coords_x, coords_y, coords_w, 1),
        );
    }

    // DISTRESS BEACON ACTIVE label with throbber
    let throbber = app.idle_throbber.frame();
    let beacon_msg = format!("{} DISTRESS BEACON ACTIVE", throbber);
    let beacon_w = beacon_msg.chars().count() as u16;
    let beacon_x = area.x + (area.width.saturating_sub(beacon_w)) / 2;
    let beacon_y = coords_y + 2;
    if beacon_y < area.y + area.height {
        let beacon_color = if (tick / 8) % 2 == 0 { pal.warn } else { pal.text_dim };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                beacon_msg,
                Style::default().fg(beacon_color).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(beacon_x, beacon_y, beacon_w, 1),
        );
    }

    // Timestamp
    let time_y = beacon_y + 2;
    if time_y < area.y + area.height {
        let elapsed = std::time::Instant::now().duration_since(app.last_input).as_secs();
        let mins = elapsed / 60;
        let secs = elapsed % 60;
        let time_str = format!("SIGNAL DURATION: {:02}:{:02}", mins, secs);
        let time_w = time_str.len() as u16;
        let time_x = area.x + (area.width.saturating_sub(time_w)) / 2;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                time_str,
                Style::default().fg(pal.text_dim).bg(pal.bg),
            ))),
            ratatui::layout::Rect::new(time_x, time_y, time_w, 1),
        );
    }
}

/// Ship terminal idle: rotating 3D planet with elliptical ship orbit.
fn render_idle_orbits(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::widgets::canvas::{Canvas, Points};
    use std::f64::consts::PI;

    let pal = app.palette;
    let tick = app.glitch_tick;
    let phase = tick as f64 * 0.02;

    // Canvas coordinate space — use the braille dot grid directly.
    // Braille: 2 dots per char horizontally, 4 dots per char vertically.
    let w = area.width as f64 * 2.0;
    let h = area.height as f64 * 4.0;
    let cx = w / 2.0;
    let cy = h / 2.0;

    // Aspect ratio correction: terminal chars are ~2x tall as wide.
    // Braille partially compensates (2w x 4h per cell), but the cell itself
    // is still taller than wide.  Typical terminal font aspect ≈ 1:2 (w:h),
    // so one braille dot-y is physically ~(1/4 cell_h) and one braille dot-x
    // is ~(1/2 cell_w).  Net physical ratio of dot-x : dot-y ≈ 1 : 1 when
    // cell_h = 2 * cell_w.  But in practice cells are closer to 1:1.8, so
    // y is slightly compressed.  We correct by stretching the x-radius
    // relative to y-radius.  A factor of ~1.0 to 1.2 works well.
    let aspect = 1.1_f64; // x stretch factor — >1 widens, making circle rounder

    let hot_color = pal.text_hot;
    let mid_color = pal.text_mid;
    let dim_color = pal.border_dim;
    let ring_color = pal.border_mid;
    let bg = pal.bg;

    // Planet radius — sized relative to available canvas height so it fills
    // a good chunk of the area.  Use ~38% of half-height.
    let ry = h * 0.38;           // vertical radius in canvas dots
    let rx = ry * aspect;        // horizontal radius, corrected

    // Orbit semi-axes — ellipse tilted toward the viewer
    let orbit_ax = rx * 2.2;    // horizontal reach
    let orbit_ay = ry * 0.65;   // vertical reach (foreshortened)

    let ship_speed = 0.4;
    let rot_speed = 0.6;

    let canvas = Canvas::default()
        .block(Block::default().style(Style::default().bg(bg)))
        .background_color(bg)
        .x_bounds([0.0, w])
        .y_bounds([0.0, h])
        .marker(ratatui::symbols::Marker::Braille)
        .paint(move |ctx| {
            // --- Orbit path split into front (visible) and back (behind planet) ---
            let orbit_steps = 160;
            let mut orbit_back: Vec<(f64, f64)> = Vec::new();
            let mut orbit_front: Vec<(f64, f64)> = Vec::new();
            for i in 0..orbit_steps {
                let a = 2.0 * PI * i as f64 / orbit_steps as f64;
                let ox = cx + orbit_ax * a.cos();
                let oy = cy + orbit_ay * a.sin();
                // Is this point occluded by the planet ellipse?
                let dx = ox - cx;
                let dy = oy - cy;
                let inside = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) < 1.0;
                if a.sin() < 0.0 && !inside {
                    orbit_back.push((ox, oy));
                } else if !inside {
                    orbit_front.push((ox, oy));
                }
            }
            ctx.draw(&Points { coords: &orbit_back, color: dim_color });

            // --- Planet sphere with 3D shading ---
            let rot_offset = phase * rot_speed;

            let mut lit_pts: Vec<(f64, f64)> = Vec::new();
            let mut mid_pts: Vec<(f64, f64)> = Vec::new();
            let mut dark_pts: Vec<(f64, f64)> = Vec::new();

            // Sample the sphere densely with lat/lon
            let steps = 80;
            for lat_i in 0..steps {
                let lat = PI * (lat_i as f64 / steps as f64) - PI / 2.0;
                let cos_lat = lat.cos();
                let sin_lat = lat.sin();
                let lon_steps = ((steps as f64 * cos_lat).abs().max(1.0)) as usize;
                for lon_i in 0..lon_steps {
                    let lon = 2.0 * PI * lon_i as f64 / lon_steps as f64 + rot_offset;

                    let sx = cos_lat * lon.cos();
                    let sy = sin_lat;
                    let sz = cos_lat * lon.sin();

                    // Cull back-facing hemisphere
                    if sz > 0.0 {
                        continue;
                    }

                    // Project to 2D with aspect correction
                    let px = cx + sx * rx;
                    let py = cy + sy * ry;

                    // Shading: light from upper-right
                    let light_x = 0.6_f64;
                    let light_y = 0.4_f64;
                    let light_z = -0.7_f64;
                    let light_len = (light_x * light_x + light_y * light_y + light_z * light_z).sqrt();
                    let dot = (sx * light_x + sy * light_y + sz * light_z) / light_len;

                    // Surface bands + storm feature
                    let band = ((lat * 5.0 + (lon + rot_offset) * 0.3).sin() * 0.15).abs();
                    let storm_lat = 0.3_f64;
                    let storm_lon = 1.5_f64 + rot_offset;
                    let storm_dist = ((lat - storm_lat).powi(2) + (lon - storm_lon).powi(2)).sqrt();
                    let storm = if storm_dist < 0.4 { 0.12 } else { 0.0 };
                    let intensity = dot + band + storm;

                    if intensity > 0.25 {
                        lit_pts.push((px, py));
                    } else if intensity > -0.1 {
                        mid_pts.push((px, py));
                    } else {
                        dark_pts.push((px, py));
                    }
                }
            }

            ctx.draw(&Points { coords: &dark_pts, color: dim_color });
            ctx.draw(&Points { coords: &mid_pts, color: mid_color });
            ctx.draw(&Points { coords: &lit_pts, color: hot_color });

            // --- Atmosphere glow on the lit edge ---
            let mut atmo_pts: Vec<(f64, f64)> = Vec::new();
            let atmo_steps = 100;
            for i in 0..atmo_steps {
                let a = 2.0 * PI * i as f64 / atmo_steps as f64;
                let ex = a.cos();
                let ey = a.sin();
                let glow = ex * 0.6 + ey * 0.4;
                if glow > 0.15 {
                    atmo_pts.push((cx + ex * (rx + 1.5), cy + ey * (ry + 1.5)));
                }
            }
            ctx.draw(&Points { coords: &atmo_pts, color: mid_color });

            // --- Front orbit path ---
            ctx.draw(&Points { coords: &orbit_front, color: ring_color });

            // --- Ship ---
            let ship_angle = phase * ship_speed;
            let ship_x = cx + orbit_ax * ship_angle.cos();
            let ship_y = cy + orbit_ay * ship_angle.sin();
            let sdx = ship_x - cx;
            let sdy = ship_y - cy;
            let ship_behind = (sdx * sdx) / (rx * rx) + (sdy * sdy) / (ry * ry) < 1.0;
            if !ship_behind {
                // Bold cross marker
                ctx.draw(&Points {
                    coords: &[
                        (ship_x, ship_y),
                        (ship_x + 1.0, ship_y),
                        (ship_x - 1.0, ship_y),
                        (ship_x, ship_y + 1.0),
                        (ship_x, ship_y - 1.0),
                        (ship_x + 1.0, ship_y + 1.0),
                        (ship_x - 1.0, ship_y - 1.0),
                    ],
                    color: hot_color,
                });
                // Trailing wake
                let mut trail_pts: Vec<(f64, f64)> = Vec::new();
                for t in 1..10 {
                    let ta = ship_angle - t as f64 * 0.035;
                    let tx = cx + orbit_ax * ta.cos();
                    let ty = cy + orbit_ay * ta.sin();
                    let tdx = tx - cx;
                    let tdy = ty - cy;
                    let occluded = (tdx * tdx) / (rx * rx) + (tdy * tdy) / (ry * ry) < 1.0;
                    if !occluded {
                        trail_pts.push((tx, ty));
                    }
                }
                ctx.draw(&Points { coords: &trail_pts, color: mid_color });
            }
        });

    f.render_widget(canvas, area);
}

/// Colony terminal idle: motion tracker sweep with radar blips.
fn render_idle_tracker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine, Points};

    let pal = app.palette;
    let tick = app.glitch_tick;
    let sweep_angle = tick as f64 * 0.05; // sweep rotation speed

    let cx = area.width as f64;  // center in canvas coords
    let cy = 24.0;
    let max_r = 22.0;

    let hot_color = pal.text_hot;
    let mid_color = pal.text_mid;
    let dim_color = pal.border_mid;
    let bg = pal.bg;

    // Fixed blip positions (angle, radius) — simulate contacts
    let blips: &[(f64, f64)] = &[
        (0.8,  14.0),
        (2.4,  18.0),
        (3.9,  9.0),
        (5.1,  20.0),
        (1.6,  6.0),
    ];

    let canvas = Canvas::default()
        .block(Block::default().style(Style::default().bg(bg)))
        .background_color(bg)
        .x_bounds([0.0, area.width as f64 * 2.0])
        .y_bounds([0.0, 48.0])
        .marker(ratatui::symbols::Marker::Braille)
        .paint(move |ctx| {
            // Concentric range rings
            for i in 1..=3 {
                let r = max_r * i as f64 / 3.0;
                ctx.draw(&Circle {
                    x: cx, y: cy, radius: r,
                    color: dim_color,
                });
            }

            // Cross-hairs
            ctx.draw(&CanvasLine {
                x1: cx - max_r, y1: cy, x2: cx + max_r, y2: cy,
                color: dim_color,
            });
            ctx.draw(&CanvasLine {
                x1: cx, y1: cy - max_r * 0.7, x2: cx, y2: cy + max_r * 0.7,
                color: dim_color,
            });

            // Sweep line
            let sx = cx + max_r * sweep_angle.cos();
            let sy = cy + max_r * sweep_angle.sin() * 0.7;
            ctx.draw(&CanvasLine {
                x1: cx, y1: cy, x2: sx, y2: sy,
                color: mid_color,
            });

            // Sweep trail (fading arc behind the sweep line)
            for i in 1..30 {
                let trail_angle = sweep_angle - i as f64 * 0.04;
                let fade = 1.0 - (i as f64 / 30.0);
                let tr = max_r * (0.3 + 0.7 * fade);
                let tx = cx + tr * trail_angle.cos();
                let ty = cy + tr * trail_angle.sin() * 0.7;
                ctx.draw(&Points {
                    coords: &[(tx, ty)],
                    color: dim_color,
                });
            }

            // Blips — only visible when sweep line has recently passed
            for &(blip_angle, blip_r) in blips {
                let angle_diff = (sweep_angle - blip_angle) % (2.0 * std::f64::consts::PI);
                let angle_diff = if angle_diff < 0.0 { angle_diff + 2.0 * std::f64::consts::PI } else { angle_diff };
                // Blip visible for ~1.5 radians after sweep passes
                if angle_diff < 1.5 {
                    let bx = cx + blip_r * blip_angle.cos();
                    let by = cy + blip_r * blip_angle.sin() * 0.7;
                    let color = if angle_diff < 0.5 { hot_color } else { mid_color };
                    ctx.draw(&Points {
                        coords: &[(bx, by), (bx + 1.0, by), (bx, by + 1.0)],
                        color,
                    });
                }
            }
        });

    f.render_widget(canvas, area);
}

/// Corporate terminal idle: rotating double helix — DNA / bioengineering.
fn render_idle_helix(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::widgets::canvas::{Canvas, Line as CanvasLine, Points};

    let pal = app.palette;
    let tick = app.glitch_tick;
    let phase = tick as f64 * 0.06;

    let x_max = area.width as f64 * 2.0;

    let hot_color = pal.text_mid;
    let dim_color = pal.border_mid;
    let bg = pal.bg;

    let canvas = Canvas::default()
        .block(Block::default().style(Style::default().bg(bg)))
        .background_color(bg)
        .x_bounds([0.0, x_max])
        .y_bounds([-1.3, 1.3])
        .marker(ratatui::symbols::Marker::Braille)
        .paint(move |ctx| {
            let steps = (area.width as usize) * 4;
            let x_step = x_max / steps as f64;

            let mut pts_a: Vec<(f64, f64)> = Vec::with_capacity(steps);
            let mut pts_b: Vec<(f64, f64)> = Vec::with_capacity(steps);

            for i in 0..steps {
                let x = i as f64 * x_step;
                let t = x * 0.08 + phase;
                pts_a.push((x, t.sin()));
                pts_b.push((x, (t + std::f64::consts::PI).sin()));
            }

            ctx.draw(&Points { coords: &pts_a, color: hot_color });
            ctx.draw(&Points { coords: &pts_b, color: hot_color });

            // Cross-link rungs
            let rung_spacing = x_max / 10.0;
            for i in 0..10 {
                let x = rung_spacing * (i as f64 + 0.5);
                let t = x * 0.08 + phase;
                let ya = t.sin();
                let yb = (t + std::f64::consts::PI).sin();
                if (ya - yb).abs() > 0.3 {
                    ctx.draw(&CanvasLine {
                        x1: x, y1: ya, x2: x, y2: yb,
                        color: dim_color,
                    });
                }
            }
        });

    f.render_widget(canvas, area);
}

/// Colony terminal signal degradation: character corruption + glitch lines.
fn render_colony_glitch(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

/// Corporate terminal effects: rare thermal flicker, clean and precise.
fn render_corporate_effects(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
