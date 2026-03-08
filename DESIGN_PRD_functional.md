# DESIGN PRD — FUNCTIONAL SPECIFICATION
## Codename: `REM` (Remote Entry Module)

*Visual styling for all components is defined in `DESIGN_PRD_theme.md`. This document covers what the application does, not how it looks.*

---

## 1. Overview

A terminal file manager built with `ratatui` + `crossterm` in Rust. REM provides vim-style navigation, file operations (copy, move, rename, delete, create), dual-pane mode, fuzzy search, jump keys, bookmarks, and a file preview pane. It can also operate as a simple path emitter for shell integration.

---

## 2. Modes

REM is modal. The current mode determines how keystrokes are interpreted.

| Mode | Purpose |
|---|---|
| `Normal` | Default. Navigate, open, trigger other modes. |
| `FuzzySearch` | Typing filters the file list in real time. |
| `JumpKey` | Single-letter labels assigned to visible entries. Pressing a letter navigates immediately. |
| `Visual` | Bulk selection. Cursor movement toggles marks on entries. |
| `Rename` | Inline text editing to rename the selected entry. |
| `Create` | Inline text input for new file or directory name. |
| `Confirm` | Awaiting `y`/`n` for a destructive operation. |
| `WaitingForG` | First `g` pressed, awaiting second `g` for jump-to-top. |
| `WaitingForMark` | `m` pressed, awaiting letter to set bookmark. |
| `WaitingForJumpToMark` | `'` pressed, awaiting letter to jump to bookmark. |

---

## 3. Layout

### 3.1 Single-Pane Mode (Default)

```
┌─────────────────────────────────────────────────────┐
│  HEADER BAR                                         │  1 row
├─────────────────────────────────────────────────────┤
│  BREADCRUMB / PATH BAR                              │  1 row
├──────────────────────────────┬──────────────────────┤
│                              │                      │
│  FILE LIST                   │  RIGHT PANEL         │
│  (primary pane)              │  (info / preview /   │
│                              │   hidden)            │
│                              │                      │
├──────────────────────────────┴──────────────────────┤
│  [OPERATION STATUS BAR — if active]                 │  0-1 row
├─────────────────────────────────────────────────────┤
│  FOOTER / KEY HINTS                                 │  1 row
└─────────────────────────────────────────────────────┘
```

Right panel cycles through three states with `Tab`:
1. **Info** — selection metadata and bookmarks (current sidebar)
2. **Preview** — text file content preview
3. **Hidden** — file list takes full width

### 3.2 Dual-Pane Mode

Activated by `ctrl+w`. Both panes are independent file lists with their own path, cursor, and scroll state.

```
┌─────────────────────────────────────────────────────┐
│  HEADER BAR                                         │  1 row
├──────────────────────────┬──────────────────────────┤
│  LEFT PATH BAR           │  RIGHT PATH BAR          │  1 row
├──────────────────────────┼──────────────────────────┤
│                          │                          │
│  LEFT FILE LIST          │  RIGHT FILE LIST         │
│                          │                          │
│                          │                          │
├──────────────────────────┴──────────────────────────┤
│  [OPERATION STATUS BAR — if active]                 │  0-1 row
├─────────────────────────────────────────────────────┤
│  FOOTER / KEY HINTS                                 │  1 row
└─────────────────────────────────────────────────────┘
```

- `Tab` switches active pane
- `ctrl+w` toggles back to single-pane
- Each pane has its own breadcrumb displayed in its path bar row
- 50/50 width split

### 3.3 Column Proportions

```
Minimum terminal width:   80 cols
Recommended:              120+ cols

Single-pane file list (with right panel):  78% / 22%
Single-pane file list (hidden panel):      100%
Dual-pane:                                 50% / 50%

List row columns:
  indicator:   2 cols  (▶ or ◆ or space)
  jump key:    4 cols  ([a] + space, or blank)
  sigil:       2 cols
  name:        flex
  type badge:  5 cols  (right-aligned)
  size:        9 cols  (right-aligned)
```

### 3.4 Responsive Collapse

- Below 100 cols: hide right panel in single-pane (list takes 100%)
- Below 90 cols: hide size column
- Below 80 cols: hide size and type columns
- Name column never collapses below 20 chars
- Dual-pane requires minimum 100 cols. Below that, `ctrl+w` is a no-op.

