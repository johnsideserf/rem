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
        Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.text_dim)),
        Span::styled(
            if app.archive.is_some() { "ARCHIVE" } else { "FILE SYSTEM" },
            Style::default().fg(if app.archive.is_some() { pal.warn } else { pal.text_mid }),
        ),
        Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.text_dim)),
        Span::styled(format!("ITEMS:{}", item_count), Style::default().fg(pal.text_hot)),
    ];

    // Git branch
    if let Some(git) = &app.git_info {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        let dirty_sigil = if git.dirty { app.symbols.git_dirty } else { "" };
        let branch_display = if git.branch.chars().count() > 20 {
            let t: String = git.branch.chars().take(19).collect();
            format!("{}\u{2026}", t)
        } else {
            git.branch.clone()
        };
        spans.push(Span::styled(
            format!("BR:{}{}", branch_display, dirty_sigil),
            Style::default().fg(pal.text_mid),
        ));
    }

    if mark_count > 0 {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled(
            format!("MARKED:{}", mark_count),
            Style::default().fg(pal.text_hot),
        ));
    }

    // Sort mode (only show if not default)
    if app.sort_mode != crate::app::SortMode::NameAsc {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled(
            format!("SORT:{}", app.sort_mode.label()),
            Style::default().fg(pal.text_hot),
        ));
    }

    if app.show_telemetry {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled("TELEM:ACTIVE", Style::default().fg(pal.text_hot)));
    }

    // I/O activity throbber (#16)
    if app.io_flash_tick > 0 {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled(
            format!("{} I/O", app.io_throbber.frame()),
            Style::default().fg(pal.text_hot),
        ));
    }

    // Hash progress (#20)
    if let Some(hop) = &app.hash_op {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        let pct = (hop.progress * 100.0) as u64;
        spans.push(Span::styled(
            format!("{} HASH:{}%", hop.throbber.frame(), pct),
            Style::default().fg(pal.text_hot),
        ));
    }

    // Disk scan progress (#21)
    if let Some(ds) = &app.disk_scan {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled(
            format!("{} SCAN:{}", ds.throbber.frame(), ds.nodes),
            Style::default().fg(pal.text_hot),
        ));
    }

    // Archive indicator (#19)
    if app.archive.is_some() {
        spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
        spans.push(Span::styled("ARCHIVE", Style::default().fg(pal.warn)));
    }

    spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
    spans.push(Span::styled(app.heartbeat.frame(), Style::default().fg(pal.text_hot)));
    spans.push(Span::styled("  ", Style::default()));

    // Signal indicator: cyan shows degraded signal (#15)
    let signal_label = if app.glitch_enabled && matches!(pal.variant, crate::throbber::PaletteVariant::Cyan) {
        let tick = app.glitch_tick;
        if tick % 37 < 3 {
            "SIGNAL:\u{2591}\u{2591}\u{2591}"
        } else if tick % 23 < 2 {
            "SIGNAL:WEAK"
        } else {
            "SIGNAL:NOMINAL"
        }
    } else {
        "SYS:NOMINAL"
    };
    spans.push(Span::styled(signal_label, Style::default().fg(pal.text_hot)));

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
            app.symbols.mark,
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
