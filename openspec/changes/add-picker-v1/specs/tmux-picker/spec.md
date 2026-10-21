# Spec Delta

## Purpose

Lets users browse, preview and manage their tmux sessions, windows and panes from a keyboard-driven TUI, and pick one as a tmux target that is printed to stdout for use in shell scripts and other pane-mux commands.

## ADDED Requirements

### Requirement: Three-column tmux tree
The picker SHALL display three always-visible columns: sessions, windows of the hovered session, and panes of the hovered window. Each row SHALL be prefixed with its level icon (session ``, window `󱂬`, pane ``) unless the theme overrides it. On open, the sessions column SHALL be focused.

#### Scenario: Initial view
- **WHEN** the user runs `pane-mux picker` with tmux sessions `work` and `dotfiles`
- **THEN** the sessions column lists `work` and `dotfiles` and is focused
- **AND** the windows column lists the windows of the hovered session
- **AND** the panes column lists the panes of that session's active window

#### Scenario: Hover changes child columns
- **WHEN** the user moves the cursor from session `work` to session `dotfiles`
- **THEN** the windows and panes columns update to show `dotfiles`' windows and its active window's panes

### Requirement: Column navigation
The picker SHALL let the user move the cursor within the focused column (next, previous, page, top, bottom) and move focus to the previous or next column. Focus SHALL NOT move past the sessions or panes columns.

#### Scenario: Move focus right
- **WHEN** the sessions column is focused and the user presses `l`
- **THEN** the windows column becomes focused with the cursor on the session's active window

#### Scenario: Boundary
- **WHEN** the panes column is focused and the user presses `l`
- **THEN** focus remains on the panes column

### Requirement: Persistent pane preview
The picker SHALL show a rounded-border preview box containing the visible contents (with colours) of one pane, chosen as follows: sessions column focused → hovered session's active window's active pane; windows column focused → hovered window's active pane; panes column focused → hovered pane.

#### Scenario: Preview follows session hover
- **WHEN** the sessions column is focused and session `work` is hovered, whose active window is `1` with active pane `0`
- **THEN** the preview shows the contents of `work:1.0`

#### Scenario: Preview follows pane hover
- **WHEN** the panes column is focused and pane `work:1.2` is hovered
- **THEN** the preview shows the contents of `work:1.2`

### Requirement: Preview layout configuration
The preview box SHALL span the full width of the screen and be placed below the columns by default, or above them when `preview_position = "top"`. Its height SHALL be `preview_ratio` percent of the available area, defaulting to 50.

#### Scenario: Defaults
- **WHEN** no user config sets preview options
- **THEN** the preview occupies the bottom 50% of the picker area

#### Scenario: Top preview
- **WHEN** config sets `preview_position = "top"` and `preview_ratio = 75`
- **THEN** the preview occupies the top 75% and the columns the remaining area below it

### Requirement: Preview scrolling
The user SHALL be able to scroll the preview contents up and down by half a page (`<C-u>` / `<C-d>` by default). Scroll position SHALL reset when the previewed pane changes.

#### Scenario: Scroll preview
- **WHEN** the user presses `<C-u>` while the preview shows a pane with history
- **THEN** the preview shows content half a page further up

### Requirement: Self-pane preview suppression
When the picker runs inside a regular tmux pane, the preview SHALL NOT render that pane's contents and SHALL show a placeholder instead. When the picker runs inside a tmux popup, no pane is suppressed.

#### Scenario: Picker in a normal pane
- **WHEN** the picker runs in pane `work:1.0` and that pane is the previewed pane
- **THEN** the preview shows a placeholder message instead of the pane contents

#### Scenario: Picker in a popup
- **WHEN** the picker runs inside `tmux display-popup` and the previewed pane is the client's current pane
- **THEN** the preview shows that pane's contents

### Requirement: Live refresh
The picker SHALL periodically refresh the tmux tree and the preview so that external changes (new/killed sessions, pane output) appear without user input, preserving the cursor on the same item when it still exists.

