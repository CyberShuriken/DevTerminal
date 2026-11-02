# devterm

A small, fast **terminal laid out like a code editor**, written in Rust: a real
shell and editor tabs on the left, a live file tree on the right, a status bar
at the bottom — all inside one terminal window.

Built with [ratatui](https://ratatui.rs) + crossterm,
[portable-pty](https://crates.io/crates/portable-pty) +
[vt100](https://crates.io/crates/vt100) for the shell, and
[syntect](https://crates.io/crates/syntect) (pure-Rust regex, no C deps) for
syntax highlighting.

> **Status: v0.3.** Linux only for now (live cwd sync reads `/proc`).

## Features

- **Real terminal tab** — tab 1 runs your `$SHELL` (fallback `/bin/bash`) in a
  PTY. `vim`, `htop`, `git`, `cd` all work. 5000 lines of scrollback (mouse
  wheel). It can't be closed; if the shell exits, press Enter to restart it.
- **More terminals** — `Alt+T` opens another shell tab in the tree's current
  folder. Extra terminal tabs close with `×` or `Alt+W` (which kills that
  shell). Each shell keeps its own history and output.
- **Editor tabs** — click a file in the tree to open it as a tab. Syntax
  highlighting (`base16-ocean.dark`), line-number gutter, auto-indent, smart
  Home, `Ctrl+S` to save, `●` marks unsaved changes, `×` closes the tab.
  Line endings (LF/CRLF) and the trailing newline are preserved. Binary,
  non-UTF-8 and >8MB files are refused with a status message.
- **File tree** — `..`, then folders, then files (case-insensitive). Unreadable
  folders are ignored; going up keeps the folder you came from selected.
- **Live sync** — `cd` in a shell and the tree follows (polls
  `/proc/<shell pid>/cwd` every ~400ms). The tree follows whichever terminal
  tab is active; switching to an editor tab leaves it where it is.
- **Mouse** — click tree entries, tabs, `×`, or inside the editor to place the
  cursor; wheel-scroll the tree, editor and terminal scrollback.

> **Selecting text:** devterm captures the mouse, so hold **Shift** while
> dragging to select/copy text with your terminal emulator.

## Build & run

```bash
cargo build --release
./target/release/devterm            # start in the current directory
./target/release/devterm ~/project  # or in a given directory
```

## Keys

Global keys use `Alt` so they don't steal keys your shell needs.

| Key | Action |
| --- | --- |
| `Ctrl+Q` | quit (press twice if any file is unsaved) |
| `Alt+E` | toggle focus: left pane ⇄ file tree |
| `Alt+←` / `Alt+→` | previous / next tab |
| `Alt+1` … `Alt+9` | jump to tab |
| `Alt+T` | open a new terminal tab |
| `Alt+W` | close the current tab (press twice if the file is unsaved; tab 1 can't be closed) |
| `Ctrl+S` | save (editor tabs; in the terminal tab it goes to the shell) |

**File tree:** `j`/`k` or ↑/↓ move, `Enter`/`l`/→ open, `h`/←/`Backspace` go up.
Opening a file focuses its tab.

**Terminal tab:** every other key goes to the shell; typing snaps scrollback
back to the bottom.

**Editor:** arrows (with remembered column), `Home` (smart) / `End`,
`PgUp`/`PgDn`, `Ctrl+Home`/`Ctrl+End`, `Tab` (tab or spaces, matching the
file), `Enter` (auto-indent), `Backspace`/`Delete` (join lines), paste.

## Project layout

```
src/
  main.rs       terminal setup/teardown, panic hook, event loop
  app.rs        state, focus, tabs, keybindings, mouse routing
  ui.rs         layout: tab bar + content | file tree, status bar
  terminal.rs   $SHELL in a PTY, vt100 screen → ratatui buffer
  keys.rs       key events → xterm byte sequences
  editor.rs     text buffer, editing, rendering
  highlight.rs  incremental syntect highlighting cache
  tree.rs       directory listing for the file tree
```

## Roadmap

- Forward mouse events to full-screen terminal apps that request them
  (vim/htop mouse mode).
- Undo/redo, search, selection and clipboard in the editor.
- Tree: expandable folders, create/rename/delete files.
- Resizable split, config file, macOS support for cwd sync.

## License

MIT — see [LICENSE](LICENSE).
