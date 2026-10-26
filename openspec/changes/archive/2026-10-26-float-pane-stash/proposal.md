
# Proposal

## Why

tmux 3.8 floating panes can be created, moved and killed, but not hidden. Users who keep long-lived floats (a REPL, logs, an agent) have to kill them or let them cover the window. pane-mux already moves floating panes, so it can offer stash and unstash commands that bring each float back exactly where it was.

## What Changes

- New `pane-mux float stash [target]`: moves every floating pane in the target window (default: the client's current window) into a stash window in a dedicated `pmux` session, keeping each pane floating at its original size and position.
- New `pane-mux float unstash [target]`: moves every stashed float for the target window back, restoring geometry, stacking order and focus. Geometry is scaled proportionally if the window size changed since the stash.
- New `pane-mux float toggle [target]`: runs stash when the window has floating panes, otherwise runs unstash.
- Moves never split or resize other panes, and a float keeps its size when the window size hasn't changed, so programs in the main pane and in the floats are not disturbed.
- Stashing into a window that already has a stash merges into it. Unstashing next to visible floats is allowed.
- Stashing moves focus off a stashed float to the last-active tiled pane. Unstash refocuses the float that was active at stash time.
- Zoom is preserved on stash. Unstash unzooms the window first.
- Each stash window starts with a placeholder pane that only labels it. Stash and unstash keep working if the user kills it.
- The `pmux` session is an ordinary session: it shows in `switch-session` and the picker, and users can filter it with `exclude_from_recency`. Stash windows whose source window no longer exists are left for the user to clean up in the picker.
- Requires tmux 3.8, like floating paste.

## Capabilities

### New Capabilities
- `floating-pane-stash`: stashing and restoring the floating panes of a window through the `pane-mux float` commands, including the stash session, geometry restore, focus, zoom and toggle rules.

### Modified Capabilities
<!-- None: pane moving, picker and session switching behave as before. -->

## Impact

- CLI: a new `float` subcommand group with `stash`, `unstash`, `toggle` (`src/cli.rs`, `src/main.rs`, a new `src/commands/float.rs`).
- tmux layer: new `Tmux` trait operations for creating a dummy float, swapping panes, pane options, window sizes, layout parsing and locking (`src/tmux/client.rs`, `src/tmux/fake.rs`, `src/tmux/format.rs`).
- Picker: pasting a pane as a float from the panes column (`float_pane_in_window`) switches to the same swap-based move. Its behavior is unchanged; it just no longer flickers the destination's panes.
- Tests: unit tests against `FakeTmux`; private-server integration tests in `tests/tmux_integration.rs`, gated on floating support.
- Docs: README and `docs/usage-examples.md` (key bindings), with a short section on the `pmux` session.
- No config or schema changes; the stash session name is fixed as `pmux`.
