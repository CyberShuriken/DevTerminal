//! The file-tree pane: one directory, with folders expandable in place.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ratatui::widgets::ListState;

#[derive(PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// Nesting level below the tree's root folder (0 = top level).
    pub depth: usize,
}

pub struct Tree {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub state: ListState,
    /// Folders currently expanded in place (absolute paths).
    pub expanded: HashSet<PathBuf>,
}

/// Folders, then files, case-insensitive; expanded folders list their
/// children right below them. Errors only if `dir` itself is unreadable.
fn read_into(
    dir: &Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    out: &mut Vec<Entry>,
) -> std::io::Result<()> {
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
                depth,
            }
        })
        .collect();
    v.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    for e in v {
        let open = e.is_dir && expanded.contains(&e.path);
        let path = e.path.clone();
        out.push(e);
        if open {
            // An unreadable subfolder just shows no children.
            let _ = read_into(&path, depth + 1, expanded, out);
        }
    }
    Ok(())
}

/// ".." (when there is a parent), then the listing of `dir`.
fn read(dir: &Path, expanded: &HashSet<PathBuf>) -> std::io::Result<Vec<Entry>> {
    let mut v = Vec::new();
    if let Some(parent) = dir.parent() {
        v.push(Entry {
            name: "..".into(),
            path: parent.to_path_buf(),
            is_dir: true,
            depth: 0,
        });
    }
    read_into(dir, 0, expanded, &mut v)?;
    Ok(v)
}

impl Tree {
    pub fn new(dir: PathBuf) -> std::io::Result<Self> {
        let expanded = HashSet::new();
        let entries = read(&dir, &expanded)?;
        Ok(Self {
            cwd: dir,
            entries,
            state: ListState::default().with_selected(Some(0)),
            expanded,
        })
    }

    /// Show `dir`. Returns false (and changes nothing) if it can't be read.
    pub fn set_dir(&mut self, dir: PathBuf) -> bool {
        let Ok(entries) = read(&dir, &self.expanded) else {
            return false;
        };
        // Coming up out of a child folder: keep that folder selected.
        let sel = self
            .cwd
            .strip_prefix(&dir)
            .ok()
            .and_then(|rel| rel.components().next())
            .and_then(|c| {
                let child = dir.join(c);
                entries
                    .iter()
                    .position(|e| e.path == child && e.name != "..")
            })
            .unwrap_or(0);
        self.entries = entries;
        self.cwd = dir;
        self.state = ListState::default().with_selected(Some(sel));
        true
    }

    /// Re-read from disk, keeping the selected path selected.
    /// Returns true if the listing changed.
    pub fn refresh(&mut self) -> bool {
        let Ok(entries) = read(&self.cwd, &self.expanded) else {
            return false;
        };
        if entries == self.entries {
            return false;
        }
        let sel = self.selected().map(|e| (e.path.clone(), e.name == ".."));
        let idx = sel
            .and_then(|(p, up)| {
                entries
                    .iter()
                    .position(|e| e.path == p && (e.name == "..") == up)
            })
            .or(self.state.selected())
            .map(|s| s.min(entries.len().saturating_sub(1)));
        self.entries = entries;
        self.state.select(idx);
        true
    }

    /// Expand or collapse the folder at `i` in place.
    pub fn toggle(&mut self, i: usize) {
        let Some(e) = self.entries.get(i).filter(|e| e.is_dir && e.name != "..") else {
            return;
        };
        let p = e.path.clone();
        if !self.expanded.remove(&p) {
            self.expanded.insert(p);
        }
        self.refresh();
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

    #[test]
    fn expand_and_refresh() {
        let d = std::env::temp_dir().join(format!("devterm-tree-x-{}", std::process::id()));
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("sub/inner.txt"), "").unwrap();
        let mut t = Tree::new(d.clone()).unwrap();
        t.toggle(1); // "sub"
        let names: Vec<_> = t
            .entries
            .iter()
            .map(|e| (e.name.as_str(), e.depth))
            .collect();
        assert_eq!(names, [("..", 0), ("sub", 0), ("inner.txt", 1)]);
        // A file created behind our back shows up on refresh.
        std::fs::write(d.join("new.txt"), "").unwrap();
        assert!(t.refresh());
        assert!(t.entries.iter().any(|e| e.name == "new.txt"));
        assert!(!t.refresh());
        t.toggle(1);
        assert_eq!(t.entries.len(), 3); // "..", "sub", "new.txt"
        std::fs::remove_dir_all(&d).unwrap();
    }
}
