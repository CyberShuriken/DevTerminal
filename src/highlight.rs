//! Incremental syntect highlighting with a per-line state cache.

use std::path::Path;
use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use syntect::highlighting::{FontStyle, HighlightIterator, HighlightState, Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

struct Sets {
    ss: SyntaxSet,
    theme: Theme,
}

fn sets() -> &'static Sets {
    static S: OnceLock<Sets> = OnceLock::new();
    S.get_or_init(|| {
        let mut ts = ThemeSet::load_defaults();
        Sets {
            ss: SyntaxSet::load_defaults_newlines(),
            theme: ts.themes.remove("base16-ocean.dark").unwrap_or_default(),
        }
    })
}

pub type Segments = Vec<(Style, String)>;

pub struct Highlight {
    syntax: &'static SyntaxReference,
    /// states[i] = parser/highlighter state at the start of line i.
    states: Vec<(ParseState, HighlightState)>,
    lines: Vec<Segments>,
}

impl Highlight {
    pub fn new(path: &Path, first_line: &str) -> Self {
        let ss = &sets().ss;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let syntax = ss
            .find_syntax_by_extension(ext)
            .or_else(|| ss.find_syntax_by_extension(name))
            .or_else(|| ss.find_syntax_by_first_line(first_line))
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        Self { syntax, states: Vec::new(), lines: Vec::new() }
    }

    pub fn name(&self) -> &str {
        &self.syntax.name
    }

    /// Forget everything from `row` on (the state at the start of `row` stays valid).
    pub fn invalidate(&mut self, row: usize) {
        self.lines.truncate(row);
        self.states.truncate(row + 1);
    }

    /// Highlight lines up to (exclusive) `upto`.
    pub fn ensure(&mut self, text: &[String], upto: usize) {
        let s = sets();
        let hl = Highlighter::new(&s.theme);
        if self.states.is_empty() {
            self.states.push((ParseState::new(self.syntax), HighlightState::new(&hl, ScopeStack::new())));
        }
        while self.lines.len() < upto.min(text.len()) {
            let i = self.lines.len();
            let (mut ps, mut hs) = self.states[i].clone();
            let line = format!("{}\n", text[i]);
            let segs = match ps.parse_line(&line, &s.ss) {
                Ok(ops) => HighlightIterator::new(&mut hs, &ops, &line, &hl)
                    .map(|(st, t)| (convert(st), t.trim_end_matches('\n').to_string()))
                    .filter(|(_, t)| !t.is_empty())
                    .collect(),
                Err(_) => vec![(Style::default(), text[i].clone())],
            };
            self.lines.push(segs);
            self.states.push((ps, hs));
        }
    }

    pub fn line(&self, i: usize) -> Option<&Segments> {
        self.lines.get(i)
    }
}

fn convert(st: syntect::highlighting::Style) -> Style {
    let f = st.foreground;
    let mut s = Style::default().fg(Color::Rgb(f.r, f.g, f.b));
    if st.font_style.contains(FontStyle::BOLD) { s = s.add_modifier(Modifier::BOLD); }
    if st.font_style.contains(FontStyle::ITALIC) { s = s.add_modifier(Modifier::ITALIC); }
    if st.font_style.contains(FontStyle::UNDERLINE) { s = s.add_modifier(Modifier::UNDERLINED); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_and_caches() {
        let mut h = Highlight::new(Path::new("a.rs"), "");
        assert_eq!(h.name(), "Rust");
        let text = vec!["fn main() {}".to_string(), "// hi".to_string()];
        h.ensure(&text, 1);
        assert_eq!(h.lines.len(), 1);
        h.ensure(&text, 10);
        assert_eq!(h.lines.len(), 2);
        let joined: String = h.line(0).unwrap().iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(joined, "fn main() {}");
        h.invalidate(1);
        assert_eq!((h.lines.len(), h.states.len()), (1, 2));
        assert_eq!(Highlight::new(Path::new("x"), "#!/bin/bash").name(), "Bourne Again Shell (bash)");
    }
}
