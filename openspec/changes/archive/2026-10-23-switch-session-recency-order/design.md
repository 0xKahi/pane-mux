# Design

## Context

- `TmuxCli::tree()` runs a single `list-panes -a -F PANE_FORMAT` call. `parse_tree` keeps tmux's first-seen order, which is session name order. pane-mux never sorts anything itself.
- `State::new` puts the session cursor on `tree.sessions.first()`. `State::refresh` replaces `self.tree` every 300ms. Cursors are tracked by stable id, so reordering cannot move the cursor to a different session.
- `switch-session` and `picker` both reach `picker::run` through `PickerOptions`. That struct is the existing way for a command to tell the picker how to behave.
- Spike on tmux 3.8 with an isolated socket: `switch-client` updates `#{session_last_attached}` for the target session, giving Unix seconds. A session that has never been attached reports an **empty string**, not `0`.

## Goals / Non-Goals

**Goals:**
- Recency ordering under `switch-session`, from data tmux already keeps.
- One sort helper that runs on both load and refresh.

**Non-Goals:**
- Frequency scoring or persisted history (frecency).
- Hiding the current session (tmux-sessionx's `filter-current`).
- Putting the cursor on the previous session.
- Making the order configurable in `config.toml`.
- Reordering windows or panes.

## Decisions

### Recency source: `#{session_last_attached}`
Add the field to `PANE_FORMAT` and parse it into `Session.last_attached: u64`. An empty value becomes `0`.
- *Alternative: `#{client_last_session}`*, which is what tmux-sessionx uses. It gives only one step of history; everything after it falls back to name order.
- *Alternative: a pane-mux history file fed by a `client-session-changed` hook.* That means extra setup, a state file, and drift when the hook isn't installed. tmux already tracks the timestamp we need.
- *Alternative: `#{session_activity}`.* It changes on any output in the session, not on switching, so busy sessions would float to the top.

The value is repeated on every pane row, like `session_name`. The parser takes it from the first row it sees for that session.

### Ordering lives in the picker, not the tmux layer
`PickerOptions` gains `order: SessionOrder` with variants `Name` (the default) and `Recent`. `State` gets a `sort_sessions()` helper that runs after every assignment to `self.tree` in `new` and `refresh`. Under `Name` it does nothing and keeps tmux's order. Under `Recent` it does a stable sort by the key `(not current, Reverse(last_attached), name)`.
- `Tmux::tree()` keeps returning sessions exactly as tmux lists them, so ordering stays a presentation choice. Integration tests of the tmux layer don't change.
- *Alternative: sort inside `tree()` based on a flag.* That would mix how we query tmux with how we present the result.

### Current session pinned explicitly
`last_attached` covers the whole server. If another client attached somewhere more recently, the invoking client's own session would not be first. Pinning by `Current.session_id` guarantees that the current session is the top row and that the cursor opens on it. If `current` is `None` (impossible under `switch-session`, but allowed by the type), sorting uses only the recency and name keys.

### Re-sort on every refresh instead of freezing the order
Cursors follow ids, so re-sorting cannot change which session is hovered. The order changes only when another client switches while the picker is open. Freezing the order would need a rank map plus a rule for sessions created while the picker is open, including ones made with the picker's own new-session command.

### Commands choose the order
`switch_session.rs` passes `SessionOrder::Recent`. `picker.rs` keeps the default `Name`. No CLI flag or config key is added.

## Risks / Trade-offs

- [Server-wide recency, not per client] → With several clients, row 2 is "most recently attached by anyone", not strictly "my previous session". This is accepted. With a single client the two are the same, and the current session is still pinned first.
- [1-second timestamp granularity] → Switches within the same second tie. The name tie-break keeps the order deterministic.
- [Rows can move while the picker is open] → Only when another client switches. The cursor stays on its session id. This is accepted in exchange for a simpler implementation.
- [tmux versions where `session_last_attached` is empty or missing] → Every session parses as `0` and the order falls back to current first, then by name. Nothing crashes.
