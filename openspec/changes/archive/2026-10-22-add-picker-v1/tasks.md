# Tasks

## 1. Rust project setup

- [x] 1.1 Run `cargo init --name pane-mux` at repo root (edition 2024, bin + `src/lib.rs`); verify `cargo run -- --help` builds and runs
- [x] 1.2 Add `rust-toolchain.toml` (stable, rustfmt, clippy) and `rustfmt.toml` (stable options only); verify `cargo fmt --check` passes
- [x] 1.3 Add `[lints]` to `Cargo.toml` (`clippy::all = "warn"`, `unsafe_code = "forbid"`); verify `cargo clippy -- -D warnings` passes on the skeleton
- [x] 1.4 Add dependencies (`clap` derive, `ratatui` with crossterm+serde, `crossterm` with `use-dev-tty`, `serde`, `toml`, `anyhow`, `ansi-to-tui`; dev `insta`); verify `cargo build` succeeds
- [x] 1.5 Add `justfile` with `fmt`, `lint`, `test`, `check` (fmt --check + clippy -D warnings + test) and ensure `/target` is gitignored and `Cargo.lock` committed; verify `just check` passes
- [x] 1.6 Create module skeleton per design D1 (`cli`, `commands`, `picker`, `tmux`, `config`) with empty modules; verify `cargo check` passes

## 2. tmux layer and spikes

- [x] 2.1 Spike: inside `tmux display-popup`, record whether `TMUX_PANE` is set/inherited; document result in design.md D7 and choose popup detection; verify by running a popup that prints `env | grep TMUX` (result: not set in popups)
- [x] 2.2 Spike: check whether `switch-client -t sess:win.pane` activates window and pane; document result in design.md D8 (result: yes, per `man tmux`; one call suffices)
- [x] 2.3 Implement `tmux/model.rs` (Session/Window/Pane/Tree, ids, `Target` with `at(level)` → `work` / `work:1` / `work:1.0`); verify unit tests for target formatting
- [x] 2.4 Implement `tmux/format.rs` parsing tab-separated `list-panes -a -F` output into a `Tree`; verify unit tests with fixture strings (multiple sessions, names with spaces)
- [x] 2.5 Implement `Tmux` trait + real `Command`-based client (tree, capture, current, new/rename/kill, switch_client) mapping non-zero exits to errors with stderr; verify opt-in integration test against `tmux -L pane-mux-test`
- [x] 2.6 Implement `FakeTmux` for tests; verify it is usable from a unit test that builds a tree and records mutations

## 3. Configuration

- [x] 3.1 Implement `config/paths.rs` resolution (`PANE_MUX_CONFIG_HOME` → `XDG_CONFIG_HOME/pane-mux` → `~/.config/pane-mux`, absolute-only overrides); verify unit tests for each branch
- [x] 3.2 Implement preset macro (debug reads from disk, release `include_str!`) and add `config-default.toml`; verify `cargo build --release` embeds presets (binary runs with no config dir)
- [x] 3.3 Implement `toml::Table` deep-merge and typed `Config` (`[picker] preview_ratio` 1-99, `preview_position` top|bottom) with validation; verify tests for partial override and invalid ratio error
- [x] 3.4 Implement `Key` parsing (string and crossterm `KeyEvent`), `Chord` (`on`/`run` one-or-many, `desc`) and `run` string → `Action` parsing; verify tests for `<C-u>`, `<PageUp>`, sequences, unknown key/command errors
- [x] 3.5 Implement keymap layer merge (`keymap` replace, `prepend_keymap`, `append_keymap`) and author `keymap-default.toml` (picker/input/confirm/help per exploration); verify tests for the three configuration-spec keymap scenarios
- [x] 3.6 Implement theme structs, `StyleDef` → ratatui `Style`, `[icons]`, and author `theme-default.toml` from the author's yazi theme; verify tests for partial theme override and custom icon
- [x] 3.7 Implement config loading entry point with error reporting (file path + message, exit 2); verify test with invalid TOML fixture via `PANE_MUX_CONFIG_HOME`
- [x] 3.8 Document config files, locations and defaults in `README.md`; verify examples parse by loading them in a test

