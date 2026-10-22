# Spec Delta

## MODIFIED Requirements

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
