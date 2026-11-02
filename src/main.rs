//! devterm — a terminal laid out like a code editor: a real shell plus
//! editor tabs on the left, a live file tree on the right.

mod app;
mod editor;
mod highlight;
mod keys;
mod terminal;
mod tree;
mod ui;

use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    event::{self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;

use app::App;

fn main() -> io::Result<()> {
    let dir = match std::env::args_os().nth(1) {
        Some(d) => std::fs::canonicalize(PathBuf::from(d))?,
        None => std::env::current_dir()?,
    };
    if !dir.is_dir() {
        return Err(io::Error::other(format!("{} is not a directory", dir.display())));
    }

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let result = App::new(dir).and_then(|mut app| {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        run(&mut terminal, &mut app)
    });
    restore();
    result
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        ratatui::crossterm::cursor::Show
    );
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut dirty = true;
    while !app.quit {
        if dirty {
            terminal.draw(|f| ui::draw(f, app))?;
            dirty = false;
        }
        if event::poll(Duration::from_millis(16))? {
            // Drain everything pending before the next redraw.
            loop {
                app.handle(event::read()?);
                if app.quit || !event::poll(Duration::ZERO)? {
                    break;
                }
            }
            dirty = true;
        }
        dirty |= app.tick();
    }
    Ok(())
}
