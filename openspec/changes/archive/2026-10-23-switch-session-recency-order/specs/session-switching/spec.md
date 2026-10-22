# Spec Delta

## ADDED Requirements

### Requirement: Sessions ordered by recency
While running under `switch-session`, the picker's sessions column SHALL list the invoking client's current session first, then every other session from most to least recently attached on the tmux server. Sessions with equal attach times, and sessions that were never attached, SHALL be ordered by name, and never-attached sessions SHALL come last. The cursor SHALL open on the first row.

#### Scenario: Current first, then most recent
- **WHEN** the client is on `pmux`, previously switched from `shuffle-be`, before that from `kahi`, and `scratch` exists, and the user runs `pane-mux switch-session`
- **THEN** the sessions column lists `pmux`, `shuffle-be`, `kahi`, `scratch` in that order, with the cursor on `pmux`

#### Scenario: Current session pinned above a more recent one
- **WHEN** another client attached to `notes` after the invoking client last switched to `work`, and the invoking client runs `pane-mux switch-session` while on `work`
- **THEN** `work` is listed first and `notes` second

#### Scenario: Never-attached sessions last, by name
- **WHEN** sessions `zeta` and `alpha` have never been attached and `work` has been attached
- **THEN** after the current session, `work` is listed before `alpha`, and `alpha` before `zeta`

#### Scenario: Order follows refresh without losing the cursor
- **WHEN** the picker is open with the cursor on `kahi` and another client switches to `scratch`
- **THEN** after the next refresh, `scratch` is listed directly after the current session and the cursor is still on `kahi`

#### Scenario: Windows and panes unaffected
- **WHEN** the user hovers a session under `switch-session`
- **THEN** its windows and panes are listed in the same order as in `pane-mux picker`
