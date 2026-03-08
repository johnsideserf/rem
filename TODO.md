# REM v2 Implementation Checklist

## Phase 0 — Already Implemented

- [x] Palette module — three variants with all 9 color tokens
- [x] CLI palette selection (`--amber`, `--cyan`)
- [x] App state core (`App` struct, `Mode` enum, `FsEntry`)
- [x] Directory reading and sorting (dirs first, case-insensitive alpha)
- [x] Basic navigation (j/k/h/l/arrows/Enter)
- [x] gg / G jump to top/bottom
- [x] Half-page scroll (ctrl+u / ctrl+d)
- [x] Navigation history (ctrl+o back, ctrl+i forward)
- [x] Fuzzy search mode (/ activate, type to filter, Enter/Esc)
- [x] Jump key mode (Space activate, homerow-first labels)
- [x] Bookmark set/jump (m + letter, ' + letter, '')
- [x] Bookmark persistence (~/.config/rem/marks.toml)
- [x] Cursor blink (550ms toggle)
- [x] Error auto-dismiss (3 seconds)
- [x] Exit behavior (path to stdout + exit 0, quit + exit 1)
- [x] Header bar UI
- [x] Breadcrumb bar UI with left-truncation
- [x] File list UI with responsive column hiding
- [x] Fuzzy search overlay on last list row
- [x] Jump key badges
- [x] Scrollbar (proportional thumb)
- [x] Info sidebar (selection metadata + bookmarks)
- [x] Footer with context-sensitive key hints
- [x] Top-level layout (header/breadcrumb/body/footer, sidebar collapse)

## Phase 1 — Structural Refactors

- [x] **1.1** Extract `input.rs` — move all `handle_*` key handlers out of `main.rs`
- [x] **1.2** Extract `nav.rs` — move directory reading, sorting, navigation methods out of `app.rs`
- [x] **1.3** Introduce `PaneState` struct — extract per-pane state into struct, `App` holds `panes: [PaneState; 2]` + `active_pane`
- [x] **1.4** Add `RightPanel` enum — Info / Preview / Hidden cycling with Tab
- [x] **1.5** Create `config.rs` — parse `~/.config/rem/config.toml` (palette, show_hidden, default_panel, boot_sequence)
- [x] **1.6** Create `throbber.rs` — `Throbber` struct with per-palette frame arrays for Data Stream, Processing, Heartbeat

## Phase 2 — New Modes

- [x] **2.1** Visual mode — `v` toggle mark + move down, `V` range select, Esc exits (marks persist), `u` clears all
- [x] **2.2** Rename mode — `r` enters inline edit, pre-filled name, Enter confirms `fs::rename`, Esc cancels
- [x] **2.3** Create mode — `o` new file, `O` new dir, empty editable name field, Enter confirms, Esc cancels
- [x] **2.4** Confirm mode — `y` confirms pending action, any other key cancels, auto-cancel 10s timeout
- [x] **2.5** Fuzzy search j/k navigation — move cursor through results while keeping search open
- [x] **2.6** Update footer hints — add hints for Visual, Rename, Create, Confirm modes; update Normal hints

## Phase 3 — File Operations

- [x] **3.1** Create `ops.rs` — `OpBuffer` struct, file copy/move/delete implementations
- [x] **3.2** Yank — `yy` copies current entry (or marked entries) into buffer
- [x] **3.3** Cut — `dd` stages current entry (or marked entries) for move
- [x] **3.4** Paste — `p` pastes buffer into current dir, copy or move per buffer type, conflict → Confirm
- [x] **3.5** Delete — `D` deletes entry/marked entries, triggers Confirm, recursive for dirs
- [x] **3.6** Background operations — large ops run on background thread, progress tracking, `✓`/`✗` feedback

## Phase 4 — Throbbers & Animation

- [x] **4.1** Heartbeat in header — always-running, palette-specific frames, advance every 3 ticks
- [x] **4.2** Operation throbbers — Data Stream (1 tick) for I/O, Processing (2 ticks) for compute
- [x] **4.3** Tick-driven advancement — extend `App::tick()` to advance all active throbbers

## Phase 5 — Preview Pane

- [x] **5.1** Create `preview.rs` data module — read text (<1MB), detect binary, cache `PreviewContent`
- [x] **5.2** Create `ui/preview.rs` — render preview in right panel, letter-spaced label, left border
- [x] **5.3** Preview scroll — ctrl+j / ctrl+k scroll preview while file list cursor stays stable
- [x] **5.4** Tab cycling — right panel cycles Info → Preview → Hidden, layout updates

## Phase 6 — Dual-Pane Mode

- [x] **6.1** Dual-pane toggle — ctrl+w toggles, minimum 100 cols
- [x] **6.2** Dual-pane layout — two side-by-side lists at 50/50, each with own breadcrumb
- [x] **6.3** Pane switching — Tab switches active pane (overrides right-panel cycle in dual mode)
- [x] **6.4** Cross-pane paste — paste targets active pane dir, inactive pane auto-refreshes
- [x] **6.5** Per-pane fuzzy search — independent filtered_indices and query per pane

## Phase 7 — UI Polish

- [x] **7.1** Bulk selection indicators — `◆` for marked, `✂`/`⊕` for yanked, dimmed non-matches
- [x] **7.2** MARKED count in header — show `MARKED:N` when marks exist, filtered count during fuzzy
- [x] **7.3** Confirmation dialog UI — inline footer replacement, `⚠` + warn styling
- [x] **7.4** Operation status bar — above footer, throbber + label + path + progress bar
- [x] **7.5** Progress bars — determinate (`███░░░ 54%`) and indeterminate (sliding bright region)
- [x] **7.6** Rename inline editing UI — name column becomes editable text field with blink cursor
- [x] **7.7** Create inline row UI — new row at cursor with sigil and empty editable field
- [x] **7.8** Fuzzy match character highlighting — per-character text_hot + Bold on matched chars

## Phase 8 — Boot Sequence

- [x] **8.1** Boot sequence rendering — sequential lines (150ms), throbber for disk scan, READY hold
- [x] **8.2** Boot sequence skip — any keypress skips, respects config + `--no-boot` flag

## Phase 9 — Configuration & CLI

- [x] **9.1** Config file loading — read `~/.config/rem/config.toml`, apply settings
- [x] **9.2** `--no-boot` CLI flag
- [x] **9.3** Hidden files toggle — respect `show_hidden` config, filter dot-prefixed entries

## Phase 10 — Extended Features

- [x] **10.1** Dead code cleanup — remove compiler warnings (#1)
- [x] **10.2** Persist theme choice — save/load palette selection in config (#2)
- [x] **10.3** File opener — launch `$EDITOR` or system default on Enter for files (#3)
- [x] **10.4** Persist bookmarks — save/load navigation marks to `~/.config/rem/marks.toml` (#4)
- [x] **10.5** Sort modes — toggle between name, size, and date with `s` key (#5)
- [x] **10.6** Recursive file search — `S` to search across all subdirectories (#6)
- [x] **10.7** Bulk rename — `R` in visual mode for find/replace pattern renaming (#7)
- [x] **10.8** Syntax-highlighted file preview — language-aware coloring in preview pane (#8)
- [x] **10.9** Nerd Font file icons — extension-based glyphs with fallback for non-nerd-font sets (#9)

## Phase 11 — Polish & Enhancement

- [x] **11.1** Smooth directory transition animations — color fade + horizontal slide on navigate (#10)
- [x] **11.2** In-app text editor — `e` to edit files with syntax highlighting, undo, and save (#11)
- [x] **11.3** Symbol set picker — 7 swappable glyph styles (Standard, ASCII, Block, Minimal, Pipeline, Braille, Scanline) (#12)
- [x] **11.4** Git branch display — show current branch and dirty status in header bar (#13)
