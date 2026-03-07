mod app;
mod marks;
mod palette;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, Mode, JUMP_KEYS};
use palette::Palette;

fn main() -> io::Result<()> {
    // Determine palette from CLI arg or env
    let palette = match std::env::args().nth(1).as_deref() {
        Some("--amber") => Palette::amber(),
        Some("--cyan") => Palette::degraded_cyan(),
        _ => Palette::phosphor_green(),
    };

    let start_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = App::new(start_dir, palette);

    // Load bookmarks
    app.marks = marks::load_marks();

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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
                handle_key(app, key);

                if app.should_quit {
                    return Ok(None);
                }
                if let Some(path) = app.selected_path.take() {
                    return Ok(Some(path));
                }
            }
        }

        app.tick();
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Dismiss error on any key
    if app.error.is_some() {
        app.error = None;
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::FuzzySearch => handle_fuzzy(app, key),
        Mode::JumpKey => handle_jump(app, key),
        Mode::WaitingForG => handle_waiting_g(app, key),
        Mode::WaitingForMark => handle_set_mark(app, key),
        Mode::WaitingForJumpToMark => handle_jump_mark(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            app.should_quit = true;
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.cursor_down();
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.cursor_up();
        }
        (KeyModifiers::NONE, KeyCode::Char('l'))
        | (KeyModifiers::NONE, KeyCode::Right)
        | (KeyModifiers::NONE, KeyCode::Enter) => {
            app.enter_selected();
        }
        (KeyModifiers::NONE, KeyCode::Char('h'))
        | (KeyModifiers::NONE, KeyCode::Left)
        | (KeyModifiers::NONE, KeyCode::Char('-')) => {
            app.go_parent();
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            app.mode = Mode::WaitingForG;
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.jump_bottom();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.scroll_half_up();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.scroll_half_down();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            app.nav_back();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('i')) => {
            app.nav_forward();
        }
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            app.mode = Mode::FuzzySearch;
            app.fuzzy_query.clear();
            app.rebuild_filtered();
        }
        (KeyModifiers::NONE, KeyCode::Char(' ')) => {
            app.mode = Mode::JumpKey;
        }
        (KeyModifiers::NONE, KeyCode::Char('m')) => {
            app.mode = Mode::WaitingForMark;
        }
        (KeyModifiers::NONE, KeyCode::Char('\'')) => {
            app.mode = Mode::WaitingForJumpToMark;
        }
        _ => {}
    }
}

fn handle_waiting_g(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('g') => {
            app.jump_top();
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
}

fn handle_set_mark(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() {
            app.set_mark(c);
        }
    }
    app.mode = Mode::Normal;
}

fn handle_jump_mark(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() || c == '\'' {
            app.jump_to_mark(c);
        }
    }
    app.mode = Mode::Normal;
}

fn handle_fuzzy(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.fuzzy_query.clear();
            app.cursor = 0;
            app.scroll_offset = 0;
            app.rebuild_filtered();
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            if !app.filtered_indices.is_empty() {
                app.cursor = 0;
                app.enter_selected();
            }
        }
        KeyCode::Backspace => {
            app.fuzzy_query.pop();
            app.cursor = 0;
            app.scroll_offset = 0;
            app.rebuild_filtered();
        }
        KeyCode::Char(c) => {
            app.fuzzy_query.push(c);
            app.cursor = 0;
            app.scroll_offset = 0;
            app.rebuild_filtered();
        }
        _ => {}
    }
}

fn handle_jump(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if c.is_ascii_lowercase() => {
            if let Some(pos) = JUMP_KEYS.iter().position(|&k| k == c) {
                if pos < app.filtered_indices.len() {
                    app.cursor = pos;
                    app.enter_selected();
                }
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
}
