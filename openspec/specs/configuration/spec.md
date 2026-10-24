# configuration Specification

## Purpose

Defines where pane-mux finds user configuration and how user config, keymap and theme files are layered over the built-in defaults, so users can customise behaviour, key bindings and appearance.

## Requirements

### Requirement: Config directory resolution
pane-mux SHALL resolve its config directory as the first of: `$PANE_MUX_CONFIG_HOME` (if absolute), `$XDG_CONFIG_HOME/pane-mux` (if `XDG_CONFIG_HOME` is absolute), `$HOME/.config/pane-mux`. This SHALL hold on macOS as well as Linux.

#### Scenario: Default location
- **WHEN** neither `PANE_MUX_CONFIG_HOME` nor `XDG_CONFIG_HOME` is set
- **THEN** pane-mux reads config files from `~/.config/pane-mux/`

#### Scenario: Override
- **WHEN** `PANE_MUX_CONFIG_HOME=/tmp/pm` is set
- **THEN** pane-mux reads config files from `/tmp/pm/`

### Requirement: Built-in defaults
pane-mux SHALL ship default `config`, `keymap` and `theme` settings inside the binary and SHALL run with them when no user files exist. Each of `config.toml`, `keymap.toml` and `theme.toml` SHALL be optional.

#### Scenario: No user config
- **WHEN** the config directory does not exist
- **THEN** pane-mux runs with the built-in defaults and reports no error

### Requirement: User overrides merge over defaults
For `config.toml` and `theme.toml`, any key the user sets SHALL override the default for that key only; unset keys SHALL keep their defaults.

#### Scenario: Partial theme override
- **WHEN** `theme.toml` sets only the preview border colour
- **THEN** the preview border uses the user's colour and every other style keeps its default

#### Scenario: Partial config override
- **WHEN** `config.toml` sets only `[picker] preview_position = "top"`
- **THEN** the preview is on top and `preview_ratio` stays at its default of 50

### Requirement: Picker settings
`config.toml` SHALL support `[picker] preview_ratio` (integer 1-99, default 50) and `[picker] preview_position` (`"top"` or `"bottom"`, default `"bottom"`).

#### Scenario: Invalid ratio
- **WHEN** `config.toml` sets `preview_ratio = 150`
- **THEN** pane-mux exits with code 2 and an error naming `preview_ratio`

### Requirement: Switch-session settings
`config.toml` SHALL support `[switch-session] exclude_from_recency`: a list of session names, default `[]`. Values that are not a list of strings SHALL be rejected as invalid config.

#### Scenario: Default is empty
- **WHEN** no user `config.toml` sets `[switch-session]`
- **THEN** `exclude_from_recency` is empty and `switch-session` orders every session by recency

#### Scenario: User list replaces the default
- **WHEN** `config.toml` sets `[switch-session] exclude_from_recency = ["_popup", "scratch"]`
- **THEN** the effective exclusion list is exactly `_popup` and `scratch`

#### Scenario: Wrong type
- **WHEN** `config.toml` sets `exclude_from_recency = "_popup"`
- **THEN** pane-mux prints the file path and an error naming `exclude_from_recency` to stderr and exits with code 2

#### Scenario: Unknown key
- **WHEN** `config.toml` sets `[switch-session] surprise = true`
- **THEN** pane-mux starts normally and ignores `surprise`

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

### Requirement: Keymap layering
Each keymap layer in `keymap.toml` SHALL support `keymap`, `prepend_keymap` and `append_keymap`. A user `keymap` SHALL replace that layer's defaults entirely. `prepend_keymap` entries SHALL take precedence over defaults. `append_keymap` entries SHALL apply only for keys the defaults do not bind.

#### Scenario: Prepend overrides a default
- **WHEN** `[picker] prepend_keymap = [{ on = "q", run = "escape" }]` is set
- **THEN** pressing `q` runs `escape` instead of quitting

