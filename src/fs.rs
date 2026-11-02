//! Filesystem helpers for the file-browser pane.

use std::io;
use std::path::{Path, PathBuf};

/// A single entry shown in the file pane.
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Read the entries of `dir`, sorted directories-first then alphabetically
/// (case-insensitive). Unreadable individual entries are skipped rather than
/// aborting the whole listing.
pub fn read_dir_entries(dir: &Path) -> io::Result<Vec<DirEntry>> {
    let mut entries: Vec<DirEntry> = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(DirEntry { name, path, is_dir });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}
