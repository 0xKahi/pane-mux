# Tasks

Prerequisite: `backend-neutral-core` is implemented (core `Mux` trait, `FloatMux`, backend-owned targets, `mux::open()`).

## 1. Rex API spikes

- [x] 1.1 Prototype `snapshot.lua` with `rex do -e` against the local server: confirm it can list, attach to and view every session and call `process` per block. Record its JSON as `src/rex/fixtures/snapshot.json` and its wall time with 2+ sessions in a comment in design.md D3. If `rex do` can't do it, switch D3 to the documented fallback
- [x] 1.2 Confirm `rex capture` options for limiting output (visible screen or line count) and how large `--format vt` output is for a long-scrollback block. Record the chosen flags in a design.md D6 note
- [x] 1.3 In a throwaway session (`rex new pane-mux-spike`, killed afterwards), verify the D9 flows: placeholder `new_window` + `move_block` + `block.close`, `move_block` + `float_block` with computed bounds, whether an emptied window auto-closes, and whether `move_block` changes the app's focused block. Record fixtures for each response
- [ ] 1.4 Verify the D7 switch flow (`client ls`, `-C <client> do session.select`) with Remote Control on and off, and record the error text for the off case as a fixture

## 2. Backend selection

- [x] 2.1 Implement detection in `mux::open()` (`PANE_MUX_BACKEND`, `TMUX`, non-blank `REX_BLOCK`, default tmux) as a pure function over an env snapshot, returning the `Backend` enum; add unit tests for every `backend-selection` scenario, including nested tmux and an invalid override (exit 2)
- [x] 2.2 Gate `pane-mux float *` on `as_float()`, so Rex prints the tmux-only error and exits 2; add a CLI test in `tests/cli_errors.rs` with `PANE_MUX_BACKEND=rex`
- [x] 2.3 Document `PANE_MUX_BACKEND` and detection order in README; verify the README example matches `pane-mux --help` output

## 3. RexCli foundation

- [x] 3.1 Add `src/rex/` with `RexCli`, the `Runner` trait (real process runner plus fixture runner), `--autostart=false` and `-S $REX_SERVER` handling, and error mapping (not found, unreachable, Rex stderr); unit-test each error path with the fixture runner
- [x] 3.2 Add the startup `api_revision >= 18` check via `whoami --json`, with a clear "Rex too old" error; test it with fixtures
- [x] 3.3 Parse the snapshot into `Tree` per D4 (order, 1-based indices, floating flag, process name with label fallback, `{}` as empty list, detached blocks dropped), plus `current()`, `in_client()` and `suppressed_pane()` from `REX_BLOCK`; verify with fixture-based unit tests covering the "Rex tree mapping", "Rex row numbering and pane names" and "Rex current-item markers" scenarios
- [x] 3.4 Implement `target_string` (full IDs), `display_target` (`label:N.M`) and `resolve_pane` via `block inspect`; unit-test the "Rex pick output" scenarios and suffix resolution

## 4. Picker on Rex

- [x] 4.1 Implement `capture()` with the D6 flags and trimming, and add `Mux::preview_interval()` (tmux keeps the current tick, Rex ~1 s, plus re-capture on selection change); verify tmux snapshots are unchanged and add a state test showing the Rex preview refreshes on selection change
- [x] 4.2 Implement create, rename and delete (`new`/`new_window` with `focus:false`, `set_label`/`set_window_label`, `destroy`/`close_window`/`block.close`); test with the fixture runner that the correct API calls are issued, and that a Rex error string is shown on screen without exiting
- [x] 4.3 Ensure the copy-mode and stashed indicators never render on Rex and that zoom refusal can't trigger; add a UI snapshot test with a Rex-shaped tree

## 5. switch-session on Rex

- [x] 5.1 Update `switch-session`'s client check to use `in_client()` with the new wording (tmux or Rex) and the forced-Rex-outside-Rex case; update `tests/cli_errors.rs` expectations for the outside-both case
- [x] 5.2 Implement the D7 switch (server-side focus, app client selection, `session.select`) and the Remote Control fallback hint with exit 0; test with fixtures for both the success and Remote Control off cases
- [x] 5.3 Implement the D8 history store (XDG state path, per-server key, atomic write, cap 50, recorded on every switch including the fallback) and synthetic `last_attached` ranking; unit-test every "Rex recency ranking" and "Rex recency history" scenario, including excluded labels and deleted sessions

## 6. Moves on Rex

- [x] 6.1 Add `Mux::cross_session_moves()` and the refusal in `State::paste` with the exact message `rex does not allow moving panes to different session`, keeping the cut buffer and the picker open (including under `move-pane`); add state tests using a fake configured with `cross_session_moves = false`
- [x] 6.2 Implement `move_pane_to_new_window` via the placeholder flow, with placeholder cleanup on errors; fixture tests for success and mid-flow failure
- [x] 6.3 Implement `float_pane_in_window` with anchor selection and `FloatingPosition`-to-bounds conversion; unit-test the bounds for all 13 positions and the "Float into another window" scenario
- [x] 6.4 Close emptied source windows when Rex doesn't, and restore app focus if spike 1.3 showed `move_block` changes it; fixture tests for the "Last block leaves its window" scenario
- [x] 6.5 Make `move-pane` accept Rex block IDs or suffixes and default to `REX_BLOCK`; test the "Rex block target" and "Rex current block" scenarios

## 7. Docs and integration

- [x] 7.1 Add a Rex section to README and `docs/usage-examples.md`: detection, launching pane-mux from a Rex key binding or custom action in a floating layer, the Remote Control requirement, pick output IDs, the cross-session limitation, and that `float` is tmux-only; verify every command shown runs as written in Rex
- [x] 7.2 Add `tests/rex_integration.rs`, gated on `PANE_MUX_REX_TESTS=1`, that creates a throwaway `pane-mux-test-<pid>` session, exercises tree listing, pick output, same-session window move, floating paste and the cross-session refusal, then kills the session; add a `just test-rex` recipe and verify it passes locally
- [x] 7.3 Run `just ci` and `just test-rex` and confirm tmux snapshots, call logs and schemas are unchanged (`git status` clean after `just schemas`)

## Workflow follow-up

- Archive `add-rex-backend` after review, which merges the `backend-selection` and `rex-backend` specs into `openspec/specs/` and applies the deltas to `tmux-picker`, `session-switching` and `pane-moving`.
