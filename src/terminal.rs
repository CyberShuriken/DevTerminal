//! A real shell running in a PTY, rendered from a vt100 screen model.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

const SCROLLBACK: usize = 5000;

pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    dirty: Arc<AtomicBool>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    size: (u16, u16),
    pub exited: bool,
}

fn other(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

impl Terminal {
    pub fn spawn(cwd: &Path, rows: u16, cols: u16) -> io::Result<Self> {
        let (rows, cols) = (rows.max(2), cols.max(2));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(other)?;
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/bin/bash".into());
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("DEVTERM", "1");
        let child = pair.slave.spawn_command(cmd).map_err(other)?;
        // Drop the slave so the reader sees EOF when the shell exits.
        drop(pair.slave);

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, SCROLLBACK)));
        let dirty = Arc::new(AtomicBool::new(true));
        let mut reader = pair.master.try_clone_reader().map_err(other)?;
        let writer = pair.master.take_writer().map_err(other)?;
        {
            let (parser, dirty) = (parser.clone(), dirty.clone());
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if let Ok(mut p) = parser.lock() {
                                p.process(&buf[..n]);
                            }
                            dirty.store(true, Ordering::Release);
                        }
                    }
                }
                dirty.store(true, Ordering::Release);
            });
        }
        Ok(Self {
            parser,
            dirty,
            master: pair.master,
            writer,
            child,
            size: (rows, cols),
            exited: false,
        })
    }

    /// True (once) if new output arrived or the shell's state changed.
    pub fn take_dirty(&mut self) -> bool {
        let mut d = self.dirty.swap(false, Ordering::AcqRel);
        if !self.exited && matches!(self.child.try_wait(), Ok(Some(_))) {
            self.exited = true;
            d = true;
        }
        d
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// The shell's current directory, read from /proc (Linux only).
    pub fn cwd(&self) -> Option<PathBuf> {
        std::fs::read_link(format!("/proc/{}/cwd", self.pid()?)).ok()
    }

    pub fn write(&mut self, bytes: &[u8]) {
        if self.exited {
            return;
        }
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn application_cursor(&self) -> bool {
        self.parser
            .lock()
            .map(|p| p.screen().application_cursor())
            .unwrap_or(false)
    }

    pub fn paste(&mut self, text: &str) {
        let bracketed = self
            .parser
            .lock()
            .map(|p| p.screen().bracketed_paste())
            .unwrap_or(false);
        if bracketed {
            let mut v = b"\x1b[200~".to_vec();
            v.extend_from_slice(text.as_bytes());
            v.extend_from_slice(b"\x1b[201~");
            self.write(&v);
        } else {
            self.write(text.replace("\r\n", "\r").replace('\n', "\r").as_bytes());
        }
    }

    /// Scroll the view into scrollback; positive = up (older).
    pub fn scroll(&mut self, delta: isize) {
        if let Ok(mut p) = self.parser.lock() {
            let cur = p.screen().scrollback() as isize;
            p.set_scrollback((cur + delta).max(0) as usize);
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        if let Ok(mut p) = self.parser.lock() {
            if p.screen().scrollback() != 0 {
                p.set_scrollback(0);
            }
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = (rows.max(2), cols.max(2));
        if self.size == (rows, cols) {
            return;
        }
        self.size = (rows, cols);
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
    }

    /// Draw into `area`; returns the absolute cursor position if it should be shown.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) -> Option<(u16, u16)> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        self.resize(area.height, area.width);
        let p = self.parser.lock().ok()?;
        let screen = p.screen();
        for row in 0..area.height {
            for col in 0..area.width {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut style = Style::default()
                    .fg(color(cell.fgcolor()))
                    .bg(color(cell.bgcolor()));
                let mut m = Modifier::empty();
                if cell.bold() {
                    m |= Modifier::BOLD;
                }
                if cell.italic() {
                    m |= Modifier::ITALIC;
                }
                if cell.underline() {
                    m |= Modifier::UNDERLINED;
                }
                if cell.inverse() {
                    m |= Modifier::REVERSED;
                }
                style = style.add_modifier(m);
                let contents = cell.contents();
                let sym = if contents.is_empty() {
                    " "
                } else {
                    contents.as_str()
                };
                if let Some(c) = buf.cell_mut((area.x + col, area.y + row)) {
                    c.set_symbol(sym).set_style(style);
                }
            }
        }
        if self.exited {
            let msg = "[shell exited — press Enter to restart]";
            buf.set_string(
                area.x,
                area.y + area.height - 1,
                msg,
                Style::default().fg(Color::Yellow),
            );
            return None;
        }
        if screen.hide_cursor() || screen.scrollback() > 0 {
            return None;
        }
        let (r, c) = screen.cursor_position();
        (r < area.height && c < area.width).then_some((area.x + c, area.y + r))
    }
}

fn color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.try_wait();
    }
}
