mod app;
mod archive;
mod config;
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
mod throbber;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, OpenRequest};
use config::Config;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = Config::load(&args);

    let start_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = App::new(start_dir, cfg.palette, cfg.symbols);
    app.right_panel = cfg.default_panel;
    app.show_hidden = cfg.show_hidden;
    app.sort_mode = cfg.sort_mode;
    app.reduce_motion = cfg.reduce_motion;
    app.glitch_enabled = cfg.glitch_enabled;
    app.load_entries(); // re-sort with configured sort mode

    // Load bookmarks
    app.marks = marks::load_marks();

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Boot sequence
    if cfg.boot_sequence {
        let _ = ui::boot::run_boot(&mut terminal, cfg.palette);
    }

    let result = run_loop(&mut terminal, &mut app);

    // Teardown
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Save bookmarks
    marks::save_marks(&app.marks);

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
            if let Event::Key(key) = event::read()? {
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
        }

        app.tick();
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
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

            let result = cmd.status();

            // Restore TUI
            terminal::enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
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
