# Proposal

## Why

pane-mux reaches tmux through a 27-method `Tmux` trait and a model and target syntax that live under `tmux::`. Supporting a second terminal backend (Rex, planned in `add-rex-backend`) would currently mean changing the picker, the commands and the tests all at once. A refactor-only change that pulls the backend seam out of the tmux module first keeps the Rex work small and keeps the existing FakeTmux test suite green throughout.

## What Changes

- Move the backend-neutral model (`Tree`, `Session`, `Window`, `Pane`, `Current`, `Level`, `Target`) out of `src/tmux/` into a neutral backend module. Picker, CLI, UI and config code import it from there.
- Split the `Tmux` trait into a **core** trait (tree, capture, current, resolve, create, rename, kill, move, switch: what `picker`, `switch-session` and `move-pane` need) and a **float** trait covering the 12 float-only primitives. Only tmux implements the float trait. `pane-mux float` keeps working against tmux unchanged.
- Delegate target formatting and parsing (today `Target::at` produces `session:window.pane`) to the backend. Picker output, preview title, cut badge and status line ask the backend for the string, and tmux keeps producing the same strings.
- Route the "inside a client" checks (`TMUX` / `TMUX_PANE` in `switch_session.rs`, `move_pane.rs`, `client.rs`, `app.rs`) through the core trait, so commands stop reading tmux environment variables directly.
- Build the backend in one factory, called from `main.rs`. In this change the factory always returns tmux.
- No user-visible behavior changes: same output, same error messages, same exit codes, same config. Snapshots and call-log assertions stay unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is an internal refactor with no spec-level behavior change (`skip_specs: true`).

## Impact

- Code: `src/tmux/*` (split into a neutral backend module plus a tmux implementation), `src/main.rs`, `src/cli.rs`, `src/commands/*`, `src/picker/{state.rs,app.rs,popup.rs,options.rs,ui/*}`, test imports in `src/picker/state/*tests.rs`, `src/picker/ui/tests.rs` and `src/commands/float.rs` tests.
- No dependency, config, schema or CLI changes.
- Unblocks `add-rex-backend`.
