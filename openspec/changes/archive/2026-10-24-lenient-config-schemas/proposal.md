# Proposal

## Why

pane-mux currently refuses to start when any config file, or the built-in preset itself, contains a key it doesn't recognize. Every config struct uses `deny_unknown_fields`. That breaks forward and backward compatibility: removing a setting (for example `[floating-pane] always_on_top`, which the upcoming cut/paste change drops) or reading a config written for a newer pane-mux leaves users unable to open the picker at all. yazi, which pane-mux follows, ignores unknown keys at runtime and catches typos in the editor through JSON schemas instead. pane-mux should do the same.

## What Changes

- **BREAKING (behaviour relaxation):** Unknown keys and unknown sections in `config.toml`, `keymap.toml` and `theme.toml` (and in the built-in presets) are ignored instead of failing with exit code 2.
- These stay hard errors (exit 2): invalid TOML, wrong value types for known keys, invalid values (e.g. `preview_ratio = 150`), unknown key names in `on`, and unknown command names in `run`.
- pane-mux publishes strict JSON schemas for `config.toml`, `keymap.toml` and `theme.toml`, committed under `schemas/` in the repository and served from a stable URL. The schemas reject unknown keys, so TOML language servers (Tombi, taplo) flag typos inside the editor.
- The shipped presets and the configuration docs start with a `#:schema <url>` directive, so users can copy it to enable editor validation.
- The schemas are generated from the config types, and a test fails when the committed schema files are stale.

## Capabilities

### New Capabilities
- `config-schema`: Published JSON schemas that describe pane-mux's config files for editor-side validation, and how users enable them.

### Modified Capabilities
- `configuration`: Unknown keys are no longer rejected. The "Switch-session settings" requirement drops its unknown-key rule and "Unknown key" scenario, and "Invalid config fails clearly" now states explicitly that unknown keys are ignored.

## Impact

- Code: `src/config/settings.rs`, `src/config/theme.rs`, `src/config/keymap/section.rs`, `src/config/keymap/chord.rs` (remove `deny_unknown_fields`, add schema derivation); tests in those modules and `tests/config_load.rs`.
- New dependency: `schemars` crate for JSON Schema generation, either a regular dependency or a dev-dependency, depending on design.
- New files: `schemas/config.json`, `schemas/keymap.json`, `schemas/theme.json`.
- Docs: `docs/configuration.md`, the preset TOML headers, and the README configuration section.
- Unblocks the uncommitted `[floating-pane]` section in `src/config/preset/config-default.toml`, which currently makes the preset fail to load.
