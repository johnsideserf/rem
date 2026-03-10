use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    render_pane(f, app, app.active_pane, area, true);
}

pub fn render_pane(f: &mut Frame, app: &mut App, pane_idx: usize, area: Rect, show_cursor: bool) {
    let pal = app.palette;

    // Archive mode: show archive path with internal directory (#19)
    let archive_path_str;
    let path_str = if let Some(archive) = &app.archive {
        let archive_name = archive.archive_path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        archive_path_str = if archive.internal_dir.is_empty() {
            format!("{}:", archive_name)
        } else {
            format!("{}:/{}", archive_name, archive.internal_dir.trim_end_matches('/'))
        };
        archive_path_str.clone()
    } else {
        app.panes[pane_idx].current_dir.to_string_lossy().into_owned()
    };
    let segments: Vec<&str> = path_str.split(['/', '\\', ':']).filter(|s| !s.is_empty()).collect();

    let mut spans = vec![Span::styled(" ", Style::default())];

    let max_width = area.width as usize - 4; // reserve space for cursor
    let mut built = String::new();

    // Track which segments survive left-truncation for click targets (#48).
    // When truncation happens we reset, so we record the index of the first
    // segment that is actually rendered after the last truncation.
    let mut first_visible_seg: usize = 0;
    let mut truncated = false;

    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let sep = " / ";
        let addition = if built.is_empty() {
            seg.to_string()
        } else {
            format!("{}{}", sep, seg)
        };

        if built.len() + addition.len() > max_width && !built.is_empty() {
            // Truncate from the left
            spans = vec![Span::styled(" ", Style::default())];
            spans.push(Span::styled("\u{2026} / ", Style::default().fg(pal.text_dim)));
            built.clear();
            first_visible_seg = i;
            truncated = true;
        }

        if !built.is_empty() {
            spans.push(Span::styled(" / ", Style::default().fg(pal.text_dim)));
        }

        let style = if is_last {
            Style::default().fg(pal.text_hot)
        } else {
            Style::default().fg(pal.text_mid)
        };
        spans.push(Span::styled(seg.to_uppercase(), style));
        built.push_str(&addition);
    }

    // Blinking cursor (only on active pane)
    if show_cursor && app.blink_on {
        spans.push(Span::styled(format!(" {}", app.symbols.text_cursor), Style::default().fg(pal.text_hot)));
    } else {
        spans.push(Span::styled("  ", Style::default()));
    }

    // Sort indicator (#56)
    spans.push(Span::styled(
        format!(" SORT:{}", app.sort_mode.label()),
        Style::default().fg(pal.text_dim),
    ));

    // Record breadcrumb click targets (#48).
    // Only for the active pane (show_cursor == true) and non-archive mode.
    if show_cursor && app.archive.is_none() {
        app.layout_areas.breadcrumb_segments.clear();
        app.layout_areas.breadcrumb_area = Some((area.x, area.y, area.width, area.height));

        let full_path = &app.panes[pane_idx].current_dir;
        let components: Vec<_> = full_path.components().collect();

        // Walk the visible segments and compute x positions.
        // The leading span is " " (1 char).  If truncated, we have " " + "... / " (1 + 5 = 6 chars prefix).
        let mut x_offset: u16 = area.x;
        if truncated {
            x_offset += 1 + 5; // " " + "... / " (ellipsis is 1 char + " / " = 4, total 5)
        } else {
            x_offset += 1; // leading space
        }

        for (vi, seg_idx) in (first_visible_seg..segments.len()).enumerate() {
            if vi > 0 {
                x_offset += 3; // " / "
            }
            let seg_text = segments[seg_idx].to_uppercase();
            let seg_width = seg_text.len() as u16;
            let start_x = x_offset;
            let end_x = x_offset + seg_width;

            // Build cumulative path up to this segment using OS path components.
            if seg_idx < components.len() {
                let mut nav_path = std::path::PathBuf::new();
                for c in components.iter().take(seg_idx + 1) {
                    nav_path.push(c);
                }
                app.layout_areas.breadcrumb_segments.push((start_x, end_x, nav_path));
            }

            x_offset = end_x;
        }
    }

    // Use pulsed border color for active pane (#18)
    let border_color = if show_cursor { app.pulsed_border() } else { pal.border_dim };
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(pal.bg));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
