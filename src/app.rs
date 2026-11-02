//! Application state and input handling.

use std::io;
use std::path::PathBuf;

use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::ListState;

use crate::fs::{read_dir_entries, DirEntry};

/// Which pane currently receives key input.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Shell,
    Files,
}

pub struct App {
    pub cwd: PathBuf,
    pub entries: Vec<DirEntry>,
    pub list_state: ListState,
    pub focus: Focus,
    pub shell_lines: Vec<String>,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        let entries = read_dir_entries(&cwd)?;

        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            cwd,
            entries,
            list_state,
            focus: Focus::Files,
            shell_lines: vec![
                "devterm — Milestone 1".into(),
                String::new(),
                "This left pane is a placeholder for now.".into(),
                "Milestone 2 will run a real shell here through a PTY".into(),
                "(portable-pty + vt100 + tui-term).".into(),
                String::new(),
                "Keys".into(),
                "  Tab        switch pane".into(),
                "  q          quit".into(),
                "  j / k      move in file pane".into(),
                "  Enter / l  open directory".into(),
                "  Backspace  go up one directory".into(),
            ],
        })
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Shell => Focus::Files,
            Focus::Files => Focus::Shell,
        };
    }

    /// Handle a key while the file pane is focused.
    pub fn handle_files_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_prev(),
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => self.enter_selected(),
            KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Left => self.go_up(),
            _ => {}
        }
    }

    fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.entries.len() - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0).saturating_sub(1);
        self.list_state.select(Some(i));
    }

    fn enter_selected(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(i) {
                if entry.is_dir {
                    let target = entry.path.clone();
                    self.set_cwd(target);
                }
            }
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            let parent = parent.to_path_buf();
            self.set_cwd(parent);
        }
    }

    /// Move into `path`. A permission error (e.g. a directory we may not read)
    /// leaves the current listing untouched instead of crashing.
    fn set_cwd(&mut self, path: PathBuf) {
        if let Ok(entries) = read_dir_entries(&path) {
            self.entries = entries;
            self.cwd = path;
            let sel = if self.entries.is_empty() { None } else { Some(0) };
            self.list_state.select(sel);
        }
    }
}
