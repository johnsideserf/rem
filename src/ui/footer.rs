use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Mode};

/// Compute how many rows the footer needs at the given width.
pub fn required_height(app: &App, width: u16) -> u16 {
    if app.error.is_some() || matches!(app.mode, Mode::Confirm { .. }) {
        return 1;
    }
    if matches!(app.mode, Mode::Command) {
        return 1;
    }
    let mut h = 0u16;
    // Disk warning row (#34)
    if app.disk_warning.is_some() {
        h += 1;
    }
    let hints = collect_hints(app);
    let sep_width = separator_width(app);
    h += lines_needed(&hints, sep_width, width as usize).max(1) as u16;
    h
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let pal = app.palette;

    // Error state overrides footer
    if let Some((msg, _)) = &app.error {
        let error_line = Line::from(vec![
            Span::styled(
                format!(" \u{26a0} {}", msg),
                Style::default().fg(pal.warn).bg(pal.surface),
            ),
        ]);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.border_dim))
            .style(Style::default().bg(pal.surface));
        let paragraph = Paragraph::new(error_line).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // Confirm mode replaces footer with warn-styled dialog
    if let Mode::Confirm { action } = &app.mode {
        let msg = match action {
            crate::app::PendingAction::Delete { paths } => {
                if paths.len() == 1 {
                    format!(" {} DELETE 1 ITEM? THIS CANNOT BE UNDONE.", app.symbols.warning)
                } else {
                    format!(" {} DELETE {} ITEMS? THIS CANNOT BE UNDONE.", app.symbols.warning, paths.len())
                }
            }
            crate::app::PendingAction::Overwrite { dest, .. } => {
                let name = dest.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!(" {} OVERWRITE {}?", app.symbols.warning, name)
            }
        };

        let spans = vec![
            Span::styled(msg, Style::default().fg(pal.warn).bg(pal.surface)),
            Span::styled("  ", Style::default().bg(pal.surface)),
            Span::styled("y", Style::default().fg(pal.text_mid).bg(pal.surface)),
            Span::styled(" confirm", Style::default().fg(pal.text_dim).bg(pal.surface)),
            Span::styled(format!("  {}  ", app.symbols.separator), Style::default().fg(pal.border_mid).bg(pal.surface)),
            Span::styled("n", Style::default().fg(pal.text_mid).bg(pal.surface)),
            Span::styled(" cancel", Style::default().fg(pal.text_dim).bg(pal.surface)),
        ];

        let block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.warn))
            .style(Style::default().bg(pal.surface));
        let paragraph = Paragraph::new(Line::from(spans)).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // Command mode (#41)
    if matches!(app.mode, Mode::Command) {
        let cursor_char = if app.blink_on { app.symbols.text_cursor } else { " " };
        let mut cmd_spans = vec![
            Span::styled(" MOTHER> ", Style::default().fg(pal.text_hot).bg(pal.surface)),
            Span::styled(app.command_state.input.clone(), Style::default().fg(pal.text_mid).bg(pal.surface)),
        ];
        // Completion hint (#49)
        if let Some(idx) = app.command_state.completion_idx {
            if let Some(completion) = app.command_state.completions.get(idx) {
                if completion.len() > app.command_state.input.len() {
                    let hint = &completion[app.command_state.input.len()..];
                    cmd_spans.push(Span::styled(
                        hint.to_string(),
                        Style::default().fg(pal.text_dim).bg(pal.surface),
                    ));
                }
            }
        }
        cmd_spans.push(Span::styled(cursor_char, Style::default().fg(pal.text_hot).bg(pal.surface)));
        let cmd_line = Line::from(cmd_spans);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(pal.border_hot))
            .style(Style::default().bg(pal.surface));
        let paragraph = Paragraph::new(cmd_line).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let hints = collect_hints(app);
    let sep = format!("  {}  ", app.symbols.separator);
    let sep_width = sep.chars().count();
    let width = area.width as usize;

    // Build wrapped lines
    let mut lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut current_width: usize = 0;
    let mut is_first_on_line = true;

    // Archive mode prefix
    let prefix = if app.archive.is_some() && matches!(app.mode, Mode::Normal) {
        Some(" ARCHIVE MODE \u{2014} READ ONLY  ")
    } else {
        None
    };

    if let Some(pfx) = prefix {
        let pfx_width = pfx.chars().count();
        current_spans.push(Span::styled(pfx, Style::default().fg(pal.warn).bg(pal.surface)));
        current_width += pfx_width;
    } else {
        current_spans.push(Span::styled(" ", Style::default().bg(pal.surface)));
        current_width += 1;
    }

    for (key, desc) in hints.iter() {
        let hint_text_width = key.chars().count() + 1 + desc.chars().count(); // "key desc"
        let segment_width = if is_first_on_line {
            hint_text_width
        } else {
            sep_width + hint_text_width
        };

        // Wrap if this segment would exceed the line width
        if !is_first_on_line && current_width + segment_width > width {
            lines.push(Line::from(current_spans));
            current_spans = vec![Span::styled(" ", Style::default().bg(pal.surface))];
            current_width = 1;
            is_first_on_line = true;
        }

        if !is_first_on_line {
            current_spans.push(Span::styled(
                sep.clone(),
                Style::default().fg(pal.border_mid).bg(pal.surface),
            ));
            current_width += sep_width;
        }

        current_spans.push(Span::styled(
            *key,
            Style::default().fg(pal.text_mid).bg(pal.surface),
        ));
        current_spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(pal.text_dim).bg(pal.surface),
        ));
        current_width += hint_text_width;
        is_first_on_line = false;
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    // Disk warning (#34) — flashing line
    if let Some(warn) = &app.disk_warning {
        if app.blink_on {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} {} {}", app.symbols.warning, warn, app.symbols.warning),
                    Style::default().fg(pal.warn).bg(pal.surface),
                ),
            ]));
        } else {
            lines.push(Line::from(Span::styled("", Style::default().bg(pal.surface))));
        }
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Collect the hint pairs for the current mode.
fn collect_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    let has_marks = !app.visual_marks.is_empty();

    // Archive mode (#19)
    if app.archive.is_some() && matches!(app.mode, Mode::Normal) {
        return vec![
            ("hjkl", "move"),
            ("enter", "open"),
            ("h", "back"),
            ("/", "fuzzy"),
            ("q", "quit"),
        ];
    }

    match app.mode {
        Mode::Normal | Mode::WaitingForG | Mode::WaitingForMark
        | Mode::WaitingForJumpToMark | Mode::WaitingForYank | Mode::WaitingForCut
        | Mode::WaitingForDeleteMark => {
            let mut h = vec![
                ("hjkl", "move"),
                ("enter", "open"),
                ("e", "edit"),
                ("/", "fuzzy"),
                ("?", "search"),
                ("v", "select"),
                ("yy", "copy"),
                ("dd", "cut"),
                ("p", "paste"),
            ];
            if has_marks {
                h.push(("D", "delete"));
                h.push(("u", "clear"));
            }
            h.push(("r", "rename"));
            h.push(("o", "new file"));
            h.push(("#", "hash"));
            h.push(("W", "usage"));
            h.push(("s", "sort"));
            h.push(("tab", "panel"));
            h.push(("[/]", "sidebar"));
            h.push(("Y", "yank path"));
            h.push(("^L", "ops log"));
            h.push((":", "command"));
            h.push(("^F", "fav"));
            h.push(("L", "lock"));
            h.push(("t", "theme"));
            h.push(("q", "quit"));
            h
        }
        Mode::FuzzySearch => {
            vec![
                ("type", "filter"),
                ("\u{2191}\u{2193}", "move"),
                ("enter", "confirm"),
                ("esc", "cancel"),
            ]
        }
        Mode::JumpKey => {
            vec![
                ("a-z", "jump to"),
                ("esc", "cancel"),
            ]
        }
        Mode::Visual => {
            vec![
                ("j/k", "move+mark"),
                ("v", "toggle"),
                ("y", "yank"),
                ("d", "cut"),
                ("D", "delete"),
                ("R", "bulk rename"),
                ("u", "clear"),
                ("esc", "exit"),
            ]
        }
        Mode::Rename => {
            vec![
                ("type", "edit"),
                ("enter", "confirm"),
                ("esc", "cancel"),
            ]
        }
        Mode::Create { .. } => {
            vec![
                ("type", "name"),
                ("enter", "create"),
                ("esc", "cancel"),
            ]
        }
        Mode::RecursiveSearch => {
            vec![
                ("type", "filter"),
                ("\u{2191}\u{2193}", "move"),
                ("enter", "go to"),
                ("esc", "cancel"),
            ]
        }
        Mode::BulkRename => {
            vec![
                ("tab", "switch field"),
                ("type", "edit"),
                ("enter", "apply"),
                ("esc", "cancel"),
            ]
        }
        Mode::Edit => {
            vec![
                ("arrows", "move"),
                ("type", "insert"),
                ("^S", "save"),
                ("^Z", "undo"),
                ("^D", "del line"),
                ("^K", "kill EOL"),
                ("esc", "exit"),
            ]
        }
        Mode::OpsLog => {
            vec![
                ("j/k", "scroll"),
                ("g/G", "top/bottom"),
                ("esc", "close"),
            ]
        }
        Mode::Command => {
            vec![] // rendered separately
        }
        Mode::Confirm { .. } => unreachable!(),
    }
}

/// Get the char-width of the separator string.
fn separator_width(app: &App) -> usize {
    // "  {sep}  " = 2 + sep.chars().count() + 2
    2 + app.symbols.separator.chars().count() + 2
}

/// Calculate how many lines the hints need at the given terminal width.
fn lines_needed(hints: &[(&str, &str)], sep_width: usize, width: usize) -> usize {
    if hints.is_empty() {
        return 1;
    }
    let mut lines = 1usize;
    let mut x = 1usize; // leading space
    for (i, (key, desc)) in hints.iter().enumerate() {
        let hint_w = key.chars().count() + 1 + desc.chars().count();
        let segment_w = if i == 0 { hint_w } else { sep_width + hint_w };
        if i > 0 && x + segment_w > width {
            lines += 1;
            x = 1 + hint_w;
        } else {
            x += segment_w;
        }
    }
    lines
}
