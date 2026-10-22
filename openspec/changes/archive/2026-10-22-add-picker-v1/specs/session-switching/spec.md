# Spec Delta

## Purpose

Lets users inside tmux jump their current client to any session, window or pane chosen through the pane-mux picker, typically from a `display-popup` key binding.

## ADDED Requirements

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
