//! Application state, focus, tabs and keybindings.

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};

use crate::editor::Editor;
use crate::keys;
use crate::terminal::Terminal;
use crate::tree::Tree;

const CWD_POLL: Duration = Duration::from_millis(400);

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Left,
    Tree,
}

/// Screen rects from the last draw, used to route mouse clicks.
#[derive(Default)]
pub struct Rects {
    /// (tab index, label rect, close "×" rect if closable)
    pub tabs: Vec<(usize, Rect, Option<Rect>)>,
    pub content: Rect,
    pub tree: Rect,
}

pub struct App {
    pub term: Terminal,
    pub editors: Vec<Editor>,
    /// 0 = terminal, n = editors[n - 1].
    pub active: usize,
    pub focus: Focus,
    pub tree: Tree,
    pub status: String,
    pub rects: Rects,
    pub quit: bool,
    quit_armed: bool,
    close_armed: bool,
    shell_cwd: Option<PathBuf>,
    last_poll: Instant,
}

fn contains(r: Rect, x: u16, y: u16) -> bool {
    r.contains(Position { x, y })
}

impl App {
    pub fn new(dir: PathBuf) -> io::Result<Self> {
        let tree = Tree::new(dir.clone())?;
        let term = Terminal::spawn(&dir, 24, 80)?;
        Ok(Self {
            term,
            editors: Vec::new(),
            active: 0,
            focus: Focus::Left,
            tree,
            status: String::new(),
            rects: Rects::default(),
            quit: false,
            quit_armed: false,
            close_armed: false,
            shell_cwd: Some(dir),
            last_poll: Instant::now(),
        })
    }

    pub fn editor(&mut self) -> Option<&mut Editor> {
        self.active.checked_sub(1).and_then(|i| self.editors.get_mut(i))
    }

