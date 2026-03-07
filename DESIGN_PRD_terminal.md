# DESIGN PRD — FILE NAVIGATOR TUI
## Codename: `REM` (Remote Entry Module)

---

## 1. Overview

A minimal, opinionated file navigator built with `ratatui` + `crossterm` in Rust. The aesthetic is grounded in the visual language of late-70s / early-80s science fiction — specifically the industrial, gritty hypercapitalist future of *Alien* (1979) and its sequels. Think phosphor CRTs, degraded signal, corporate bureaucracy, mining operations. The tool must feel like it was built by a corporation that doesn't care about you, running on hardware that's seen better decades.

This document defines the visual language, layout system, color palette, typography conventions, interaction feedback, and component rules — everything a coding agent needs to implement the UI faithfully in `ratatui`.

---

## 2. Design Philosophy

### 2.1 Core Principles

**Phosphor first.** Every element should read as light emitted from a phosphor-coated screen. Colors are not flat — they have glow, falloff, and a sense of self-illumination. Implement this through layered brightness: a dim base, a mid tone, and a hot highlight for selected/active states.

**Scanlines are structural.** The feeling of scanlines isn't just decorative — it informs density decisions. Text should never feel cramped. Rows have breathing room because phosphor CRTs needed it. In practice: one blank line between major sections, consistent vertical rhythm.

**Corporate ugly.** The UI is not designed by an artist. It was spec'd by a mining corporation in 2122. Labels are terse, uppercase, bureaucratic. Errors are clinical. There is no "friendly" language. No icons beyond simple ASCII glyphs. No rounded corners. No softness.

**Degradation is honest.** The terminal has been used. Signal degrades. Things flicker. Status indicators exist because things fail. The design should accommodate and even celebrate imperfection — a `SIGNAL: WEAK` badge is a feature, not a bug.

**Output is the product.** The navigator exists to emit a path and exit. Every UI decision should reduce friction toward that goal. Nothing decorative that slows navigation.

### 2.2 Anti-patterns to Avoid

- No color gradients (terminal limitation, but also philosophically wrong)
- No rounded border styles — use only `Plain` borders in ratatui
- No friendly microcopy ("Oops!", "Nothing here yet", etc.)
- No icons or emoji — ASCII box-drawing and sigils only
- No centered layouts — everything is left-aligned or grid-aligned
- No pastel or desaturated colors — phosphor colors are vivid against dark

---

## 3. Color System

Three palette variants. Each represents a different "unit type" from the aesthetic world. The implementation should default to one (recommend Phosphor Green) and allow switching via config.

### 3.1 Palette A — Phosphor Green (Default)
*The mainframe. Institutional. Cold. The computer that knows things it won't tell you.*

```
BACKGROUND        #030303   — near black, not pure
SURFACE           #020c02   — very dark green-tinted black
BORDER_DIM        #001a08   — barely visible
BORDER_MID        #003d10   — structural separators
BORDER_HOT        #007a22   — active / focused

TEXT_DIM          #005218   — readable but recessed
TEXT_MID          #00a828   — default body text
TEXT_HOT          #00ff41   — selected, active, cursor

WARN              #ff4444   — errors and warnings only
```

**ratatui Color mapping:**
```rust
const BG:           Color = Color::Rgb(3, 3, 3);
const SURFACE:      Color = Color::Rgb(2, 12, 2);
const TEXT_DIM:     Color = Color::Rgb(0, 82, 24);
const TEXT_MID:     Color = Color::Rgb(0, 168, 40);
const TEXT_HOT:     Color = Color::Rgb(0, 255, 65);
const BORDER_DIM:   Color = Color::Rgb(0, 26, 8);
const BORDER_MID:   Color = Color::Rgb(0, 61, 16);
const BORDER_HOT:   Color = Color::Rgb(0, 122, 34);
const WARN:         Color = Color::Rgb(255, 68, 68);
```

### 3.2 Palette B — Amber Corporate
*The executive terminal. Weyland-Yutani ops. Every access is logged.*

