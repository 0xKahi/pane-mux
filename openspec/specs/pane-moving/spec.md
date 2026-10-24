# pane-moving Specification

## Purpose

Lets users move tmux panes between windows and sessions from the pane-mux picker with yazi-style cut and paste, and through a one-shot `pane-mux move-pane` command for scripts and key bindings.

## Requirements

### Requirement: Cut a pane
With the panes column focused, the `cut` action (default `x`) SHALL put the hovered pane in a cut buffer that holds at most one pane. Cutting another pane SHALL replace the buffer, and cutting the pane already in the buffer SHALL empty it. Cutting does not change tmux.

#### Scenario: Cut a pane
- **WHEN** the panes column is focused on `work:1.2` and the user presses `x`
- **THEN** `work:1.2` is the cut pane and tmux is unchanged

#### Scenario: Cut another pane
- **WHEN** `work:1.2` is cut and the user presses `x` on `notes:0.0`
- **THEN** `notes:0.0` is the cut pane and `work:1.2` is no longer marked

#### Scenario: Uncut
- **WHEN** `work:1.2` is cut and the user presses `x` on `work:1.2`
- **THEN** the cut buffer is empty and no pane is marked

#### Scenario: Cut a floating pane
- **WHEN** the hovered pane is floating and the user presses `x`
- **THEN** the floating pane is the cut pane

### Requirement: Cut is refused outside the panes column
The `cut` action SHALL show a short on-screen message and leave the buffer unchanged when the sessions or windows column is focused.

#### Scenario: Cut from the windows column
- **WHEN** the windows column is focused and the user presses `x`
- **THEN** a message says only panes can be cut and the buffer is unchanged

### Requirement: Floating panes move like tiled panes
A cut floating pane SHALL be pasteable in all three ways. Pasted as a new session or a new window, it SHALL become a normal tiled pane, the only pane of that window. Pasted into another window from the panes column, it SHALL float there, sized and positioned by `[floating-pane]`.

#### Scenario: Floating pane to a new window
- **WHEN** a floating pane in `work:1` is cut and pasted from the windows column on session `notes` as window `scratch`
- **THEN** `notes` has a new window `scratch` whose only pane is the former floating pane, no longer floating, and `work:1` keeps its other panes

#### Scenario: Floating pane to a new session
- **WHEN** a floating pane is cut and pasted from the sessions column as session `logs`
- **THEN** session `logs` has exactly one window, holding the former floating pane as a tiled pane

#### Scenario: Floating pane to another window
- **WHEN** a floating pane in `work:1` is cut and pasted from the panes column on window `notes:0`
- **THEN** the pane floats in `notes:0` at the `[floating-pane]` size and position

### Requirement: Paste needs a cut pane
The `paste` action (default `p`) SHALL show a short on-screen message and do nothing when the cut buffer is empty.

#### Scenario: Paste with nothing cut
- **WHEN** the buffer is empty and the user presses `p`
- **THEN** a message says nothing is cut and tmux is unchanged

### Requirement: Paste into a new session
With the sessions column focused, `paste` SHALL prompt for a session name and then for a window name. On submitting both, pane-mux SHALL create a detached session with that name whose only window holds the cut pane under that window name. An empty name SHALL use tmux's default naming. Cancelling either prompt SHALL change nothing.

#### Scenario: Paste as a new session
- **WHEN** `work:1.2` is cut, the sessions column is focused, the user presses `p`, enters `logs`, then enters `tail`
- **THEN** session `logs` exists with exactly one window named `tail` containing the former `work:1.2` pane, and the cursor is on session `logs`

#### Scenario: Cancel at the window prompt
- **WHEN** the user enters a session name and presses `<Esc>` at the window name prompt
- **THEN** no session is created, the pane is not moved and the buffer still holds it

#### Scenario: Session name already taken
- **WHEN** the user enters the name of an existing session
- **THEN** tmux's error is shown, the picker stays open, the pane is not moved and no new session remains

### Requirement: Paste into a new window
With the windows column focused, `paste` SHALL prompt for a window name and, on submit, move the cut pane into a new window with that name in the hovered session. An empty name SHALL use tmux's default naming. Cancelling the prompt SHALL change nothing.

#### Scenario: Paste as a new window
- **WHEN** `work:1.2` is cut, the windows column is focused on session `notes`, the user presses `p` and enters `scratch`
- **THEN** `notes` has a new window `scratch` containing only the former `work:1.2` pane, and the cursor is on that window

#### Scenario: Paste a pane into its own session
- **WHEN** the cut pane is in session `work` and the user pastes it into a new window in `work`
- **THEN** the pane moves to the new window in `work`

### Requirement: Paste into an existing window as a floating pane
With the panes column focused, `paste` SHALL move the cut pane into the hovered window as a floating pane, sized and positioned by `[floating-pane]`, without prompting. When the hovered window is the one the cut pane is already in, `paste` SHALL do nothing. When the hovered window is zoomed, `paste` SHALL show a short message and do nothing.

#### Scenario: Paste as a floating pane
- **WHEN** `work:1.2` is cut, the panes column shows window `notes:0`, and the user presses `p`
- **THEN** the pane is floating in `notes:0`, sized `width`% by `height`% of the window at `position`, and the cursor is on it

#### Scenario: Paste into the same window
- **WHEN** `work:1.2` is cut and the panes column shows window `work:1`, and the user presses `p`
- **THEN** nothing changes and the buffer still holds the pane

