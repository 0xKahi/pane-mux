# Tasks

## 1. Neutral model module

- [x] 1.1 Create `src/mux/` (`mod.rs`, `model.rs`) and move `Tree/Session/Window/Pane/Current/Level/Target` out of `src/tmux/model.rs`, keeping float-only types in `src/tmux/`; verify `cargo build` succeeds
- [x] 1.2 Update imports in `cli.rs`, `commands/*`, `picker/*`, `picker/ui/*` and tests to use `crate::mux`; verify `just test` passes with no snapshot changes (`cargo insta test` reports nothing pending)

## 2. Trait split

- [x] 2.1 Check that the pinned toolchain in `rust-toolchain.toml` supports trait upcasting, and pick the `&dyn FloatMux` to `&dyn Mux` strategy; record the choice in a code comment on the trait
- [x] 2.2 Define `Mux` (core picker/switch/move methods) in `src/mux/backend.rs` and `FloatMux: Mux` (the 12 float primitives) in `src/tmux/`; implement both for `TmuxCli` and `FakeTmux`; verify `cargo build` succeeds
- [x] 2.3 Switch `commands/float.rs` to `&dyn FloatMux` and every other consumer to `&dyn Mux`; verify `just test` passes and `grep -rn "dyn Tmux" src` returns nothing

## 3. Backend-owned targets

- [x] 3.1 Make `Target` id-based and add `target_string` / `display_target` to `Mux`, with the tmux implementation reproducing today's `session`, `session:N` and `session:N.M` strings; add unit tests that compare them with the old `Target::at` outputs
- [x] 3.2 Make `Exit::Pick` carry `Target` + `Level` and format at the edge in `commands/picker.rs` and `commands/switch_session.rs`; route the preview title, cut badge and status line through `display_target`; verify all insta snapshots are unchanged and the state tests pass

## 4. Client context and construction

- [x] 4.1 Add `in_client()` and `suppressed_pane()` to `Mux`, implemented for `TmuxCli` from `TMUX` / `TMUX_PANE` and with setters on `FakeTmux`; replace the direct env reads in `picker/app.rs`, `commands/switch_session.rs` and `commands/move_pane.rs`; verify `tests/cli_errors.rs` passes unchanged
- [x] 4.2 Add `mux::open()` and use it in `main.rs` as the only backend construction point (tmux only for now); verify `grep -n "TmuxCli" src/main.rs` returns nothing and `cargo run -- --help` is unchanged

## 5. Integration check

- [x] 5.1 Run `just ci` (fmt, clippy, unit, snapshots, `PANE_MUX_TMUX_TESTS=1` integration) and confirm everything is green with no snapshot or schema diffs (`just schemas` leaves `git status` clean)

## Workflow follow-up

- Archive `backend-neutral-core` once merged, then start `add-rex-backend`.
