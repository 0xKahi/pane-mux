# Spec Delta

## MODIFIED Requirements

### Requirement: Sessions ordered by recency
While running under `switch-session`, the picker's sessions column SHALL list the invoking client's current session first, then every other session from most to least recently attached on the tmux server. Sessions with equal attach times, and sessions that were never attached, SHALL be ordered by name, and never-attached sessions SHALL come last. Sessions whose names exactly match an entry in `[switch-session] exclude_from_recency` SHALL instead be listed after all ranked sessions, ordered by name and regardless of attach time; the current session SHALL stay first even when excluded. The cursor SHALL open on the first non-current session, else on the first row.

#### Scenario: Current first, then most recent
- **WHEN** the client is on `pmux`, previously switched from `shuffle-be`, before that from `kahi`, and `scratch` exists, and the user runs `pane-mux switch-session`
- **THEN** the sessions column lists `pmux`, `shuffle-be`, `kahi`, `scratch` in that order, with the cursor on `shuffle-be`

#### Scenario: Enter right away switches to the previous session
- **WHEN** the client is on `pmux`, previously switched from `shuffle-be`, and the user runs `pane-mux switch-session` and presses `<Enter>` without moving the cursor
- **THEN** the client switches to `shuffle-be`

#### Scenario: Only the current session exists
- **WHEN** `work` is the only session and the client on `work` runs `pane-mux switch-session`
- **THEN** the sessions column lists only `work`, with the cursor on `work`

#### Scenario: Current session unknown
- **WHEN** the invoking client's current session cannot be determined or is not in the session list, and sessions `notes` and `work` exist with `notes` attached more recently
- **THEN** the sessions column lists `notes` then `work`, with the cursor on `notes`

#### Scenario: Current session pinned above a more recent one
- **WHEN** another client attached to `notes` after the invoking client last switched to `work`, and the invoking client runs `pane-mux switch-session` while on `work`
- **THEN** `work` is listed first and `notes` second, with the cursor on `notes`

#### Scenario: Never-attached sessions last, by name
- **WHEN** sessions `zeta` and `alpha` have never been attached and `work` has been attached
- **THEN** after the current session, `work` is listed before `alpha`, and `alpha` before `zeta`

#### Scenario: Order follows refresh without losing the cursor
- **WHEN** the picker is open with the cursor on `kahi` and another client switches to `scratch`
- **THEN** after the next refresh, `scratch` is listed directly after the current session and the cursor is still on `kahi`

#### Scenario: Windows and panes unaffected
- **WHEN** the user hovers a session under `switch-session`
- **THEN** its windows and panes are listed in the same order as in `pane-mux picker`

#### Scenario: Excluded session listed after ranked sessions
- **WHEN** sessions `session1` (current), `session2`, `session3` and `session4` were attached in recency order `session2`, `session3`, `session4`, and `exclude_from_recency = ["session2"]`
- **THEN** the sessions column lists `session1`, `session3`, `session4`, `session2`, with the cursor on `session3`

#### Scenario: Excluded sessions ordered by name
- **WHEN** `exclude_from_recency = ["zeta", "alpha"]`, `zeta` was attached more recently than `alpha`, and `work` is a ranked session
- **THEN** after the current session, `work` is listed before `alpha`, and `alpha` before `zeta`

#### Scenario: Excluded sessions after never-attached sessions
- **WHEN** `scratch` is excluded and was attached most recently, and `fresh` has never been attached and is not excluded
- **THEN** `fresh` is listed before `scratch`

#### Scenario: Current session excluded stays first
- **WHEN** the client is on `_popup`, `exclude_from_recency = ["_popup"]`, and `work` was attached before it
- **THEN** `_popup` is listed first, with the cursor on `work`

#### Scenario: Only excluded sessions besides current
- **WHEN** the client is on `work` and every other session (`b-scratch`, `a-popup`) is excluded
- **THEN** the sessions column lists `work`, `a-popup`, `b-scratch`, with the cursor on `a-popup`

#### Scenario: Unknown excluded name ignored
- **WHEN** `exclude_from_recency = ["missing"]` and no session is named `missing`
- **THEN** sessions are ordered exactly as with an empty exclusion list

#### Scenario: Excluded session attached by another client does not move
- **WHEN** the picker is open, `scratch` is excluded, and another client switches to `scratch`
- **THEN** after the next refresh, `scratch` keeps its position among the excluded sessions
