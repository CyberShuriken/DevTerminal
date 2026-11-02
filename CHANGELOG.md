# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Desktop integration: `packaging/install.sh` installs a `devterm` app menu
  entry, icon and `devterm-launch` launcher, so devterm can be pinned to the
  dock like any terminal app.

## [0.3.0] - 2026-11-02

### Added
- `Alt+T` opens a new terminal tab in the file tree's current folder.
- Extra terminal tabs can be closed with `×` or `Alt+W`.
- The file tree follows whichever terminal tab is active.
- CI workflow (rustfmt, clippy, tests, release build).

## [0.2.0] - 2026-11-02

### Added
- Real interactive terminal (`$SHELL` in a PTY) with scrollback and bracketed paste.
- Editor tabs with syntax highlighting, line numbers, auto-indent and `Ctrl+S` save.
- Live file tree that follows the shell's `cd`.
- Mouse support, status bar, and `Alt`-based keybindings.

### Changed
- `fs.rs` replaced by `tree.rs`.

## [0.1.0]

### Added
- Two-pane layout with a working file browser.

[0.3.0]: https://github.com/CyberShuriken/DevTerminal/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/CyberShuriken/DevTerminal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/CyberShuriken/DevTerminal/releases/tag/v0.1.0
