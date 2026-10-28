# Spec Delta

## MODIFIED Requirements

### Requirement: Theme styles
Theme styles SHALL accept `fg` and `bg` as named colours, `"#rrggbb"`, or `"reset"`, plus boolean modifiers (`bold`, `italic`, `underline`, `reversed`, `dim`). The theme SHALL include configurable icon text and colour for sessions, windows and panes, and for the picker's row indicators: `[icons] stashed` (default `"\U000F09A4 $count"`, `lightcyan`, where `$count` is replaced by the number of stashed floats) and `[icons] copy_mode` (default `"\U000F18F1"`, `lightyellow`). Empty icon text SHALL hide that icon or indicator. The theme SHALL also include an `[indicator]` section with yazi-style `parent` / `current` / `preview` hovered-row styles, `padding = { open, close }` (hovered-row caps) and `active = { text, fg }` (current-item marker, default `*` magenta). The `[input]` section SHALL style input prompts with `border` (default yellow), `title` (default red), `value` (default white) and `selected` (the text cursor, default reversed). `[picker]` SHALL include `find_keyword` (default yellow, bold, italic, underline) and `find_position` (default magenta on `reset`, bold, italic) for find highlights. The `[marker]` section SHALL style the cut indicator with `cut` (default `{ fg = "lightred", bg = "lightred" }`), `symbol` (default `"│"`) and `badge_cut` (the header badge, default white on red).

#### Scenario: Custom icon
- **WHEN** `theme.toml` sets `[icons] session = { text = "S" }`
- **THEN** session rows are prefixed with `S`

#### Scenario: Input cursor style
- **WHEN** `theme.toml` sets `[input] selected = { bg = "blue", reversed = false }`
- **THEN** the input prompt's cursor cell has a blue background and is not reversed

#### Scenario: Partial input override keeps new defaults
- **WHEN** `theme.toml` sets only `[input] border = { fg = "green" }`
- **THEN** the prompt border is green and `title`, `value` and `selected` keep their defaults

#### Scenario: Find keyword style
- **WHEN** `theme.toml` sets `[picker] find_keyword = { fg = "red" }` and a finder is active
- **THEN** the matched text in the focused column is red

#### Scenario: Custom cut marker
- **WHEN** `theme.toml` sets `[marker] cut = { fg = "yellow", bg = "yellow" }` and a pane is cut
- **THEN** the cut pane's marker is yellow and `symbol` and `badge_cut` keep their defaults

#### Scenario: Custom stash indicator text
- **WHEN** `theme.toml` sets `[icons] stashed = { text = "stash:$count" }` and window `work:1` has three stashed floats
- **THEN** the `work:1` row shows `stash:3` in the default `lightcyan`

#### Scenario: Stash indicator without a count
- **WHEN** `theme.toml` sets `[icons] stashed = { text = "S" }` and a window has stashed floats
- **THEN** its row shows `S` with no number

#### Scenario: Hidden mode indicator
- **WHEN** `theme.toml` sets `[icons] copy_mode = { text = "" }` and a pane is in copy mode
- **THEN** its row shows no mode indicator and no trailing gap after the name