#### Scenario: Zoomed target window
- **WHEN** the hovered window is zoomed and the user presses `p`
- **THEN** a message says the window is zoomed and the pane is not moved

### Requirement: Moves are detached and empty containers close
Pasting SHALL NOT change which session, window or pane the tmux client shows. When the moved pane was the last pane in its window, that window SHALL close, and when it was the last window in its session, that session SHALL close, following tmux's normal behaviour.

#### Scenario: Client view unchanged
- **WHEN** the client shows `work:1` and the user moves a pane from `work:1` to session `logs`
- **THEN** the client still shows `work:1`

#### Scenario: Last pane in a window
- **WHEN** the cut pane is the only pane in window `work:3` and it is pasted elsewhere
- **THEN** window `work:3` no longer exists

### Requirement: Result of a paste
After a successful paste in `pane-mux picker`, the picker SHALL stay open, the cut buffer SHALL be empty and the cursor SHALL move to the created session or window, or to the floating pane. If tmux fails, the picker SHALL show tmux's error, stay open and keep the buffer.

#### Scenario: Picker stays open
- **WHEN** the user pastes a pane in `pane-mux picker`
- **THEN** the picker remains open with no pane marked as cut

#### Scenario: tmux failure
- **WHEN** tmux rejects the floating paste (e.g. floating panes are unsupported)
- **THEN** the picker shows tmux's error, stays open, and the buffer still holds the pane

### Requirement: Cut buffer follows the pane
The cut buffer SHALL track the pane by its stable tmux id, so the indicator follows the pane if it moves or is renumbered. When the cut pane no longer exists, the buffer SHALL be cleared on the next refresh.

#### Scenario: Cut pane exits
- **WHEN** `work:1.2` is cut and its process exits, closing the pane
- **THEN** after the next refresh the buffer is empty and no badge is shown

### Requirement: Cut indicator
The cut pane's row SHALL show the theme's `[marker] symbol` in `[marker] cut` style in the panes column's left edge cell, independent of hover and current-item styling. While the buffer holds a pane, the header SHALL show a right-aligned badge `cut <session>:<window_index>.<pane_index>` in `[marker] badge_cut` style.

#### Scenario: Marker on the cut row
- **WHEN** `work:1.2` is cut and window `work:1` is shown in the panes column
- **THEN** pane `2`'s row has a light red bar at the column's left edge

#### Scenario: Hovered cut pane
- **WHEN** the cut pane's row is also hovered
- **THEN** the row shows both the hover style and the light red bar

#### Scenario: Header badge
- **WHEN** `work:1.2` is cut
- **THEN** the header shows `cut work:1.2` at its right edge in the badge style

### Requirement: Move-pane command
`pane-mux move-pane [TARGET]` SHALL resolve `TARGET` (any tmux pane target, e.g. `test:1.2` or `%17`) to a pane, or use the invoking client's current pane when `TARGET` is omitted, and open the picker with that pane cut and the header `[Move]`, with the sessions column focused and the cursor on the session that holds the cut pane. An unresolvable target, or an omitted target outside tmux, SHALL print an error to stderr and exit with code 2 without opening the picker.

#### Scenario: Cursor starts on the source session
- **WHEN** the user runs `pane-mux move-pane notes:2.1` and sessions are listed `alpha`, `notes`, `work`
- **THEN** the sessions column is focused with the cursor on `notes`

#### Scenario: Explicit target
- **WHEN** the user runs `pane-mux move-pane test:1.2`
- **THEN** the picker opens with header `[Move]` and `test:1.2` cut

#### Scenario: Current pane
- **WHEN** the user runs `pane-mux move-pane` from a popup opened on pane `work:0.1`
- **THEN** the picker opens with `work:0.1` cut

#### Scenario: Unknown target
- **WHEN** the user runs `pane-mux move-pane nosuch:9.9`
- **THEN** stderr explains the target could not be found and the exit code is 2

#### Scenario: Outside tmux without a target
- **WHEN** the user runs `pane-mux move-pane` with no `TMUX` environment
- **THEN** stderr explains that a target is required outside tmux and the exit code is 2

### Requirement: Move-pane cut is locked
Under `pane-mux move-pane`, the `cut` action SHALL NOT change the buffer and SHALL show a short message instead.

#### Scenario: Cut in move-pane mode
- **WHEN** the user presses `x` on any pane under `pane-mux move-pane`
- **THEN** a message says the cut is locked and the original pane stays cut

### Requirement: Move-pane exits after paste
Under `pane-mux move-pane`, a successful paste SHALL close the picker and exit with code 0. A paste that does nothing (same window, cancelled prompt) or fails SHALL keep the picker open. Quitting without pasting SHALL move nothing and exit with code 130.

#### Scenario: Paste closes the picker
- **WHEN** the user pastes the pane into a new window under `pane-mux move-pane`
- **THEN** the pane is moved, the picker closes and the exit code is 0

#### Scenario: Quit
- **WHEN** the user presses `q` under `pane-mux move-pane`
- **THEN** nothing is moved and the exit code is 130

#### Scenario: Cut pane disappears
- **WHEN** the locked pane closes while `pane-mux move-pane` is open
- **THEN** the buffer is cleared on the next refresh, a message says the pane no longer exists, and `q` exits with code 130
