# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**rem** (Remote Entry Module) is a terminal file manager TUI built in Rust with ratatui + crossterm. It provides file navigation, file operations (copy/move/rename/delete/create), dual-pane mode, fuzzy search, jump keys, bookmarks, and file preview. The aesthetic is sci-fi CRT phosphor terminal (Alien 1979 / Weyland-Yutani era).

## Build & Run

```bash
cargo build              # debug build
cargo run                # run with default phosphor green palette
cargo run -- --amber     # amber palette
cargo run -- --cyan      # degraded cyan palette
```

No tests exist yet. No linter configured beyond `cargo check`.

## Architecture

The binary crate (`rem`) uses Rust 2024 edition. Entry point is `src/main.rs`.

### Core modules

- **`main.rs`** — Terminal setup/teardown (crossterm alternate screen + raw mode), event loop (`run_loop`). Bookmark load/save wraps the loop.
- **`app.rs`** — `App` struct (all state), `Mode` enum (Normal, FuzzySearch, JumpKey, Visual, Rename, Create, Confirm, RecursiveSearch, BulkRename, Edit, and waiting states), `FsEntry`, `GitInfo`, `EditorState`. Contains `JUMP_KEYS`, helper functions `format_size`, `file_type_badge`, `icon_for`.
- **`input.rs`** — All key event handlers. `handle_key` dispatches by `Mode` to dedicated handlers (normal, visual, rename, create, confirm, fuzzy, edit, bulk rename, theme picker, etc.).
- **`nav.rs`** — Directory reading, sorting, navigation methods. Git info detection, animation triggers, editor open/close.
- **`palette.rs`** — `Palette` struct with 9 named RGB colors. Three constructors: `phosphor_green()`, `amber()`, `degraded_cyan()`. Passed by copy into all render functions.
- **`symbols.rs`** — `SymbolVariant` enum (7 variants) and `SymbolSet` struct defining all UI glyphs. Seven constructors: `standard()`, `ascii()`, `block()`, `minimal()`, `pipeline()`, `braille()`, `scanline()`.
- **`config.rs`** — Parse/save `~/.config/rem/config.toml` (palette, symbols, show_hidden, default_panel, boot_sequence).
- **`marks.rs`** — Bookmark persistence to `~/.config/rem/marks.toml` via serde + toml.
- **`ops.rs`** — `OpBuffer` struct, file copy/move/delete implementations, background operations.
- **`throbber.rs`** — `Throbber` struct with frame arrays for Data Stream, Processing, Heartbeat. `from_frames()` for symbol-set-driven animations.
- **`highlight.rs`** — Syntax highlighting for preview pane and editor, language detection by extension.
- **`sysmon.rs`** — System telemetry (CPU, RAM, disk, network) via `sysinfo` crate.

### UI modules (`src/ui/`)

- **`mod.rs`** — Top-level `render()` that splits the frame into header (2 rows), breadcrumb (2 rows), body (flex), telemetry (conditional), status bar (conditional), footer (1 row). Manages single-pane vs dual-pane layout. Bulk rename popup overlay.
- **`header.rs`** — Title bar with item count, git branch/dirty status, system heartbeat.
- **`breadcrumb.rs`** — Path segments with left-truncation and blinking cursor.
- **`list.rs`** — File list with responsive column hiding, scrollbar, fuzzy/recursive search overlays, jump key overlay, animated transitions (color fade + slide).
- **`sidebar.rs`** — Selection metadata and bookmarks panel.
- **`preview.rs`** — Syntax-highlighted file preview with scroll support.
- **`editor.rs`** — In-app text editor with gutter, syntax highlighting, cursor blink, unsaved changes confirmation.
- **`statusbar.rs`** — Operation progress bar (throbber + label + path + determinate bar) and feedback messages.
- **`telemetry.rs`** — CPU/RAM/disk/network gauges with symbol-set-aware bars.
- **`theme_picker.rs`** — Popup for selecting color profiles and symbol sets.
- **`footer.rs`** — Context-sensitive key hints per mode; warning display.
- **`boot.rs`** — Boot sequence animation.

### Key data flow

1. `App::load_entries()` reads the filesystem and sorts (dirs first, then case-insensitive alpha).
2. `App::rebuild_filtered()` applies fuzzy matching to produce `filtered_indices` — an ordered subset of `entries`.
3. All UI rendering reads from `filtered_indices` to index into `entries`.
4. `App::viewport_height` is updated each frame by `ui::mod::render` based on actual terminal size.

## Design Reference

**`DESIGN_PRD_theme.md`** is the visual style guide: palettes, typography, sigils, borders, animation timing, throbber/spinner system, progress bars, boot sequence, confirmation dialogs, component styling.

Key rules from the theme doc:
- All borders use `BorderType::Plain`, never Rounded or Double
- Colors always come from `&Palette`, never hardcoded in render logic
- Labels are UPPERCASE, terse, bureaucratic
- Sigils come from the active `SymbolSet` (cursor, mark, cut, copy, checkmark, warning, etc.), no emoji
- Text truncates with `…`, never wraps. Paths truncate on the left; names on the right.
- Tick rate 100ms, cursor blink toggles every 550ms
- Throbbers/heartbeat frames are driven by the active symbol set
