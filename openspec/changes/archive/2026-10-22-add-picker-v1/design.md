# Design

## Context

Greenfield Rust crate (the author's first Rust project; prior CLIs were Bun/TypeScript). Requirements live in `specs/tmux-picker`, `specs/session-switching` and `specs/configuration`; motivation in proposal.md.

Two references shape the approach:
- **yazi** (Rust TUI file manager) for keymap/theme/config design and popup UX. Yazi is a 31-crate workspace with Lua plugins, tokio and a custom terminal backend; we borrow its *ideas*, not its architecture.
- **llw** (author's TypeScript CLI) for project conventions: noun/verb command layout, a single wrapper around the external CLI, errors mapped once at the top level, data on stdout / diagnostics on stderr.

Constraint: tmux 3.8 on macOS is the development target.

## Goals / Non-Goals

**Goals:**
- A single-crate binary that is easy for a Rust newcomer to navigate.
- The picker is a reusable library function so future commands (`float`, etc.) build on it.
- Every tmux interaction goes through one module that can be faked in tests.
- Rendering is snapshot-testable without a real terminal or tmux server.

**Non-Goals:**
- Async runtime, plugin system, Lua, workspace of multiple crates.
- Vim-mode text editing in the input prompt (insert-only in V1).
- tmux control mode (`tmux -C`) event streaming.
- Release pipeline / crates.io publishing.

## Decisions

### D1. Project layout: single crate, lib + bin
```
src/
  main.rs          parse CLI, call lib, map Result -> exit code (0 / 130 / 2)
  lib.rs
  cli.rs           clap derive: Picker { --session|--window|--pane }, SwitchSession
  commands/
    picker.rs      pick() -> print target
    switch_session.rs  require $TMUX, pick() -> tmux switch-client
  picker/          reusable TUI: app.rs (loop), state.rs, action.rs, which.rs, ui/*
  tmux/            client.rs (Tmux trait + real impl), model.rs, format.rs
  config/          paths.rs, merge.rs, config.rs, keymap/{key,chord,section}.rs, theme.rs
  config/preset/   config-default.toml, keymap-default.toml, theme-default.toml
```
`lib.rs` exists so integration tests in `tests/` can reach internals. *Alternative:* cargo workspace like yazi — rejected as overhead for one binary.

### D2. Picker as a function returning an outcome
`picker::run(opts) -> anyhow::Result<Option<Target>>` where `None` = cancelled. `Target { session, window_index, pane_index }` with `fn at(level) -> String` producing `work` / `work:1` / `work:1.0`. Commands decide what to do with it (print vs switch-client). `PickerOptions` carries the output level and a header label (`[Pick session]`, `[Switch]`).

### D3. UI on the tty, result on stdout
The TUI renders to **stderr** via `CrosstermBackend::new(stderr())`, and crossterm's `use-dev-tty` feature reads keys from `/dev/tty`, so `$(pane-mux picker)` captures only the final line. Terminal is restored (raw mode off, leave alternate screen) before anything is printed or exec'd, including on panic (install a panic hook that restores the terminal). *Alternative:* open `/dev/tty` explicitly for output (fzf style) — equivalent; stderr is simpler and well-trodden in ratatui.

### D4. Synchronous event loop, no tokio
```
loop {
  if dirty { terminal.draw(|f| ui::render(f, &state, &theme))?; dirty = false }
  if event::poll(tick_remaining)? { handle key -> Vec<Action> -> apply -> dirty = true }
  if tick elapsed { refresh tree + preview; dirty |= changed }
}
```
Tick ~300 ms. *Alternative:* tokio + channels (yazi) — adds async, `Send`/`Arc` complexity for no benefit at this scale; can be introduced later if `float` needs it.

### D5. tmux access through one trait
```rust
trait Tmux {
  fn tree(&self) -> Result<Tree>;                 // one list-panes -a -F call
  fn capture(&self, pane_id: &str) -> Result<String>; // capture-pane -e -p -J -t
  fn current(&self) -> Result<Option<Current>>;  // display -p session/window/pane ids
  fn new_session(&self, name: Option<&str>) -> Result<String>; // new-session -d -P [-s]; returns new id
  fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String>; // returns new id
  fn rename_session / rename_window / kill_session / kill_window / kill_pane
  fn switch_client(&self, target: &str) -> Result<()>;
}
```
The real impl calls `std::process::Command::new("tmux").args([...])` (args array, no shell), mapping non-zero exit to an error carrying stderr. The tree is built from a single `list-panes -a -F` with tab-separated fields (`session_id, session_name, window_id, window_index, window_name, window_active, pane_id, pane_index, pane_active, pane_current_command, ...`), so sessions/windows/panes are always consistent. Internally items are addressed by stable ids (`$3`, `@5`, `%12`); the printed target uses names/indexes. A `FakeTmux` backs unit and snapshot tests. *Alternative:* a tmux library crate — none are mature enough to justify the dependency.

### D6. Cursor stability across refresh
Cursor positions are stored as item ids, not indexes. After each refresh, the cursor re-resolves the id; if the item vanished, it clamps to the nearest index.

### D7. Preview rendering and self-suppression
`capture-pane -e -p -J -t <pane_id>` output is converted to styled `Text` with `ansi-to-tui` and rendered in a `Block` with `BorderType::Rounded`, title = target. Scrolling uses a line offset reset on target change. Self-suppression: if `$TMUX_PANE` is set and equals the previewed pane id, show a placeholder.

Verified on tmux 3.8 (task 2.1): inside `display-popup`, `TMUX` is set but `TMUX_PANE` is **not** set, and `tmux display -p '#{pane_id}'` returns the opening client's active pane. Therefore:
- `TMUX` set + `TMUX_PANE` set → running in a regular pane; suppress preview of `TMUX_PANE`.
- `TMUX` set + `TMUX_PANE` unset → running in a popup; suppress nothing.
- `TMUX` unset → outside tmux; suppress nothing.

The "current" session/window/pane (for markers and delete refusal) comes from `display -p '#{session_id} #{window_id} #{pane_id}'`, which resolves to the client's current items in both regular panes and popups.

### D8. switch-session
Requires `$TMUX`; otherwise error before opening the UI. The picked target string is passed to a single `switch-client -t <target>`. Per `man tmux` (switch-client), a target containing `:`, `.` or `%` changes session, window **and** pane, so no follow-up `select-window`/`select-pane` is needed; a bare session target switches to that session's current window. Inside a popup, `switch-client` without `-c` targets the popup's client.

### D9. Config: yazi-style presets, simpler merge
- **Paths** (`config/paths.rs`): `$PANE_MUX_CONFIG_HOME` → `$XDG_CONFIG_HOME/pane-mux` → `$HOME/.config/pane-mux`. Deliberately *not* `dirs::config_dir()` (returns `~/Library/Application Support` on macOS).
- **Presets** embedded like yazi's `config_preset!`: a small `macro_rules!` that in debug builds reads `src/config/preset/<name>-default.toml` from disk at runtime (edit TOML, rerun, no recompile) and in release uses `include_str!`.
- **Merge** for `config.toml` / `theme.toml`: parse preset and user file to `toml::Table`, recursive deep-merge (tables merge, other values replace), then `serde` deserialize into fully-typed structs. *Alternative:* yazi's `DeserializeOver` derive macro — rejected as proc-macro complexity for a first project; deep-merge gives identical semantics for our shapes.
- **Keymap**: per layer `{ keymap, prepend_keymap, append_keymap }`. User `keymap` replaces the preset list; final = `prepend ++ base.filter(not shadowed by prepend) ++ append.filter(not bound in base/prepend)`, where "bound" compares the `on` sequence prefix (yazi's rule).
- **Errors**: unknown keys/commands or type mismatches fail with path + message (serde `deny_unknown_fields` on user-facing structs), exit code 2.

### D10. Keymap engine
- `Key` parsed from strings (`j`, `<C-u>`, `<S-Tab>`, `<PageUp>`) and from crossterm `KeyEvent`, compared by value.
- `Chord { on: Vec<Key>, run: Vec<Action>, desc }`; `on`/`run` accept one-or-many.
- `run` strings parsed by splitting on whitespace into `name` + args (`arrow next`, `arrow -100%`, `seek 50%`). Unknown names are config errors.
- Layers: `picker`, `input`, `confirm`, `help`. Active layer priority: which > help > confirm > input > picker.
- Which state machine (ported from yazi `yazi-core/src/which/which.rs`): on a key that prefixes any multi-key chord, collect candidates; each subsequent key filters by `on[times]`; exactly one full match → dispatch; no match → dismiss.

### D11. Actions (V1)
`picker`: `escape`, `quit`, `arrow <next|prev|top|bot|N|N%>`, `focus <prev|next>`, `seek <N%>`, `pick`, `create`, `rename`, `remove`, `help`. `input`: `close [--submit]`, `backspace`, `kill-word`, `kill-bol`, character insertion is implicit. `confirm`: `close [--submit]`. `help`: `arrow`, `close`. One `enum Action` + one `match` in `apply()`; no actor/registry layer.

### D12. Theme
Structs mirror the default `theme.toml` sections: `[picker]` (border, header, session/window/pane row text styles), `[preview]` (border, title), `[icons]` (session/window/pane `{ text, fg }`), `[indicator]` (yazi-style `parent`/`current`/`preview` hovered-row styles by column position relative to focus; `padding { open, close }` caps, drawn only when the style has bg or reversed; `active { text, fg }` current-item marker), `[mode]`, `[status]`, `[which]`, `[confirm]`, `[input]`, `[help]`, `[notify]`. Colours via ratatui's `serde` feature (named, `#rrggbb`, `reset`); a small `StyleDef { fg, bg, bold, italic, underline, reversed, dim }` → `ratatui::style::Style`. Defaults taken from the author's yazi theme.

### D13. Layout
```
Layout::vertical([Length(1) header, Fill columns/preview area, Length(1) status])
area -> Layout::vertical([Percentage(100-r), Percentage(r)])  (swapped for "top")
columns -> Layout::horizontal([Ratio(1,3); 3])
```

### D14. Tooling
- `rust-toolchain.toml`: stable, components `rustfmt`, `clippy`. Edition 2024.
- `rustfmt.toml`: stable options only (default 4-space indent, `max_width = 100`).
- `Cargo.toml [lints]`: `clippy::all = warn`, `unsafe_code = forbid`; CI-equivalent `just check` runs `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.
- Tests: unit tests in-module (`#[cfg(test)]`), `insta` snapshots of `ratatui::backend::TestBackend` buffers with `FakeTmux`, and a small set of opt-in integration tests against a real tmux server on a private socket (`tmux -L pane-mux-test`) gated behind an env var.
- Dependencies: `clap` (derive), `ratatui` (crossterm + serde features), `crossterm` (`use-dev-tty`), `serde`, `toml`, `anyhow`, `ansi-to-tui`; dev: `insta`.

## Risks / Trade-offs

- [Popup detection relies on tmux not exporting `TMUX_PANE` to popups (verified on 3.8)] → keep detection in one function so it can change if tmux behaviour changes.
- [Polling every ~300 ms costs two `tmux` spawns per tick] → acceptable for V1 (tmux calls are ~2 ms); only capture the previewed pane; skip redraw when output unchanged.
- [Rendering to stderr breaks if user redirects stderr (`2>/dev/null`)] → detect non-tty stderr and fall back to opening `/dev/tty` for output.
- [Session names with special characters in printed targets] → tmux already sanitises `.`/`:` in session names; window/pane addressed by index, so targets are safe for `-t`.
- [Deep-merge cannot express "remove a default key"] → acceptable; keymap layers use explicit `keymap` replacement for that need.
- [Learning curve: borrow checker vs. mutable app state] → single owned `State` struct mutated only in `apply()`; render takes `&State`.

## Open Questions

- Exact default colours for new theme keys (`current`, preview border/title, icon colours) — cosmetic, can be tuned during implementation.
