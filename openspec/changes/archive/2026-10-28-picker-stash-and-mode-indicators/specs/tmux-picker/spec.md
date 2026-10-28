# Spec Delta

## ADDED Requirements

### Requirement: Stashed-float indicator on window rows
A window row SHALL show the theme's `[icons] stashed` indicator when that window has floats stashed by `pane-mux float stash`, with `$count` replaced by the number of stashed floats. The stash SHALL be matched to its source window by identity, so renaming or renumbering the window keeps the indicator. Windows with nothing stashed SHALL show no stash indicator.

#### Scenario: Window with stashed floats
- **WHEN** two floats of window `work:1` are stashed and the picker shows the windows of `work`
- **THEN** the `1: editor` row reads `1: editor  󰦤 2` with `󰦤 2` in `lightcyan`

#### Scenario: Nothing stashed
- **WHEN** window `work:2` has no stashed floats
- **THEN** its row shows no stash indicator

#### Scenario: Renamed source window
- **WHEN** floats of window `work:1` are stashed and the window is then renamed to `build`
- **THEN** the `1: build` row shows the stash indicator with the same count

#### Scenario: Placeholder pane killed
- **WHEN** one float of `work:1` is stashed and the user killed that stash window's placeholder pane
- **THEN** the `work:1` row shows `󰦤 1`

#### Scenario: Live update after unstash
- **WHEN** the picker is open showing `work:1` with a stash indicator and another client runs `pane-mux float unstash` on `work:1`
- **THEN** the indicator disappears from the `work:1` row within about one second

### Requirement: Pane-mode indicator on pane rows
A pane row SHALL show the theme's `[icons] copy_mode` indicator when the pane is in tmux `copy-mode` or `view-mode`. Panes in no mode, or in any other mode, SHALL show no mode indicator.

#### Scenario: Pane in copy mode
- **WHEN** pane `work:1.1` running `zsh` is in copy mode and the picker shows the panes of `work:1`
- **THEN** the `1: zsh` row reads `1: zsh  󱣱` with `󱣱` in `lightyellow`

#### Scenario: Pane in view mode
- **WHEN** pane `work:1.2` shows `run-shell` output in view mode
- **THEN** its row shows the mode indicator

#### Scenario: Pane in another mode
- **WHEN** pane `work:1.0` is in clock mode
- **THEN** its row shows no mode indicator

### Requirement: Row indicator placement
A row's stash or mode indicator SHALL follow its name after two spaces, inline rather than aligned to the column edge. When the find `[X/Y]` badge is also shown, it SHALL follow the indicator. When the row is too narrow, the name SHALL be clipped before the indicator and badge are.

#### Scenario: Indicator before find badge
- **WHEN** a finder matches the hovered window `1: editor`, which has two stashed floats
- **THEN** the row reads `1: editor  󰦤 2 [X/Y]`

#### Scenario: Long name in a narrow column
- **WHEN** a window with stashed floats has a name longer than its column
- **THEN** the name is clipped and the stash indicator is fully visible before the row's end cap

### Requirement: Row indicator colours follow hover fill
On rows filled by `bg` or `reversed`, stash and mode indicators SHALL drop their colours, so the row background stays uniform. On unhovered and underline-only rows they SHALL keep their own colours.

#### Scenario: Reversed hovered row
- **WHEN** a pane in copy mode is hovered in the focused column with the default theme
- **THEN** the mode indicator cells have the same background as the name cells

#### Scenario: Underlined hovered row
- **WHEN** a window with stashed floats is hovered in a column right of the focused one
- **THEN** the row is underlined and the stash indicator keeps its `lightcyan` colour
