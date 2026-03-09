use std::path::PathBuf;
use std::time::Instant;

use std::sync::mpsc;

use crate::app::{App, ArchiveContext, EditorState, FsEntry, GitInfo, Mode, PaneState, SortMode};
use crate::app::{DiskScanMessage, DiskScanOp, DiskUsageData, DiskUsageEntry};
use crate::app::{HashMessage, HashOp};
use crate::throbber::{Throbber, ThrobberKind};

impl App {
    pub fn load_entries(&mut self) {
        // Don't load FS entries when browsing an archive (#19)
        if self.archive.is_some() {
            self.populate_archive_entries();
            return;
        }
        // Flash I/O indicator (#16)
        self.io_flash_tick = 3;
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
        self.git_info = GitInfo::detect(&dir);
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
        // Archive mode: go up within archive or exit (#19)
        if let Some(archive) = &self.archive {
            if archive.internal_dir.is_empty() {
                // At archive root — exit archive
                self.exit_archive();
            } else {
                // Go up one level within archive
                let dir = archive.internal_dir.trim_end_matches('/');
                let parent = match dir.rfind('/') {
                    Some(pos) => format!("{}/", &dir[..pos]),
                    None => String::new(),
                };
                if let Some(a) = &mut self.archive {
                    a.internal_dir = parent;
                }
                self.populate_archive_entries();
            }
            return;
        }

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
        // Archive mode navigation (#19)
        if self.archive.is_some() {
            let pane = self.pane();
            if let Some(&idx) = pane.filtered_indices.get(pane.cursor) {
                let entry = &pane.entries[idx];
                if entry.is_dir {
                    let new_dir = entry.path.to_string_lossy().into_owned();
                    if let Some(archive) = &mut self.archive {
                        archive.internal_dir = new_dir;
                    }
                    self.populate_archive_entries();
                }
                // Files inside archives are read-only — no action
            }
            return;
        }

        let pane = self.pane();
        if let Some(&idx) = pane.filtered_indices.get(pane.cursor) {
            let entry = &pane.entries[idx];
            if entry.is_dir {
                let path = entry.path.clone();
                self.navigate_to(path);
            } else if crate::archive::is_archive(&entry.path) {
                // Enter archive (#19)
                let path = entry.path.clone();
                self.enter_archive(path);
            } else {
                self.open_request = Some(crate::app::OpenRequest::SystemDefault(entry.path.clone()));
            }
        }
    }

    /// Enter an archive for browsing (#19).
    fn enter_archive(&mut self, path: PathBuf) {
        match crate::archive::read_zip(&path) {
            Ok(entries) => {
                self.archive = Some(ArchiveContext {
                    archive_path: path,
                    internal_dir: String::new(),
                    all_entries: entries,
                });
                self.populate_archive_entries();
            }
            Err(e) => {
                self.error = Some((e, Instant::now()));
            }
        }
    }

    /// Exit archive mode (#19).
    pub fn exit_archive(&mut self) {
        if let Some(archive) = self.archive.take() {
            // Navigate back to the directory containing the archive
            if let Some(parent) = archive.archive_path.parent() {
                self.pane_mut().current_dir = parent.to_path_buf();
            }
            self.load_entries();
        }
    }

    pub fn edit_selected(&mut self) {
        if let Some(entry) = self.current_entry() {
            if entry.is_dir {
                return;
            }
            let path = entry.path.clone();
            match EditorState::open(path) {
                Ok(state) => {
                    self.editor = Some(state);
                    self.mode = Mode::Edit;
                }
                Err(msg) => {
                    self.error = Some((msg, Instant::now()));
                }
            }
        }
    }

