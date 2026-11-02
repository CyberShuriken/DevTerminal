//! devterm — a two-pane terminal UI (shell left, file browser right).
//!
//! Milestone 1: layout, focus switching, and a working file-browser pane.
//! Milestone 2 (planned): a real interactive shell in the left pane via a PTY.

mod app;
mod fs;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;

use app::{App, Focus};

fn main() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new()?;

    let result = run(&mut terminal, &mut app);

    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(out))
}

fn restore_terminal<B: Backend + io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll so the UI stays responsive; 250ms is plenty for a file browser.
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => app.toggle_focus(),
                code if app.focus == Focus::Files => app.handle_files_key(code),
                _ => {}
            }
        }
    }
}
