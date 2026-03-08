use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::sysmon::{format_capacity, format_throughput, sparkline_str};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    if app.sysmon.is_none() {
        return;
    }

    // Split telemetry area: left (disks + vitals) | right (network)
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1)));

    // Top border with label
    render_top_border(f, app, Rect::new(area.x, area.y, area.width, 1));

    // Left side: Disks + Vitals
    render_disks_and_vitals(f, app, halves[0]);

    // Right side: Network
    render_network(f, app, halves[1]);
}

fn render_top_border(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;

    // Monitoring throbber
    let throb = app.telemetry_throbber.as_ref()
        .map(|t| t.frame())
        .unwrap_or(" ");

    let label = " S Y S T E M  T E L E M E T R Y ";
    let left_rule_len = 2;
    let right_rule_len = width.saturating_sub(left_rule_len + label.len() + 4 + 3);

    let spans = vec![
        Span::styled(
            "\u{2576}\u{2500}".to_string(),
            Style::default().fg(pal.border_mid),
        ),
        Span::styled(
            format!(" {} ", throb),
            Style::default().fg(pal.text_hot).bg(pal.bg),
        ),
        Span::styled(
            label,
            Style::default().fg(pal.text_dim).bg(pal.bg),
        ),
        Span::styled(
            "\u{2500}".repeat(right_rule_len),
            Style::default().fg(pal.border_mid),
        ),
        Span::styled(
            "\u{2574}",
            Style::default().fg(pal.border_mid),
        ),
    ];

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(pal.bg));
    f.render_widget(paragraph, area);
}

fn render_disks_and_vitals(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let sysmon = app.sysmon.as_ref().unwrap();
    let width = area.width as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Disk section
    let bar_width = 22usize.min(width.saturating_sub(30));

    for disk in &sysmon.disk_info {
        let pct = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as u64
        } else {
            0
        };

        let filled = (pct as usize * bar_width / 100).min(bar_width);
        let empty = bar_width.saturating_sub(filled);
        let bar_filled = "\u{2588}".repeat(filled);
        let bar_empty = "\u{2591}".repeat(empty);

        // Disk label (mount point, truncated)
        let mount_display = if disk.mount.len() > 4 {
            let t: String = disk.mount.chars().take(3).collect();
            format!("{}.", t)
        } else {
            format!("{:<4}", disk.mount)
        };

        let capacity = format_capacity(disk.total);
        let pct_str = format!("{}%", pct);

        // Color the bar based on usage
        let bar_color = if pct > 90 {
            pal.warn
        } else if pct > 75 {
            pal.text_mid
        } else {
            pal.text_hot
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", mount_display),
                Style::default().fg(pal.text_mid).bg(pal.bg),
            ),
            Span::styled(
                bar_filled,
                Style::default().fg(bar_color).bg(pal.bg),
            ),
            Span::styled(
                bar_empty,
                Style::default().fg(pal.border_dim).bg(pal.bg),
            ),
            Span::styled(
                format!(" {:>3}", pct_str),
                Style::default().fg(pal.text_mid).bg(pal.bg),
            ),
            Span::styled(
                format!("  {}", capacity),
                Style::default().fg(pal.text_dim).bg(pal.bg),
            ),
        ]));
    }

    // Blank line then CPU/MEM
    if !sysmon.disk_info.is_empty() {
        lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    }

    // CPU bar
    let cpu_pct = sysmon.cpu_pct as u64;
    let cpu_filled = (cpu_pct as usize * 12 / 100).min(12);
    let cpu_empty = 12usize.saturating_sub(cpu_filled);
    let cpu_color = if cpu_pct > 90 { pal.warn } else if cpu_pct > 70 { pal.text_mid } else { pal.text_hot };

    // MEM bar
    let mem_pct = if sysmon.mem_total > 0 {
        (sysmon.mem_used as f64 / sysmon.mem_total as f64 * 100.0) as u64
    } else { 0 };
    let mem_filled = (mem_pct as usize * 12 / 100).min(12);
    let mem_empty = 12usize.saturating_sub(mem_filled);
    let mem_color = if mem_pct > 90 { pal.warn } else if mem_pct > 70 { pal.text_mid } else { pal.text_hot };
    let mem_str = format!(
        "{}/{}",
        format_capacity(sysmon.mem_used),
        format_capacity(sysmon.mem_total),
    );

    lines.push(Line::from(vec![
        Span::styled(" CPU ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            "\u{2588}".repeat(cpu_filled),
            Style::default().fg(cpu_color).bg(pal.bg),
        ),
        Span::styled(
            "\u{2591}".repeat(cpu_empty),
            Style::default().fg(pal.border_dim).bg(pal.bg),
        ),
        Span::styled(
            format!(" {:>3}%", cpu_pct),
            Style::default().fg(pal.text_mid).bg(pal.bg),
        ),
        Span::styled("   MEM ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            "\u{2588}".repeat(mem_filled),
            Style::default().fg(mem_color).bg(pal.bg),
        ),
        Span::styled(
            "\u{2591}".repeat(mem_empty),
            Style::default().fg(pal.border_dim).bg(pal.bg),
        ),
        Span::styled(
            format!(" {:>3}%", mem_pct),
            Style::default().fg(pal.text_mid).bg(pal.bg),
        ),
        Span::styled(
            format!("  {}", mem_str),
            Style::default().fg(pal.text_dim).bg(pal.bg),
        ),
    ]));

    // Pad
    let height = area.height as usize;
    while lines.len() < height {
        lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(pal.bg));
    f.render_widget(paragraph, area);
}