---

## 4. Navigation

### 4.1 Basic Movement

| Key | Action |
|---|---|
| `j` / `↓` | Cursor down |
| `k` / `↑` | Cursor up |
| `l` / `→` / `Enter` | Enter directory, or select file |
| `h` / `←` / `-` | Go to parent directory |
| `gg` | Jump to top of list |
| `G` | Jump to bottom of list |
| `ctrl+u` | Scroll up half page |
| `ctrl+d` | Scroll down half page |

### 4.2 History

| Key | Action |
|---|---|
| `ctrl+o` | Navigate back in history |
| `ctrl+i` | Navigate forward in history |

Navigation history is a linear stack. Navigating to a new directory after going back truncates forward history.

### 4.3 Directory Ordering

Entries sorted: directories first, then files. Within each group, case-insensitive alphabetical. Hidden files (dot-prefixed) are included but could be toggled in a future version.

---

## 5. File Operations

### 5.1 Yank / Paste Model

File operations use a vim-style buffer. Yanked entries are held in an operation buffer with a mode tag (copy or cut).

| Key | Action |
|---|---|
| `yy` | Yank current entry (copy to buffer) |
| `dd` | Cut current entry (move to buffer) |
| `p` | Paste buffer contents into current directory |

In Visual mode, `y` and `d` operate on all marked entries.

**Buffer rules:**
- Buffer holds one operation at a time. A new yank/cut replaces the previous.
- Buffer persists across directory changes within the session.
- `dd` does not immediately delete — it stages for move. The entry is visually marked (see theme doc) but remains in place until `p`.
- Pasting a cut buffer moves the files. Pasting a copy buffer copies them.
- If destination has a name conflict, enter `Confirm` mode: `OVERWRITE filename? y/n`

### 5.2 Delete

| Key | Action |
|---|---|
| `D` (shift+d) | Delete current entry (or all marked entries in Visual mode) |

- Triggers `Confirm` mode immediately. See theme doc section 9 for dialog styling.
- Deletion is recursive for directories.
- On failure, show error in footer (auto-dismiss 3 seconds).

### 5.3 Rename

| Key | Action |
|---|---|
| `r` | Rename current entry |

- Enters `Rename` mode. The name column for the selected entry becomes an editable text field.
- Pre-filled with the current name, cursor at end.
- `Enter` confirms rename. `Esc` cancels.
- Standard text editing: `Backspace` deletes, typing inserts, `ctrl+u` clears line, `ctrl+w` deletes word.
- On name conflict: error in footer, rename not applied.

### 5.4 Create

| Key | Action |
|---|---|
| `o` | Create new file |
| `O` (shift+o) | Create new directory |

- Enters `Create` mode. A new row appears at the cursor position with an empty editable name field.
- Sigil shows `◻` for file, `▣` for directory.
- `Enter` confirms creation. `Esc` cancels.
- On failure (permissions, name conflict): error in footer.

### 5.5 Operation Status

Long-running operations (large file copy, recursive delete of deep trees) show a status bar above the footer with:
- Throbber animation (see theme doc section 6)
- Operation description
- Progress bar if size is known (see theme doc section 7)
- Transfer rate if applicable

Operations run on a background thread. The UI remains responsive. On completion, the status bar shows `✓` for 1 second then disappears. The affected pane auto-refreshes.

---

## 6. Bulk Selection (Visual Mode)

| Key | Action |
|---|---|
| `v` | Toggle mark on current entry and move cursor down |
| `V` | Enter Visual range mode — cursor movement marks everything in the range |
| `Esc` | Exit Visual mode (marks persist) |
| `u` | Clear all marks |

### 6.1 Behavior

- Marked entries show `◆` indicator (see theme doc section 10.3).
- In Normal mode after exiting Visual, marks persist until cleared with `u` or until a file operation consumes them.
- `yy`, `dd`, `D` in Normal mode: if marks exist, operate on all marked entries. If no marks, operate on cursor entry.
- Mark count shown in header: `MARKED:5`

---

## 7. Fuzzy Search

Activated by `/`. The file list filters in real time as the user types.

### 7.1 Behavior