    /// Open current file in external $EDITOR.
    pub fn edit_external(&mut self) {
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

    /// Start SHA-256 hash computation on the selected file (#20).
    pub fn hash_selected(&mut self) {
        let entry = match self.current_entry() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };
        let path = entry.path.clone();

        if self.hash_op.is_some() {
            self.error = Some(("HASH ALREADY IN PROGRESS".to_string(), Instant::now()));
            return;
        }

        let (tx, rx) = mpsc::channel();
        let hash_path = path.clone();
        let variant = self.palette.variant;

        self.hash_op = Some(HashOp {
            path: path.clone(),
            progress: 0.0,
            throbber: Throbber::new(ThrobberKind::Processing, variant),
            receiver: rx,
        });

        std::thread::spawn(move || {
            use sha2::{Sha256, Digest};
            use std::io::Read;

            let file = match std::fs::File::open(&hash_path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(HashMessage::Error(format!("{}", e)));
                    return;
                }
            };
            let file_size = file.metadata().map(|m| m.len()).unwrap_or(1).max(1);
            let mut reader = std::io::BufReader::new(file);
            let mut hasher = Sha256::new();
            let mut buf = [0u8; 65536];
            let mut total_read = 0u64;

            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        let _ = tx.send(HashMessage::Error(format!("{}", e)));
                        return;
                    }
                };
                hasher.update(&buf[..n]);
                total_read += n as u64;
                let _ = tx.send(HashMessage::Progress(total_read as f64 / file_size as f64));
            }

            let result = hasher.finalize();
            let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
            let _ = tx.send(HashMessage::Complete(hex));
        });
    }

    /// Start recursive disk usage scan on a directory (#21).
    pub fn scan_disk_usage(&mut self) {
        let entry = match self.current_entry() {
            Some(e) if e.is_dir => e,
            _ => {
                // Scan current directory if cursor isn't on a dir
                let dir = self.pane().current_dir.clone();
                let dir_name = dir.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| dir.to_string_lossy().into_owned());
                self.start_disk_scan(dir, dir_name);
                return;
            }
        };
        let path = entry.path.clone();
        let dir_name = entry.name.clone();
        self.start_disk_scan(path, dir_name);
    }

    fn start_disk_scan(&mut self, path: PathBuf, dir_name: String) {
        if self.disk_scan.is_some() {
            self.error = Some(("SCAN ALREADY IN PROGRESS".to_string(), Instant::now()));
            return;
        }

        let (tx, rx) = mpsc::channel();
        let variant = self.palette.variant;

        self.disk_scan = Some(DiskScanOp {
            dir_name: dir_name.clone(),
            nodes: 0,
            throbber: Throbber::new(ThrobberKind::DataStream, variant),
            receiver: rx,
        });

        let scan_path = path.clone();
        std::thread::spawn(move || {
            let mut size_map: std::collections::HashMap<String, (u64, bool)> = std::collections::HashMap::new();
            let mut total_size = 0u64;
            let mut total_items = 0u64;

            fn walk(
                dir: &std::path::Path,
                size_map: &mut std::collections::HashMap<String, (u64, bool)>,
                total_size: &mut u64,
                total_items: &mut u64,
                base: &std::path::Path,
                tx: &mpsc::Sender<DiskScanMessage>,
            ) {
                let rd = match std::fs::read_dir(dir) {
                    Ok(rd) => rd,
                    Err(_) => return,
                };
                for entry in rd.flatten() {
                    let meta = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    *total_items += 1;
                    if *total_items % 500 == 0 {
                        let _ = tx.send(DiskScanMessage::Progress(*total_items));
                    }

                    let entry_path = entry.path();
                    // Get top-level child name relative to base
                    let rel = match entry_path.strip_prefix(base) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                    let top_name = rel.components().next()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .unwrap_or_default();

                    if meta.is_dir() {
                        size_map.entry(top_name.clone()).or_insert((0, true));
                        walk(&entry_path, size_map, total_size, total_items, base, tx);
                    } else {
                        let sz = meta.len();
                        *total_size += sz;
                        let e = size_map.entry(top_name).or_insert((0, false));
                        e.0 += sz;
                        // If we already set is_dir=true, keep it
                    }
                }
            }

            // First, get direct children to know which are dirs
            if let Ok(rd) = std::fs::read_dir(&scan_path) {
                for entry in rd.flatten() {
                    if let Ok(m) = entry.metadata() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        size_map.insert(name, (if m.is_dir() { 0 } else { m.len() }, m.is_dir()));
                    }
                }
            }

            walk(&scan_path, &mut size_map, &mut total_size, &mut total_items, &scan_path, &tx);

            let mut entries: Vec<DiskUsageEntry> = size_map.into_iter()
                .map(|(name, (size, is_dir))| DiskUsageEntry { name, size, is_dir })
                .collect();
            entries.sort_by(|a, b| b.size.cmp(&a.size));
            entries.truncate(50);

            let _ = tx.send(DiskScanMessage::Complete(DiskUsageData {
                path: scan_path,
                entries,
                total_size,
                total_items,
            }));
        });
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
