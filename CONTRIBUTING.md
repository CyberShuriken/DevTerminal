# Contributing to devterm

Thanks for your interest! Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

Open an issue with:
- what you did, what you expected, and what happened;
- your OS, terminal emulator and `$SHELL`;
- `devterm` version (`Cargo.toml`) or commit.

## Making a change

1. Fork the repo and create a branch from `main`.
2. Make your change. Keep it focused, and add a test for new logic.
3. Run the same checks as CI:

   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. Add a line to `CHANGELOG.md` under an `Unreleased` heading.
5. Open a pull request describing the change and how you tested it.

## Project layout

See [How it works](README.md#how-it-works) in the README.
