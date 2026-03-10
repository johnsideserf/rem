# DESIGN PRD — UI THEME & VISUAL LANGUAGE
## Codename: `REM` (Remote Entry Module)

---

## 1. Design Philosophy

### 1.1 Core Principles

**Phosphor first.** Every element reads as light emitted from a phosphor-coated screen. Colors have glow, falloff, and self-illumination. Implement through layered brightness: a dim base, a mid tone, and a hot highlight for selected/active states.

**Scanlines are structural.** The feeling of scanlines informs density decisions. Text never feels cramped. Rows have breathing room. One blank line between major sections, consistent vertical rhythm.

**Corporate ugly.** The UI was spec'd by a mining corporation in 2122. Labels are terse, uppercase, bureaucratic. Errors are clinical. No "friendly" language. No icons beyond ASCII glyphs. No rounded corners. No softness.

**Degradation is honest.** The terminal has been used. Signal degrades. Things flicker. Status indicators exist because things fail. `SIGNAL: WEAK` is a feature, not a bug.

**Output is the product.** Every UI decision reduces friction toward the user's goal. Nothing decorative that slows interaction.

### 1.2 Anti-patterns

- No color gradients
- No rounded border styles — `BorderType::Plain` only
- No friendly microcopy ("Oops!", "Nothing here yet", etc.)
- No icons or emoji — ASCII box-drawing and sigils only
- No centered layouts — left-aligned or grid-aligned
- No pastel or desaturated colors — phosphor colors are vivid against dark

---

## 2. Color System

Three palette variants, each representing a different unit type. Default is Phosphor Green. Selectable via CLI flag or config.

### 2.1 Palette A — Phosphor Green (Default)
*The ship terminal. Nostromo, Sulaco, Covenant. The computer that knows things it won't tell you.*

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

### 2.2 Palette B — Amber Colony
*The colony terminal. Hadley's Hope, frontier ops. Dropped one too many times. Still works. Barely.*

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

### 2.3 Palette C — Corporate Cyan
*The executive terminal. Weyland-Yutani ops. Clean. Clinical. Every access is logged.*

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

### 2.4 Color Usage Rules

| Role | Token | Usage |
|---|---|---|
| Background | `bg` | Terminal fill, default row background |
| Surface | `surface` | Elevated rows (header, footer, cursor row, overlays) |
| Text dim | `text_dim` | Labels, metadata, secondary info |
| Text mid | `text_mid` | Body text, readable content |
| Text hot | `text_hot` | Selected, active, cursor, important values |
| Border dim | `border_dim` | Inactive panel separators, track lines |
| Border mid | `border_mid` | Active/focused separators, dot separators |
| Border hot | `border_hot` | Active mode borders (fuzzy search) |
| Warn | `warn` | Errors, destructive confirmations, warnings |

Never hardcode RGB values in render logic. Always reference `&Palette`.

---

## 3. Typography & Text Conventions

### 3.1 Case Rules

| Context | Case |
|---|---|
| Section headers, labels | `UPPERCASE` |
| File and directory names | Preserve exact filesystem case |
| Status values | `UPPERCASE` |
| Key hint labels | lowercase (e.g. `hjkl move`) |
| Path display | Preserve exact case |
| Error/warning messages | `UPPERCASE. TERSE.` |
| Confirmation prompts | `UPPERCASE` |

### 3.2 Sigil System

Core navigation sigils:

```
▣   Directory (filled square)
◻   File (empty square)
▸   Collapsed tree node
▾   Expanded tree node
▶   Selected row indicator (prepended)
─   Horizontal rule / separator
│   Vertical separator
·   Dot separator in status bars
⚠   Warning indicator
█   Cursor / blink block
▋   Partial cursor
```

File operation sigils:

```
✂   Cut/delete pending
⊕   Yanked (in buffer)
⟳   Operation in progress
✓   Operation complete
✗   Operation failed
◆   Marked/selected for bulk operation
◇   Unmarked
```

All sigils must be single-width characters. Verify Unicode width before use.

### 3.3 Spacing Rhythm

```
Header bar height:        1 row
Section label padding:    0 left-pad, 1 row gap below
Row height:               1 row (no padding between entries)
Panel internal padding:   1 col left, 1 col right
Major section separation: 1 border line (no blank rows)
Footer bar height:        1 row
Breadcrumb bar height:    1 row
Confirmation overlay:     1 row (replaces footer)
```

### 3.4 Truncation

All text that may overflow truncates with `…` (U+2026). Never wrap.
- File/dir names: truncate on the right
- Paths: truncate on the left (show tail, not head)
- Preview content: truncate lines on the right

---

## 4. Border & Panel Rules

All borders use `BorderType::Plain`. Never `Rounded` or `Double`.

