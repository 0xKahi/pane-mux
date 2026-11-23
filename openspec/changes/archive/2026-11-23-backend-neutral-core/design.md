# Design

## Context

See proposal.md for motivation. Current coupling, from a recon of the tree:

- `src/tmux/client.rs:10-68`: `pub trait Tmux` has 27 methods and is consumed as `&dyn Tmux`. `TmuxCli` is the only real implementation and is built once at `src/main.rs:13`.
- About 15 methods serve `picker`, `switch-session` and `move-pane`. The other 12 are float-only (`float_window`, `window_layout`, `new_dummy_float`, `swap_and_kill`, `set_option`, `option`, `resize_window`, `toggle_zoom`, `select_pane`, `display_message`, `stash_window`, `socket_path`) and are used only by `src/commands/float.rs`.
- `src/tmux/model.rs` defines `Tree/Session/Window/Pane/Current/Level/Target`, and they are used outside `tmux::` by `cli.rs`, `picker/options.rs`, `picker/state.rs`, `picker/popup.rs` and `picker/ui/*`.
- `Target::at` (`model.rs:136-148`) builds `session:window.pane` from session name + window index + pane index. That string is the picker's stdout contract (`commands/picker.rs:29-30`), the argument to `switch_client(&str)`, and is shown in `ui/preview.rs:17`, `ui/mod.rs:49,95`.
- Environment gating reads `TMUX` / `TMUX_PANE` directly in `client.rs:86-127,448-457`, `picker/app.rs:17`, `commands/switch_session.rs:14` and `commands/move_pane.rs:35`.
- The picker draws itself on /dev/tty. It is not a tmux popup, so the display layer is already backend-agnostic.
- Tests: `FakeTmux` (`src/tmux/fake.rs`) simulates tmux semantics and records tmux-like call strings. About 24 insta snapshots embed `work:1.2`-style targets.

## Goals / Non-Goals

**Goals:**
- A second backend can implement one core trait without stubbing float primitives.
- Picker, commands and UI depend only on the neutral module and the core trait.
- Zero diff in snapshots, call-log assertions, CLI error text and exit codes.

**Non-Goals:**
- Adding Rex or any other backend, or backend detection. That is `add-rex-backend`.
- Renaming tmux-specific model fields for taste, or changing `FakeTmux` semantics.
- Changing the `float` command or `floating-pane-stash` behavior.

## Decisions

### D1. Neutral module `src/mux/` with `model.rs` and `backend.rs`
Move `Tree/Session/Window/Pane/Current/Level/Target` into `src/mux/model.rs`. `src/tmux/` keeps `client.rs` (TmuxCli), `format.rs`, `fake.rs` and the float-specific model (`FloatPane/FloatWindow/FloatGeometry`), because those are tmux-only. `src/tmux/mod.rs` may re-export from `mux` for a smaller diff while call sites migrate, but the end state imports from `mux` directly.
*Alternative:* keep everything in `tmux::` and add a `rex::` sibling. Rejected because Rex code would then depend on `tmux::model`, which is backwards.

### D2. Trait split: `Mux` (core) + `FloatMux` (tmux only)
`Mux` holds the 15 picker/switch/move methods, including the intent-level move operations (`move_pane_to_new_session`, `move_pane_to_new_window`, `float_pane_in_window`) and `switch_client`. `FloatMux: Mux` holds the 12 float primitives. `commands/float.rs` takes `&dyn FloatMux`. Everything else takes `&dyn Mux`. `FakeTmux` and `TmuxCli` implement both.
*Alternative:* one trait with default "unsupported" methods, as workmux does. Rejected because it hides missing support until runtime and makes the float boundary invisible.

### D3. The backend owns target strings
Add to `Mux`:
- `fn target_string(&self, tree: &Tree, target: &Target, level: Level) -> String`, used for picker stdout and as the argument to `switch_client`
- `fn display_target(&self, tree: &Tree, target: &Target, level: Level) -> String`, used in the UI (preview title, cut badge, status)

For tmux both return today's `Target::at` output. `Target` becomes id-based (session/window/pane ids plus the display fields it already carries), and `Exit::Pick` carries the `Target` + level instead of a pre-formatted string. Formatting happens at the edge (`commands/picker.rs`, `switch_session.rs`).
*Alternative:* keep `Target::at` and let Rex reinterpret `s:w.p`. Rejected because Rex labels are not unique and Rex has no positional block target grammar (verified with `rex block inspect`).

### D4. Client context via the trait
Add `fn in_client(&self) -> bool` (replaces direct `TMUX` checks) and `fn suppressed_pane(&self) -> Option<String>` (replaces `suppressed_pane_from_env`) to `Mux`. `TmuxCli` answers them from `TMUX` / `TMUX_PANE` exactly as today. `FakeTmux` gets setters. Error messages stay in the commands, worded as today.

### D5. One construction point
Add `mux::open() -> Result<Box<dyn FloatMux>>` (or an enum wrapper) called from `main.rs`. In this change it returns `TmuxCli::default()`. `add-rex-backend` extends it with detection. Commands that do not need floats receive it as `&dyn Mux`.

## Risks / Trade-offs

- [Wide but shallow diff touches most picker files] → Do it in mechanical steps (move model, split trait, targets, env), each ending with `just test` green and no snapshot changes.
- [`Exit::Pick` type change ripples into tests that assert on pick strings] → Keep a test helper that formats with the tmux backend, so assertions keep comparing the same strings.
- [Trait objects across a supertrait (`FloatMux: Mux`) need upcasting] → Pass `&dyn Mux` derived from the concrete type at the call site, or use the Rust 1.86+ trait upcasting pinned by `rust-toolchain.toml`. Check the toolchain version during task 2.

## Migration Plan

Internal refactor, no migration. Rollback is reverting the change.
