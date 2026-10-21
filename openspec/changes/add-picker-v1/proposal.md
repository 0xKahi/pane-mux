# Proposal

## Why

tmux sessions, windows and panes are hard to see and navigate as a whole: built-in `choose-tree` is rigid, unthemeable and cannot be composed into user scripts. A yazi-style, keyboard-driven picker that browses the full tmux tree with a live preview — and prints what was picked to stdout — gives a fast navigator plus a building block for shell scripts and future `pane-mux` commands (e.g. `float`).

## What Changes

- New Rust CLI binary `pane-mux` (installable via `cargo install`; crates.io publishing is out of scope for V1).
- New `pane-mux picker` command: full-screen TUI with three always-visible columns (sessions | windows | panes) and a persistent, rounded-border preview box of the relevant pane's contents.
  - Fully navigable across all levels; `--session | --window | --pane` only selects which level of the hovered item is printed on `<Enter>`.
  - Prints a tmux target (`work`, `work:1`, `work:1.0`) to stdout; the UI is drawn to the terminal, never to stdout, so `$(pane-mux picker)` works.
  - Can create sessions and windows, rename sessions and windows, and delete sessions, windows and panes (never the current one).
  - Preview position (`top`/`bottom`) and size ratio are configurable.
- New `pane-mux switch-session` command: runs the picker and switches the tmux client to whatever was picked; errors when not inside tmux.
- New yazi-style configuration: `config.toml`, `keymap.toml`, `theme.toml` under `~/.config/pane-mux/`, layered over defaults embedded in the binary, with `prepend_keymap` / `append_keymap` support.
- Rust project scaffolding: toolchain pin, formatter, linter, test setup and task runner.

Not in V1: creating panes, `--action`/attach behaviour, multi-select, filtering/search, vim-mode text input, `float`, crates.io release pipeline.

## Capabilities

### New Capabilities
- `tmux-picker`: interactive TUI for browsing, previewing, creating, renaming and deleting tmux sessions/windows/panes, and printing the picked target for scripting.
- `session-switching`: `switch-session` command that switches the current tmux client to a target chosen through the picker.
- `configuration`: discovery, layering and validation of user config, keymap and theme files over built-in defaults.

### Modified Capabilities
<!-- None: no existing specs. -->

## Impact

- New Rust crate at the repository root (`Cargo.toml`, `src/`, embedded preset TOML files).
- Runtime dependency on the `tmux` binary (developed against tmux 3.8).
- New crate dependencies: CLI parsing, TUI rendering, TOML/serde config, ANSI-to-styled-text conversion; snapshot testing as a dev dependency.
- Developer tooling: `rust-toolchain.toml`, `rustfmt.toml`, clippy lint config, `justfile`.
