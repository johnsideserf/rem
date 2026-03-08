use std::path::PathBuf;
use std::time::Instant;

use crate::app::{App, FsEntry, PaneState};

impl App {
    pub fn load_entries(&mut self) {
        let pane = self.pane_mut();
        pane.entries.clear();
        match std::fs::read_dir(&pane.current_dir) {
            Ok(rd) => {
                for entry in rd.flatten() {
                    let meta = entry.metadata().ok();
                    let is_dir = meta.as_ref().map_or(false, |m| m.is_dir());
                    let size = meta.as_ref().and_then(|m| if !m.is_dir() { Some(m.len()) } else { None });
                    let modified = meta.as_ref().and_then(|m| m.modified().ok());
                    pane.entries.push(FsEntry {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: entry.path(),
                        is_dir,
                        size,
                        modified,
                    });
                }
                // Sort: dirs first, then alphabetical case-insensitive
                pane.entries.sort_by(|a, b| {
                    b.is_dir.cmp(&a.is_dir)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(e) => {
                self.error = Some((format!("CANNOT READ: {}", e), Instant::now()));
            }
        }
        self.rebuild_filtered();
    }

    pub fn rebuild_filtered(&mut self) {
        let show_hidden = self.show_hidden;
        let pane = self.pane_mut();
        pane.fuzzy_match_positions.clear();
        if pane.fuzzy_query.is_empty() {
            pane.filtered_indices = (0..pane.entries.len())
                .filter(|&i| show_hidden || !pane.entries[i].name.starts_with('.'))
                .collect();
        } else {
            use fuzzy_matcher::FuzzyMatcher;
            use fuzzy_matcher::skim::SkimMatcherV2;
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64, Vec<usize>)> = pane.entries.iter().enumerate()
                .filter(|&(_, e)| show_hidden || !e.name.starts_with('.'))
                .filter_map(|(i, e)| {
                    matcher.fuzzy_indices(&e.name, &pane.fuzzy_query)
                        .map(|(score, indices)| (i, score, indices))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            for &(idx, _, ref positions) in &scored {
                pane.fuzzy_match_positions.insert(idx, positions.clone());
            }
            pane.filtered_indices = scored.into_iter().map(|(i, _, _)| i).collect();
        }
    }

    pub fn navigate_to(&mut self, dir: PathBuf) {
        {
            let pane = self.pane_mut();
            pane.current_dir = dir.clone();
            pane.cursor = 0;
            pane.scroll_offset = 0;
            pane.fuzzy_query.clear();
        }
        self.load_entries();
        let pane = self.pane_mut();
        if pane.nav_history_cursor + 1 < pane.nav_history.len() {
            pane.nav_history.truncate(pane.nav_history_cursor + 1);
        }
        pane.nav_history.push(dir);
        pane.nav_history_cursor = pane.nav_history.len() - 1;
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.pane().current_dir.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(parent);
        }
    }

    pub fn nav_back(&mut self) {
        if self.pane().nav_history_cursor > 0 {
            {
                let pane = self.pane_mut();
                pane.nav_history_cursor -= 1;
                let dir = pane.nav_history[pane.nav_history_cursor].clone();
                pane.current_dir = dir;
                pane.cursor = 0;
                pane.scroll_offset = 0;
                pane.fuzzy_query.clear();
            }
            self.load_entries();
        }
    }

    pub fn nav_forward(&mut self) {
        if self.pane().nav_history_cursor + 1 < self.pane().nav_history.len() {
            {
                let pane = self.pane_mut();
                pane.nav_history_cursor += 1;
                let dir = pane.nav_history[pane.nav_history_cursor].clone();
                pane.current_dir = dir;
                pane.cursor = 0;
                pane.scroll_offset = 0;
                pane.fuzzy_query.clear();
            }
            self.load_entries();
        }
    }

    pub fn enter_selected(&mut self) {
        let pane = self.pane();
        if let Some(&idx) = pane.filtered_indices.get(pane.cursor) {
            let entry = &pane.entries[idx];
            if entry.is_dir {
                let path = entry.path.clone();
                self.navigate_to(path);
            } else {
                self.open_request = Some(crate::app::OpenRequest::SystemDefault(entry.path.clone()));
            }
        }
    }

    pub fn edit_selected(&mut self) {
        if let Some(entry) = self.current_entry() {
            if !entry.is_dir {
                self.open_request = Some(crate::app::OpenRequest::Editor(entry.path.clone()));
            }
        }
    }

    pub fn cursor_down(&mut self) {
        let pane = self.pane_mut();
        if pane.cursor + 1 < pane.filtered_indices.len() {
            pane.cursor += 1;
            ensure_visible(pane);
        }
    }

    pub fn cursor_up(&mut self) {
        let pane = self.pane_mut();
        if pane.cursor > 0 {
            pane.cursor -= 1;
            ensure_visible(pane);
        }
    }

    pub fn jump_top(&mut self) {
        let pane = self.pane_mut();
        pane.cursor = 0;
        pane.scroll_offset = 0;
    }

    pub fn jump_bottom(&mut self) {
        let pane = self.pane_mut();
        if !pane.filtered_indices.is_empty() {
            pane.cursor = pane.filtered_indices.len() - 1;
            ensure_visible(pane);
        }
    }

    pub fn scroll_half_up(&mut self) {
        let pane = self.pane_mut();
        let half = pane.viewport_height / 2;
        pane.cursor = pane.cursor.saturating_sub(half);
        ensure_visible(pane);
    }

    pub fn scroll_half_down(&mut self) {
        let pane = self.pane_mut();
        let half = pane.viewport_height / 2;
        pane.cursor = (pane.cursor + half).min(pane.filtered_indices.len().saturating_sub(1));
        ensure_visible(pane);
    }

    pub fn current_entry(&self) -> Option<&FsEntry> {
        let pane = self.pane();
        pane.filtered_indices.get(pane.cursor).map(|&i| &pane.entries[i])
    }

    pub fn set_mark(&mut self, c: char) {
        let dir = self.pane().current_dir.clone();
        self.marks.insert(c, dir);
        crate::marks::save_marks(&self.marks);
    }

    pub fn jump_to_mark(&mut self, c: char) {
        if c == '\'' {
            if let Some(dir) = self.last_dir_before_jump.clone() {
                let old = self.pane().current_dir.clone();
                self.navigate_to(dir);
                self.last_dir_before_jump = Some(old);
            }
        } else if let Some(dir) = self.marks.get(&c).cloned() {
            if !dir.exists() {
                self.marks.remove(&c);
                crate::marks::save_marks(&self.marks);
                self.error = Some((format!("MARK '{}' PATH NO LONGER EXISTS", c), Instant::now()));
                return;
            }
            self.last_dir_before_jump = Some(self.pane().current_dir.clone());
            self.navigate_to(dir);
        } else {
            self.error = Some((format!("MARK '{}' NOT SET", c), Instant::now()));
        }
    }
}

fn ensure_visible(pane: &mut PaneState) {
    if pane.cursor < pane.scroll_offset {
        pane.scroll_offset = pane.cursor;
    }
    if pane.cursor >= pane.scroll_offset + pane.viewport_height {
        pane.scroll_offset = pane.cursor - pane.viewport_height + 1;
    }
}