```rust
const BG:           Color = Color::Rgb(12, 8, 0);
const SURFACE:      Color = Color::Rgb(17, 10, 0);
const TEXT_DIM:     Color = Color::Rgb(90, 58, 0);
const TEXT_MID:     Color = Color::Rgb(196, 122, 0);
const TEXT_HOT:     Color = Color::Rgb(255, 176, 0);
const BORDER_DIM:   Color = Color::Rgb(58, 40, 0);
const BORDER_MID:   Color = Color::Rgb(107, 74, 0);
const BORDER_HOT:   Color = Color::Rgb(128, 88, 0);
const WARN:         Color = Color::Rgb(255, 68, 68);
```

### 3.3 Palette C — Degraded Cyan
*The field unit. Dropped one too many times. Still works. Barely.*

```rust
const BG:           Color = Color::Rgb(1, 10, 13);
const SURFACE:      Color = Color::Rgb(1, 13, 16);
const TEXT_DIM:     Color = Color::Rgb(0, 96, 112);
const TEXT_MID:     Color = Color::Rgb(0, 149, 168);
const TEXT_HOT:     Color = Color::Rgb(0, 229, 255);
const BORDER_DIM:   Color = Color::Rgb(0, 21, 32);
const BORDER_MID:   Color = Color::Rgb(0, 48, 64);
const BORDER_HOT:   Color = Color::Rgb(0, 96, 122);
const WARN:         Color = Color::Rgb(255, 68, 68);
```

---

## 4. Typography & Text Conventions

Terminal fonts are fixed. There is no font choice in a TUI. The design system instead governs **character choices, casing, spacing, and density.**

### 4.1 Case Rules

| Context | Case |
|---|---|
| Section headers, labels | `UPPERCASE` |
| File and directory names | Preserve exact case from filesystem |
| Status values | `UPPERCASE` |
| Key hint labels | lowercase (e.g. `hjkl move`) |
| Path display | Preserve exact case |
| Error messages | `UPPERCASE. TERSE.` |

### 4.2 Sigil System

Use consistent ASCII/Unicode sigils for entity types. No emoji.

```
▣   Directory (filled square)
◻   File (empty square)
▸   Collapsed tree node
▾   Expanded tree node
▶   Selected row indicator (prepended to row)
─   Horizontal rule / separator
│   Vertical separator
·   Dot separator in status bars
⚠   Warning indicator
█   Cursor / blink block
▋   Partial cursor
```

All sigils must be single-width characters. Verify Unicode width before use — double-width characters break column alignment.

### 4.3 Spacing Rhythm

```
Header bar height:        1 row
Section label padding:    0 left-pad, 1 row gap below
Row height:               1 row (no padding rows between entries)
Panel internal padding:   1 col left, 1 col right
Major section separation: 1 border line (no blank rows)
Footer bar height:        1 row
```

### 4.4 Truncation

All text that may overflow a column must truncate with `…` (U+2026) or `..` as fallback. Never wrap. Truncate file names on the right. Truncate paths on the left — show the tail, not the head.

---

## 5. Layout System

### 5.1 Primary Layout

```
┌─────────────────────────────────────────────────────┐
│  HEADER BAR                                         │  1 row
├─────────────────────────────────────────────────────┤
│  BREADCRUMB / PATH BAR                              │  1 row
├──────────────────────────────┬──────────────────────┤
│                              │                      │
│  FILE LIST                   │  INFO SIDEBAR        │
│  (primary pane)              │  ~22% width          │
│                              │  (optional)          │
│                              │                      │
├──────────────────────────────┴──────────────────────┤
│  FOOTER / KEY HINTS                                 │  1 row
└─────────────────────────────────────────────────────┘
```

### 5.2 Column Proportions

```
Minimum terminal width:   80 cols
Recommended:              120+ cols

File list (no sidebar):   100% width
File list (with sidebar): 78% width
Sidebar:                  22% width

List row columns:
  jump key:    4 cols  (fixed, includes brackets)
  sigil:       2 cols  (fixed)
  name:        flex    (takes remaining space)
  type badge:  5 cols  (fixed, right-aligned)
  size:        9 cols  (fixed, right-aligned)
```