#### Scenario: Append does not override
- **WHEN** `[picker] append_keymap = [{ on = "j", run = "arrow prev" }]` is set
- **THEN** `j` still moves to the next item

#### Scenario: Replace a layer
- **WHEN** `[picker] keymap = [{ on = "x", run = "quit" }]` is set
- **THEN** `x` quits and none of the default picker bindings apply

### Requirement: Key binding format
A binding SHALL have `on` (a single key or a list of keys, using `<C-x>`, `<S-x>`, `<A-x>`, `<Enter>`, `<Esc>`, `<PageUp>`-style names), `run` (a command string with optional arguments, or a list of them run in order) and an optional `desc`.

#### Scenario: Sequence binding
- **WHEN** a binding has `on = ["g", "g"]` and `run = "arrow top"`
- **THEN** pressing `g` then `g` moves the cursor to the top

#### Scenario: Unknown key name
- **WHEN** a binding uses `on = "<Foo>"`
- **THEN** pane-mux exits with code 2 and an error identifying the invalid key

### Requirement: Cut and paste key bindings
The default `[picker]` keymap SHALL bind `x` to `cut` and `p` to `paste`, and `cut` and `paste` SHALL be valid `run` commands that take no arguments.

#### Scenario: Default bindings
- **WHEN** no user `keymap.toml` overrides them
- **THEN** pressing `x` cuts the hovered pane and `p` pastes it, and both appear in the help popup

#### Scenario: Rebinding
- **WHEN** `[picker] prepend_keymap = [{ on = "<C-x>", run = "cut" }]` is set
- **THEN** `<C-x>` cuts the hovered pane

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

### Requirement: Invalid config fails clearly
A user file that is not valid TOML, or that contains a value of the wrong type, an invalid value for a known key, an unknown key name in `on`, or an unknown command name in `run`, SHALL cause pane-mux to print the file path and the problem to stderr and exit with code 2. Unknown keys and sections are not invalid config (see "Unknown keys are ignored").

#### Scenario: Syntax error
- **WHEN** `keymap.toml` contains invalid TOML
- **THEN** stderr names `keymap.toml` and the parse error, and the exit code is 2

#### Scenario: Unknown command name
- **WHEN** a binding uses `run = "pste"`
- **THEN** stderr names `keymap.toml` and `pste`, and the exit code is 2

#### Scenario: Wrong type for a known theme key
- **WHEN** `theme.toml` sets `[picker] border = "gray"` instead of a style table
- **THEN** stderr names `theme.toml` and `border`, and the exit code is 2

### Requirement: Unknown keys are ignored
pane-mux SHALL ignore unknown keys and unknown sections in `config.toml`, `keymap.toml` and `theme.toml`, and in its built-in presets, at any nesting level, including unknown fields inside a key binding. Ignored keys SHALL NOT change any setting, SHALL NOT produce an error and SHALL NOT change the exit code.

#### Scenario: Unknown top-level section
- **WHEN** `config.toml` contains `[future-feature] enabled = true`
- **THEN** pane-mux starts normally with every other setting applied

#### Scenario: Unknown key in a known section
- **WHEN** `config.toml` sets `[switch-session] surprise = true` alongside `exclude_from_recency = ["_popup"]`
- **THEN** pane-mux starts normally and `exclude_from_recency` is exactly `_popup`

#### Scenario: Removed setting left in user config
- **WHEN** `config.toml` still sets a key that a newer pane-mux no longer supports
- **THEN** pane-mux starts normally and ignores that key

#### Scenario: Unknown theme key
- **WHEN** `theme.toml` sets `[picker] sparkle = { fg = "red" }`
- **THEN** pane-mux starts normally and every known style keeps its configured or default value

#### Scenario: Unknown field in a binding
- **WHEN** a binding is written `{ on = "q", run = "quit", note = "x" }`
- **THEN** the binding works as `q` → `quit`
