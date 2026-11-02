//! A small text editor buffer with syntax highlighting.

use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;

use crate::highlight::Highlight;

const TAB: usize = 4;
const MAX_SIZE: u64 = 8 * 1024 * 1024;

pub struct Editor {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub row: usize,
    /// Cursor column in chars.
    pub col: usize,
    goal: Option<usize>,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub dirty: bool,
    crlf: bool,
    trailing_newline: bool,
    uses_tabs: bool,
    follow: bool,
    view_h: usize,
    hl: Highlight,
}

fn char_w(c: char, at: usize) -> usize {
    if c == '\t' {
        TAB - at % TAB
    } else {
        c.width().unwrap_or(0)
    }
}

/// Display width of the first `col` chars of `s` (tabs expanded).
fn disp_col(s: &str, col: usize) -> usize {
    s.chars().take(col).fold(0, |w, c| w + char_w(c, w))
}

/// Char index whose cell covers display column `x`.
fn col_at(s: &str, x: usize) -> usize {
    let mut w = 0;
    for (i, c) in s.chars().enumerate() {
        let cw = char_w(c, w);
        if w + cw > x {
            return i;
        }
        w += cw;
    }
    s.chars().count()
}

fn byte_idx(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map_or(s.len(), |(i, _)| i)
}

impl Editor {
    pub fn open(path: &Path) -> Result<Self, String> {
        let name = path.display();
        let meta = std::fs::metadata(path).map_err(|e| format!("{name}: {e}"))?;
        if meta.len() > MAX_SIZE {
            return Err(format!("{name}: larger than 8MB, not opened"));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("{name}: {e}"))?;
        if bytes.contains(&0) {
            return Err(format!("{name}: binary file, not opened"));
        }
        let text =
            String::from_utf8(bytes).map_err(|_| format!("{name}: not valid UTF-8, not opened"))?;
        Ok(Self::from_text(path, &text))
    }

    fn from_text(path: &Path, text: &str) -> Self {
        let crlf = text.contains("\r\n");
        let trailing_newline = text.ends_with('\n');
        let body = text.strip_suffix('\n').unwrap_or(text);
        let mut lines: Vec<String> = body
            .split('\n')
            .map(|l| {
                if crlf {
                    l.strip_suffix('\r').unwrap_or(l)
                } else {
                    l
                }
                .to_string()
            })
            .collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let uses_tabs = lines.iter().any(|l| l.starts_with('\t'));
        let hl = Highlight::new(path, &lines[0]);
        Self {
            path: path.to_path_buf(),
            lines,
            row: 0,
            col: 0,
            goal: None,
            scroll_row: 0,
            scroll_col: 0,
            dirty: false,
            crlf,
            trailing_newline,
            uses_tabs,
            follow: true,
            view_h: 20,
            hl,
        }
    }

    pub fn text(&self) -> String {
        let mut s = self.lines.join(if self.crlf { "\r\n" } else { "\n" });
        if self.trailing_newline {
            s.push_str(if self.crlf { "\r\n" } else { "\n" });
        }
        s
    }

    pub fn save(&mut self) -> Result<String, String> {
        std::fs::write(&self.path, self.text()).map_err(|e| format!("save failed: {e}"))?;
        self.dirty = false;
        Ok(format!("saved {}", self.path.display()))
    }

