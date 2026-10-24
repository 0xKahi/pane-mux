# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
