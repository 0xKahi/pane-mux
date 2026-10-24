# Design

## Context

- The picker state lives in `src/picker/state.rs`. Actions are parsed from `run` strings (`src/config/keymap/run.rs`) into `Action` (`src/picker/action.rs`). Modal prompts are `Popup::Input { purpose: InputPurpose }` (`src/picker/popup.rs`) and are executed in `State::submit`. `create` already shows the target pattern: prompt, tmux call, `refresh`, then `select` the created id.
- All tmux access goes through the `Tmux` trait (`src/tmux/client.rs`), with a `FakeTmux` for unit tests (`src/tmux/fake.rs`). The tree comes from one `list-panes -a -F` query (`src/tmux/format.rs`).
- Commands (`src/commands/*.rs`) build `PickerOptions` and interpret `picker::run`'s result. `switch-session` is the closest model for `move-pane`.
- The reference behaviour is `~/dotfiles/tmux/scripts/move-pane-to-session.sh`: detached moves (`-d`), a placeholder window for a new session (killed afterwards, and the session killed if the move fails), and `join-pane` then `break-pane -W` to float.
- tmux 3.8: `break-pane -W` floats a pane in its own window and takes `-x/-y` sizes (with `%`) but no named position and no `-A` (always on top). Named positions exist only on `move-pane -P`. `break-pane -W` fails on a zoomed window or an already floating pane. A floating pane can still be the source of every paste: `break-pane` (to a new window or session) and `join-pane` (into another window) both accept it and leave it tiled, so the `break-pane -W` that follows a `join-pane` never sees a floating pane. The script does not check for floating panes either. The format variable `pane_floating_flag` exists.
- Depends on `lenient-config-schemas`: new config and theme keys must also be regenerated into `schemas/`.

## Goals / Non-Goals

**Goals:**
- One cut-buffer model shared by `picker` and `move-pane`, with the mode differences (lock, exit after paste) as options rather than separate code paths.
- Every tmux sequence for a paste is a single `Tmux` trait method, so it is testable with `FakeTmux` and rolls back in one place.

**Non-Goals:**
- Cutting sessions or windows, or cutting several panes at once.
- Copy (duplicating a pane is not a tmux concept).
- Moving into an existing window as a tiled split. Pane-column paste is always floating.
- Keeping the client on the moved pane (`switch-client` after the move).
- Reading the `move-pane` target from stdin.

## Decisions

### D1. Cut buffer in `State`, configured by `PickerOptions`
Add `cut: Option<String>` (a pane id) to `State`, and two fields to `PickerOptions`: `cut: Option<String>` (the initial buffer) and `cut_locked: bool`. `pane-mux picker` and `switch-session` pass `None/false`, and `move-pane` passes `Some(id)/true`. Tracking the id, not the target, keeps the marker on the pane through moves and renumbering. `refresh` clears `cut` when the id is missing from the new tree and sets the "no longer exists" message.
- *Alternative:* a separate `MoveState` wrapper for move-pane. Rejected: it would duplicate key handling, and the differences are only a lock and an exit rule.

### D2. New actions `Cut` and `Paste`, and new prompt purposes
`Action::Cut` and `Action::Paste` with run names `cut` and `paste`, no arguments, added to `COMMAND_NAMES` from the schema change. Defaults `x` and `p` in `keymap-default.toml`. Paste prompts reuse `Popup::Input` with new purposes:
- `PasteSession { pane_id }`: on submit, opens a second input `PasteSessionWindow { pane_id, session_name }`. Nothing runs in tmux until the second submit, so cancelling at either prompt changes nothing.
- `PasteWindow { pane_id, session_id }`: on submit, moves the pane.
`pane_id` is captured when the prompt opens. Submit rechecks that it still exists, so a pane that vanished while the prompt was open gives a message instead of a tmux error.

### D3. Three `Tmux` trait methods, one per paste kind
```
move_pane_to_new_session(pane, session_name?, window_name?) -> (session_id, window_id)
move_pane_to_new_window(pane, session_id, window_name?)      -> window_id
float_pane_in_window(pane, window_id, &FloatingPane)         -> ()
```
`TmuxCli` sequences:
- **New session:** `new-session -d -P -F '#{session_id} #{window_id}' [-s name]` creates a placeholder. Then `break-pane -d [-n window] -s P -t '<sid>:'`, then `kill-window -t <placeholder>`, then `move-window -r -t <sid>` so the only window sits at `base-index`. If `break-pane` fails, `kill-session -t <sid>`, then return the error.
- **New window:** `break-pane -d [-n name] -s P -t '<sid>:'` (`-P -F '#{window_id}'` returns the id).
- **Floating:** `join-pane -d -s P -t <window_id>`, then `break-pane -W -d -s P -x W% -y H%`, then `move-pane -P <position> -t P`. Sizes go on `break-pane`. Position needs `move-pane -P` because `break-pane -W` only accepts absolute coordinates.
`FakeTmux` implements the same effects on its in-memory tree, so state tests can cover every paste path, including failure injection.
- *Alternative:* a generic `run(&[&str])` called from state. Rejected: it would leak tmux syntax into the picker and make rollback hard to test.