    pub fn file_name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into(),
        )
    }

    pub fn language(&self) -> &str {
        self.hl.name()
    }

    fn line_len(&self) -> usize {
        self.lines[self.row].chars().count()
    }

    fn edited(&mut self, from_row: usize) {
        self.dirty = true;
        self.goal = None;
        self.follow = true;
        self.hl.invalidate(from_row);
    }

    fn moved(&mut self) {
        self.follow = true;
    }

    pub fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.row];
        line.insert(byte_idx(line, self.col), c);
        self.col += 1;
        self.edited(self.row);
    }

    fn insert_str_plain(&mut self, s: &str) {
        let line = &mut self.lines[self.row];
        line.insert_str(byte_idx(line, self.col), s);
        self.col += s.chars().count();
    }

    pub fn newline(&mut self, auto_indent: bool) {
        let line = &mut self.lines[self.row];
        let rest = line.split_off(byte_idx(line, self.col));
        let indent: String = if auto_indent {
            line.chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        } else {
            String::new()
        };
        self.col = indent.chars().count();
        self.lines.insert(self.row + 1, indent + &rest);
        self.edited(self.row);
        self.row += 1;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            let line = &mut self.lines[self.row];
            line.remove(byte_idx(line, self.col - 1));
            self.col -= 1;
            self.edited(self.row);
        } else if self.row > 0 {
            let cur = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len();
            self.lines[self.row].push_str(&cur);
            self.edited(self.row);
        }
    }

    pub fn delete(&mut self) {
        if self.col < self.line_len() {
            let line = &mut self.lines[self.row];
            line.remove(byte_idx(line, self.col));
            self.edited(self.row);
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
            self.edited(self.row);
        }
    }

    fn tab(&mut self) {
        if self.uses_tabs {
            self.insert_char('\t');
        } else {
            let x = disp_col(&self.lines[self.row], self.col);
            self.insert_str_plain(&" ".repeat(TAB - x % TAB));
            self.edited(self.row);
        }
    }

    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let start = self.row;
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.newline(false);
            }
            self.insert_str_plain(part);
        }
        self.edited(start);
    }

    fn vertical(&mut self, delta: isize) {
        let goal = *self
            .goal
            .get_or_insert_with(|| disp_col(&self.lines[self.row], self.col));
        let max = self.lines.len() as isize - 1;
        self.row = (self.row as isize + delta).clamp(0, max) as usize;
        self.col = col_at(&self.lines[self.row], goal);
        self.moved();
    }

    fn horizontal(&mut self, f: impl FnOnce(&mut Self)) {
        f(self);
        self.goal = None;
        self.moved();
    }

    /// Returns true if the key was handled.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char(c) if !ctrl && !alt => self.insert_char(c),
            KeyCode::Enter => self.newline(true),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Tab => self.tab(),
            KeyCode::Up => self.vertical(-1),
            KeyCode::Down => self.vertical(1),
            KeyCode::PageUp => self.vertical(-(self.view_h.max(1) as isize)),
            KeyCode::PageDown => self.vertical(self.view_h.max(1) as isize),
            KeyCode::Left => self.horizontal(|e| {
                if e.col > 0 {
                    e.col -= 1;
                } else if e.row > 0 {
                    e.row -= 1;
                    e.col = e.line_len();
                }
            }),
            KeyCode::Right => self.horizontal(|e| {
                if e.col < e.line_len() {
                    e.col += 1;
                } else if e.row + 1 < e.lines.len() {
                    e.row += 1;
                    e.col = 0;
                }
            }),
            KeyCode::Home if ctrl => self.horizontal(|e| (e.row, e.col) = (0, 0)),
            KeyCode::End if ctrl => self.horizontal(|e| {
                e.row = e.lines.len() - 1;
                e.col = e.line_len();
            }),
            KeyCode::Home => self.horizontal(|e| {
                let first = e.lines[e.row]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .count();
                e.col = if e.col == first { 0 } else { first };
            }),
            KeyCode::End => self.horizontal(|e| e.col = e.line_len()),
            _ => return false,
        }
        true
    }

    pub fn scroll(&mut self, delta: isize) {
        let max = self.lines.len().saturating_sub(1) as isize;
        self.scroll_row = (self.scroll_row as isize + delta).clamp(0, max) as usize;
        self.follow = false;
    }

    fn gutter_width(&self) -> u16 {
        self.lines.len().to_string().len() as u16 + 2
    }

    /// Place the cursor at a click position relative to the editor area.
    pub fn click(&mut self, x: u16, y: u16) {
        let row = (self.scroll_row + y as usize).min(self.lines.len() - 1);
        let tx = x.saturating_sub(self.gutter_width()) as usize + self.scroll_col;
        self.row = row;
        self.col = col_at(&self.lines[row], tx);
        self.goal = None;
        self.moved();
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) -> Option<(u16, u16)> {
        let gw = self.gutter_width();
        if area.width <= gw || area.height == 0 {
            return None;
        }
        let (h, w) = (area.height as usize, (area.width - gw) as usize);
        self.view_h = h;
        let cx = disp_col(&self.lines[self.row], self.col);
        if self.follow {
            if self.row < self.scroll_row {
                self.scroll_row = self.row;
            } else if self.row >= self.scroll_row + h {
                self.scroll_row = self.row + 1 - h;
            }
            if cx < self.scroll_col {
                self.scroll_col = cx;
            } else if cx >= self.scroll_col + w {
                self.scroll_col = cx + 1 - w;
            }
            self.follow = false;
        }
        self.hl.ensure(&self.lines, self.scroll_row + h);

        for y in 0..h {
            let i = self.scroll_row + y;
            if i >= self.lines.len() {
                break;
            }
            let sy = area.y + y as u16;
            let num_style = if i == self.row {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            buf.set_string(
                area.x,
                sy,
                format!("{:>width$} ", i + 1, width = gw as usize - 1),
                num_style,
            );

            let fallback = vec![(Style::default(), self.lines[i].clone())];
            let segs = self.hl.line(i).unwrap_or(&fallback);
            let mut x = 0usize; // display column within the line
            for (style, text) in segs {
                for c in text.chars() {
                    let cw = char_w(c, x);
                    if x >= self.scroll_col && x + cw <= self.scroll_col + w {
                        let px = area.x + gw + (x - self.scroll_col) as u16;
                        if c == '\t' {
                            buf.set_string(px, sy, " ".repeat(cw), *style);
                        } else if cw > 0 {
                            if let Some(cell) = buf.cell_mut((px, sy)) {
                                cell.set_char(c).set_style(*style);
                            }
                        }
                    }
                    x += cw;
                }
            }
        }

        let vy = self.row.checked_sub(self.scroll_row).filter(|y| *y < h)?;
        let vx = cx.checked_sub(self.scroll_col).filter(|x| *x < w)?;
        Some((area.x + gw + vx as u16, area.y + vy as u16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ed(t: &str) -> Editor {
        Editor::from_text(Path::new("t.txt"), t)
    }
    #[test]
    fn roundtrip_endings() {
        for t in ["a\nb\n", "a\r\nb\r\n", "a\nb", "", "\n"] {
            assert_eq!(ed(t).text(), t);
        }
    }
    #[test]
    fn editing() {
        let mut e = ed("    foo\n");
        e.col = 7;
        e.newline(true);
        assert_eq!(e.lines, ["    foo", "    "]);
        e.insert_char('x');
        e.backspace();
        e.col = 0;
        e.backspace();
        assert_eq!(e.lines, ["    foo    "]);
        e.col = 7;
        e.delete();
        assert_eq!(e.lines, ["    foo   "]);
        e.paste("a\nb");
        assert_eq!(e.lines, ["    fooa", "b   "]);
        assert!(e.dirty);
    }
    #[test]
    fn columns() {
        assert_eq!(disp_col("\tab", 1), 4);
        assert_eq!(disp_col("日本", 2), 4);
        assert_eq!(col_at("日本x", 3), 1);
        assert_eq!(col_at("ab", 9), 2);
    }
}
