
# Spec Delta

## Purpose

Lets users stash the floating panes of a tmux window and later unstash them exactly where they were, using the `pane-mux float stash`, `unstash` and `toggle` commands backed by a dedicated `pmux` stash session.

## ADDED Requirements

### Requirement: Float commands target a window
`pane-mux float stash`, `pane-mux float unstash` and `pane-mux float toggle` SHALL each accept an optional tmux target and act on the window containing it. Without a target they SHALL act on the current client's window. Without a target outside tmux, or with an unknown target, they SHALL exit 2 with an error.

#### Scenario: Default to the current window
- **WHEN** the user runs `pane-mux float stash` from a key binding in window `work:1`
- **THEN** the floating panes of `work:1` are stashed

#### Scenario: Explicit target
- **WHEN** the user runs `pane-mux float unstash notes:0.1`
- **THEN** the floats stashed for window `notes:0` are restored

#### Scenario: Outside tmux without a target
- **WHEN** the user runs `pane-mux float toggle` outside tmux with no target
- **THEN** the command exits 2 with an error and changes nothing

#### Scenario: Unknown target
- **WHEN** the user runs `pane-mux float stash nosuch:9`
- **THEN** the command exits 2 with an error and changes nothing

### Requirement: Stash moves all floating panes of the window
`float stash` SHALL move every floating pane of the target window into that window's stash window in the `pmux` session, where each stays floating at its original size and position. The tiled panes of the target window SHALL be unchanged.

#### Scenario: Stash two floats
- **WHEN** window `work:1` has one tiled pane and two floating panes, and the user runs `float stash`
- **THEN** `work:1` has only its tiled pane, and both floats are in the stash window for `work:1`, floating at their original size and position

#### Scenario: Stash with no floats
- **WHEN** the target window has no floating panes and nothing stashed, and the user runs `float stash`
- **THEN** nothing moves, tmux shows a "no floating panes" message, and the command exits 0

### Requirement: Moves do not disturb other panes
Stash and unstash SHALL NOT split or resize any tiled pane of the source window or the stash window, at any point during the command, except for the zoom changes required by Zoom handling. A moved float SHALL NOT be resized when the size of the window it moves into matches the size it was recorded at.

#### Scenario: Main pane undisturbed
- **WHEN** the main pane of `work:1` runs a program that redraws on resize, and the user runs `float stash` and then `float unstash`
- **THEN** the main pane receives no resize at any point

#### Scenario: Float undisturbed
- **WHEN** a float is stashed and unstashed while the source window keeps its size
- **THEN** the float's program receives no resize

### Requirement: Stash session and stash windows
Stashed floats SHALL live in a session named `pmux`, created on first stash, with one stash window per source window. A stash window SHALL be found by the identity of its source window, so renaming or renumbering the source does not lose it. If a `pmux` session exists that pane-mux did not create, stash SHALL fail.

#### Scenario: First stash creates the session
- **WHEN** no `pmux` session exists and the user stashes the floats of `work:1`
- **THEN** a `pmux` session exists with a window named `work:1` holding a placeholder pane and the stashed floats

#### Scenario: Renamed source window
- **WHEN** floats of window `work:1` are stashed, the window is renamed to `build`, and the user runs `float unstash` on it
- **THEN** the stashed floats are restored to that window

#### Scenario: Foreign pmux session
- **WHEN** the user already has their own session named `pmux` and runs `float stash`
- **THEN** the command exits 2 with an error, and neither session changes

### Requirement: Stash windows carry a placeholder label
Each stash window SHALL be created with a placeholder tiled pane, named after the source `session:window` and saying how to restore. The placeholder is informational: stash and unstash SHALL work when it no longer exists.

#### Scenario: Placeholder was killed
- **WHEN** the user killed the placeholder pane of a stash window that still holds floats, then stashes more floats from the same source window
- **THEN** all floats for that source, old and new, are stashed in that one stash window, and a later `float unstash` restores them all

### Requirement: Stash merges into an existing stash
When the target window already has stashed floats, `float stash` SHALL add the window's current floats to the same stash window. Previously stashed floats SHALL keep their recorded geometry.

#### Scenario: Stash after creating a new float
- **WHEN** floats A and B of `work:1` are stashed, the user creates float C in `work:1`, and runs `float stash`
- **THEN** the stash for `work:1` holds A, B and C, and `work:1` has no floats

### Requirement: Unstash restores stashed floats exactly
`float unstash` SHALL move every float stashed for the target window back into it as floating panes with their size, position and stacking order at stash time. If the window's size differs from the size at stash time, size and position SHALL be scaled proportionally to the new size. Resizing the stash window SHALL NOT affect the restored geometry. Unstash SHALL keep any floats already visible in the window.

#### Scenario: Unchanged window size
- **WHEN** a 60x15 float at column 120, line 5 of a 200x50 window is stashed and unstashed with the window still 200x50
- **THEN** it is again 60x15 at column 120, line 5

