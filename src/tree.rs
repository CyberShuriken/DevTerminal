//! The file-tree pane: a flat listing of one directory.

use std::path::{Path, PathBuf};

use ratatui::widgets::ListState;

pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct Tree {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub state: ListState,
}

/// ".." (when there is a parent), then folders, then files; case-insensitive.
/// Errors on unreadable directories so callers can keep the old listing.
fn read(dir: &Path) -> std::io::Result<Vec<Entry>> {
    let mut v: Vec<Entry> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| {
            let path = e.path();
            // metadata() follows symlinks, so a link to a folder counts as a folder.
            let is_dir = std::fs::metadata(&path)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            Entry {
                name: e.file_name().to_string_lossy().into(),
                path,
                is_dir,
            }
        })
        .collect();
    v.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    if let Some(parent) = dir.parent() {
        v.insert(
            0,
            Entry {
                name: "..".into(),
                path: parent.to_path_buf(),
                is_dir: true,
            },
        );
    }
    Ok(v)
}

impl Tree {
    pub fn new(dir: PathBuf) -> std::io::Result<Self> {
        let entries = read(&dir)?;
        Ok(Self {
            cwd: dir,
            entries,
            state: ListState::default().with_selected(Some(0)),
        })
    }

    /// Show `dir`. Returns false (and changes nothing) if it can't be read.
    pub fn set_dir(&mut self, dir: PathBuf) -> bool {
        let Ok(entries) = read(&dir) else {
            return false;
        };
        // Coming up out of a child folder: keep that folder selected.
        let sel = self
            .cwd
            .strip_prefix(&dir)
            .ok()
            .and_then(|rel| rel.components().next())
            .and_then(|c| {
                entries
                    .iter()
                    .position(|e| e.name == c.as_os_str().to_string_lossy() && e.name != "..")
            })
            .unwrap_or(0);
        self.entries = entries;
        self.cwd = dir;
        self.state = ListState::default().with_selected(Some(sel));
        true
    }

    pub fn refresh(&mut self) {
        let sel = self.state.selected();
        if let Ok(entries) = read(&self.cwd) {
            self.entries = entries;
            self.state
                .select(sel.map(|s| s.min(self.entries.len().saturating_sub(1))));
        }
    }

    pub fn go_up(&mut self) {
        if let Some(p) = self.cwd.parent() {
            self.set_dir(p.to_path_buf());
        }
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0) as isize + delta;
        self.state
            .select(Some(i.clamp(0, self.entries.len() as isize - 1) as usize));
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.state.selected()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn order_and_up() {
        let d = std::env::temp_dir().join(format!("devterm-tree-{}", std::process::id()));
        std::fs::create_dir_all(d.join("Beta")).unwrap();
        std::fs::create_dir_all(d.join("alpha")).unwrap();
        std::fs::write(d.join("Zed.txt"), "").unwrap();
        std::fs::write(d.join("apple.txt"), "").unwrap();
        let mut t = Tree::new(d.clone()).unwrap();
        let names: Vec<_> = t.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["..", "alpha", "Beta", "apple.txt", "Zed.txt"]);
        assert!(t.set_dir(d.join("Beta")));
        t.go_up();
        assert_eq!(t.selected().unwrap().name, "Beta");
        assert!(!t.set_dir(d.join("missing")));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
