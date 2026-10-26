# Tasks

## 1. tmux primitives

- [x] 1.1 Add a pane query to `Tmux` returning, for one window, each pane's id, floating flag, active flag, `pane_z`, inner geometry and `@pmux_*` options, plus the window size and zoom flag. Parse it in `src/tmux/format.rs` and verify with format unit tests, including an empty floating flag (tmux < 3.8) and missing options.
- [x] 1.2 Add the mutation primitives to `Tmux` and `TmuxCli`: set pane and window options, `join-pane -d -s -t`, `break-pane -d -W -s -x -y -X -Y`, `move-pane -z`, `resize-window -x -y`, `resize-pane -Z`, `select-pane`, `kill-window`, `display-message`, `new-session`/`new-window` with a command and size, and reading `#{socket_path}`. Verify that `cargo build` passes and each `TmuxCli` method passes an explicit `-t`/`-s`.
- [x] 1.3 Mirror every new primitive in `FakeTmux`: track floating state, geometry, z-order, pane and window options, zoom and active pane, and record calls in the `calls()` format. Verify with fake unit tests that `join-pane` makes a float tiled and `break-pane -W` floats it at the given geometry.

## 2. Stash model and locking

- [x] 2.1 Implement geometry capture (inner to border-inclusive: `x=w+2`, `y=h+2`, `X=left-1`, `Y=top-1`) and the show-time scale-and-clamp function. Verify with unit tests: exact when the size is unchanged, proportional when it shrinks or grows, clamped so the pane always fits, and a minimum size of 3x3.
- [x] 2.2 Implement stash lookup and creation: find `pmux` by `@pmux_stash=1`, fail if a `pmux` session exists without the tag, find the stash window by `@pmux_source`, otherwise create the session or window with the placeholder command at the source size and tag it. Verify with `FakeTmux` tests for first creation, reuse, the foreign `pmux` error, and lookup after the source window is renamed.
- [x] 2.3 Implement the per-server file lock on `<socket_path>.pane-mux-float.lock` with `std::fs::File::lock`, held for the whole command. Verify with a unit test that a second lock on the same path blocks until the first is dropped.
- [x] 2.4 Implement target resolution: the positional target resolves to its window, otherwise `current()`; error outside tmux without a target, on an unknown target, and when the target window is a `pmux` stash window. Verify with `FakeTmux` tests for each case.

## 3. Hide

- [x] 3.1 Implement `hide` in `src/commands/float.rs`: record `@pmux_*` options on each float, move floats back to front into the stash (`join-pane` to the placeholder, then `break-pane -W` with the stored geometry), merge into an existing stash, re-zoom a zoomed window, and send "no floating panes" via `display-message` when there is nothing to hide. Verify with `FakeTmux` tests for two floats, merge, no floats, focused float (last-active tiled pane ends up active) and zoomed window.
- [x] 3.2 Implement placeholder recovery: when the stash window has no tiled pane, create a fresh stash window, move the existing stashed floats into it using their stored options, then stash the new ones. Verify with a `FakeTmux` test where the placeholder was killed.
- [x] 3.3 Implement best-effort error collection for hide: continue after a per-pane failure and return an error listing the failed ids. Verify with a `FakeTmux` `fail_next` test that the other panes moved and rerunning stashes the remaining one.
- [x] 3.4 Add a private-server integration test in `tests/tmux_integration.rs`, gated on `supports_floating`: hide two floats, then assert the source has only tiled panes and the `pmux` stash window has a placeholder plus two floats at their original inner geometry.

## 4. Show

- [x] 4.1 Implement `show`: unzoom if zoomed; restore back to front by descending `@pmux_z` (`join-pane` to a tiled pane of the source, then `break-pane -W` with the stored or scaled geometry); refocus the `@pmux_active` pane; kill the stash window once no stashed float remains in it; send "nothing stashed" when there is nothing to show; collect per-pane failures as in hide. Verify with `FakeTmux` tests for exact restore, scaled restore, z-order, next to visible floats, focus restore, unchanged focus, zoomed window, cleanup with and without other stashes, and a partial failure that leaves the stash window in place.
- [x] 4.2 Extend the integration test with a full round trip: hide, then show, then assert identical inner geometry and z-order, that the hidden active float is active again, and that the `pmux` session is gone. Also cover a show after `resize-window` on the source, asserting that the float fits and its position is scaled.

## 5. Toggle, CLI and docs

