mod header;
mod breadcrumb;
mod list;
mod sidebar;
mod footer;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Style};
use ratatui::widgets::{Block};

use crate::app::App;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let pal = app.palette;

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(pal.bg)),
        area,
    );

    // Update viewport height for scroll calculations
    // total area minus header(1) + header border(1) + breadcrumb(1) + breadcrumb border(1) + footer(1) = 5
    app.viewport_height = area.height.saturating_sub(5) as usize;

    // Vertical layout: header(1), breadcrumb(1), body(flex), footer(1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // header + border
            Constraint::Length(2),  // breadcrumb + border
            Constraint::Min(3),    // body
            Constraint::Length(1), // footer
        ])
        .split(area);

    header::render(f, app, outer[0]);
    breadcrumb::render(f, app, outer[1]);

    let show_sidebar = area.width >= 100;
    if show_sidebar {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(78),
                Constraint::Percentage(22),
            ])
            .split(outer[2]);
        list::render(f, app, body[0]);
        sidebar::render(f, app, body[1]);
    } else {
        list::render(f, app, outer[2]);
    }

    footer::render(f, app, outer[3]);
}
