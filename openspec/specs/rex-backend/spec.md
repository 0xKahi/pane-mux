# rex-backend Specification

## Purpose

Defines how pane-mux's picker, `switch-session` and `move-pane` behave when driving Rex sessions, windows and blocks instead of tmux, including what Rex cannot do and how pane-mux reports it.

## Requirements

### Requirement: Rex tree mapping
On the Rex backend the picker's three columns SHALL list Rex sessions, the windows (tabs) of the hovered session, and the blocks of the hovered window. Session and window rows SHALL show their Rex labels. Windows SHALL follow Rex's window order. Blocks SHALL list tiled blocks in layout order, then blocks in floating layers, minimized or not. Detached blocks SHALL NOT be listed. The block the picker runs in, identified by `REX_BLOCK`, SHALL NOT be listed when it is in a floating layer.

#### Scenario: Tiled and floating blocks
- **WHEN** window `config` has one tiled block and one floating block
- **THEN** the panes column lists the tiled block first and the floating block second

#### Scenario: Minimized float still listed
- **WHEN** a window has a minimized floating layer containing one block
- **THEN** that block appears in the panes column

#### Scenario: Picker session order
- **WHEN** Rex sessions `sidecar` and `home` exist and the user runs `pane-mux picker`
- **THEN** the sessions column lists `home` then `sidecar`, ordered by label

#### Scenario: Picker float hidden
- **WHEN** window `pi` has tiled blocks running `nvim` and `zsh`, and pane-mux runs in a floating block opened in `pi`
- **THEN** the panes column for `pi` lists only `nvim` and `zsh`

#### Scenario: Picker float hidden after refresh
- **WHEN** the picker runs in a floating block and the tree refreshes while the picker is open
- **THEN** the picker's block is still not listed

#### Scenario: Other floats stay listed
- **WHEN** pane-mux runs in a floating block and the same window has another floating block running `htop`
- **THEN** the `htop` block is listed and the picker's block is not

#### Scenario: Tiled self block stays listed
- **WHEN** pane-mux is run from a shell in a tiled block of window `pi`
- **THEN** that block is listed in the panes column for `pi`

### Requirement: Rex row numbering and pane names
On the Rex backend, window and pane rows SHALL be prefixed with their 1-based position in the window list or block list. Pane positions SHALL count only listed blocks. A pane row's name SHALL be the block's foreground process name, falling back to the block label when no process name is available.

#### Scenario: Positions
- **WHEN** the hovered session has windows `config`, `pi`, `pane-mux`
- **THEN** the rows read `1: config`, `2: pi`, `3: pane-mux`

#### Scenario: Process name
- **WHEN** a block labelled `shell` is running `nvim` in the foreground
- **THEN** its pane row shows `nvim`

#### Scenario: No gap for the hidden picker float
- **WHEN** window `pi` has a tiled `nvim` block, then the picker's floating block, then another floating block running `htop`
- **THEN** the pane rows read `1: nvim` and `2: htop`

### Requirement: Rex current-item markers
On the Rex backend the current session, window and pane SHALL be those of the Rex block the picker runs in, identified by `REX_BLOCK`. The picker SHALL mark them as it does under tmux. When that block is in a floating layer and therefore not listed, its session and window SHALL still be marked current and no pane SHALL be marked current. When that block cannot be found, no item SHALL be marked current.

#### Scenario: Picker in a floating block
- **WHEN** the picker runs in a floating block in window `pi` of session `home`
- **THEN** `home` and `pi` carry the current-item marker and no pane row carries it

#### Scenario: Picker in a tiled block
- **WHEN** the picker runs in a tiled block in window `pi` of session `home`
- **THEN** `home`, `pi` and that block carry the current-item marker

#### Scenario: Recency unaffected by the hidden float
- **WHEN** `switch-session` runs in a floating block in session `home`
- **THEN** `home` is listed first as the current session and a switch records `home` as the session left

### Requirement: Rex pick output
On the Rex backend, `<Enter>` in `pane-mux picker` SHALL print exactly one line: the full Rex ID of the hovered item at the output level, `session:<uuid>`, `window:<uuid>` or `block:<uuid>`. Level flags and exit codes SHALL behave as on tmux.

#### Scenario: Pane level
- **WHEN** the user runs `pane-mux picker --pane` under Rex and picks a block
- **THEN** stdout is exactly that block's ID, for example `block:01a20544-961c-77fa-a9ed-4987cd6e22b7\n`

#### Scenario: Session level from pane column
- **WHEN** the user runs `pane-mux picker --session` under Rex, focuses the panes column and presses `<Enter>`
- **THEN** stdout is the hovered session's ID, for example `session:01a1fa96-984e-7cf4-987f-c3267e96063a\n`

