use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Terminal;
use sysinfo::System;

use crate::logo;
use crate::palette::Palette;
use crate::throbber::{PaletteVariant, Throbber, ThrobberKind};

/// A resolved boot line with owned strings (so we can inject live values).
struct ResolvedBootLine {
    label: String,
    dots: String,
    value: String,
    /// If true, this line gets the disk-throbber treatment during boot.
    is_disk: bool,
}

/// Build the boot line sequence for a given palette variant, with live system values.
fn build_boot_lines(variant: PaletteVariant) -> Vec<ResolvedBootLine> {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let total_ram_mb = sys.total_memory() / (1024 * 1024);
    let cpu_count = sys.cpus().len();

    let ram_str = if total_ram_mb >= 1024 {
        format!("{}GB", total_ram_mb / 1024)
    } else {
        format!("{}MB", total_ram_mb)
    };

    let cpu_str = if cpu_count > 0 {
        format!("{} CORES", cpu_count)
    } else {
        "OK".to_string()
    };

    let tail_line: (&str, &str, &str) = match variant {
        PaletteVariant::Green => ("NAV SYS",   " ........... ", "ONLINE"),
        PaletteVariant::Amber => ("ATMO PROC", " .......... ", "NOMINAL"),
        PaletteVariant::Cyan  => ("CLEARANCE", " ......... ", "GRANTED"),
    };

    vec![
        ResolvedBootLine {
            label: "BIOS".into(),
            dots:  " .............. ".into(),
            value: "OK".into(),
            is_disk: false,
        },
        ResolvedBootLine {
            label: "CPU".into(),
            dots:  " ............... ".into(),
            value: cpu_str,
            is_disk: false,
        },
        ResolvedBootLine {
            label: "MEMORY".into(),
            dots:  " ............ ".into(),
            value: ram_str,
            is_disk: false,
        },
        ResolvedBootLine {
            label: "DISK".into(),
            dots:  " .............. ".into(),
            value: String::new(),
            is_disk: true,
        },
        ResolvedBootLine {
            label: tail_line.0.into(),
            dots:  tail_line.1.into(),
            value: tail_line.2.into(),
            is_disk: false,
        },
    ]
}

/// Total logo block lines: logo art (11) + blank + corp name + tagline + rule = 15
const LOGO_LINE_COUNT: usize = 15;

/// Run a CRT warm-up effect: ramp the screen from black to `palette.bg` over 5 frames.
/// Skippable on any keypress. Returns `Ok(true)` if completed, `Ok(false)` if skipped.
pub fn run_warmup(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: Palette,
) -> io::Result<bool> {
    const FRAMES: u32 = 5;
    const FRAME_MS: u64 = 100;

    // Extract target RGB components from palette.bg
    let (tr, tg, tb) = match palette.bg {
        ratatui::style::Color::Rgb(r, g, b) => (r, g, b),
        _ => return Ok(true), // non-RGB bg, skip warmup
    };

    for frame in 1..=FRAMES {
        // Check for keypress to skip
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(false);
                }
            }
        }

        // Linearly interpolate from black (0,0,0) to (tr, tg, tb)
        let r = (tr as u32 * frame / FRAMES) as u8;
        let g = (tg as u32 * frame / FRAMES) as u8;
        let b = (tb as u32 * frame / FRAMES) as u8;
        let color = ratatui::style::Color::Rgb(r, g, b);

        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(
                Block::default().style(Style::default().bg(color)),
                area,
            );
        })?;

        std::thread::sleep(Duration::from_millis(FRAME_MS));
    }

    Ok(true)
}

