# rem

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

### Phosphor Green -- Standard Issue
![Phosphor Green theme](screenshots/green.png)

### Amber -- Corporate Mainframe
![Amber theme](screenshots/amber.png)

### Degraded Cyan -- Field Unit (with theme picker)
![Degraded Cyan theme with theme picker](screenshots/cyan-w-theme-picker.png)

## Operator Manual

### Terminal Capabilities

- **Navigation** -- vim-keyed traversal (`hjkl`), jump-to-top/bottom (`gg`/`G`), smooth animated transitions
- **Dual-pane operations** -- `Tab` to deploy split-view for cross-directory transfers
- **Visual targeting** -- `v` to enter selection mode, mark multiple assets with `j`/`k`
- **Asset management** -- `yy` copy, `dd` cut, `p` paste, `D` purge (requires confirmation)
- **Bulk rename** -- `R` in visual mode for find/replace pattern renaming across selections
- **Fuzzy search** -- `/` to locate assets in the current directory via pattern matching
- **Recursive search** -- `S` to search across all subdirectories
- **Jump keys** -- `f` to display single-key target labels on all visible entries
- **Navigation marks** -- `m` + key to designate a waypoint, `'` + key to return
- **Asset preview** -- side panel with syntax-highlighted, scrollable file contents
- **In-app editor** -- `e` to edit files with syntax highlighting, undo stack, and save
- **Rename & provision** -- `r` to rename, `a` to create file, `A` to create directory
- **Sort modes** -- `s` to cycle between name, size, and date ordering
- **Git integration** -- current branch and dirty status displayed in header
- **Nerd Font icons** -- extension-based file icons with fallback for standard terminals
- **Symbol sets** -- 7 swappable glyph styles via theme picker (Standard, ASCII, Block, Minimal, Pipeline, Braille, Scanline)
- **System telemetry** -- `` ` `` to monitor CPU, RAM, disk, and network diagnostics
- **Display profiles** -- `t` to open the theme and symbol set selector
- **Viewport adjustment** -- `[` / `]` to resize the preview panel
- **Boot sequence** -- corporate authentication splash with animated WY mark

### Display Profiles

| Profile | Designation | Deployment |
|---------|-------------|------------|
| **PHOSPHOR GREEN** | WY-CRT-01 | Standard colony terminals |
| **AMBER** | WY-CRT-02 | Corporate mainframe consoles |
| **DEGRADED CYAN** | WY-CRT-03 | Field units, survey equipment |

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
| `H` | Navigate back in history |
| `L` | Navigate forward in history |
| `-` | Ascend to parent directory |

#### Search & Targeting
| Input | Function |
|-------|----------|
| `/` | Fuzzy search current directory |
| `S` | Recursive search across subdirectories |
| `f` | Deploy jump key overlay |
| `m` + key | Set navigation mark |
| `'` + key | Jump to navigation mark |

#### Asset Operations
| Input | Function |
|-------|----------|
| `yy` | Copy current asset or selection |
| `dd` | Cut current asset or selection |
| `p` | Paste from operations buffer |
| `D` | Purge selection (confirmation required) |
| `r` | Rename asset |
| `R` | Bulk rename (visual mode -- find/replace) |
| `a` | Provision new file |
| `A` | Provision new directory |
| `e` | Open in-app text editor |
| `E` | Open in external `$EDITOR` |

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
| `Tab` | Toggle dual-pane / switch active pane |
| `i` | Cycle right panel (info / preview / hidden) |
| `[` | Contract sidebar |
| `]` | Expand sidebar |
| `t` | Open display profile / symbol set selector |
| `` ` `` | Toggle telemetry readout |
| `.` | Toggle hidden assets |

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