#### Scenario: External session created
- **WHEN** another process runs `tmux new-session -d -s extra` while the picker is open
- **THEN** `extra` appears in the sessions column within about one second

### Requirement: Pick and print target
On `<Enter>`, the picker SHALL exit and print exactly one line to stdout: the tmux target of the hovered item at the output level, formatted `session`, `session:window_index` or `session:window_index.pane_index`. The output level SHALL be set by `--session`, `--window` or `--pane` (mutually exclusive); without a flag it SHALL be the focused column's level. The UI SHALL never be written to stdout.

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

### Requirement: Cancel and exit codes
Quitting without picking SHALL print nothing to stdout and exit with code 130. Errors (tmux unavailable, no tmux server, invalid config) SHALL print a message to stderr and exit with code 2.

#### Scenario: Cancel
- **WHEN** the user presses `q` in the picker
- **THEN** stdout is empty and the exit code is 130

#### Scenario: No tmux server
- **WHEN** no tmux server is running
- **THEN** the picker prints an error to stderr and exits with code 2

### Requirement: Current-item markers
The picker SHALL visually mark the current session, window and pane (those of the tmux client the picker belongs to) using the theme's current style.

#### Scenario: Current session marked
- **WHEN** the picker is opened from client attached to session `work`
- **THEN** `work` is rendered with the current style

### Requirement: Create session and window
The picker SHALL let the user create a session (sessions column focused) or a window in the hovered session (windows column focused) via a name input prompt. Submitting an empty name SHALL use tmux's default naming. Creation SHALL be a no-op when the panes column is focused.

#### Scenario: Create session
- **WHEN** the sessions column is focused, the user presses `a`, types `notes` and presses `<Enter>`
- **THEN** a detached session `notes` is created and the cursor moves to it

#### Scenario: Create window
- **WHEN** the windows column is focused on session `work`, the user presses `a`, types `logs` and presses `<Enter>`
- **THEN** a window named `logs` is created in `work` and the cursor moves to it

#### Scenario: Cancel input
- **WHEN** the input prompt is open and the user presses `<Esc>`
- **THEN** the prompt closes and nothing is created

### Requirement: Rename session and window
The picker SHALL let the user rename the hovered session or window via an input prompt pre-filled with the current name. Rename SHALL be a no-op when the panes column is focused.

#### Scenario: Rename session
- **WHEN** session `work` is hovered, the user presses `r`, replaces the text with `job` and presses `<Enter>`
- **THEN** the session is renamed to `job` and remains hovered

### Requirement: Delete with confirmation
The picker SHALL let the user delete the hovered session, window or pane after a confirmation prompt. Deleting the current session, window or pane SHALL be refused with an on-screen error and no prompt.

#### Scenario: Delete confirmed
- **WHEN** the user hovers window `dotfiles:2`, presses `d d` and confirms with `y`
- **THEN** window `dotfiles:2` is killed and removed from the list

#### Scenario: Delete declined
- **WHEN** the confirmation prompt is open and the user presses `n`
- **THEN** nothing is deleted

#### Scenario: Delete current refused
- **WHEN** the user hovers the current session and presses `d d`
- **THEN** no prompt appears, an error message is shown, and nothing is deleted

### Requirement: tmux errors are non-fatal in the UI
If a create, rename or delete operation fails in tmux, the picker SHALL show the tmux error message on screen and remain open.

#### Scenario: Duplicate session name
- **WHEN** the user creates a session named `work` while `work` already exists
- **THEN** the picker shows tmux's error and stays open

### Requirement: Multi-key sequences and help
The picker SHALL support key bindings made of multiple keys (e.g. `g g`, `d d`), showing a popup of possible continuations after a prefix key. A help popup (`?` / `<F1>`) SHALL list all active bindings with descriptions.

#### Scenario: Which popup
- **WHEN** the user presses `g`
- **THEN** a popup lists bindings starting with `g`, and pressing `g` again moves the cursor to the top

#### Scenario: Help
- **WHEN** the user presses `?`
- **THEN** a popup lists the key bindings and their descriptions