/// Run the boot sequence. Returns true if completed, false if skipped.
pub fn run_boot(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: Palette,
) -> io::Result<bool> {
    let mut throbber = Throbber::new(ThrobberKind::DataStream, palette.variant);
    let boot_lines = build_boot_lines(palette.variant);
    let disk_index = boot_lines.iter().position(|bl| bl.is_disk);
    let start = Instant::now();

    let mut visible_logo_lines: usize = 0;
    let mut visible_boot_lines: usize = 0;
    let mut boot_value_shown: Vec<bool> = vec![false; boot_lines.len()];
    let mut disk_resolved = false;
    let mut ready_shown = false;
    let mut phase_timer = Instant::now();
    let mut sub_timer = Instant::now();

    enum Phase {
        Logo,
        BootLines,
        DiskThrobber,
        Ready,
        Done,
    }
    let mut phase = Phase::Logo;

    loop {
        // Check for keypress to skip
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Flash "BOOT OVERRIDE" feedback
                    terminal.draw(|f| {
                        let area = f.area();
                        f.render_widget(
                            Block::default().style(Style::default().bg(palette.bg)),
                            area,
                        );
                        let msg = Line::from(Span::styled(
                            "  BOOT OVERRIDE \u{2014} SEQUENCE BYPASSED",
                            Style::default().fg(palette.warn).add_modifier(Modifier::BOLD),
                        ));
                        f.render_widget(
                            Paragraph::new(msg),
                            Rect::new(area.x, area.y + 1, area.width, 1),
                        );
                    })?;
                    std::thread::sleep(Duration::from_millis(200));
                    return Ok(false);
                }
            }
        }

        throbber.tick();

        let elapsed = phase_timer.elapsed();
        let sub_elapsed = sub_timer.elapsed();

        match phase {
            Phase::Logo => {
                if visible_logo_lines < LOGO_LINE_COUNT && elapsed >= Duration::from_millis(80) {
                    visible_logo_lines += 1;
                    phase_timer = Instant::now();
                }
                if visible_logo_lines >= LOGO_LINE_COUNT && elapsed >= Duration::from_millis(150) {
                    phase = Phase::BootLines;
                    phase_timer = Instant::now();
                    sub_timer = Instant::now();
                }
            }
            Phase::BootLines => {
                if visible_boot_lines < boot_lines.len() {
                    if !boot_value_shown[visible_boot_lines] {
                        if sub_elapsed >= Duration::from_millis(100) {
                            if disk_index == Some(visible_boot_lines) {
                                boot_value_shown[visible_boot_lines] = true;
                                phase = Phase::DiskThrobber;
                                phase_timer = Instant::now();
                            } else {
                                boot_value_shown[visible_boot_lines] = true;
                                sub_timer = Instant::now();
                            }
                        }
                    } else if sub_elapsed >= Duration::from_millis(150) {
                        visible_boot_lines += 1;
                        sub_timer = Instant::now();
                    }
                } else {
                    phase = Phase::Ready;
                    phase_timer = Instant::now();
                }
            }
            Phase::DiskThrobber => {
                if elapsed >= Duration::from_millis(400) {
                    disk_resolved = true;
                    visible_boot_lines += 1;
                    phase = Phase::BootLines;
                    phase_timer = Instant::now();
                    sub_timer = Instant::now();
                }
            }
            Phase::Ready => {
                if !ready_shown {
                    ready_shown = true;
                    phase_timer = Instant::now();
                }
                if elapsed >= Duration::from_millis(1500) {
                    phase = Phase::Done;
                }
            }
            Phase::Done => {
                return Ok(true);
            }
        }

        // Safety: don't let boot exceed 6 seconds total
        if start.elapsed() >= Duration::from_secs(6) {
            return Ok(true);
        }

        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(
                Block::default().style(Style::default().bg(palette.bg)),
                area,
            );

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(Span::raw(""))); // top padding

            // Logo art — dual-color dotmatrix: @ in text_hot, . in border_dim
            let logo_art = logo::logo_for(palette.variant);
            for (i, logo_line) in logo_art.iter().enumerate() {
                if i >= visible_logo_lines {
                    break;
                }
                let mut spans: Vec<Span> = Vec::new();
                let mut run = String::new();
                let mut run_is_fill = true;
                for ch in logo_line.chars() {
                    let is_fill = ch == '@';
                    if !run.is_empty() && is_fill != run_is_fill {
                        let color = if run_is_fill { palette.text_hot } else { palette.border_dim };
                        spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(color)));
                    }
                    run_is_fill = is_fill;
                    run.push(ch);
                }
                if !run.is_empty() {
                    let color = if run_is_fill { palette.text_hot } else { palette.border_dim };
                    spans.push(Span::styled(run, Style::default().fg(color)));
                }
                lines.push(Line::from(spans));
            }

            // Blank line after logo
            let logo_art_count = logo_art.len();
            if visible_logo_lines > logo_art_count {
                lines.push(Line::from(Span::raw("")));
            }

            // Corporate name
            if visible_logo_lines > logo_art_count + 1 {
                lines.push(Line::from(Span::styled(
                    format!("  {}", logo::CORP_NAME),
                    Style::default().fg(palette.text_mid),
                )));
            }

            // Tagline
            if visible_logo_lines > logo_art_count + 2 {
                lines.push(Line::from(Span::styled(
                    format!("  {}", logo::CORP_TAG),
                    Style::default().fg(palette.text_dim),
                )));
            }

            // Rule
            if visible_logo_lines >= LOGO_LINE_COUNT {
                let rule_width = 38usize.min(area.width.saturating_sub(4) as usize);
                lines.push(Line::from(Span::styled(
                    format!("  {}", "\u{2501}".repeat(rule_width)),
                    Style::default().fg(palette.border_mid),
                )));
                lines.push(Line::from(Span::raw("")));
            }

            // Boot check lines
            for (i, bl) in boot_lines.iter().enumerate() {
                if i >= visible_boot_lines && !(i == visible_boot_lines && boot_value_shown.get(i) == Some(&true)) {
                    if i > visible_boot_lines {
                        break;
                    }
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots.as_str(), Style::default().fg(palette.text_dim)),
                    ]));
                    break;
                }

                if bl.is_disk && !disk_resolved {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots.as_str(), Style::default().fg(palette.text_dim)),
                        Span::styled(throbber.frame(), Style::default().fg(palette.text_hot)),
                        Span::styled(" SCANNING", Style::default().fg(palette.text_dim)),
                    ]));
                } else {
                    let value = if bl.is_disk { "OK" } else { bl.value.as_str() };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots.as_str(), Style::default().fg(palette.text_dim)),
                        Span::styled(value, Style::default().fg(palette.text_hot)),
                    ]));
                }
            }

            // READY
            if ready_shown {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(Span::styled(
                    "  READY.",
                    Style::default().fg(palette.text_hot).add_modifier(Modifier::BOLD),
                )));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, Rect::new(area.x, area.y, area.width, area.height));
        })?;
    }
}