- [x] 5.1 Implement `toggle` (any visible float in the target window means hide, otherwise show) and verify with `FakeTmux` tests for the visible, none-visible and mixed-state double-toggle scenarios.
- [x] 5.2 Wire `pane-mux float hide|show|toggle [target]` in `src/cli.rs` and `src/main.rs`, mapping results to exit 0 or 2. Verify with `tests/cli_errors.rs` cases: `float toggle` outside tmux without a target exits 2, and `pane-mux float --help` lists the three actions.
- [x] 5.3 Document the commands in `README.md` and `docs/usage-examples.md`: the key binding (`bind f run-shell 'pane-mux float toggle'`), the `pmux` session, filtering it with `exclude_from_recency`, orphan cleanup through the picker, and the tmux 3.8 requirement. Verify that the documented commands match the CLI help output.

## 6. Revision: swap-based moves, stash/unstash

Groups 1–5 implemented the first design (`join-pane` then `break-pane -W`, `hide`/`show`, placeholder recovery, `@pmux_restoring`). This group carries the revised design.md and spec into the code.

- [x] 6.1 Add the swap primitives to `Tmux` and `TmuxCli`: create a dummy float (`new-pane -d -A -E -T pane-mux-dummy -t <window> -x -y -X -Y -P -F '#{pane_id}'`, also accepting `%` sizes), swap and kill in one invocation (`swap-pane -d -Z -s -t \; kill-pane -t`), `kill-pane`, and reading `#{pane_title}`, `#{pane_flags}` and `#{window_layout}`. Mirror them in `FakeTmux`: `new-pane` adds a float at the front at the given geometry; a swap exchanges the two panes' slots (window, floating state, geometry, `z`, active flag) and keeps each pane's id and options; `-Z` and `-A` keep zoom. Remove `join_float` and `break_float` once nothing uses them. Verify with fake unit tests for a cross-window swap, z and active-slot inheritance, and swaps in a zoomed window.
- [x] 6.2 Find floats in zoomed windows: when `window_zoomed_flag` is set, take float membership, geometry and `z` from `#{window_layout}`. First confirm the layout's coordinate convention and z field on a private tmux 3.8 server, then convert to the same values as `FloatGeometry::capture`. Verify with format unit tests on captured layout strings and a fake test that toggle and stash see floats hidden by zoom.
- [x] 6.3 Rework stash on the swap helper: at the start, kill leftover `pane-mux-dummy` panes in the source and its stash windows; size the stash window; move floats back to front through a dummy in the stash window at their captured geometry; keep zoom through `-A`/`-Z`, re-zooming the same pane only if tmux unzoomed; remove placeholder recovery, the "no placeholder" checks and `@pmux_restoring`. A missing placeholder must not stop stashing. Update the `FakeTmux` tests: two floats, merge, no floats, focused float, zoomed window, killed placeholder, partial failure and rerun, and leftover-dummy cleanup.
- [x] 6.4 Rework unstash on the swap helper: leftover-dummy cleanup; unzoom if zoomed; move back to front through a dummy in the source window at the stored or scaled geometry; `select-pane` the `@pmux_active` pane; `kill-window` a stash window holding only the placeholder, and rely on tmux when it has none. Update the `FakeTmux` tests: exact restore, scaled restore, z-order, next to visible floats, focus restore, unchanged focus, zoomed window, cleanup with and without other stashes, cleanup without a placeholder, and partial failure.
- [x] 6.5 Move the picker's floating paste (`float_pane_in_window`) to the swap helper: a dummy created with `-x W% -y H%` from `[floating-pane]`, positioned with `move-pane -P <position>` before the swap. Verify that the existing pane-moving unit and integration tests pass unchanged, including the cursor on the pasted pane and the zoomed-target refusal.
- [x] 6.6 Rename the actions to `pane-mux float stash|unstash|toggle` in `src/cli.rs`, `src/main.rs`, `src/commands/float.rs`, the placeholder text, tests, `README.md` and `docs/usage-examples.md`. Verify that `pane-mux float --help` lists `stash`, `unstash` and `toggle`, and that the docs match it.
- [x] 6.7 Update `tests/tmux_integration.rs`: rename to stash/unstash; add a round trip in which a `SIGWINCH`-logging script in the main pane and in a float records no resize; stash in a zoomed window with floats hidden by zoom, asserting they are stashed at their geometry and the window is still zoomed; stash, kill the placeholder, stash again, unstash everything; and a leftover `pane-mux-dummy` pane that the next command removes.

## 7. Integration check

- [x] 7.1 Run `just check` and `just test-tmux` (fmt, clippy with `-D warnings`, all tests including the tmux integration tests) and verify that both pass.
- [x] 7.2 Manually verify in a real tmux 3.8 session: bind toggle, double-tap it quickly, confirm that no pane is left stranded between stash and unstash, and that the main pane (for example a Neovim dashboard) shows no redraw glitch.