| Context | Color |
|---|---|
| Panel separators (inactive) | `border_dim` |
| Panel separators (active/focused) | `border_mid` |
| Fuzzy search row | `border_hot` |
| Section dividers within panels | inline `─` repeated, `border_dim` |
| Active pane indicator (dual-pane) | `border_hot` top border |
| Confirmation prompts | `warn` border |

### 4.1 Pane Focus Indicator

In dual-pane mode, the active pane's top edge uses `border_hot`. The inactive pane uses `border_dim`. This is the only visual distinction — no title bars, no colored backgrounds for entire panes.

---

## 5. Animation & Timing

### 5.1 Tick Rate

Event loop polls at **100ms** intervals. 10 redraws per second.

### 5.2 Cursor Blink

Toggle every **550ms** (~5-6 ticks). Applies to:
- Trailing `▋` in breadcrumb bar
- Cursor `▋` in fuzzy search input
- Cursor `▋` in rename/create input

Boolean `blink_on` in app state, toggled by `Instant` comparison.

### 5.3 No Transition Animations

Every frame is a full redraw. No fade, slide, or easing. The only temporal elements are:
- Cursor blink
- Throbber frame advancement
- Error auto-dismiss (3 seconds)
- Status refresh (disk usage, etc. — every 5 seconds)
- Boot sequence (startup only)

---

## 6. Throbber & Spinner System

Throbbers indicate background work: recursive size calculation, large file copy, directory scanning. Each palette variant has its own throbber character set to reinforce the unit's personality.

### 6.1 Throbber Types

**Data Stream** — For ongoing I/O operations (directory scan, file copy).

```
Green (ship):         ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
Amber (colony):       ⠁ ⠈ ⠐ ⠠ ⢀ ⡀ ⠄ ⠂
Cyan  (corporate):    ⣾ ⣽ ⣻ ⢿ ⡿ ⣟ ⣯ ⣷
```

**Processing** — For compute-bound work (recursive size calc, search indexing).

```
Green (ship):         ░ ▒ ▓ █ ▓ ▒ ░
Amber (colony):       ╸ ╺ ╸ ╺   ╸   ╺ ╸
Cyan  (corporate):    ◰ ◳ ◲ ◱
```

The amber variant intentionally skips frames (empty entries in the array) to simulate signal degradation.

**Heartbeat** — Persistent system status indicator in the header bar. Always running. Shows the system is alive.

```
Green (ship):         ·  ∙  •  ●  •  ∙  ·
Amber (colony):       ⡀ ⡀ ⣀ ⣠ ⣤ ⣶ ⣿ ⣶ ⣤ ⣠ ⣀ ⡀     ⡀
Cyan  (corporate):    ▁ ▂ ▃ ▄ ▅ ▆ ▇ █ ▇ ▆ ▅ ▄ ▃ ▂ ▁
```

Again, amber has gaps — the colony terminal's signal drops out periodically.

### 6.2 Throbber Timing

Each throbber type has its own tick divisor:

| Type | Frame advance rate | Effective FPS |
|---|---|---|
| Data Stream | Every 1 tick (100ms) | 10 |
| Processing | Every 2 ticks (200ms) | 5 |
| Heartbeat | Every 3 ticks (300ms) | ~3.3 |

### 6.3 Throbber Rendering Rules

- Throbber character uses `text_hot` color
- Trailing label (e.g. `SCANNING…`) uses `text_dim`
- Always left of descriptive text: `⠹ CALCULATING SIZE…`
- When operation completes, briefly flash `✓` for 1 second, then remove
- When operation fails, show `✗` in `warn` color with error text

### 6.4 Implementation

```rust
pub struct Throbber {
    frames: &'static [&'static str],
    current: usize,
    tick_divisor: u32,
    tick_count: u32,
}

impl Throbber {
    pub fn advance(&mut self) {
        self.tick_count += 1;
        if self.tick_count >= self.tick_divisor {
            self.tick_count = 0;
            self.current = (self.current + 1) % self.frames.len();
        }
    }

    pub fn frame(&self) -> &str {
        self.frames[self.current]
    }
}
```

The `Palette` struct (or a companion `ThrobberSet`) provides the frame arrays. Render functions receive the throbber alongside the palette.

---

## 7. Progress Bars

For operations with known size (file copy, move).

### 7.1 Standard Progress Bar

```
[███████████░░░░░░░░░] 54%  12.4 MB/s
```

- Filled portion: `█` in `text_hot`
- Empty portion: `░` in `border_dim`
- Brackets: `border_mid`
- Percentage and rate: `text_mid`
- Width: fills available space minus label columns

### 7.2 Indeterminate Progress

For operations with unknown total (recursive directory scan):

```
[░░░▒▓█▓▒░░░░░░░░░░░] SCANNING…
```

A 3-char bright region slides back and forth across the bar. Uses the Processing throbber timing (200ms per step).

---

## 8. Boot Sequence

On startup, before the main UI renders, a brief boot sequence plays. This is purely cosmetic and must complete in under 2 seconds. Skippable with any keypress.