#### Scenario: Window shrank
- **WHEN** a float occupying the right third of a window is stashed, the terminal shrinks, and the user runs `float unstash`
- **THEN** the float occupies the right third of the smaller window and the unstash succeeds

#### Scenario: Stacking order preserved
- **WHEN** float A is above float B at stash time
- **THEN** after unstash, A is above B

#### Scenario: Unstash next to visible floats
- **WHEN** the window has a visible float C and stashed floats A and B, and the user runs `float unstash`
- **THEN** A, B and C are all visible in the window

#### Scenario: Nothing stashed
- **WHEN** the target window has nothing stashed and the user runs `float unstash`
- **THEN** nothing moves, tmux shows a "nothing stashed" message, and the command exits 0

### Requirement: Unstash cleans up the stash
After every stashed float of a window has been restored, its stash window SHALL be removed. When no stash windows remain, the `pmux` session SHALL no longer exist.

#### Scenario: Last stash restored
- **WHEN** `work:1` is the only window with stashed floats and the user runs `float unstash` on it
- **THEN** the `pmux` session no longer exists

#### Scenario: Other stashes remain
- **WHEN** both `work:1` and `notes:0` have stashed floats and the user unstashes `work:1`
- **THEN** the stash window for `work:1` is gone and the one for `notes:0` remains

### Requirement: Toggle chooses by floats in the window
`float toggle` SHALL behave as `float stash` when the target window has at least one floating pane, including floats hidden by zoom, and as `float unstash` otherwise.

#### Scenario: Visible floats
- **WHEN** the window has floating panes and the user runs `float toggle`
- **THEN** they are stashed

#### Scenario: No floats
- **WHEN** the window has no floating panes, has stashed floats, and the user runs `float toggle`
- **THEN** the stashed floats are restored

#### Scenario: Mixed state
- **WHEN** the window has a visible float C and stashed floats A and B, and the user runs `float toggle` twice
- **THEN** the first toggle stashes C with A and B, and the second restores A, B and C

### Requirement: Focus follows stash and unstash
When the active pane of the target window is stashed, the last-active tiled pane of that window SHALL become active. On unstash, the float that was active at stash time SHALL become the active pane again. When no stashed float was active, unstash SHALL NOT change the active pane.

#### Scenario: Stash the focused float
- **WHEN** the user was in tiled pane `%0`, moved into float `%3`, and runs `float stash`
- **THEN** `%0` is the active pane

#### Scenario: Unstash refocuses
- **WHEN** float `%3` was active when it was stashed and the user runs `float unstash`
- **THEN** `%3` is the active pane

#### Scenario: Unfocused floats
- **WHEN** a tiled pane was active at stash time and the user runs `float unstash`
- **THEN** the active pane is unchanged

### Requirement: Zoom handling
Stashing floats in a zoomed window SHALL stash every float of the window, including floats that zoom hides, and SHALL leave the window zoomed on the same pane. Unstashing floats into a zoomed window SHALL unzoom it first.

#### Scenario: Stash while zoomed
- **WHEN** tiled pane `%0` is zoomed, the window has two floats hidden by the zoom, and the user runs `float stash`
- **THEN** both floats are stashed at their original geometry and `%0` is still zoomed

#### Scenario: Unstash while zoomed
- **WHEN** the window is zoomed and has stashed floats, and the user runs `float unstash`
- **THEN** the window is unzoomed and the floats are visible

### Requirement: Float commands refuse stash windows
`float stash`, `unstash` and `toggle` SHALL exit 2 with an error when the target window is a stash window in the `pmux` session.

#### Scenario: Toggle inside pmux
- **WHEN** the user switches to the `pmux` session and runs `float toggle` there
- **THEN** the command exits 2 with an error and nothing moves

### Requirement: Partial failures are reported
If moving one pane fails, the command SHALL still try the remaining panes, then exit 2 with an error naming each failed pane. Panes already moved SHALL stay where they are. Running the same command again SHALL finish the job, and no temporary pane created by pane-mux SHALL remain after it.

#### Scenario: One pane fails
- **WHEN** stashing three floats and tmux rejects the second
- **THEN** the first and third are stashed, the command exits 2 naming the second, and running `float stash` again stashes the second

### Requirement: Concurrent float commands are serialized
Float commands running at the same time on one tmux server SHALL run one after another, so a quickly repeated toggle cannot interleave a stash with an unstash.

#### Scenario: Double-tapped toggle
- **WHEN** a window has visible floats and the user presses a `float toggle` binding twice quickly
- **THEN** the first toggle stashes all floats and the second restores them all, with no pane left in between

### Requirement: Orphaned stashes are left alone
A stash window whose source window no longer exists SHALL remain in `pmux` until the user removes it. Float commands SHALL NOT delete or move it.

#### Scenario: Source window closed
- **WHEN** floats of `work:1` are stashed and the user kills `work:1`
- **THEN** the stash window and its floats remain in `pmux`, and the user can kill them or cut and paste them from the picker
