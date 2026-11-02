# devterm

A small, fast **two-pane terminal UI** written in Rust: a shell on the left, a
file browser on the right — the layout of a code editor, in a single terminal.

Built with [ratatui](https://ratatui.rs) + crossterm. Compiles to one static
binary, no runtime.

> **Status: Milestone 1.** The layout, pane focus, and file browser work. The
> left pane is currently a placeholder — Milestone 2 turns it into a real
> interactive shell (see [Roadmap](#roadmap)).

## Features (Milestone 1)

- Two-column layout — file browser pane and (placeholder) shell pane.
- Working file browser: navigate directories, see dirs-first sorted listings.
- Focus switching between panes; the focused pane is highlighted.
- Starts in your current working directory.

## Build & run

Requires a Rust toolchain ([rustup](https://rustup.rs)).

```bash
git clone https://github.com/YOUR_USERNAME/devterm
cd devterm
cargo run            # debug
cargo build --release && ./target/release/devterm   # optimized
```

## Keys

| Key | Action |
| --- | --- |
| `Tab` | switch focused pane |
| `q` | quit |
| `j` / `k` (or ↓ / ↑) | move selection in the file pane |
| `Enter` / `l` / → | open the selected directory |
| `Backspace` / `h` / ← | go up one directory |

## Project layout

```
src/
  main.rs   terminal setup/teardown + event loop
  app.rs    application state, focus, input handling
  ui.rs     rendering (the two-column layout)
  fs.rs     directory reading + sorting
```

## Roadmap

- **Milestone 2 — real shell pane.** Run an interactive shell in the left pane
  through a PTY, using [`portable-pty`](https://crates.io/crates/portable-pty) +
  [`vt100`](https://crates.io/crates/vt100) +
  [`tui-term`](https://crates.io/crates/tui-term) so programs like `vim` and
  `htop` work.
- **Live directory sync.** When you `cd` in the shell, update the file pane to
  match.
- Open files in `$EDITOR`, previews, resizable split, config file.

## License

MIT — see [LICENSE](LICENSE).