    /// Periodic work. Returns true if a redraw is needed.
    pub fn tick(&mut self) -> bool {
        let mut changed = self.term.take_dirty();
        if self.last_poll.elapsed() >= CWD_POLL {
            self.last_poll = Instant::now();
            let cwd = self.term.cwd();
            if cwd.is_some() && cwd != self.shell_cwd {
                self.shell_cwd = cwd.clone();
                changed |= cwd.is_some_and(|d| self.tree.set_dir(d));
            }
        }
        changed
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::Key(k) if k.kind != KeyEventKind::Release => self.key(k),
            Event::Mouse(m) => self.mouse(m),
            Event::Paste(s) if self.focus == Focus::Left => match self.editor() {
                Some(e) => e.paste(&s),
                None => self.term.paste(&s),
            },
            _ => {}
        }
    }

    fn key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let (quit_armed, close_armed) = (self.quit_armed, self.close_armed);
        self.quit_armed = false;
        self.close_armed = false;
        self.status.clear();

        match k.code {
            KeyCode::Char('q') if ctrl => {
                if quit_armed || !self.editors.iter().any(|e| e.dirty) {
                    self.quit = true;
                } else {
                    self.quit_armed = true;
                    self.status = "Unsaved changes! Press Ctrl+Q again to quit without saving.".into();
                }
            }
            KeyCode::Char('e') if alt => {
                self.focus = if self.focus == Focus::Left { Focus::Tree } else { Focus::Left };
            }
            KeyCode::Left if alt => self.switch(self.active.checked_sub(1).unwrap_or(self.editors.len())),
            KeyCode::Right if alt => self.switch((self.active + 1) % (self.editors.len() + 1)),
            KeyCode::Char(c @ '1'..='9') if alt => self.switch(c as usize - '1' as usize),
            KeyCode::Char('w') if alt => self.close(self.active, close_armed),
            _ if self.focus == Focus::Tree => self.tree_key(k),
            KeyCode::Char('s') if ctrl && self.active > 0 => self.save(),
            _ if self.active > 0 => {
                if let Some(e) = self.editor() {
                    e.handle_key(k);
                }
            }
            _ if self.term.exited => {
                if k.code == KeyCode::Enter {
                    self.restart_shell();
                }
            }
            _ => {
                self.term.scroll_to_bottom();
                let bytes = keys::to_bytes(k, self.term.application_cursor());
                self.term.write(&bytes);
            }
        }
    }

    fn tree_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('j') | KeyCode::Down => self.tree.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.tree.move_by(-1),
            KeyCode::PageDown => self.tree.move_by(10),
            KeyCode::PageUp => self.tree.move_by(-10),
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => self.open_selected(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => self.tree.go_up(),
            _ => {}
        }
    }

    fn switch(&mut self, idx: usize) {
        if idx <= self.editors.len() {
            self.active = idx;
            self.focus = Focus::Left;
        }
    }

    fn save(&mut self) {
        if let Some(e) = self.editor() {
            self.status = e.save().unwrap_or_else(|err| err);
            self.tree.refresh();
        }
    }

    fn close(&mut self, idx: usize, armed: bool) {
        let Some(e) = idx.checked_sub(1).and_then(|i| self.editors.get(i)) else {
            self.status = "The terminal tab can't be closed.".into();
            return;
        };
        if e.dirty && !armed {
            self.close_armed = true;
            self.status = format!("{} has unsaved changes! Press Alt+W again to discard.", e.file_name());
            return;
        }
        self.editors.remove(idx - 1);
        if self.active >= idx {
            self.active -= 1;
        }
    }

    fn open_selected(&mut self) {
        let Some(entry) = self.tree.selected() else { return };
        let (path, is_dir) = (entry.path.clone(), entry.is_dir);
        if is_dir {
            if !self.tree.set_dir(path.clone()) {
                self.status = format!("cannot read {}", path.display());
            }
        } else {
            self.open_file(path);
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if let Some(i) = self.editors.iter().position(|e| e.path == path) {
            self.switch(i + 1);
            return;
        }
        match Editor::open(&path) {
            Ok(e) => {
                self.status = format!("opened {} ({})", e.file_name(), e.language());
                self.editors.push(e);
                self.switch(self.editors.len());
            }
            Err(msg) => self.status = msg,
        }
    }

    fn restart_shell(&mut self) {
        let (r, c) = (self.rects.content.height.max(2), self.rects.content.width.max(2));
        match Terminal::spawn(&self.tree.cwd, r, c) {
            Ok(t) => {
                self.term = t;
                self.shell_cwd = Some(self.tree.cwd.clone());
            }
            Err(e) => self.status = format!("cannot start shell: {e}"),
        }
    }

    fn mouse(&mut self, m: MouseEvent) {
        let (x, y) = (m.column, m.row);
        let scroll = match m.kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            MouseEventKind::Down(MouseButton::Left) => 0,
            _ => return,
        };
        let r = &self.rects;
        if scroll != 0 {
            if contains(r.tree, x, y) {
                self.tree.move_by(scroll);
            } else if contains(r.content, x, y) {
                match self.editor() {
                    Some(e) => e.scroll(scroll),
                    None => self.term.scroll(-scroll),
                }
            }
            return;
        }

        // Left click.
        if let Some(&(idx, _, close)) = r.tabs.iter().find(|(_, t, c)| contains(*t, x, y) || c.is_some_and(|c| contains(c, x, y))) {
            if close.is_some_and(|c| contains(c, x, y)) {
                let armed = self.close_armed && self.active == idx;
                self.close_armed = false;
                self.active = idx;
                self.close(idx, armed);
            } else {
                self.switch(idx);
            }
        } else if contains(r.tree, x, y) {
            self.focus = Focus::Tree;
            // +1 for the border row.
            let i = self.tree.state.offset() + (y - r.tree.y).saturating_sub(1) as usize;
            if y > r.tree.y && i < self.tree.entries.len() {
                self.tree.state.select(Some(i));
                self.open_selected();
            }
        } else if contains(r.content, x, y) {
            self.focus = Focus::Left;
            let (cx, cy) = (x - r.content.x, y - r.content.y);
            if let Some(e) = self.editor() {
                e.click(cx, cy);
            }
        }
    }
}
