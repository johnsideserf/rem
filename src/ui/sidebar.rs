use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;
    let width = area.width as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Visual selection summary (#55)
    if !app.visual_marks.is_empty() {
        lines.push(Line::from(Span::styled(
            " S E L E C T I O N   S U M M A R Y",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
        let pane = app.pane();
        let mut total_size: u64 = 0;
        let mut file_count: usize = 0;
        let mut dir_count: usize = 0;
        for &idx in &app.visual_marks {
            if let Some(entry) = pane.entries.get(idx) {
                if entry.is_dir {
                    dir_count += 1;
                } else {
                    file_count += 1;
                    if let Some(s) = entry.size {
                        total_size += s;
                    }
                }
            }
        }
        lines.push(Line::from(vec![
            Span::styled(" FILES ", Style::default().fg(pal.text_dim).bg(pal.bg)),
            Span::styled(file_count.to_string(), Style::default().fg(pal.text_hot).bg(pal.bg)),
        ]));
        if dir_count > 0 {
            lines.push(Line::from(vec![
                Span::styled(" DIRS  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                Span::styled(dir_count.to_string(), Style::default().fg(pal.text_hot).bg(pal.bg)),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled(" TOTAL ", Style::default().fg(pal.text_dim).bg(pal.bg)),
            Span::styled(
                crate::app::format_size(total_size),
                Style::default().fg(pal.text_hot).bg(pal.bg),
            ),
        ]));
        lines.push(Line::from(Span::raw("")));
    }

    // SELECTION section
    lines.push(Line::from(Span::styled(
        " S E L E C T I O N",
        Style::default().fg(pal.text_dim).bg(pal.bg),
    )));

    if let Some(entry) = app.current_entry() {
        let max_name = width.saturating_sub(9);
        let name_display = if entry.name.chars().count() > max_name {
            let t: String = entry.name.chars().take(max_name.saturating_sub(1)).collect();
            format!("{}\u{2026}", t)
        } else {
            entry.name.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(" NAME  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
            Span::styled(name_display, Style::default().fg(pal.text_hot).bg(pal.bg)),
        ]));

        let type_str = if entry.is_dir { "DIR" } else { "FILE" };
        lines.push(Line::from(vec![
            Span::styled(" TYPE  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
            Span::styled(type_str, Style::default().fg(pal.text_hot).bg(pal.bg)),
        ]));

        if let Some(size) = entry.size {
            lines.push(Line::from(vec![
                Span::styled(" SIZE  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                Span::styled(
                    crate::app::format_size(size),
                    Style::default().fg(pal.text_hot).bg(pal.bg),
                ),
            ]));

            // Size sparkline (#59)
            let pane = app.pane();
            let sibling_sizes: Vec<u64> = pane.entries.iter()
                .filter_map(|e| e.size)
                .collect();
            if sibling_sizes.len() > 1 {
                let max_size = sibling_sizes.iter().copied().max().unwrap_or(1).max(1);
                let bar_width = (width).saturating_sub(8);
                let blocks = ["\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}"];
                let bucket_count = bar_width.min(sibling_sizes.len());
                let mut sorted = sibling_sizes.clone();
                sorted.sort();
                let bucket_size = (sorted.len() + bucket_count - 1) / bucket_count;
                let mut spark = String::new();
                for i in 0..bucket_count {
                    let start = i * bucket_size;
                    let end = ((i + 1) * bucket_size).min(sorted.len());
                    if start >= sorted.len() { break; }
                    let avg = sorted[start..end].iter().sum::<u64>() / (end - start) as u64;
                    let level = ((avg as f64 / max_size as f64) * 7.0) as usize;
                    spark.push_str(blocks[level.min(7)]);
                }
                let rank = sorted.iter().filter(|&&s| s <= size).count();
                let pct = if !sorted.is_empty() { rank * 100 / sorted.len() } else { 0 };
                lines.push(Line::from(vec![
                    Span::styled(" RANK  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                    Span::styled(
                        format!("P{}", pct),
                        Style::default().fg(pal.text_hot).bg(pal.bg),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" ", Style::default().bg(pal.bg)),
                    Span::styled(spark, Style::default().fg(pal.text_mid).bg(pal.bg)),
                ]));
            }
        }

        if let Some(modified) = entry.modified {
            if let Ok(duration) = modified.elapsed() {
                let secs = duration.as_secs();
                let age = if secs < 60 {
                    format!("{}s ago", secs)
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else if secs < 86400 {
                    format!("{}h ago", secs / 3600)
                } else {
                    format!("{}d ago", secs / 86400)
                };
                lines.push(Line::from(vec![
                    Span::styled(" MOD   ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                    Span::styled(age, Style::default().fg(pal.text_hot).bg(pal.bg)),
                ]));
            }
        }

        if entry.is_dir {
            if let Ok(rd) = std::fs::read_dir(&entry.path) {
                let count = rd.count();
                lines.push(Line::from(vec![
                    Span::styled(" ITEMS ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                    Span::styled(
                        count.to_string(),
                        Style::default().fg(pal.text_hot).bg(pal.bg),
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            " EMPTY",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
    }

    // Tags (#58)
    if let Some(entry) = app.current_entry() {
        if let Some(tags) = app.tags.get(&entry.path) {
            lines.push(Line::from(vec![
                Span::styled(" TAGS  ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                Span::styled(
                    tags.join(", "),
                    Style::default().fg(pal.text_hot).bg(pal.bg),
                ),
            ]));
        }
    }

    // SHA-256 hash display (#20)
    if let Some(entry) = app.current_entry() {
        if let Some((hash_path, hash_val)) = &app.last_hash {
            if *hash_path == entry.path {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(Span::styled(
                    " I N T E G R I T Y",
                    Style::default().fg(pal.text_dim).bg(pal.bg),
                )));
                // Show hash in two lines (32 chars each)
                let max_hash = width.saturating_sub(10);
                let display_hash = if hash_val.len() > max_hash {
                    let t = &hash_val[..max_hash.saturating_sub(1)];
                    format!("{}\u{2026}", t)
                } else {
                    hash_val.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(" SHA256 ", Style::default().fg(pal.text_dim).bg(pal.bg)),
                    Span::styled(display_hash, Style::default().fg(pal.text_hot).bg(pal.bg)),
                ]));
            }
        }
    }

    // Blank separator
    lines.push(Line::from(Span::raw("")));

    // BOOKMARKS section
    lines.push(Line::from(Span::styled(
        " B O O K M A R K S",
        Style::default().fg(pal.text_dim).bg(pal.bg),
    )));

    if app.marks.is_empty() {
        lines.push(Line::from(Span::styled(
            " NONE",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
    } else {
        let mut sorted_marks: Vec<_> = app.marks.iter().collect();
        sorted_marks.sort_by_key(|(k, _)| *k);
        for (key, path) in sorted_marks.iter().take(8) {
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let max_bm = width.saturating_sub(7);
            let display = if dir_name.chars().count() > max_bm {
                let t: String = dir_name.chars().take(max_bm.saturating_sub(1)).collect();
                format!("{}\u{2026}", t)
            } else {
                dir_name
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" '{} ", key), Style::default().fg(pal.text_hot).bg(pal.bg)),
                Span::styled(display, Style::default().fg(pal.text_mid).bg(pal.bg)),
            ]));
        }
    }

    // FAVORITES section (#54)
    if !app.favorites.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            " F A V O R I T E S",
            Style::default().fg(pal.text_dim).bg(pal.bg),
        )));
        for (i, fav) in app.favorites.iter().take(9).enumerate() {
            let dir_name = fav.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| fav.to_string_lossy().into_owned());
            let max_fav = width.saturating_sub(7);
            let display = if dir_name.chars().count() > max_fav {
                let t: String = dir_name.chars().take(max_fav.saturating_sub(1)).collect();
                format!("{}\u{2026}", t)
            } else {
                dir_name
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", i + 1), Style::default().fg(pal.text_hot).bg(pal.bg)),
                Span::styled(display, Style::default().fg(pal.text_mid).bg(pal.bg)),
            ]));
        }
    }

    // Frecency section (#84)
    {
        let top = app.frecency.top_dirs(5);
        if !top.is_empty() {
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                " F R E Q U E N T",
                Style::default().fg(pal.text_dim).bg(pal.bg),
            )));
            for (path_str, _score) in &top {
                let dir_name = std::path::Path::new(path_str)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path_str.clone());
                let max_w = width.saturating_sub(4);
                let display = if dir_name.chars().count() > max_w {
                    let t: String = dir_name.chars().take(max_w.saturating_sub(1)).collect();
                    format!("{}\u{2026}", t)
                } else {
                    dir_name
                };
                lines.push(Line::from(Span::styled(
                    format!(" {}", display),
                    Style::default().fg(pal.text_mid).bg(pal.bg),
                )));
            }
        }
    }

    // Pad to fill area
    let height = area.height as usize;
    while lines.len() < height {
        lines.push(Line::from(Span::styled(
            "",
            Style::default().bg(pal.bg),
        )));
    }

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(pal.border_dim))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
