# Design

## Context

`State::new` (`src/picker/state.rs`) sorts sessions with `sort_sessions()`, then sets the session cursor to `tree.sessions.first()` and calls `reset_window()` to select that session's active window and pane. `SessionOrder::Recent` is only used by `switch-session`, and it already pins `current.session_id` to the top. During refresh, the cursor is preserved by session id, so only the opening position needs to change.

## Goals / Non-Goals

**Goals:**
- Choose the opening session cursor from the current session's id, not from a fixed index.

**Non-Goals:**
- Per-client "last session" tracking (`switch-client -l` semantics). Row 2 stays the server-wide most recently attached session.
- A configuration option for the opening cursor.
- Any change to `SessionOrder::Name` (plain `picker`).

## Decisions

**Pick the first session whose id differs from `current.session_id`, falling back to the first session.**
Under `SessionOrder::Recent` only, the initial cursor is the first session in sorted order that is not the current one. If none qualifies (single session), or `current` is `None` or not in the tree, it is the first session.
- *Alternative: always index 1.* Rejected. When the current session is unknown, row 0 is already a non-current session, and index 1 would skip it. With a single session, index 1 does not exist.
- *Alternative: apply to both orders.* Rejected. Under name order the current session can be anywhere, so "skip current" has no consistent "go back" meaning, and plain `picker` is not a switcher.

**Keep `reset_window()` after choosing the session.**
The windows and panes columns then follow the hovered session's active window and pane, exactly as for any other hovered session. No new code path is needed.

## Risks / Trade-offs

- [With several clients, row 2 may be a session another client attached, not this client's previous one] -> Accepted. This matches the existing ordering spec, and the common single-client setup is unaffected.
- [Existing test `recent_order_pins_current_and_opens_cursor_on_it` asserts the old behaviour] -> Update it as part of this change instead of adding a parallel test.