- Fuzzy search row occupies the last visible list row.
- Uses `SkimMatcherV2` for scoring.
- Results sorted by match score (best first).
- Only searches current directory entries (not recursive).
- `Enter` confirms: exits fuzzy mode with cursor on top match.
- `Esc` cancels: restores full unfiltered list.
- `Backspace` deletes last character.
- `j`/`k` or `↑`/`↓` move cursor through filtered results while keeping the search open.

### 7.2 Display

- Non-matching entries are dimmed but remain visible (see theme doc).
- Matching characters within entry names are highlighted.
- Match count shown right-aligned in the fuzzy row.

---

## 8. Jump Keys

Activated by `Space`. Assigns single-letter labels to all visible entries for instant navigation.

### 8.1 Key Assignment Order

Homerow first: `a s d f g h j k l`, then remaining alphabet: `q w e r t y u i o p z x c v b n m`.

### 8.2 Behavior

- While active, non-jump-key content dims.
- Pressing a letter immediately navigates (enters directory or selects file).
- Any non-alpha key cancels the overlay.
- No `Enter` required.

---

## 9. Bookmarks

### 9.1 Set / Jump

| Key | Action |
|---|---|
| `m` + `a-z` | Set bookmark to current directory |
| `'` + `a-z` | Jump to bookmarked directory |
| `''` | Jump to last position before previous jump |

### 9.2 Persistence

Bookmarks saved to `~/.config/rem/marks.toml` on exit, loaded on startup. Format:

```toml
[marks]
a = "/home/user/projects"
b = "/var/log"
```

### 9.3 Sidebar Display

When the right panel is in Info mode, bookmarks are listed in the BOOKMARKS section (up to 8, sorted by key).

---

## 10. Preview Pane

When the right panel is in Preview mode, it shows a read-only preview of the file under the cursor.

### 10.1 Supported Content

| File type | Preview |
|---|---|
| Text files (< 1 MB) | Raw text, first N lines that fit the pane |
| Binary files | `BINARY — n BYTES` message |
| Directories | Item count and total size (with throbber while calculating) |
| Empty files | `EMPTY — 0 BYTES` |
| Unreadable files | `ACCESS DENIED` in warn color |

### 10.2 Behavior

- Preview updates as cursor moves.
- No syntax highlighting in v2 (plain text only).
- Preview is read-only — no editing.
- File read is capped at 1 MB. Larger files show `FILE EXCEEDS PREVIEW LIMIT`.
- Preview content scrolls: `ctrl+j`/`ctrl+k` scroll the preview pane while keeping the file list cursor stable.

---

## 11. Dual-Pane Operations

### 11.1 Activation

`ctrl+w` toggles dual-pane mode. Each pane is a fully independent file list with its own:
- Current directory and breadcrumb
- Cursor position and scroll offset
- Filtered indices (fuzzy search is per-pane)

### 11.2 Pane Switching

`Tab` switches the active pane. Only the active pane responds to navigation and file operation keys.

### 11.3 Cross-Pane Operations

When in dual-pane mode, paste (`p`) targets the **active** pane's current directory. The typical workflow:

1. Navigate to source in left pane
2. `yy` to yank (or `v` to mark multiple, then `y`)
3. `Tab` to switch to right pane
4. Navigate to destination
5. `p` to paste

The inactive pane auto-refreshes when a paste operation completes that targets its directory.

---

## 12. Header Bar

Single row showing:

```
 REM  ·  FILE SYSTEM  ·  ITEMS:1847  ·  MARKED:5  ·  [heartbeat]  SYS:NOMINAL
```

- `ITEMS` count reflects filtered count if fuzzy is active, total count otherwise
- `MARKED` only shown when marks exist (> 0)
- Heartbeat throbber always runs (see theme doc section 6.1)
- In dual-pane mode, items count reflects the active pane

---

## 13. Footer / Key Hints

Context-sensitive. Shows relevant keys for current mode.

| Mode | Hints |
|---|---|
| Normal | `hjkl move · enter open · / fuzzy · space jump · mx mark · 'x goto · v select · yy copy · dd cut · p paste · q quit` |
| Normal (marks exist) | Adds `D delete · u clear` |
| FuzzySearch | `type filter · j/k move · enter confirm · esc cancel` |
| JumpKey | `a-z jump to · esc cancel` |
| Visual | `j/k move+mark · v toggle · y yank · d cut · D delete · esc exit · u clear` |
| Rename | `type edit · enter confirm · esc cancel` |
| Create | `type name · enter create · esc cancel` |
| Confirm | `y confirm · n cancel` |
| Dual-pane normal | Adds `tab switch pane · ctrl+w close pane` |

