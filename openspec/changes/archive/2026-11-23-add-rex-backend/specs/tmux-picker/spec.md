# Spec Delta

## MODIFIED Requirements

### Requirement: Pick and print target
On `<Enter>`, the picker SHALL exit and print exactly one line to stdout: the target of the hovered item at the output level, in the active backend's target format. On the tmux backend the target SHALL be formatted `session`, `session:window_index` or `session:window_index.pane_index`. On the Rex backend the format is defined by `rex-backend`. The output level SHALL be set by `--session`, `--window` or `--pane` (mutually exclusive); without a flag it SHALL be the focused column's level. The UI SHALL never be written to stdout.

#### Scenario: Session flag from pane column
- **WHEN** the user runs `pane-mux picker --session`, focuses the panes column, hovers `work:1.2` and presses `<Enter>`
- **THEN** stdout is exactly `work\n` and the exit code is 0

#### Scenario: Default level follows focus
- **WHEN** the user runs `pane-mux picker` without a level flag, focuses the windows column on window `1` of `work` and presses `<Enter>`
- **THEN** stdout is exactly `work:1\n`

#### Scenario: Captured in a script
- **WHEN** a script runs `selected=$(pane-mux picker --session)` and the user picks `work`
- **THEN** the picker UI is visible on the terminal and `$selected` equals `work`

#### Scenario: Conflicting flags
- **WHEN** the user runs `pane-mux picker --session --pane`
- **THEN** the command prints a usage error to stderr and exits with code 2 without opening the UI

#### Scenario: Rex backend
- **WHEN** the Rex backend is active and the user picks a block with `pane-mux picker --pane`
- **THEN** stdout is exactly that block's full Rex ID followed by a newline
