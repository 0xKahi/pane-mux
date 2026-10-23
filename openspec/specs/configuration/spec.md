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
`config.toml` SHALL support `[switch-session] exclude_from_recency`: a list of session names, default `[]`. Unknown keys in `[switch-session]` and values that are not a list of strings SHALL be rejected as invalid config.

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
- **THEN** pane-mux exits with code 2 and an error naming `surprise`

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

### Requirement: Theme styles
Theme styles SHALL accept `fg` and `bg` as named colours, `"#rrggbb"`, or `"reset"`, plus boolean modifiers (`bold`, `italic`, `underline`, `reversed`, `dim`). The theme SHALL include configurable icon text and colour for sessions, windows and panes, and an `[indicator]` section with yazi-style `parent` / `current` / `preview` hovered-row styles, `padding = { open, close }` (hovered-row caps) and `active = { text, fg }` (current-item marker, default `*` magenta). The `[input]` section SHALL style input prompts with `border` (default yellow), `title` (default red), `value` (default white) and `selected` (the text cursor, default reversed). `[picker]` SHALL include `find_keyword` (default yellow, bold, italic, underline) and `find_position` (default magenta on `reset`, bold, italic) for find highlights.

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

### Requirement: Invalid config fails clearly
A user file that is not valid TOML, or that contains a value of the wrong type or an unknown command name, SHALL cause pane-mux to print the file path and the problem to stderr and exit with code 2.

#### Scenario: Syntax error
- **WHEN** `keymap.toml` contains invalid TOML
- **THEN** stderr names `keymap.toml` and the parse error, and the exit code is 2
