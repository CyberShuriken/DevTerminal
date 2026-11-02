//! Translate crossterm key events into the bytes an xterm would send.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn to_bytes(key: KeyEvent, app_cursor: bool) -> Vec<u8> {
    let m = key.modifiers;
    let alt = m.contains(KeyModifiers::ALT);
    let ctrl = m.contains(KeyModifiers::CONTROL);
    // xterm modifier parameter: 1 + shift(1) + alt(2) + ctrl(4)
    let modp = 1 + m.contains(KeyModifiers::SHIFT) as u8 + 2 * alt as u8 + 4 * ctrl as u8;

    // CSI-style key with final letter: arrows, Home, End, F1-F4.
    let letter = |ch: char, ss3: bool| -> Vec<u8> {
        if modp > 1 {
            format!("\x1b[1;{modp}{ch}").into_bytes()
        } else if ss3 {
            format!("\x1bO{ch}").into_bytes()
        } else {
            format!("\x1b[{ch}").into_bytes()
        }
    };
    // ESC [ n ~ keys.
    let tilde = |n: u8| -> Vec<u8> {
        if modp > 1 {
            format!("\x1b[{n};{modp}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };

    let mut out = match key.code {
        KeyCode::Char(c) => {
            let mut v = Vec::new();
            if alt {
                v.push(0x1b);
            }
            if ctrl {
                match c.to_ascii_lowercase() {
                    l @ 'a'..='z' => v.push(l as u8 - b'a' + 1),
                    ' ' | '@' | '2' => v.push(0),
                    '[' | '3' => v.push(0x1b),
                    '\\' | '4' => v.push(0x1c),
                    ']' | '5' => v.push(0x1d),
                    '^' | '6' => v.push(0x1e),
                    '_' | '7' | '/' => v.push(0x1f),
                    '8' | '?' => v.push(0x7f),
                    _ => v.extend_from_slice(c.to_string().as_bytes()),
                }
            } else {
                v.extend_from_slice(c.to_string().as_bytes());
            }
            return v;
        }
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => {
            if ctrl {
                vec![0x08]
            } else {
                vec![0x7f]
            }
        }
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Up => return letter('A', app_cursor),
        KeyCode::Down => return letter('B', app_cursor),
        KeyCode::Right => return letter('C', app_cursor),
        KeyCode::Left => return letter('D', app_cursor),
        KeyCode::Home => return letter('H', app_cursor),
        KeyCode::End => return letter('F', app_cursor),
        KeyCode::Insert => return tilde(2),
        KeyCode::Delete => return tilde(3),
        KeyCode::PageUp => return tilde(5),
        KeyCode::PageDown => return tilde(6),
        KeyCode::F(n @ 1..=4) => return letter((b'P' + n - 1) as char, true),
        KeyCode::F(n @ 5..=12) => return tilde([15, 17, 18, 19, 20, 21, 23, 24][n as usize - 5]),
        _ => return Vec::new(),
    };
    if alt {
        out.insert(0, 0x1b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn k(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, m)
    }
    #[test]
    fn encodes() {
        assert_eq!(
            to_bytes(k(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            [3]
        );
        assert_eq!(
            to_bytes(k(KeyCode::Char('x'), KeyModifiers::ALT), false),
            b"\x1bx"
        );
        assert_eq!(
            to_bytes(k(KeyCode::Up, KeyModifiers::NONE), false),
            b"\x1b[A"
        );
        assert_eq!(
            to_bytes(k(KeyCode::Up, KeyModifiers::NONE), true),
            b"\x1bOA"
        );
        assert_eq!(
            to_bytes(k(KeyCode::Right, KeyModifiers::CONTROL), true),
            b"\x1b[1;5C"
        );
        assert_eq!(
            to_bytes(k(KeyCode::F(5), KeyModifiers::NONE), false),
            b"\x1b[15~"
        );
        assert_eq!(
            to_bytes(k(KeyCode::F(1), KeyModifiers::NONE), false),
            b"\x1bOP"
        );
        assert_eq!(
            to_bytes(k(KeyCode::Backspace, KeyModifiers::NONE), false),
            [0x7f]
        );
    }
}
