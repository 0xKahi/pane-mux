# Proposal

## Why

`switch-session` lists sessions by recency so `<Enter>` jumps back to the previous session. Utility sessions that get attached often (popup and scratch sessions, for example) take the top recency spot and push real work sessions down. Users need to keep these sessions pickable without letting them affect the ranking.

## What Changes

- New `config.toml` section `[switch-session]` with `exclude_from_recency` (a list of session names, default `[]`).
- Under `switch-session`, sessions named in `exclude_from_recency` are still listed. They come after all recency-ranked sessions and are ordered by name, with their attach time ignored.
- The invoking client's current session stays first, even if it is on the exclusion list.
- Names match exactly. Names that match no session are ignored.
- The cursor's opening rule doesn't change (first non-current row), so it skips excluded sessions whenever a ranked session exists.
- Plain `pane-mux picker` (name order) and the windows/panes columns don't change.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `configuration`: adds the `[switch-session] exclude_from_recency` setting.
- `session-switching`: the "Sessions ordered by recency" requirement now places excluded sessions after the ranked ones, ordered by name.

## Impact

- `src/config/settings.rs`: new `SwitchSessionConfig` on `Config` (the section is renamed to `switch-session`).
- `src/config/preset/config-default.toml`: default `[switch-session] exclude_from_recency = []`. This replaces the uncommitted `recency_exclusion` key.
- `src/picker/state.rs` / `src/picker/options.rs`: the recency sort key gets an "excluded" bucket, and picker options need to carry the exclusion list.
- `src/commands/switch_session.rs`: passes the configured list into the picker options.
- No new dependencies. Not breaking: the default keeps current behaviour.