#### Scenario: Usable by rex
- **WHEN** a script runs `rex block focus "$(pane-mux picker --pane)"`
- **THEN** Rex focuses the picked block without needing a `-s` flag

### Requirement: Rex preview and self-block suppression
On the Rex backend the preview SHALL show the previewed block's visible contents with colours. Visible contents are the rows currently on the block's screen, one preview line per screen row, and SHALL NOT include the block's scrollback history. When the previewed block is the block the picker runs in, which can only happen when that block is tiled, the preview SHALL show the self-pane placeholder instead.

#### Scenario: Colours preserved
- **WHEN** the previewed block shows coloured `ls` output
- **THEN** the preview renders those colours

#### Scenario: Own block
- **WHEN** the picker runs in tiled block `B` and `B` is the previewed block
- **THEN** the preview shows the placeholder message

#### Scenario: Long history shows only the screen
- **WHEN** the previewed block is 40 rows tall, has printed 2,000 numbered lines, and its screen shows lines 1961 to 2000
- **THEN** the preview's first line is line 1961 and its last line is line 2000
- **AND** no line before 1961 appears in the preview

#### Scenario: Full-screen program
- **WHEN** the previewed block is running a full-screen program, such as an editor on the alternate screen
- **THEN** the preview shows that program's screen and none of the shell history behind it

#### Scenario: Wrapped line
- **WHEN** a line on the previewed block's screen is wider than the block and wraps onto a second screen row
- **THEN** the preview shows it as two lines, the same as the screen

#### Scenario: Partly filled screen
- **WHEN** the previewed block has no scrollback and only its first 3 screen rows hold text
- **THEN** the preview shows those 3 lines

### Requirement: Rex preview capture cost bounded
On the Rex backend, capturing a preview SHALL use a single `rex` invocation and SHALL transfer at most a fixed amount of block content to pane-mux. That amount SHALL NOT depend on the size of the block's scrollback, so a block with a long history previews about as quickly as one without.

#### Scenario: Large scrollback
- **WHEN** the previewed block holds 10 MB of scrollback
- **THEN** pane-mux runs one `rex` command to capture the preview
- **AND** that command's output to pane-mux is no larger than for a block whose history fits within that fixed amount

#### Scenario: Small block
- **WHEN** the previewed block holds 700 bytes of output
- **THEN** pane-mux runs one `rex` command to capture the preview and shows all of it

### Requirement: Rex availability errors
On the Rex backend, a missing `rex` executable or an unreachable Rex server SHALL make pane-mux print an error naming the problem to stderr and exit with code 2. pane-mux SHALL act on the server named by `REX_SERVER` when set, and SHALL never start a Rex server.

#### Scenario: No rex CLI
- **WHEN** the Rex backend is forced and `rex` is not on `PATH`
- **THEN** stderr says the `rex` command was not found and the exit code is 2

#### Scenario: Server not running
- **WHEN** the Rex backend is active and no Rex server answers
- **THEN** stderr says the Rex server is unreachable, no server is started, and the exit code is 2

### Requirement: Rex create, rename and delete
On the Rex backend the picker's create, rename and delete actions SHALL act on Rex sessions, windows and blocks with the same prompts, refusals and on-screen error handling as on tmux. A created session SHALL get one window with one shell block.

#### Scenario: Create a window
- **WHEN** the user creates a window named `logs` in session `home` under Rex
- **THEN** session `home` gains a window labelled `logs`, the app's active window does not change, and the cursor moves to `logs`

#### Scenario: Rex error shown
- **WHEN** a rename fails in Rex
- **THEN** the picker shows Rex's error message and stays open

### Requirement: Rex switch-session
On the Rex backend, `pane-mux switch-session` SHALL, on `<Enter>`, make the Rex app show the picked session and focus the picked window or block at the focused column's level, then exit with code 0.

#### Scenario: Switch to a block in another session
- **WHEN** the user runs `pane-mux switch-session` under Rex and picks block `B` in window `W` of session `sidecar`
- **THEN** the Rex app shows session `sidecar` with window `W` active and block `B` focused, and the exit code is 0

### Requirement: Remote Control fallback
When Rex refuses to change the app's view because Remote Control is off or no app client is connected, `switch-session` SHALL still focus the picked window or block inside its session, print to stderr a hint naming the target session and saying how to turn on Remote Control, and exit with code 0.

#### Scenario: Remote Control off
- **WHEN** Remote Control is off and the user picks session `sidecar` under Rex
- **THEN** stderr names `sidecar` and explains how to turn on Remote Control, the app view is unchanged, and the exit code is 0

