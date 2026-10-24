# Spec Delta

## ADDED Requirements

### Requirement: Floating-pane settings
`config.toml` SHALL support `[floating-pane]` with `width` and `height` (integer percent of the target window, 1-100, default 50 each) and `position` (one of tmux's floating positions: `top-left`, `top-centre`, `top-right`, `centre-left`, `centre`, `centre-right`, `bottom-left`, `bottom-centre`, `bottom-right`, `top-left-centre`, `top-right-centre`, `bottom-left-centre`, `bottom-right-centre`; default `centre`). Out-of-range or unknown values SHALL be invalid config.

#### Scenario: Defaults
- **WHEN** no user `config.toml` sets `[floating-pane]`
- **THEN** a pane pasted as floating is 50% wide, 50% high and centred in the target window

#### Scenario: Custom placement
- **WHEN** `config.toml` sets `[floating-pane] width = 80`, `height = 30` and `position = "bottom-centre"`
- **THEN** a pane pasted as floating is 80% wide, 30% high and placed at the bottom centre

#### Scenario: Invalid position
- **WHEN** `config.toml` sets `[floating-pane] position = "middle"`
- **THEN** pane-mux prints the file path and an error naming `position` to stderr and exits with code 2

#### Scenario: Invalid size
- **WHEN** `config.toml` sets `[floating-pane] width = 0`
- **THEN** pane-mux exits with code 2 and an error naming `width`

### Requirement: Cut and paste key bindings
The default `[picker]` keymap SHALL bind `x` to `cut` and `p` to `paste`, and `cut` and `paste` SHALL be valid `run` commands that take no arguments.

#### Scenario: Default bindings
- **WHEN** no user `keymap.toml` overrides them
- **THEN** pressing `x` cuts the hovered pane and `p` pastes it, and both appear in the help popup

#### Scenario: Rebinding
- **WHEN** `[picker] prepend_keymap = [{ on = "<C-x>", run = "cut" }]` is set
- **THEN** `<C-x>` cuts the hovered pane

## MODIFIED Requirements

### Requirement: Theme styles
Theme styles SHALL accept `fg` and `bg` as named colours, `"#rrggbb"`, or `"reset"`, plus boolean modifiers (`bold`, `italic`, `underline`, `reversed`, `dim`). The theme SHALL include configurable icon text and colour for sessions, windows and panes, and an `[indicator]` section with yazi-style `parent` / `current` / `preview` hovered-row styles, `padding = { open, close }` (hovered-row caps) and `active = { text, fg }` (current-item marker, default `*` magenta). The `[input]` section SHALL style input prompts with `border` (default yellow), `title` (default red), `value` (default white) and `selected` (the text cursor, default reversed). `[picker]` SHALL include `find_keyword` (default yellow, bold, italic, underline) and `find_position` (default magenta on `reset`, bold, italic) for find highlights. The `[marker]` section SHALL style the cut indicator with `cut` (default `{ fg = "lightred", bg = "lightred" }`), `symbol` (default `"│"`) and `badge_cut` (the header badge, default white on red).

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
