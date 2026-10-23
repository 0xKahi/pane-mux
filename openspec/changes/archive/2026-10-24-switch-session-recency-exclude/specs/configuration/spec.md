# Spec Delta

## ADDED Requirements

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
