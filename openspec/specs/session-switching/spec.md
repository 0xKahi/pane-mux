# session-switching Specification

## Purpose

Lets users inside tmux jump their current client to any session, window or pane chosen through the pane-mux picker, typically from a `display-popup` key binding.

## Requirements

### Requirement: Switch client to picked target
`pane-mux switch-session` SHALL open the picker and, on `<Enter>`, switch the current tmux client to the picked target at the focused column's level, making that session, window and pane active.

#### Scenario: Switch to a session
- **WHEN** the user runs `pane-mux switch-session` inside tmux, focuses the sessions column on `dotfiles` and presses `<Enter>`
- **THEN** the client switches to session `dotfiles` and the command exits with code 0

#### Scenario: Switch to a pane
- **WHEN** the user focuses the panes column on `work:2.1` and presses `<Enter>`
- **THEN** the client shows session `work`, window `2`, with pane `1` active

#### Scenario: From a popup
- **WHEN** the command runs inside `tmux display-popup -E "pane-mux switch-session"` and the user picks `dotfiles`
- **THEN** the popup closes and the client that opened the popup switches to `dotfiles`

### Requirement: Picker header shows mode
While running under `switch-session`, the picker header SHALL indicate that `<Enter>` switches (e.g. `[Switch]`), distinct from plain `pane-mux picker`.

#### Scenario: Header
- **WHEN** the user runs `pane-mux switch-session`
- **THEN** the picker header shows the switch mode indicator

### Requirement: Requires tmux client
`pane-mux switch-session` SHALL fail with a message on stderr and exit code 2, without opening the picker, when not running inside tmux.

#### Scenario: Outside tmux
- **WHEN** the user runs `pane-mux switch-session` in a terminal with no `TMUX` environment
- **THEN** stderr explains that switch-session must run inside tmux and the exit code is 2

### Requirement: Cancel does not switch
Quitting the picker without picking SHALL leave the client unchanged and exit with code 130.

#### Scenario: Cancel
- **WHEN** the user presses `q` in the picker opened by `switch-session`
- **THEN** the client is not switched and the exit code is 130

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