### 5.3 Responsive Collapse

- Below 100 cols: hide the sidebar entirely
- Below 90 cols: hide the size column
- Below 80 cols: hide size and type columns
- Never let the name column collapse below 20 chars

---

## 6. Components

### 6.1 Header Bar

Single row. Left: unit/session identifier. Right: system status fields.

```
 USCSS NOSTROMO  ·  FILE SYSTEM              DISK:74%  ·  ITEMS:1847  ·  SYS:NOMINAL
```

**Rules:**
- Background: `SURFACE`
- Text: `TEXT_MID`
- Identifier (leftmost label): `TEXT_HOT`, Bold
- Separator ` · `: `TEXT_DIM`
- Status values: `TEXT_HOT`
- Warning values (e.g. `SIGNAL: WEAK`): `WARN`, blinking modifier
- Bottom border: `BORDER_MID`, single line

### 6.2 Breadcrumb / Path Bar

Single row below header. Shows current path as a sequence of slash-separated segments.

```
 /SYSSTOR / MISSION / LV426 / ▋
```

**Rules:**
- Current (final) segment: `TEXT_HOT`
- Parent segments: `TEXT_MID`
- Separators ` / `: `TEXT_DIM`
- Left-pad 1 col
- Trailing cursor `▋` blinks (see Section 8)
- Bottom border: `BORDER_DIM`, single line

### 6.3 File List

The primary pane. Scrollable list of directory entries.

**Row anatomy:**
```
 [a]  ▣  raw-ore/                               DIR      —
 [s]  ▣  refined/                               DIR      —
 [d]  ◻  batch_log.txt                          TXT   22 KB
```

**Row states:**

| State | Background | Text | Left indicator |
|---|---|---|---|
| Default | `BG` | `TEXT_DIM` | none |
| Hovered (cursor on row) | `SURFACE` | `TEXT_MID` | none |
| Selected | `SURFACE` | `TEXT_HOT` | `▶ ` prepended |
| Dimmed (filtered out by fuzzy) | `BG` | `BORDER_DIM` | none |

**Jump key column:**
- 4 chars: `[a] `, `[s] `, or `    ` when no key assigned
- Color: `TEXT_HOT`, Bold
- Background of badge: slightly elevated — one step brighter than `BG`
- Keys assigned homerow-first for ergonomics: `a s d f g h j k l`, then remaining alphabet

**Directory entries:** type badge `DIR`, size `—`

**Scrollbar:** If list exceeds viewport, render a 1-col scrollbar on the far right using `█` for thumb and `│` for track, in `BORDER_DIM`. Thumb size proportional to viewport/total ratio.

### 6.4 Info Sidebar

Right panel showing metadata for the highlighted entry, bookmarks, and disk usage. Separated from file list by a `BORDER_DIM` vertical line.

**Content sections:**

```
 SELECTION
 NAME    raw-ore/
 TYPE    DIR
 ITEMS   34
 OWNER   ops
 PERMS   rwxr--
 MOD     03.14

 BOOKMARKS
 'a  /ore-proc/
 'b  /manifests/
 'c  /quarantine/

 DISK
 [███████████░░░]
 USED   73%
 FREE   124 GB
```

**Rules:**
- Section labels: `UPPERCASE`, `TEXT_DIM`, simulate letter-spacing with spaces (`D I S K`)
- Keys: `TEXT_DIM`
- Values: `TEXT_HOT`
- Disk bar: filled portion `TEXT_HOT` background, empty portion `BORDER_DIM` background, 1 row tall, full panel width
- Bookmark jump keys: `TEXT_HOT`
- Section separator: blank row between sections

### 6.5 Footer / Key Hints

Single row. Compact key hint display left-aligned.

```
 hjkl move  ·  a–z jump  ·  / fuzzy  ·  'x mark  ·  ctrl+o back  ·  q quit
```