### D4. Prechecks in state, not tmux errors
Before calling tmux, state refuses with a message:
- `cut` when focus isn't the panes column. Floating panes can be cut like any other pane.
- `cut` when `cut_locked`.
- `paste` when the buffer is empty.
- A pane-column paste into a zoomed window.
A same-window pane-column paste is a silent no-op, as the user asked. This needs a new tree field from the existing single query: `Window.zoomed` (`#{window_zoomed_flag}`). `Pane.floating` (`#{pane_floating_flag}`) is also added so the tree and `FakeTmux` reflect floating state, but no precheck depends on it.

A floating source pane pasted into a new session or new window ends up tiled there, because `break-pane` creates a normal window. This matches the script. Keeping it floating in an otherwise empty window would add steps and has little use.

### D5. Exit after paste for move-pane
Add `Exit::Moved` to `src/picker/state.rs`. After a successful paste, state returns `Some(Exit::Moved)` when `cut_locked` is set, and otherwise refreshes, selects the result (session, window, or the pane under its new window) and clears the buffer. `picker::run` maps `Exit::Moved` to a value that `commands/move_pane.rs` turns into `Outcome::Done` (exit 0). `pane-mux picker` never gets `Exit::Moved`, because it is never locked.
- *Alternative:* a separate `exit_after_paste` flag. Rejected for now: lock and exit always go together. They can be split later without spec changes.

### D6. Indicator rendering
- **Marker:** in `src/picker/ui/columns.rs`, after the panes column draws its block, paint the cut row's left border cell (`area.x`, row `y`) with `[marker] symbol` in `[marker] cut` style. Using the border cell keeps it clear of the hovered-row open cap and the `*` current marker, so hover, current and cut all show together.
- **Badge:** in `src/picker/ui/mod.rs`, split the header row and render ` cut <target> ` right-aligned in `[marker] badge_cut`. The target is computed from the current tree on every frame, so it reflects moves and renumbering.
- Theme: a new `MarkerTheme { cut, symbol, badge_cut }` in `src/config/theme.rs` and a `[marker]` section in `theme-default.toml`.

### D7. `[floating-pane]` config
`FloatingPaneConfig { width: u8, height: u8, position: FloatingPosition }` in `src/config/settings.rs`. `FloatingPosition` is a kebab-case enum of the 13 tmux positions, and its `Display` gives the tmux `-P` argument. `validate()` checks 1..=100 for both sizes. Remove `always_on_top` from `config-default.toml`. With the lenient loader, existing user configs that set it are ignored.

### D8. `move-pane` command
`Command::MovePane { target: Option<String> }` in `src/cli.rs`. `commands/move_pane.rs`:
1. Resolve the pane id. With a target, use a new `Tmux::resolve_pane(target)` (`display-message -p -t <target> '#{pane_id}'`); an error exits 2 with "pane not found: <target>". Without a target, require `TMUX`, as `switch-session` does, and use `tmux.current()?.pane_id`.
2. Open the picker with header `[Move]`, session order `Name`, `cut: Some(id)` and `cut_locked: true`.
3. `Exit::Moved` maps to `Outcome::Done`, and a cancel maps to `Outcome::Cancelled` (130).
The picker opens on the sessions column, like every other command, with the cursor on the session that holds the cut pane, so the user starts from the source and navigates to the destination from there. Opening deeper, on the cut pane's window in the panes column, was considered and rejected: `p` there is a no-op, and the destination is usually elsewhere anyway.

## Risks / Trade-offs

- [Running `move-pane` without a target from a regular pane (not a popup) cuts the pane running pane-mux itself. After the detached move, the picker's shell keeps running in the background window] → The docs recommend `display-popup -E "pane-mux move-pane"`, as for `switch-session`.
- [Moving the last pane of the attached session destroys it, and with `detach-on-destroy on` the client detaches, closing the popup] → Matches the script. Documented as a known edge. No special handling.
- [Three-step floating paste can partially fail, e.g. `join-pane` succeeds and `break-pane -W` fails] → The pane is still moved, just tiled. Report tmux's error and keep the buffer. Re-floating by hand is possible, and no data is lost.
- [Floating panes need tmux 3.8] → Only the pane-column paste fails on older tmux, and the picker shows tmux's error. The README notes the requirement.
- [`move-window -r` renumbers the new session's windows] → The new session has exactly one window, so only that one moves.

## Migration Plan

Additive. Users with `always_on_top` in their config are unaffected thanks to the lenient loader. Rollback: revert the change, and no persisted state remains.