Error state overrides the footer (see theme doc section 9).

---

## 14. Exit Behavior

**File selected** (`Enter` or `l` on a file in Normal mode):
- Print absolute path to stdout
- Exit code 0

**Quit** (`q` or `Esc` in Normal mode):
- Print nothing
- Exit code 1

Shell integration:

```bash
function r() {
  local result=$(rem)
  [ $? -eq 0 ] && [ -n "$result" ] && cd "$(dirname "$result")"
}
```

---

## 15. Boot Sequence

On startup, a brief boot animation plays (< 2 seconds). Skippable with any keypress. See theme doc section 8 for visual spec.

Functional steps:
1. Initialize terminal (alternate screen, raw mode)
2. Load bookmarks from `~/.config/rem/marks.toml`
3. Read starting directory
4. Play boot sequence
5. Enter main event loop

---

## 16. Configuration

Config file: `~/.config/rem/config.toml`

```toml
[appearance]
palette = "green"  # "green", "amber", "cyan"

[behavior]
show_hidden = true
default_panel = "info"  # "info", "preview", "hidden"
boot_sequence = true
```

CLI flags override config: `--amber`, `--cyan`, `--no-boot`.

---

## 17. Implementation Notes

### 17.1 App State Additions (vs v1)

```rust
pub struct App {
    // ... existing fields ...

    // Dual pane
    pub panes: [PaneState; 2],
    pub active_pane: usize,         // 0 = left, 1 = right
    pub dual_pane: bool,

    // File operations
    pub op_buffer: Option<OpBuffer>,
    pub marks: HashSet<usize>,      // indices into filtered_indices
    pub background_op: Option<BackgroundOp>,

    // Right panel
    pub right_panel: RightPanel,    // Info | Preview | Hidden
    pub preview_scroll: usize,
    pub preview_content: Option<PreviewContent>,

    // Throbbers
    pub heartbeat: Throbber,
    pub op_throbber: Option<Throbber>,
}

pub struct PaneState {
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub filtered_indices: Vec<usize>,
    pub fuzzy_query: String,
    pub nav_history: Vec<PathBuf>,
    pub nav_history_cursor: usize,
}

pub struct OpBuffer {
    pub paths: Vec<PathBuf>,
    pub op: OpType,  // Copy | Cut
}

pub enum RightPanel { Info, Preview, Hidden }
```

### 17.2 Recommended File Structure (v2)

```
src/
  main.rs          — terminal setup/teardown, boot sequence, event loop
  app.rs           — App state, PaneState, tick logic
  input.rs         — key event handling, mode dispatch (extracted from main.rs)
  nav.rs           — directory read, sort, navigation methods
  ops.rs           — file operations (copy, move, rename, delete, create)
  marks.rs         — bookmark load/save
  palette.rs       — Palette struct and variants
  throbber.rs      — Throbber struct, frame sets per palette
  config.rs        — config file parsing
  preview.rs       — file preview reading and caching
  ui/
    mod.rs         — top-level render, layout split
    header.rs      — header bar
    breadcrumb.rs  — path bar
    list.rs        — file list + scrollbar
    sidebar.rs     — info sidebar
    preview.rs     — preview pane rendering
    footer.rs      — key hints and error state
    fuzzy.rs       — fuzzy overlay row
    jumpkey.rs     — jump key overlay
    confirm.rs     — confirmation dialog
    status.rs      — operation status bar
    boot.rs        — boot sequence rendering
```

---

## 18. Out of Scope (v2)

- Syntax highlighting in preview
- Image rendering
- Plugin system
- Mouse support
- Git status indicators
- Tabs (multiple tab groups)
- Networked / remote filesystems
- File search across directory tree (recursive search)
- Archive inspection (zip/tar contents)
- File permissions editing (chmod)

---

*Document version: 2.0 — Functional specification for `rem`*
*Visual reference: DESIGN_PRD_theme.md*
