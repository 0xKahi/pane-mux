# Tasks

## 1. Opening cursor under recency order

- [x] 1.1 In `State::new` (`src/picker/state.rs`), when `options.order == SessionOrder::Recent`, set the initial session cursor to the first sorted session whose id differs from `current.session_id`, falling back to the first session; keep `reset_window()` afterwards. Verify `cargo build` succeeds.
- [x] 1.2 Update `recent_order_pins_current_and_opens_cursor_on_it` in `src/picker/state/tests.rs` (rename to reflect the new behaviour) to assert the cursor opens on the second-listed session, and that the hovered window/pane are that session's active ones. Verify with `cargo test recent_order`.
- [x] 1.3 Add tests in `src/picker/state/tests.rs` for: single session (cursor on it), `current` = `None` (cursor on first row), `current` not in tree (cursor on first row), and `SessionOrder::Name` unchanged (cursor on first row even when it is the current session). Verify with `cargo test`.
- [x] 1.4 Update `README.md` (around the "Sessions are listed current first..." paragraph) to say the cursor opens on the most recent session after the current one. Verify the wording matches the `session-switching` spec delta.

## 2. Integration check

- [x] 2.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `openspec validate cursor-on-next-recent-session --strict`; all pass.
- [x] 2.2 Manually run `pane-mux switch-session` in tmux with at least two sessions, press `<Enter>` immediately, and confirm the client switches to the previously used session.

## Workflow follow-up

- Archive the change with `/opsx-archive` after review to sync the `session-switching` spec.