## 4. Picker core (state and actions)

- [x] 4.1 Implement `State` (tree, focused column, cursor ids per column, preview scroll, popup states, message) and derived "hovered" / "preview target" logic per spec; verify unit tests for preview selection per focused column
- [x] 4.2 Implement navigation actions (`arrow` next/prev/top/bot/N%, `focus` prev/next with boundaries) with id-based cursors; verify unit tests including boundary scenarios
- [x] 4.3 Implement refresh with cursor re-resolution by id and clamping when items vanish; verify unit tests with `FakeTmux` removing hovered item
- [x] 4.4 Implement which state machine for multi-key chords (prefix → candidates → filter → dispatch/dismiss); verify tests for `g g`, `d d` and an invalid second key
- [x] 4.5 Implement `pick` producing `Target` at the output level (flag or focused column) and `quit` producing cancel; verify unit tests for `--session` from pane column and default-level behaviour
- [x] 4.6 Implement create (session/window via input, empty name → tmux default, no-op on panes) and rename (prefilled input); verify tests against `FakeTmux`
- [x] 4.7 Implement remove with confirm popup and refusal for current session/window/pane; verify tests for confirmed, declined and refused cases
- [x] 4.8 Surface tmux operation errors as on-screen messages without exiting; verify test with `FakeTmux` returning an error

## 5. Picker UI and event loop

- [x] 5.1 Implement layout (header, columns, preview area ordered by `preview_position`/`preview_ratio`, status line); verify `insta` snapshots for bottom/50 and top/75
- [x] 5.2 Render three columns with icons, hovered/unfocused/current styles; verify snapshot with a fake tree incl. current markers
- [x] 5.3 Render preview (`ansi-to-tui`, rounded border, title target, scroll offset, self-pane placeholder); verify snapshots for coloured content and placeholder
- [x] 5.4 Render popups: which, input, confirm, help, and error message; verify snapshots for each
- [x] 5.5 Implement terminal setup on stderr + `/dev/tty` input, restore on exit and on panic (panic hook); verify manually that `x=$(pane-mux picker)` shows the UI and the terminal is clean after `q` and after a forced panic
- [x] 5.6 Implement synchronous loop (poll keys with ~300 ms tick, refresh tree + preview, redraw only when dirty); verify manually that an external `tmux new-session -d -s extra` appears within ~1 s

## 6. Commands

- [x] 6.1 Implement `cli.rs` with `picker [--session|--window|--pane]` (mutually exclusive) and `switch-session`; verify `--session --pane` exits 2 with usage error
- [x] 6.2 Implement `main.rs` outcome mapping: pick → print target + exit 0, cancel → exit 130, error → stderr + exit 2; verify integration test with no tmux server (`TMUX_TMPDIR` temp dir) exits 2
- [x] 6.3 Implement `commands/picker.rs` (header `[Pick <level>]`, print target to stdout); verify manually `selected=$(pane-mux picker --session); echo "$selected"` prints only the name
- [x] 6.4 Implement `commands/switch_session.rs` (require `$TMUX`, header `[Switch]`, single `switch-client -t <target>`); verify outside-tmux error exits 2 and manual popup switch via `tmux display-popup -E "pane-mux switch-session"`
- [x] 6.5 Document commands, exit codes, scripting example and a `display-popup` tmux binding in `README.md`; verify commands in README run as written

## 7. Integration check

- [x] 7.1 Run `just check` and the opt-in tmux integration tests; verify all pass
- [x] 7.2 `cargo install --path .` and walk through every tmux-picker and session-switching spec scenario manually in a popup and in a regular pane; verify each scenario's THEN holds

## Workflow follow-up

- Discuss and set up the crates.io publishing pipeline after V1.
- Archive the change after review.