**Rules:**
- Key glyphs (e.g. `hjkl`): `TEXT_MID`
- Descriptions (e.g. `move`): `TEXT_DIM`
- Separator ` · `: `BORDER_MID`
- Top border: `BORDER_DIM`, single line
- Background: `SURFACE`

### 6.6 Fuzzy Search Overlay

Activated by `/`. Renders as an **inline row** embedded at the bottom of the file list — not a modal, not a popup. As the user types, the list filters in real time.

```
 [/] ore▋                                          [12 matches]
```

**Rules:**
- The fuzzy row occupies the last visible list row while active
- Border on this row: `BORDER_HOT` (stands out from the list)
- Prompt `/`: `TEXT_HOT`
- Input text: `TEXT_HOT`
- Cursor `▋`: blinking
- Match count: right-aligned, `TEXT_DIM`
- Matching characters within entry names: `TEXT_HOT`, Bold
- Non-matching entries: `BORDER_DIM` (dimmed but visible)
- `ESC` cancels, restores full list
- `ENTER` confirms and navigates to first match

### 6.7 Jump Key Overlay

Activated by `Space` (or configurable leader). Assigns single-letter labels to all visible entries. Pressing the letter immediately navigates — no `ENTER` required.

**Rules:**
- Labels assigned homerow-first: `a s d f g h j k l`, then remaining alphabet
- While active, non-jump-key content dims to `TEXT_DIM`
- Jump key badges: `TEXT_HOT`, Bold, background `BORDER_MID`
- Any non-alpha key press cancels the overlay
- Navigation is immediate on keypress

---

## 7. Interaction Model

### 7.1 Keybindings

```
h / ←       Go to parent directory
l / →       Enter selected directory, or emit file path and exit
j / ↓       Cursor down
k / ↑       Cursor up
gg          Jump to top of list
G           Jump to bottom of list
ctrl+u      Scroll up half page
ctrl+d      Scroll down half page
ctrl+o      Jump back in navigation history
ctrl+i      Jump forward in navigation history
ENTER       Enter directory or emit file and exit
-           Alternate: go to parent
Space       Activate jump key overlay
/           Activate fuzzy search
ESC         Cancel current mode / quit
q           Quit without emitting path
```

### 7.2 Bookmark System

```
mx          Set bookmark x (any letter a–z)
'x          Jump to bookmark x
''          Jump to last position before the last jump
```

Bookmarks persist to `~/.config/rem/marks.toml`.

### 7.3 Exit Behavior

**File selected** (`ENTER` or `l` on a file):
- Print absolute path to stdout
- Exit code 0

**Quit** (`q` or `ESC`):
- Print nothing
- Exit code 1

This enables clean shell integration:

```bash
# Add to .bashrc / .zshrc
function r() {
  local result=$(rem)
  [ $? -eq 0 ] && [ -n "$result" ] && cd "$result"
}
```

---

## 8. Animation & Timing

### 8.1 Tick Rate

Drive the event loop at **100ms** ticks. This gives 10 redraws per second — enough for responsive blink without thrashing.

### 8.2 Cursor Blink

Toggle blink state every **550ms** (approximately 5–6 ticks). Apply to:
- Trailing `▋` in the breadcrumb bar
- Cursor `▋` in fuzzy search input

Blink is a simple boolean in app state, toggled by comparing `Instant::now()` against `last_blink`.

### 8.3 No Transition Animations

Every frame is a full redraw. No fade, no slide, no easing. The only temporal elements are cursor blink and periodic status refresh (disk usage etc., every 5 seconds).

---

## 9. Border Style Reference

All borders use `BorderType::Plain` in ratatui. Never use `Rounded` or `Double`.

| Context | Color |
|---|---|
| Panel separators (inactive) | `BORDER_DIM` |
| Panel separators (active/focused) | `BORDER_MID` |
| Fuzzy search row | `BORDER_HOT` |
| Section dividers within panels | inline `─` repeated, `BORDER_DIM` |

---

## 10. Error States

Errors replace the footer row. They are terse, uppercase, and auto-dismiss after 3 seconds.

```
 ⚠ PERMISSION DENIED: /root/secrets                        [ANY KEY TO DISMISS]
```

