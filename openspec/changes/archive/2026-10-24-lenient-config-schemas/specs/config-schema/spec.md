# Spec Delta

## Purpose

Gives users editor-side validation of pane-mux's TOML config files through published JSON schemas, so typos and unknown keys are flagged while editing even though pane-mux ignores them at runtime.

## ADDED Requirements

### Requirement: Published schemas for each config file
pane-mux SHALL publish one JSON schema each for `config.toml`, `keymap.toml` and `theme.toml`. The schemas SHALL be committed in the repository under `schemas/` (`config.json`, `keymap.json`, `theme.json`) and SHALL be reachable at a stable URL derived from the repository's default branch.

#### Scenario: Schema files present
- **WHEN** a user browses the repository's `schemas/` directory
- **THEN** it contains `config.json`, `keymap.json` and `theme.json`

#### Scenario: Schema URL resolves
- **WHEN** a TOML language server fetches the documented URL for `config.json`
- **THEN** it receives the same JSON schema committed at `schemas/config.json` on the default branch

### Requirement: Schemas are strict about unknown keys
Each schema SHALL describe every key pane-mux accepts, with its type and its allowed values where finite, and SHALL mark unknown keys and unknown sections as invalid. This applies even though pane-mux ignores unknown keys at runtime.

#### Scenario: Typo flagged in the editor
- **WHEN** a user with schema validation enabled writes `[picker] preveiw_ratio = 70` in `config.toml`
- **THEN** the language server reports `preveiw_ratio` as not allowed

#### Scenario: Enum value flagged
- **WHEN** a user writes `[picker] preview_position = "left"`
- **THEN** the language server reports that the value must be `"top"` or `"bottom"`

#### Scenario: Unknown command flagged
- **WHEN** a user writes a binding with `run = "pste"` in `keymap.toml`
- **THEN** the language server reports the `run` value as invalid

### Requirement: Schemas match the shipped config types
The committed schema files SHALL be generated from the same definitions pane-mux uses to load config. The test suite SHALL fail when a committed schema differs from the generated one, and the docs SHALL name the command that regenerates them.

#### Scenario: Stale schema caught
- **WHEN** a developer adds a new config key without regenerating `schemas/config.json`
- **THEN** the test suite fails, naming the stale schema file

#### Scenario: Built-in presets validate
- **WHEN** the shipped `config-default.toml`, `keymap-default.toml` and `theme-default.toml` are validated against their schemas
- **THEN** they produce no errors

### Requirement: Schema directive in presets and docs
The built-in preset files and the configuration documentation SHALL begin their example files with a `#:schema <url>` directive pointing at the matching published schema, so that users who copy them get editor validation without further setup.

#### Scenario: Preset header
- **WHEN** a user opens the shipped `config-default.toml`
- **THEN** the file's leading comment block contains `#:schema` followed by the URL of `config.json`

#### Scenario: Documented setup
- **WHEN** a user reads the configuration docs
- **THEN** they find the `#:schema` line for each of the three files and a note that Tombi or taplo will use it
