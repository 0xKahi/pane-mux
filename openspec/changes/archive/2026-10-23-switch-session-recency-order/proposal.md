# Proposal

## Why

`pane-mux switch-session` lists sessions in tmux's name order, so the sessions you jump between most often can sit anywhere in the list. Session switchers like tmux-sessionx surface the previously used session, and `switch-session` should do better: order the whole session list by how recently each session was used.

## What Changes

- `pane-mux switch-session` lists sessions in most-recently-used order. The client's current session comes first, then the other sessions from most to least recently attached. Ties, and sessions that were never attached, are ordered by name.
- The order is kept while the picker refreshes. The cursor stays on the hovered session even when the order changes underneath it, for example because another client switched sessions.
- The cursor still opens on the top row, which in this mode is the current session.
- `pane-mux picker` keeps listing sessions by name. That order is now written down as a requirement instead of being implicit.
- The windows and panes columns keep their existing order in both commands.
- Recency comes from tmux's own session attach timestamps. pane-mux does not keep a history file and does not install hooks.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `session-switching`: adds a requirement that `switch-session` orders sessions by recency, with the current session first.
- `tmux-picker`: adds a requirement that the plain picker orders sessions by name, so the existing behaviour is pinned down now that a second order exists.

## Impact

- `src/tmux/format.rs`, `src/tmux/model.rs`, `src/tmux/fake.rs`: the tmux snapshot gains each session's last-attached time. An empty value from tmux means the session was never attached.
- `src/picker/options.rs`, `src/picker/state.rs`: picker options gain a session order, and sessions are sorted when the picker opens and on every refresh.
- `src/commands/switch_session.rs`, `src/commands/picker.rs`: each command chooses its order.
- README: document the `switch-session` ordering.
- No new dependencies and no config changes.
