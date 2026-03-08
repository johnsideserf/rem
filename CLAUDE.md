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

- **`main.rs`** — Terminal setup/teardown (crossterm alternate screen + raw mode), event loop (`run_loop`), and all key event handlers (`handle_key` dispatches by `Mode`). Bookmark load/save wraps the loop.
- **`app.rs`** — `App` struct (all state), `Mode` enum (Normal, FuzzySearch, JumpKey, WaitingForG, WaitingForMark, WaitingForJumpToMark), `FsEntry`, navigation methods, fuzzy filtering via `fuzzy-matcher` SkimMatcherV2. Contains `JUMP_KEYS` (homerow-first letter ordering) and helper functions `format_size`, `file_type_badge`.
- **`palette.rs`** — `Palette` struct with 9 named RGB colors. Three constructors: `phosphor_green()`, `amber()`, `degraded_cyan()`. Passed by copy into all render functions.
- **`marks.rs`** — Bookmark persistence to `~/.config/rem/marks.toml` via serde + toml.

### UI modules (`src/ui/`)

- **`mod.rs`** — Top-level `render()` that splits the frame into header (2 rows), breadcrumb (2 rows), body (flex), footer (1 row). Sidebar shows when terminal width >= 100.
- **`header.rs`** — Title bar with item count and system status.
- **`breadcrumb.rs`** — Path segments with left-truncation and blinking cursor.
- **`list.rs`** — File list with responsive column hiding (size at 90+, type at 80+), scrollbar rendering, fuzzy search overlay on last row, jump key overlay.
- **`sidebar.rs`** — Selection metadata and bookmarks panel.
- **`footer.rs`** — Context-sensitive key hints; error state overrides footer with auto-dismiss.

### Key data flow

1. `App::load_entries()` reads the filesystem and sorts (dirs first, then case-insensitive alpha).
2. `App::rebuild_filtered()` applies fuzzy matching to produce `filtered_indices` — an ordered subset of `entries`.
3. All UI rendering reads from `filtered_indices` to index into `entries`.
4. `App::viewport_height` is updated each frame by `ui::mod::render` based on actual terminal size.

## Design Reference

Two PRD documents define the spec (the original `DESIGN_PRD_terminal.md` is superseded):

- **`DESIGN_PRD_theme.md`** — Visual language: palettes, typography, sigils, borders, animation timing, throbber/spinner system, progress bars, boot sequence, confirmation dialogs, component styling.
- **`DESIGN_PRD_functional.md`** — Feature spec: modes, layout, navigation, file operations, bulk selection, dual-pane, preview, keybindings, config.

Key rules from the theme doc:
- All borders use `BorderType::Plain`, never Rounded or Double
- Colors always come from `&Palette`, never hardcoded in render logic
- Labels are UPPERCASE, terse, bureaucratic
- Sigils are ASCII/Unicode only (▣ dir, ◻ file, ▶ selected, ◆ marked), no emoji
- Text truncates with `…`, never wraps. Paths truncate on the left; names on the right.
- Tick rate 100ms, cursor blink toggles every 550ms
- Throbbers use per-palette character sets (braille for green, block elements for amber, glitchy sparse for cyan)
