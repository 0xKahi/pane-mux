# Proposal

## Why

`pane-mux switch-session` opens with the cursor on the current session. Pressing `<Enter>` right away therefore switches to the session the user is already on, which does nothing. In practice, the session the user most often wants is the previously used one, which is listed directly below the current session. Starting the cursor there makes "open switcher, press Enter" jump back to the previous session, like alt-tab.

## What Changes

- Under `switch-session`, the cursor SHALL open on the first session that is not the invoking client's current session. With the recency order, that is the row directly below the current session.
- If the only session is the current one, the cursor stays on it.
- If there is no current session, or it is not in the list, the cursor opens on the first row, so no other session is skipped.
- The session order is unchanged: current session first, then by recency.
- Plain `pane-mux picker` is unchanged: name order, with the cursor on the first row.
- Documentation describing the opening cursor position in `README.md` is updated.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `session-switching`: the "Sessions ordered by recency" requirement changes where the cursor opens, from the first row to the first session that is not the current one, and adds scenarios for the single-session and no-current-session cases.

## Impact

- `src/picker/state.rs` sets the initial session cursor in `State::new`.
- `src/picker/state/tests.rs` covers the opening-cursor tests under `SessionOrder::Recent`.
- `README.md` describes the opening cursor position for `switch-session`.
- There are no API, configuration, or dependency changes.
