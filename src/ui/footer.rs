use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Mode};

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
                    format!(" \u{26a0} DELETE 1 ITEM? THIS CANNOT BE UNDONE.")
                } else {
                    format!(" \u{26a0} DELETE {} ITEMS? THIS CANNOT BE UNDONE.", paths.len())
                }
            }
            crate::app::PendingAction::Overwrite { dest, .. } => {
                let name = dest.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!(" \u{26a0} OVERWRITE {}?", name)
            }
        };

        let spans = vec![
            Span::styled(msg, Style::default().fg(pal.warn).bg(pal.surface)),
            Span::styled("  ", Style::default().bg(pal.surface)),
            Span::styled("y", Style::default().fg(pal.text_mid).bg(pal.surface)),
            Span::styled(" confirm", Style::default().fg(pal.text_dim).bg(pal.surface)),
            Span::styled("  \u{00b7}  ", Style::default().fg(pal.border_mid).bg(pal.surface)),
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

    let has_marks = !app.visual_marks.is_empty();

    let hints: Vec<(&str, &str)> = match app.mode {
        Mode::Normal | Mode::WaitingForG | Mode::WaitingForMark
        | Mode::WaitingForJumpToMark | Mode::WaitingForYank | Mode::WaitingForCut
        | Mode::WaitingForDeleteMark => {
            let mut h = vec![
                ("hjkl", "move"),
                ("enter", "open"),
                ("e", "edit"),
                ("/", "fuzzy"),
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
            h.push(("tab", "panel"));
            h.push(("[]", "resize"));
            h.push(("t", "theme"));
            h.push(("`", "telem"));
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
        Mode::Confirm { .. } => unreachable!(), // handled above
    };

    let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default().bg(pal.surface))];
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  \u{00b7}  ",
                Style::default().fg(pal.border_mid).bg(pal.surface),
            ));
        }
        spans.push(Span::styled(
            *key,
            Style::default().fg(pal.text_mid).bg(pal.surface),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(pal.text_dim).bg(pal.surface),
        ));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
