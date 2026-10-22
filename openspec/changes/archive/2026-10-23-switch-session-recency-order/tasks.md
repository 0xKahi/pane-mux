# Tasks

## 1. Session recency in the tmux snapshot

- [x] 1.1 Add `last_attached: u64` to `Session` in `src/tmux/model.rs`, and set it (default `0`) wherever sessions are built in `src/tmux/fake.rs` (`new_session`, `sample_tree`); verify `cargo build` succeeds
- [x] 1.2 Append `#{session_last_attached}` to `PANE_FORMAT` and parse it in `src/tmux/format.rs`, so 11 fields are expected, an empty value becomes `0`, and a non-numeric value is a row error; verify with unit tests in `format.rs` covering a numeric value, an empty value, and an invalid value
- [x] 1.3 Check that `tests/tmux_integration.rs` still passes against a private server and asserts that a freshly created, never-attached session parses with `last_attached == 0`; verify with `cargo test --test tmux_integration`

## 2. Session order in the picker

- [x] 2.1 Add `SessionOrder { Name, Recent }` (default `Name`) and an `order` field to `PickerOptions` in `src/picker/options.rs`; verify `cargo build` succeeds with the existing constructors updated
- [x] 2.2 Add `State::sort_sessions()` in `src/picker/state.rs`: no-op for `Name`; for `Recent`, a stable sort by (not current, newest `last_attached` first, name). Call it after every assignment to `self.tree` in `new` and `refresh`, before cursors are resolved. Verify with state tests in `src/picker/state/tests.rs` covering: current pinned first even when another session is newer, newest-first order, ties and never-attached sessions by name (last), the cursor opening on the current session, and `Name` order unchanged
- [x] 2.3 Add a state test where `refresh` changes `last_attached` so sessions reorder while the cursor is on a non-current session; verify the cursor id is unchanged and the new order matches the spec scenario "Order follows refresh without losing the cursor"

## 3. Commands and docs

- [x] 3.1 Set `order: SessionOrder::Recent` in `src/commands/switch_session.rs` and `SessionOrder::Name` in `src/commands/picker.rs`; verify `just lint` and `just test` pass
- [x] 3.2 Document in `README.md` (switch-session section) that sessions are listed current first, then most recently attached, while `pane-mux picker` lists them by name; verify the README section reads correctly next to the existing `display-popup` example

## 4. Integration check

- [x] 4.1 Run `just check` and `just test`, then manually run `pane-mux switch-session` in a popup after switching between three sessions; verify the order is current, previous, then the one before that, and that `pane-mux picker` is still alphabetical
