use std::path::PathBuf;
use std::time::Instant;

use crate::app::{App, FsEntry, Mode, PaneState, SortMode};

impl App {
    pub fn load_entries(&mut self) {
        let sort_mode = self.sort_mode;
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
                // Sort: dirs first, then by current sort mode
                pane.entries.sort_by(|a, b| {
                    b.is_dir.cmp(&a.is_dir).then_with(|| match sort_mode {
                        SortMode::NameAsc => {
                            a.name.to_lowercase().cmp(&b.name.to_lowercase())
                        }
                        SortMode::NameDesc => {
                            b.name.to_lowercase().cmp(&a.name.to_lowercase())
                        }
                        SortMode::SizeDesc => {
                            b.size.unwrap_or(0).cmp(&a.size.unwrap_or(0))
                        }
                        SortMode::SizeAsc => {
                            a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0))
                        }
                        SortMode::DateNewest => {
                            b.modified.cmp(&a.modified)
                        }
                        SortMode::DateOldest => {
                            a.modified.cmp(&b.modified)
                        }
                    })
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
        if !self.reduce_motion {
            self.anim_frame = 1;
            self.anim_tick = Instant::now();
        }
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
    /// Walk the current directory tree and populate rsearch_paths.
    pub fn rsearch_walk(&mut self) {
        use ignore::WalkBuilder;

        let base = self.pane().current_dir.clone();
        self.rsearch_paths.clear();
        self.rsearch_results.clear();
        self.rsearch_query.clear();
        self.rsearch_cursor = 0;
        self.rsearch_scroll = 0;

        let walker = WalkBuilder::new(&base)
            .hidden(!self.show_hidden)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .max_depth(Some(12))
            .build();

        let mut count = 0usize;
        const MAX_ENTRIES: usize = 50_000;

        for entry in walker.into_iter().flatten() {
            if count >= MAX_ENTRIES {
                break;
            }
            let path = entry.into_path();
            if path == base {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(&base) {
                self.rsearch_paths.push(rel.to_path_buf());
                count += 1;
            }
        }

        // Show all results initially
        self.rsearch_results = (0..self.rsearch_paths.len())
            .map(|i| (i, 0))
            .collect();
    }

    /// Re-filter rsearch results based on current query.
    pub fn rsearch_filter(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        self.rsearch_cursor = 0;
        self.rsearch_scroll = 0;

        if self.rsearch_query.is_empty() {
            self.rsearch_results = (0..self.rsearch_paths.len())
                .map(|i| (i, 0))
                .collect();
            return;
        }

        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(usize, i64)> = self.rsearch_paths.iter().enumerate()
            .filter_map(|(i, p)| {
                let s = p.to_string_lossy();
                matcher.fuzzy_match(&s, &self.rsearch_query)
                    .map(|score| (i, score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        // Cap results for rendering performance
        scored.truncate(1000);
        self.rsearch_results = scored;
    }

    /// Navigate to the selected recursive search result.
    pub fn rsearch_confirm(&mut self) {
        if let Some(&(idx, _)) = self.rsearch_results.get(self.rsearch_cursor) {
            let rel = &self.rsearch_paths[idx];
            let full = self.pane().current_dir.join(rel);
            if full.is_dir() {
                self.navigate_to(full);
            } else if let Some(parent) = full.parent() {
                let file_name = full.file_name()
                    .map(|n| n.to_string_lossy().into_owned());
                self.navigate_to(parent.to_path_buf());
                // Try to place cursor on the file
                if let Some(name) = file_name {
                    let pane = self.pane_mut();
                    for (vi, &ei) in pane.filtered_indices.iter().enumerate() {
                        if pane.entries[ei].name == name {
                            pane.cursor = vi;
                            ensure_visible(pane);
                            break;
                        }
                    }
                }
            }
        }
        self.mode = Mode::Normal;
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
