pub mod boot;
mod header;
mod breadcrumb;
mod list;
mod sidebar;
mod preview;
mod statusbar;
mod telemetry;
mod footer;
pub mod theme_picker;

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
    app.pane_mut().viewport_height = area.height.saturating_sub(5 + telemetry_height as u16) as usize;

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

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // header + border
            Constraint::Length(2),             // breadcrumb + border
            Constraint::Min(3),               // body
            Constraint::Length(telem_height),  // telemetry (conditional)
            Constraint::Length(status_height), // status bar (conditional)
            Constraint::Length(1),             // footer
        ])
        .split(area);

    header::render(f, app, outer[0]);
    breadcrumb::render(f, app, outer[1]);

    // Recursive search takes over the full body area
    if app.mode == crate::app::Mode::RecursiveSearch {
        list::render_rsearch(f, app, outer[2]);
    } else {
        // Right panel visibility: show if wide enough AND not Hidden
        let show_right = area.width >= 100 && app.right_panel != RightPanel::Hidden;
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

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),             // header
            Constraint::Length(2),             // breadcrumbs (both panes)
            Constraint::Min(3),               // body (both panes)
            Constraint::Length(telem_height),  // telemetry
            Constraint::Length(status_height), // status bar
            Constraint::Length(1),             // footer
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

    // Render a subtle border between panes
    let border_area = ratatui::layout::Rect::new(
        body_halves[1].x, body_halves[1].y, 1, body_halves[1].height,
    );
    for row in 0..border_area.height {
        f.render_widget(
            ratatui::widgets::Paragraph::new(ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    "\u{2502}",
                    Style::default().fg(pal.border_dim).bg(pal.bg),
                ),
            )),
            ratatui::layout::Rect::new(border_area.x, border_area.y + row, 1, 1),
        );
    }

    list::render_pane(f, app, 1, body_halves[1]);

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
}