### 8.1 Sequence

```
REM v0.2.0
WEYLAND-YUTANI CORP — FILE MANAGEMENT TERMINAL
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

BIOS .............. OK
MEMORY ............ 640K
DISK .............. [scanning]    ← throbber runs here
INTERFACE ......... NOMINAL

READY.
```

### 8.2 Timing

- Lines appear sequentially, 150ms apart
- `OK`/values appear 100ms after their label
- `[scanning]` shows the Data Stream throbber for 400ms before resolving to disk info
- `READY.` holds for 300ms, then transitions to main UI
- Any keypress during boot skips to main UI immediately

### 8.3 Styling

- Title line: `text_hot`, bold
- Corporate line: `text_dim`
- Rule: `border_mid`
- Labels: `text_mid`
- Dots: `text_dim`
- Values: `text_hot`
- Background: `bg`

---

## 9. Confirmation Dialogs

Destructive operations (delete, overwrite) show an inline confirmation that replaces the footer row. Never a modal or popup.

### 9.1 Layout

```
 ⚠ DELETE 3 ITEMS? THIS CANNOT BE UNDONE.                     y confirm  ·  n cancel
```

### 9.2 Styling

- `⚠` and message text: `warn`
- Key hints: `text_mid` for keys, `text_dim` for labels
- Background: `surface`
- Top border: `warn` (the only time a border uses warn color)

### 9.3 Behavior

- Only `y` confirms. All other keys cancel.
- No default action — the user must explicitly press a key.
- Auto-cancel after 10 seconds with no input.

---

## 10. Component Styling Reference

This section defines the visual treatment for each UI component. Functional behavior is defined in the functional PRD.

### 10.1 Header Bar

- Background: `surface`
- Text: `text_mid`
- Identifier (leftmost label): `text_hot`, Bold
- Separator ` · `: `text_dim`
- Status values: `text_hot`
- Heartbeat throbber: `text_hot`, right side
- Warning values: `warn`, blinking modifier
- Bottom border: `border_mid`

### 10.2 Breadcrumb / Path Bar

- Current (final) segment: `text_hot`
- Parent segments: `text_mid`
- Separators ` / `: `text_dim`
- Left-pad 1 col
- Trailing cursor `▋` blinks
- Bottom border: `border_dim`
- Background: `bg`

### 10.3 File List Rows

| State | Background | Text | Left indicator |
|---|---|---|---|
| Default | `bg` | `text_dim` | none |
| Hovered (cursor) | `surface` | `text_hot` | `▶` |
| Marked (bulk select) | `bg` | `text_mid` | `◆` |
| Marked + hovered | `surface` | `text_hot` | `◆` |
| Dimmed (fuzzy non-match) | `bg` | `border_dim` | none |
| Yanked (in cut/copy buffer) | `bg` | `text_dim` italic | `✂` or `⊕` |

Jump key badges: `text_hot`, Bold, background `border_mid`.

### 10.4 Scrollbar

- Track: `│` in `border_dim`
- Thumb: `█` in `text_dim`
- Thumb size proportional to viewport/total ratio
- 1 column wide, far right of file list

### 10.5 Info Sidebar

- Section labels: `UPPERCASE` with letter-spacing (`S E L E C T I O N`), `text_dim`
- Keys: `text_dim`
- Values: `text_hot`
- Bookmark jump keys: `text_hot`
- Section separator: blank row
- Left border: `border_dim`

### 10.6 Preview Pane

- Same position as info sidebar
- Content text: `text_mid`
- Line numbers (if shown): `text_dim`
- Left border: `border_dim`
- "PREVIEW" label at top: `text_dim`, letter-spaced
- Binary/unreadable file: show `BINARY — n BYTES` in `text_dim`
- Large file warning: `FILE EXCEEDS PREVIEW LIMIT` in `text_dim`

### 10.7 Footer / Key Hints

- Key glyphs: `text_mid`
- Descriptions: `text_dim`
- Separator ` · `: `border_mid`
- Background: `surface`

### 10.8 Fuzzy Search Overlay

- Occupies last visible list row
- Border: `border_hot`
- Prompt `/`: `text_hot`
- Input text: `text_hot`
- Cursor `▋`: blinking
- Match count: right-aligned, `text_dim`
- Matching chars within entry names: `text_hot`, Bold

### 10.9 Operation Status Bar

When a background operation is running, a status line appears above the footer:

```
 ⠹ COPYING: logs/access.log → /backup/logs/    ████████░░░░ 67%  4.2 MB/s
```

- Throbber: `text_hot`
- Operation label: `text_mid`
- File path: `text_dim`
- Progress bar: as defined in Section 7
- Background: `surface`
- Top border: `border_dim`

---

*Document version: 2.0 — Theme & visual language for `rem`*
*Referenced by: DESIGN_PRD_functional.md*
