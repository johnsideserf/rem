use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Mode};
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

    // Map current mode to an UPPERCASE label and color
    let (mode_label, mode_color) = match &app.mode {
        Mode::Normal => ("NORMAL", pal.text_mid),
        Mode::FuzzySearch => ("FUZZY", pal.text_mid),
        Mode::JumpKey => ("JUMP", pal.text_mid),
        Mode::Visual => ("VISUAL", pal.text_hot),
        Mode::Rename => ("RENAME", pal.text_mid),
        Mode::Create { .. } => ("CREATE", pal.text_mid),
        Mode::Confirm { .. } => ("CONFIRM", pal.text_hot),
        Mode::WaitingForG
        | Mode::WaitingForMark
        | Mode::WaitingForJumpToMark
        | Mode::WaitingForYank
        | Mode::WaitingForCut
        | Mode::WaitingForDeleteMark => ("NORMAL", pal.text_mid),
        Mode::RecursiveSearch => ("SEARCH", pal.text_mid),
        Mode::BulkRename => ("BULK-RN", pal.text_mid),
        Mode::Edit => ("EDIT", pal.text_mid),
        Mode::OpsLog => ("LOG", pal.text_mid),
        Mode::Command => ("COMMAND", pal.text_mid),
        Mode::TagInput => ("TAG", pal.text_mid),
    };

    // Left side: status info
    let mut spans = vec![
        Span::styled(" REM", Style::default().fg(pal.text_hot).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.text_dim)),
    ];

    // Tab indicators (#81)
    if app.tab_count() > 1 {
        for i in 0..app.tabs.len() {
            let is_active = i == app.active_tab;
            let label = &app.tabs[i].label;
            let tab_display = if label.chars().count() > 8 {
                let t: String = label.chars().take(7).collect();
                format!("{}\u{2026}", t)
            } else {
                label.clone()
            };
            let color = if is_active { pal.text_hot } else { pal.text_dim };
            spans.push(Span::styled(
                format!("[{}:{}]", i + 1, tab_display),
                Style::default().fg(color),
            ));
            spans.push(Span::styled(" ", Style::default()));
        }
        spans.push(Span::styled(format!("{}  ", app.symbols.separator), Style::default().fg(pal.text_dim)));
    }

    spans.push(Span::styled(format!("[{}]", mode_label), Style::default().fg(mode_color).add_modifier(Modifier::BOLD)));
    spans.push(Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.text_dim)));
    spans.push(Span::styled(
        if app.archive.is_some() { "ARCHIVE" } else { "FILE SYSTEM" },
        Style::default().fg(if app.archive.is_some() { pal.warn } else { pal.text_mid }),
    ));
    spans.push(Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.text_dim)));
    spans.push(Span::styled(format!("ITEMS:{}", item_count), Style::default().fg(pal.text_hot)));

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

        // Network activity indicator with animated braille glyph tied to throughput
        if let Some(mon) = &app.sysmon {
            let combined = mon.net.tx_bytes_sec + mon.net.rx_bytes_sec;
            let tick = app.glitch_tick;
            // Braille frames get denser with more traffic, rotation speed increases
            let (frames, label): (&[&str], &str) = if combined > 1024.0 * 1024.0 {
                (&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], "HEAVY")
            } else if combined > 1024.0 * 10.0 {
                (&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], "ACTIVE")
            } else if combined > 100.0 {
                (&["⠁", "⠈", "⠐", "⠠", "⢀", "⡀", "⠄", "⠂"], "LOW")
            } else {
                (&["·", " ", "·", " "], "IDLE")
            };
            // Higher throughput = faster rotation
            let speed = if combined > 1024.0 * 1024.0 { 1 } else if combined > 1024.0 * 10.0 { 2 } else { 4 };
            let frame_idx = (tick / speed) as usize % frames.len();
            spans.push(Span::styled("  \u{00b7}  ", Style::default().fg(pal.text_dim)));
            spans.push(Span::styled(
                format!("{} UPLINK:{}", frames[frame_idx], label),
                Style::default().fg(if combined > 1024.0 * 10.0 { pal.text_hot } else { pal.text_mid }),
            ));
        }
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

    // Signal indicator: amber colony terminal shows degraded signal
    let signal_label = if app.glitch_enabled && matches!(pal.variant, crate::throbber::PaletteVariant::Amber) {
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
