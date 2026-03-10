# rem

[![Docs](https://img.shields.io/badge/docs-johnsideserf.github.io%2Frem-FFB000?style=flat&logo=github)](https://johnsideserf.github.io/rem/)

> **REMOTE ENTRY MODULE** -- Weyland-Yutani Corp. Standard-Issue File Navigation Terminal
>
> *Classified under WY-DOC-4789. Unauthorized access is a violation of ICC corporate law.*

```
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@...........@@@@...........@@@@..........@@@@...........@@@@...........@@@@@
@@@@@@@...........@@@@.......@@@@..............@@@@.......@@@@...........@@@@@@@
@@@@@@@@@...........@@@@...@@@@..................@@@@...@@@@...........@@@@@@@@@
@@@@@@@@@@@...........@@@@@@........................@@@@@@...........@@@@@@@@@@@
@@@@@@@@@@@@@...........@@.............@@.............@@...........@@@@@@@@@@@@@
@@@@@@@@@@@@@@.......................@@..@@.......................@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@..................@@......@@..................@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@..............@@@..........@@@..............@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@..........@@@@..........@@@@..........@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
              W E Y L A N D - Y U T A N I  C O R P O R A T I O N
                       BUILDING  BETTER  WORLDS
```

A corporate-grade terminal file navigator built in Rust with [ratatui](https://ratatui.rs). Designed for operatives who need to manage filesystem assets efficiently in hostile or low-bandwidth environments. Three CRT display profiles are provided to match your installation's hardware specifications.

## Screenshots

![Boot sequence](screenshots/boot-amber.gif)

### Phosphor Green -- Ship Terminal
![Phosphor Green theme](screenshots/green.png)

### Amber -- Colony Terminal
![Amber theme](screenshots/amber.png)

### Corporate Cyan -- Executive Terminal (with theme picker)
![Corporate Cyan theme with theme picker](screenshots/cyan-w-theme-picker.png)

## Operator Manual

### Terminal Capabilities

- **Navigation** -- vim-keyed traversal (`hjkl`), jump-to-top/bottom (`gg`/`G`), smooth animated transitions
- **Dual-pane operations** -- `Ctrl+W` to toggle split-view, `Tab` to switch panes
- **Visual targeting** -- `v` to enter selection mode, mark multiple assets with `j`/`k`
- **Asset management** -- `yy` copy, `dd` cut, `p` paste, `D` purge (requires confirmation)
- **Bulk rename** -- `R` in visual mode for find/replace pattern renaming across selections
- **Fuzzy search** -- `/` to locate assets in the current directory via pattern matching
- **Recursive search** -- `?` to search across all subdirectories
- **Jump keys** -- `Space` to display single-key target labels on all visible entries
- **Navigation marks** -- `m` + key to designate a waypoint, `'` + key to return
- **Asset preview** -- side panel with syntax-highlighted, scrollable file contents
- **In-app editor** -- `e` to edit files with syntax highlighting, undo stack, and save
- **Rename & provision** -- `r` to rename, `o` to create file, `O` to create directory
- **Sort modes** -- `s` to cycle between name, size, and date ordering
- **Git integration** -- current branch and dirty status displayed in header
- **Nerd Font icons** -- extension-based file icons with fallback for standard terminals
- **Symbol sets** -- 7 swappable glyph styles via theme picker (Standard, ASCII, Block, Minimal, Pipeline, Braille, Scanline)
- **System telemetry** -- `` ` `` to monitor CPU, RAM, disk, and network diagnostics
- **Display profiles** -- `t` to open the theme and symbol set selector
- **Sidebar adjustment** -- `[` / `]` to resize the sidebar panel
- **File integrity** -- `#` to compute SHA-256 hash of selected file
- **Disk usage** -- `W` to scan recursive directory size allocation
- **Archive browsing** -- inspect zip/tar contents as a virtual read-only directory
- **Lock screen** -- `L` to activate per-palette animated screensaver
- **Boot sequence** -- per-palette corporate authentication splash with animated WY mark

### Display Profiles

| Profile | Designation | Deployment |
|---------|-------------|------------|
| **PHOSPHOR GREEN** | WY-CRT-01 | Ship terminals (Nostromo, Sulaco) |
| **AMBER** | WY-CRT-02 | Colony terminals (Hadley's Hope) |
| **CORPORATE CYAN** | WY-CRT-03 | Executive consoles, MedPods |

### Command Reference

#### Navigation
| Input | Function |
|-------|----------|
| `h` / `Left` | Ascend to parent directory |
| `l` / `Right` / `Enter` | Enter directory |
| `j` / `Down` | Cursor down |
| `k` / `Up` | Cursor up |
| `gg` | Jump to first entry |
| `G` | Jump to last entry |
| `Ctrl+O` | Navigate back in history |
| `Ctrl+I` | Navigate forward in history |
| `Ctrl+U` / `Ctrl+D` | Half-page scroll up / down |
| `-` | Ascend to parent directory |

#### Search & Targeting
| Input | Function |
|-------|----------|
| `/` | Fuzzy search current directory |
| `?` | Recursive search across subdirectories |
| `Space` | Deploy jump key overlay |
| `m` + key | Set navigation mark |
| `'` + key | Jump to navigation mark |
| `M` + key | Delete navigation mark |

#### Asset Operations
| Input | Function |
|-------|----------|
| `yy` | Copy current asset or selection |
| `dd` | Cut current asset or selection |
| `p` | Paste from operations buffer |
| `D` | Purge selection (confirmation required) |
| `r` | Rename asset |
| `R` | Bulk rename (visual mode -- find/replace) |
| `o` | Provision new file |
| `O` | Provision new directory |
| `e` | Open in-app text editor |
| `E` | Open in external `$EDITOR` |
| `#` | Compute SHA-256 hash |
| `W` | Scan disk usage recursively |

#### Sorting
| Input | Function |
|-------|----------|
| `s` | Cycle sort mode (name / size / date) |

#### Selection
| Input | Function |
|-------|----------|
| `v` | Toggle visual targeting mode |
| `u` | Clear all marks |

#### Display & Panels
| Input | Function |
|-------|----------|
| `Ctrl+W` | Toggle dual-pane mode |
| `Tab` | Switch active pane / cycle right panel |
| `Ctrl+J` / `Ctrl+K` | Scroll preview pane down / up |
| `[` | Contract sidebar |
| `]` | Expand sidebar |
| `t` | Open display profile / symbol set selector |
| `` ` `` | Toggle telemetry readout |
| `H` | Toggle hidden assets |
| `L` | Lock screen (activate screensaver) |

#### General
| Input | Function |
|-------|----------|
| `q` | Terminate session |
| `Esc` | Abort current operation |

## Deployment

### Quick install

```sh
cargo install --path .
```

### Build from source

```sh
git clone https://github.com/johnsideserf/rem.git
cd rem
cargo build --release
./target/release/rem
```

## System Requirements

- Rust 2024 edition (1.85+)
- Terminal with Unicode rendering capability
- Recommended: a [Nerd Font](https://www.nerdfonts.com/) for optimal glyph display

---

*Weyland-Yutani Corporation. Building Better Worlds.*

*This software is provided under the MIT license. The Company assumes no liability for data loss, xenomorph encounters, or crew expenditure resulting from use of this terminal.*