**Rules:**
- Background: `SURFACE`
- `⚠` glyph and full message: `WARN` color
- Auto-dismiss via `error_timestamp: Option<Instant>` in app state — compare elapsed on each tick
- No modal, no overlay — footer replacement only

---

## 11. Implementation Guide

### 11.1 Crate Dependencies

```toml
[dependencies]
ratatui    = "0.29"
crossterm  = "0.28"
fuzzy-matcher = "0.3"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
dirs       = "5"
```

### 11.2 App State

```rust
pub struct App {
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,       // sorted: dirs first, then files
    pub cursor: usize,               // index into entries
    pub scroll_offset: usize,
    pub mode: Mode,
    pub nav_history: Vec<PathBuf>,
    pub nav_history_cursor: usize,
    pub marks: HashMap<char, PathBuf>,
    pub fuzzy_query: String,
    pub filtered_indices: Vec<usize>,
    pub error: Option<(String, Instant)>,
    pub blink_on: bool,
    pub last_blink: Instant,
    pub palette: Palette,
}

pub enum Mode {
    Normal,
    FuzzySearch,
    JumpKey,
}

pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub permissions: Option<String>,
}
```

### 11.3 Palette Struct

```rust
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub text_dim: Color,
    pub text_mid: Color,
    pub text_hot: Color,
    pub border_dim: Color,
    pub border_mid: Color,
    pub border_hot: Color,
    pub warn: Color,
}

impl Palette {
    pub fn phosphor_green() -> Self { ... }
    pub fn amber() -> Self { ... }
    pub fn degraded_cyan() -> Self { ... }
}
```

Pass `&Palette` into every render function. Never hardcode colors in render logic.

### 11.4 Render Order Per Frame

1. Fill terminal background with `BG`
2. Render header bar (top, 1 row)
3. Render breadcrumb bar (below header, 1 row)
4. Render file list pane (fills remaining height minus 1 for footer)
5. Render info sidebar (right of list, if terminal width >= 100)
6. Render footer bar (bottom, 1 row)
7. If `mode == FuzzySearch`: draw fuzzy row over last list row
8. If `error.is_some()`: draw error state over footer

### 11.5 Event Loop Skeleton

```rust
loop {
    terminal.draw(|f| ui::render(f, &app))?;

    if crossterm::event::poll(Duration::from_millis(100))? {
        match crossterm::event::read()? {
            Event::Key(key) => {
                if let Some(path) = handle_key(&mut app, key)? {
                    // File selected — emit path and exit
                    println!("{}", path.display());
                    break;
                }
                if app.should_quit {
                    eprintln!(); // exit cleanly with no output
                    std::process::exit(1);
                }
            }
            Event::Resize(_, _) => {} // ratatui handles resize
            _ => {}
        }
    }

    app.tick(); // update blink, dismiss expired errors
}
```

---

## 12. Recommended File Structure

```
src/
  main.rs          — terminal setup/teardown, event loop
  app.rs           — App, Mode, FsEntry, tick logic
  nav.rs           — directory read, sort, frecency tracking
  marks.rs         — bookmark load/save (~/.config/rem/marks.toml)
  palette.rs       — Palette struct and variants
  config.rs        — config file (palette choice, keybinds)
  ui/
    mod.rs         — top-level render fn, layout split
    header.rs      — header bar
    breadcrumb.rs  — path bar
    list.rs        — file list + scrollbar
    sidebar.rs     — info sidebar
    footer.rs      — key hints and error state
    fuzzy.rs       — fuzzy overlay row
    jumpkey.rs     — jump key overlay logic
```

---

## 13. Out of Scope (v1)

The following are explicitly excluded:

- File operations (copy, move, rename, delete)
- Preview pane
- Image rendering
- Plugin system
- Mouse support
- Git status indicators
- Tabs or split panes
- Networked / remote filesystems

The tool does one thing: navigate a file tree ergonomically and emit a path.

---

*Document version: 1.0 — Design PRD for `rem`, implemented in Rust with ratatui + crossterm*