fn render_network(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let sysmon = app.sysmon.as_ref().unwrap();

    let mut lines: Vec<Line> = Vec::new();

    // TX line with sparkline
    let tx_str = format_throughput(sysmon.net.tx_bytes_sec);
    let tx_sparkline = sparkline_str(&sysmon.net.tx_sparkline, pal.variant);

    lines.push(Line::from(vec![
        Span::styled(" \u{25b2} TX ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            format!("{:>10}", tx_str),
            Style::default().fg(pal.text_hot).bg(pal.bg),
        ),
        Span::styled("  ", Style::default().bg(pal.bg)),
        Span::styled(
            tx_sparkline,
            Style::default().fg(pal.text_hot).bg(pal.bg),
        ),
    ]));

    // RX line with sparkline
    let rx_str = format_throughput(sysmon.net.rx_bytes_sec);
    let rx_sparkline = sparkline_str(&sysmon.net.rx_sparkline, pal.variant);

    lines.push(Line::from(vec![
        Span::styled(" \u{25bc} RX ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            format!("{:>10}", rx_str),
            Style::default().fg(pal.text_mid).bg(pal.bg),
        ),
        Span::styled("  ", Style::default().bg(pal.bg)),
        Span::styled(
            rx_sparkline,
            Style::default().fg(pal.text_mid).bg(pal.bg),
        ),
    ]));

    // Blank then aggregate totals
    lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));

    // Show combined throughput indicator
    let combined = sysmon.net.tx_bytes_sec + sysmon.net.rx_bytes_sec;
    let activity = if combined > 1024.0 * 1024.0 {
        "HEAVY"
    } else if combined > 1024.0 * 10.0 {
        "ACTIVE"
    } else if combined > 100.0 {
        "NOMINAL"
    } else {
        "IDLE"
    };

    let activity_color = if combined > 1024.0 * 1024.0 {
        pal.text_hot
    } else if combined > 1024.0 * 10.0 {
        pal.text_mid
    } else {
        pal.text_dim
    };

    lines.push(Line::from(vec![
        Span::styled(" LINK: ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            activity,
            Style::default().fg(activity_color).bg(pal.bg),
        ),
    ]));

    // Pad
    let height = area.height as usize;
    while lines.len() < height {
        lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    }

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_dim))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
