mod app;
mod archive;
mod config;
mod favorites;
mod highlight;
mod input;
mod logo;
mod marks;
mod nav;
mod ops;
mod palette;
mod preview;
mod symbols;
mod sysmon;
mod session;
mod tags;
mod throbber;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind, MouseEvent, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, OpenRequest};
use config::Config;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Handle --help and --version before terminal setup
    for arg in &args {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("rem — Remote Entry Module (Weyland-Yutani Corp.)");
                println!();
                println!("USAGE: rem [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!("  --green     Phosphor green profile (default)");
                println!("  --amber     Amber colony terminal profile");
                println!("  --cyan      Corporate cyan profile");
                println!("  --no-boot   Skip boot sequence");
                println!("  --no-mouse  Disable mouse support");
                println!("  --help      Show this message");
                println!("  --version   Show version");
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("rem {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {}
        }
    }

    let cfg = Config::load(&args);

    let start_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = App::new(start_dir, cfg.palette, cfg.symbols);
    app.right_panel = cfg.default_panel;
    app.show_hidden = cfg.show_hidden;
    app.sort_mode = cfg.sort_mode;
    app.reduce_motion = cfg.reduce_motion;
    app.glitch_enabled = cfg.glitch_enabled;
    app.mouse_enabled = cfg.mouse_enabled;
    app.load_entries(); // re-sort with configured sort mode

    // Show config warnings
    if let Some(warn) = cfg.warnings.first() {
        app.error = Some((format!("CONFIG: {}", warn), std::time::Instant::now()));
    }

    // Load bookmarks
    app.marks = marks::load_marks();

    // Load favorites (#54)
    app.favorites = favorites::load_favorites();

    // Load tags (#58)
    app.tags = tags::load_tags();

    // Load session (#80)
    if let Some(sess) = session::load_session() {
        session::apply_session(&mut app, sess);
    }

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    if cfg.mouse_enabled {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Boot sequence
    if cfg.boot_sequence {
        let _ = ui::boot::run_boot(&mut terminal, cfg.palette);
    }

    let result = run_loop(&mut terminal, &mut app);

    // Teardown
    terminal::disable_raw_mode()?;
    if cfg.mouse_enabled {
        execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }

    // Save session (#80)
    session::save_session(&app);

    // Save bookmarks
    marks::save_marks(&app.marks);

    // Save tags (#58)
    tags::save_tags(&app.tags);

    match result {
        Ok(Some(path)) => {
            println!("{}", path.display());
            std::process::exit(0);
        }
        Ok(None) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(2);
        }
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<Option<std::path::PathBuf>> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    // Reset idle timer on any input (#17)
                    app.last_input = std::time::Instant::now();
                    app.idle_active = false;
                    app.idle_locked = false;
                    input::handle_key(app, key);

                    if app.should_quit {
                        return Ok(None);
                    }
                    if let Some(path) = app.selected_path.take() {
                        return Ok(Some(path));
                    }
                    if let Some(req) = app.open_request.take() {
                        handle_open(terminal, app, req)?;
                    }
                }
                Event::Mouse(mouse) if app.mouse_enabled => {
                    app.last_input = std::time::Instant::now();
                    app.idle_active = false;
                    app.idle_locked = false;
                    handle_mouse(app, mouse);
                }
                _ => {}
            }
        }

        app.tick();
    }
}

/// Handle mouse events (#38).
fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.cursor_up();
        }
        MouseEventKind::ScrollDown => {
            app.cursor_down();
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Check breadcrumb segment clicks first (#48)
            let mut nav_target: Option<std::path::PathBuf> = None;
            if let Some((_, by, _, bh)) = app.layout_areas.breadcrumb_area {
                if mouse.row >= by && mouse.row < by + bh {
                    for (start_x, end_x, path) in &app.layout_areas.breadcrumb_segments {
                        if mouse.column >= *start_x && mouse.column < *end_x {
                            nav_target = Some(path.clone());
                            break;
                        }
                    }
                }
            }
            if let Some(path) = nav_target {
                if path.is_dir() {
                    app.navigate_to(path);
                }
                return;
            }

            // Click to select in file list area; click again to open
            if let Some((lx, ly, _lw, lh)) = app.layout_areas.list_area {
                let mx = mouse.column;
                let my = mouse.row;
                if mx >= lx && my >= ly && my < ly + lh {
                    let row = (my - ly) as usize;
                    let target = app.pane().scroll_offset + row;
                    if target < app.pane().filtered_indices.len() {
                        if app.pane().cursor == target {
                            // Already selected — open it
                            app.enter_selected();
                        } else {
                            app.pane_mut().cursor = target;
                            app.preview_scroll = 0;
                            app.declassify_tick = Some(0);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Suspend the TUI, open a file, then restore.
fn handle_open(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    req: OpenRequest,
) -> io::Result<()> {
    let path = match &req {
        OpenRequest::Editor(p) | OpenRequest::SystemDefault(p) => p.clone(),
    };

    let mut cmd = match &req {
        OpenRequest::Editor(_) => {
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| {
                    if cfg!(windows) { "notepad".to_string() }
                    else { "vi".to_string() }
                });
            let mut c = std::process::Command::new(&editor);
            c.arg(&path);
            c
        }
        OpenRequest::SystemDefault(_) => {
            if cfg!(windows) {
                let mut c = std::process::Command::new("cmd");
                c.args(["/C", "start", "", &path.to_string_lossy()]);
                c
            } else if cfg!(target_os = "macos") {
                let mut c = std::process::Command::new("open");
                c.arg(&path);
                c
            } else {
                let mut c = std::process::Command::new("xdg-open");
                c.arg(&path);
                c
            }
        }
    };

    // For editor: suspend TUI, wait for exit, restore
    // For system default: just spawn detached (don't suspend)
    match &req {
        OpenRequest::Editor(_) => {
            // Suspend TUI
            terminal::disable_raw_mode()?;
            if app.mouse_enabled {
                execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
            } else {
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            }

            let result = cmd.status();

            // Restore TUI
            terminal::enable_raw_mode()?;
            if app.mouse_enabled {
                execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
            } else {
                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            }
            terminal.clear()?;

            if let Err(e) = result {
                app.error = Some((
                    format!("EDITOR FAILED: {}", e),
                    std::time::Instant::now(),
                ));
            }
            // Reload entries in case the file was modified
            app.load_entries();
        }
        OpenRequest::SystemDefault(_) => {
            match cmd.spawn() {
                Ok(_) => {}
                Err(e) => {
                    app.error = Some((
                        format!("OPEN FAILED: {}", e),
                        std::time::Instant::now(),
                    ));
                }
            }
        }
    }

    Ok(())
}