### Requirement: Rex recency ranking
Under `switch-session` on Rex, sessions SHALL be listed current first, then sessions in pane-mux's recently-switched list from most to least recent, then the remaining sessions in Rex's session order. Sessions whose labels match `exclude_from_recency` SHALL come after all of those, ordered by label. The current session SHALL stay first even when excluded. The cursor SHALL open on the first non-current session.

#### Scenario: Recent first, then Rex order
- **WHEN** Rex orders sessions `home`, `sidecar`, `notes`, `scratch`, the client is on `home`, and pane-mux previously switched to `notes` and before that to `home`
- **THEN** the sessions column lists `home`, `notes`, `sidecar`, `scratch`, with the cursor on `notes`

#### Scenario: No history yet
- **WHEN** pane-mux has never switched sessions under Rex and Rex orders sessions `home`, `sidecar`, `notes`, with the client on `sidecar`
- **THEN** the sessions column lists `sidecar`, `home`, `notes`

#### Scenario: Excluded session
- **WHEN** `exclude_from_recency = ["scratch"]` and `scratch` is the most recently switched-to session
- **THEN** `scratch` is listed after every non-excluded session

### Requirement: Rex recency history
Every switch made by `pane-mux switch-session` on Rex, including one that ends in the Remote Control fallback, SHALL record the session it left and the session it targeted in a per-server history that persists between invocations. Sessions that no longer exist SHALL be ignored when ranking. Switches made outside pane-mux are not recorded.

#### Scenario: History persists
- **WHEN** the user switches to `notes` with `pane-mux switch-session`, then runs `pane-mux switch-session` again from `notes`
- **THEN** the session they came from is listed directly after `notes`

#### Scenario: Deleted session ignored
- **WHEN** the history contains a session that has since been killed
- **THEN** ranking skips it and no error is shown

### Requirement: Rex cross-session moves are refused
On the Rex backend, any paste that would move a block into a different session SHALL show the message `rex does not allow moving panes to different session` and change nothing. This SHALL include every paste with the sessions column focused, and pastes into windows of other sessions. The picker SHALL stay open with the cut buffer kept, and no prompt SHALL open.

#### Scenario: Paste into a new session
- **WHEN** a block is cut and the user presses `paste` with the sessions column focused under Rex
- **THEN** the message `rex does not allow moving panes to different session` is shown, no prompt opens, and nothing moves

#### Scenario: Paste into another session's window
- **WHEN** a block from session `home` is cut and the user pastes into a window of session `sidecar`
- **THEN** the same message is shown and nothing moves

#### Scenario: Under move-pane
- **WHEN** the user runs `pane-mux move-pane` under Rex and pastes into another session
- **THEN** the message is shown, the picker stays open, and the command has not exited

### Requirement: Rex same-session paste into a new window
On the Rex backend, with the windows column focused on the cut block's own session, `paste` SHALL prompt for a window name and move the cut block into a new window with that label in that session. The block's program SHALL keep running.

#### Scenario: Move to a new window
- **WHEN** a block running `htop` in session `home` is cut and pasted with the windows column focused on `home`, named `mon`
- **THEN** session `home` gains a window `mon` containing that block, still running `htop`, and no other block is left behind in it

### Requirement: Rex same-session floating paste
On the Rex backend, with the panes column focused on a window in the cut block's own session, `paste` SHALL move the cut block into the hovered window as a floating block sized and positioned by `[floating-pane]` width, height and position, without prompting.

#### Scenario: Float into another window
- **WHEN** a tiled block from window `pi` is cut and pasted with the panes column on window `config` of the same session, with `[floating-pane] width = 50, height = 50, position = "centre"`
- **THEN** the block floats in `config`, centred, at half the window's width and height, and is no longer in `pi`

#### Scenario: Same window is a no-op
- **WHEN** the hovered window already contains the cut block
- **THEN** paste does nothing

### Requirement: Rex moves keep the view and close empty windows
On the Rex backend a paste SHALL NOT change which session the Rex app shows. When the moved block was the last block in its window, that window SHALL close.

#### Scenario: Last block leaves its window
- **WHEN** the cut block is the only block in window `tmp` and it is pasted into another window of the same session
- **THEN** window `tmp` no longer exists

### Requirement: Indicators Rex does not support
On the Rex backend the copy-mode indicator and the stashed-float indicator SHALL never be shown, and paste SHALL NOT refuse a target because it is zoomed.

#### Scenario: No copy-mode indicator
- **WHEN** a Rex block is in Rex copy mode
- **THEN** its pane row shows no copy-mode indicator
