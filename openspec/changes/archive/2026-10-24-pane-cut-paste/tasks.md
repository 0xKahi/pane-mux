# Tasks

Prerequisite: `lenient-config-schemas` is implemented on this branch first.

## 1. Config and theme

- [x] 1.1 Add `FloatingPaneConfig { width, height, position }` and the `FloatingPosition` enum (13 tmux positions, kebab-case, `Display` gives the `-P` argument) to `src/config/settings.rs`, validate sizes 1..=100, and remove `always_on_top` from `config-default.toml`; verify with unit tests for defaults, custom values, `position = "middle"` and `width = 0` (exit-2 errors name the key)
- [x] 1.2 Add `MarkerTheme { cut, symbol, badge_cut }` to `src/config/theme.rs` and a `[marker]` section to `theme-default.toml` (`cut = { fg = "lightred", bg = "lightred" }`, `symbol = "│"`, `badge_cut = { fg = "white", bg = "red" }`); verify with a unit test that a partial `[marker] cut` override keeps the other defaults
- [x] 1.3 Regenerate schemas (`just schemas`) and verify the stale-check and preset-validation tests pass with `[floating-pane]` and `[marker]` present
- [x] 1.4 Document `[floating-pane]` and `[marker]` in `docs/configuration.md` (including the full-defaults block); verify the documented defaults match the presets

## 2. tmux layer

- [x] 2.1 Add `Pane.floating` (`#{pane_floating_flag}`) and `Window.zoomed` (`#{window_zoomed_flag}`) to `src/tmux/model.rs` and `PANE_FORMAT`/parsing in `src/tmux/format.rs`; verify with format parsing unit tests
- [x] 2.2 Add `resolve_pane`, `move_pane_to_new_session`, `move_pane_to_new_window` and `float_pane_in_window` to the `Tmux` trait and `TmuxCli` with the command sequences from design D3, including the new-session rollback (`kill-session` when `break-pane` fails) and `move-window -r`; verify the crate compiles
- [x] 2.3 Implement the same methods in `FakeTmux` (`src/tmux/fake.rs`) with in-memory effects (closing emptied windows/sessions, floating flag) and failure injection; verify with fake unit tests
- [x] 2.4 Add opt-in integration tests in `tests/tmux_integration.rs` (private socket) for each paste kind: new session with exactly one window holding the pane, new window, floating pane at the configured size/position, rollback on duplicate session name, and that the source window closes when it held only that pane; verify with `just test-tmux`

## 3. Picker cut/paste

- [x] 3.1 Add `Action::Cut` / `Action::Paste`, the `cut` / `paste` run names (also in `COMMAND_NAMES`) and the `x` / `p` defaults with descriptions in `keymap-default.toml`; verify with keymap parsing tests and that the help popup snapshot lists both
- [x] 3.2 Add `cut` / `cut_locked` to `PickerOptions` and the cut buffer to `State` (toggle, replace, refusal messages for non-pane focus and locked mode, clearing on refresh when the pane is gone); verify with state tests for every "Cut a pane", "Cut is refused…", "Cut buffer follows the pane" and "Move-pane cut is locked" scenario
- [x] 3.3 Implement paste: the empty-buffer message, the `PasteSession` → `PasteSessionWindow` and `PasteWindow` prompts, the pane-column float with zoomed-window refusal and the same-window no-op, then refresh, select the result and clear the buffer, with tmux errors shown and the buffer kept; verify with `FakeTmux` state tests for every paste scenario in the `pane-moving` spec, including cancelling at either prompt
- [x] 3.4 Add `Exit::Moved`, returned after a successful paste when `cut_locked`; verify with a state test that locked mode exits and unlocked mode stays open

- [x] 3.5 Allow cutting floating panes: remove the floating-pane cut refusal and its message, replace its state test with `FakeTmux` tests for the "Cut a floating pane" and "Floating panes move like tiled panes" scenarios (FakeTmux must clear the floating flag on break/join and set it on float), and extend `tests/tmux_integration.rs` with a floating source for each paste kind; verify `cargo test` and `just test-tmux` pass

## 4. Indicator

- [x] 4.1 Draw the cut marker in the panes column's left border cell (`src/picker/ui/columns.rs`) and the right-aligned `cut <target>` header badge (`src/picker/ui/mod.rs`); verify with insta snapshots for a cut pane unhovered, hovered and also current, and for the header badge

## 5. move-pane command

- [x] 5.1 Add `Command::MovePane { target: Option<String> }` in `src/cli.rs`, `src/commands/move_pane.rs` (resolve target or current pane, `[Move]` header, locked cut, `Exit::Moved` → exit 0, cancel → 130) and wire it in `src/main.rs`; verify with `tests/cli_errors.rs` cases for an unknown target (exit 2) and no target outside tmux (exit 2), and an opt-in tmux test that the picker opens with the target cut
- [x] 5.2 Document `pane-mux move-pane` and cut/paste in the README commands section and `docs/usage-examples.md`, including the recommended `bind … display-popup -E "pane-mux move-pane"` binding, the tmux 3.8 requirement for floating paste and the detach-on-destroy edge; verify the documented commands run as written
- [x] 5.3 Start `move-pane` with the sessions column focused and the cursor on the cut pane's session (e.g. through a `PickerOptions` initial-selection field used by `State` on startup); verify with a state test for the "Cursor starts on the source session" scenario, check that `picker` and `switch-session` still start at the top, and note the start position in `docs/picker-actions.md`
- [x] 5.4 Move the "cut and paste in the picker" section from `docs/usage-examples.md` into a new `docs/picker-actions.md`, and update links from `README.md`, `docs/configuration.md` and `docs/usage-examples.md`; verify no link points at the old anchor

## 6. Integration

- [x] 6.1 Run `just ci` and `openspec validate pane-cut-paste --strict`, then manually move a pane in a real tmux 3.8 session through each paste kind in both `pane-mux picker` and `pane-mux move-pane`; verify all pass and behave as specified
