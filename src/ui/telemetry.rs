use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::sysmon::{cpu_sparkline_str, format_capacity, format_throughput, sparkline_str};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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

    let sym = &app.symbols;
    let spans = vec![
        Span::styled(
            sym.rule_left.to_string(),
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
            sym.rule_fill.repeat(right_rule_len),
            Style::default().fg(pal.border_mid),
        ),
        Span::styled(
            sym.rule_right,
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
        let sym = &app.symbols;
        let bar_filled = sym.bar_fill.repeat(filled);
        let bar_empty = sym.bar_empty.repeat(empty);

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

    // CPU sparkline waveform (braille-rendered history)
    let cpu_spark = cpu_sparkline_str(&sysmon.cpu_history, pal.variant);

    let sym = &app.symbols;
    lines.push(Line::from(vec![
        Span::styled(" CPU ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            sym.bar_fill.repeat(cpu_filled),
            Style::default().fg(cpu_color).bg(pal.bg),
        ),
        Span::styled(
            sym.bar_empty.repeat(cpu_empty),
            Style::default().fg(pal.border_dim).bg(pal.bg),
        ),
        Span::styled(
            format!(" {:>3}%", cpu_pct),
            Style::default().fg(pal.text_mid).bg(pal.bg),
        ),
        Span::styled("  ", Style::default().bg(pal.bg)),
        Span::styled(
            cpu_spark,
            Style::default().fg(cpu_color).bg(pal.bg),
        ),
        Span::styled("   MEM ", Style::default().fg(pal.text_dim).bg(pal.bg)),
        Span::styled(
            sym.bar_fill.repeat(mem_filled),
            Style::default().fg(mem_color).bg(pal.bg),
        ),
        Span::styled(
            sym.bar_empty.repeat(mem_empty),
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

    // I/O Waveform oscilloscope (#77)
    lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    lines.push(Line::from(Span::styled(
        " I/O WAVEFORM",
        Style::default().fg(pal.text_dim).bg(pal.bg),
    )));
    {
        let osc_width = (area.width as usize).saturating_sub(2).min(40);
        let samples = &app.io_history;
        let sample_count = samples.len();
        for row in 0..2u8 {
            let threshold = 1.0 - (row as f32 + 0.5) / 2.0;
            let mut row_str = String::from(" ");
            for col in 0..osc_width.min(sample_count) {
                let idx = if sample_count > osc_width { sample_count - osc_width + col } else { col };
                let val = samples.get(idx).copied().unwrap_or(0.0);
                let ch = if val >= threshold {
                    match pal.variant {
                        crate::throbber::PaletteVariant::Green => '\u{2847}',
                        crate::throbber::PaletteVariant::Amber => '\u{2588}',
                        crate::throbber::PaletteVariant::Cyan => '\u{2580}',
                    }
                } else if val >= threshold * 0.5 {
                    match pal.variant {
                        crate::throbber::PaletteVariant::Green => '\u{2801}',
                        crate::throbber::PaletteVariant::Amber => '\u{2584}',
                        crate::throbber::PaletteVariant::Cyan => '\u{2581}',
                    }
                } else {
                    ' '
                };
                row_str.push(ch);
            }
            let color = if row == 0 { pal.text_hot } else { pal.text_mid };
            lines.push(Line::from(Span::styled(row_str, Style::default().fg(color).bg(pal.bg))));
        }
    }

    // Pad
    let height = area.height as usize;
    while lines.len() < height {
        lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(pal.bg));
    f.render_widget(paragraph, area);
}

fn render_network(f: &mut Frame, app: &mut App, area: Rect) {
    let pal = app.palette;
    let sysmon = app.sysmon.as_ref().unwrap();

    let mut lines: Vec<Line> = Vec::new();

    // TX line with sparkline
    let tx_str = format_throughput(sysmon.net.tx_bytes_sec);
    let tx_sparkline = sparkline_str(&sysmon.net.tx_sparkline, pal.variant);

    let sym = &app.symbols;
    lines.push(Line::from(vec![
        Span::styled(format!(" {} TX ", sym.tx_indicator), Style::default().fg(pal.text_dim).bg(pal.bg)),
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
        Span::styled(format!(" {} RX ", sym.rx_indicator), Style::default().fg(pal.text_dim).bg(pal.bg)),
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

    // Telemetry animation in remaining space
    let height = area.height as usize;
    let anim_width = (area.width as usize).saturating_sub(3); // border + padding
    let remaining = height.saturating_sub(lines.len());
    if remaining > 0 && anim_width > 4 {
        lines.push(Line::from(Span::styled("", Style::default().bg(pal.bg))));
        let anim_rows = remaining.saturating_sub(1);
        // Seed GoL grid if needed (cyan palette)
        let dot_w = anim_width * 2;
        let dot_h = anim_rows * 4;
        if matches!(pal.variant, crate::throbber::PaletteVariant::Cyan) {
            let needs_seed = app.gol_grid.is_empty()
                || app.gol_grid.len() != dot_h
                || app.gol_grid[0].len() != dot_w;
            if needs_seed {
                app.seed_gol(dot_w, dot_h);
            }
        }
        let tick = app.glitch_tick as usize;
        let anim_lines = render_telemetry_animation(app, tick, anim_width, anim_rows);
        for (text, color) in anim_lines {
            lines.push(Line::from(Span::styled(
                format!(" {}", text),
                Style::default().fg(color).bg(pal.bg),
            )));
        }
    }

    // Pad any leftover
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

/// Generate per-palette braille animation for the network panel.
/// Returns Vec<(String, Color)> — one entry per row.
fn render_telemetry_animation(
    app: &App,
    tick: usize,
    width: usize,
    rows: usize,
) -> Vec<(String, ratatui::style::Color)> {
    let pal = app.palette;
    if rows == 0 || width == 0 {
        return vec![];
    }
    match pal.variant {
        crate::throbber::PaletteVariant::Green => render_radar_sweep(pal, tick, width, rows),
        crate::throbber::PaletteVariant::Amber => render_seismograph(pal, tick, width, rows),
        crate::throbber::PaletteVariant::Cyan => render_game_of_life(app, width, rows),
    }
}

/// Green: horizontal radar sweep with phosphor echo decay.
/// A vertical beam scans left-to-right, leaving fading blips.
fn render_radar_sweep(
    pal: crate::palette::Palette,
    tick: usize,
    width: usize,
    rows: usize,
) -> Vec<(String, ratatui::style::Color)> {
    // Braille cell = 2 wide × 4 tall dots
    // We build a dot grid of (rows*4) tall × (width*2) wide, then encode to braille
    let dot_h = rows * 4;
    let dot_w = width * 2;
    let mut grid = vec![vec![0u8; dot_w]; dot_h]; // 0=off, 1=dim, 2=mid, 3=bright

    // Sweep beam position (column in dot-space)
    let sweep_period = dot_w + 8;
    let beam_x = tick % sweep_period;

    // Draw beam as a vertical line
    for y in 0..dot_h {
        if beam_x < dot_w {
            grid[y][beam_x] = 3;
            // Slight spread
            if beam_x + 1 < dot_w { grid[y][beam_x + 1] = 2; }
        }
    }

    // Echo blips — deterministic from LCG seeded by position
    for i in 0..((width * rows) / 3).max(4) {
        let seed = i.wrapping_mul(7919).wrapping_add(31);
        let bx = seed % dot_w;
        let by = (seed.wrapping_mul(13)) % dot_h;
        // Only show blip if the beam has passed it recently
        if beam_x < dot_w {
            let dist = if beam_x >= bx { beam_x - bx } else { sweep_period - bx + beam_x };
            if dist < 6 {
                grid[by][bx] = if dist < 2 { 3 } else if dist < 4 { 2 } else { 1 };
            }
        }
    }

    encode_braille_grid(&grid, rows, width, pal.text_hot)
}

/// Amber: scrolling seismograph waveform.
/// Height values scroll left with new readings on the right.
fn render_seismograph(
    pal: crate::palette::Palette,
    tick: usize,
    width: usize,
    rows: usize,
) -> Vec<(String, ratatui::style::Color)> {
    let dot_h = rows * 4;
    let dot_w = width * 2;
    let mut grid = vec![vec![0u8; dot_w]; dot_h];
    let midline = dot_h / 2;

    for col in 0..dot_w {
        // Generate height using layered sine waves for organic feel
        let t = (col + tick) as f64;
        let v1 = (t * 0.15).sin() * 0.4;
        let v2 = (t * 0.37).sin() * 0.25;
        let v3 = (t * 0.73).sin() * 0.15;
        // Occasional spike from LCG hash
        let hash = ((col + tick).wrapping_mul(2654435761)) % 100;
        let spike = if hash < 3 { 0.3 } else { 0.0 };
        let displacement = v1 + v2 + v3 + spike;
        let y_offset = (displacement * midline as f64) as i32;
        let y = (midline as i32 + y_offset).clamp(0, dot_h as i32 - 1) as usize;

        // Draw the waveform point and a faint trace below
        grid[y][col] = 3;
        // Vertical fill toward midline for body
        let (from, to) = if y < midline { (y, midline) } else { (midline, y) };
        for fy in from..=to {
            if grid[fy][col] < 1 { grid[fy][col] = 1; }
        }
    }

    encode_braille_grid(&grid, rows, width, pal.text_hot)
}

/// Cyan: Conway's Game of Life cellular automaton.
/// Reads live state from app.gol_grid, encodes to braille.
fn render_game_of_life(
    app: &App,
    width: usize,
    rows: usize,
) -> Vec<(String, ratatui::style::Color)> {
    let pal = app.palette;
    let dot_h = rows * 4;
    let dot_w = width * 2;

    // Build grid from GoL state (may be different size — clamp to fit)
    let mut grid = vec![vec![0u8; dot_w]; dot_h];
    for y in 0..dot_h {
        for x in 0..dot_w {
            if y < app.gol_grid.len() && x < app.gol_grid[0].len() && app.gol_grid[y][x] {
                grid[y][x] = 2;
            }
        }
    }

    encode_braille_grid(&grid, rows, width, pal.text_hot)
}

/// Encode a dot grid into braille characters.
/// Grid values: 0=off, 1+=on. Each braille cell is 2 wide × 4 tall.
/// Returns one (String, Color) per text row.
fn encode_braille_grid(
    grid: &[Vec<u8>],
    rows: usize,
    width: usize,
    color: ratatui::style::Color,
) -> Vec<(String, ratatui::style::Color)> {
    let dot_h = grid.len();
    let dot_w = if dot_h > 0 { grid[0].len() } else { 0 };
    let mut result = Vec::with_capacity(rows);

    for row in 0..rows {
        let mut s = String::with_capacity(width);
        for col in 0..width {
            let dx = col * 2;
            let dy = row * 4;
            let mut cp: u32 = 0x2800;

            // Braille dot mapping:
            // (0,0)=1  (1,0)=8
            // (0,1)=2  (1,1)=16
            // (0,2)=4  (1,2)=32
            // (0,3)=64 (1,3)=128
            for &(ox, oy, bit) in &[
                (0,0,1), (0,1,2), (0,2,4), (0,3,64),
                (1,0,8), (1,1,16), (1,2,32), (1,3,128),
            ] {
                let gx = dx + ox;
                let gy = dy + oy;
                if gx < dot_w && gy < dot_h && grid[gy][gx] > 0 {
                    cp |= bit;
                }
            }
            s.push(char::from_u32(cp).unwrap_or(' '));
        }
        result.push((s, color));
    }
    result
}
