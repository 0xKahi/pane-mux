# Spec Delta

## MODIFIED Requirements

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

### Requirement: Rex preview and self-block suppression
On the Rex backend the preview SHALL show the previewed block's visible contents with colours. When the previewed block is the block the picker runs in, which can only happen when that block is tiled, the preview SHALL show the self-pane placeholder instead.

#### Scenario: Colours preserved
- **WHEN** the previewed block shows coloured `ls` output
- **THEN** the preview renders those colours

#### Scenario: Own block
- **WHEN** the picker runs in tiled block `B` and `B` is the previewed block
- **THEN** the preview shows the placeholder message
