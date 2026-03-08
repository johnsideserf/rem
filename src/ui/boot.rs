use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Terminal;

use crate::logo;
use crate::palette::Palette;
use crate::throbber::{Throbber, ThrobberKind};

struct BootLine {
    label: &'static str,
    dots: &'static str,
    value: &'static str,
}

const BOOT_LINES: &[BootLine] = &[
    BootLine { label: "BIOS",      dots: " .............. ", value: "OK" },
    BootLine { label: "MEMORY",    dots: " ............ ", value: "640K" },
    BootLine { label: "DISK",      dots: " .............. ", value: "" },
    BootLine { label: "INTERFACE", dots: " ......... ", value: "NOMINAL" },
];

/// Total logo block lines: logo art (11) + blank + corp name + tagline + rule = 15
const LOGO_LINE_COUNT: usize = 15;

/// Run the boot sequence. Returns true if completed, false if skipped.
pub fn run_boot(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: Palette,
) -> io::Result<bool> {
    let mut throbber = Throbber::new(ThrobberKind::DataStream, palette.variant);
    let start = Instant::now();

    let mut visible_logo_lines: usize = 0;
    let mut visible_boot_lines: usize = 0;
    let mut boot_value_shown: Vec<bool> = vec![false; BOOT_LINES.len()];
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
                if visible_boot_lines < BOOT_LINES.len() {
                    if !boot_value_shown[visible_boot_lines] {
                        if sub_elapsed >= Duration::from_millis(100) {
                            if visible_boot_lines == 2 {
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
                if elapsed >= Duration::from_millis(300) {
                    phase = Phase::Done;
                }
            }
            Phase::Done => {
                return Ok(true);
            }
        }

        // Safety: don't let boot exceed 4 seconds total
        if start.elapsed() >= Duration::from_secs(4) {
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
            for (i, bl) in BOOT_LINES.iter().enumerate() {
                if i >= visible_boot_lines && !(i == visible_boot_lines && boot_value_shown.get(i) == Some(&true)) {
                    if i > visible_boot_lines {
                        break;
                    }
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots, Style::default().fg(palette.text_dim)),
                    ]));
                    break;
                }

                if i == 2 && !disk_resolved {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots, Style::default().fg(palette.text_dim)),
                        Span::styled(throbber.frame(), Style::default().fg(palette.text_hot)),
                        Span::styled(" SCANNING", Style::default().fg(palette.text_dim)),
                    ]));
                } else {
                    let value = if i == 2 { "OK" } else { bl.value };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}", bl.label), Style::default().fg(palette.text_mid)),
                        Span::styled(bl.dots, Style::default().fg(palette.text_dim)),
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
