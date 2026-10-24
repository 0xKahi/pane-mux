# Tasks

## 1. Lenient loading

- [x] 1.1 Remove `#[serde(deny_unknown_fields)]` from every struct in `src/config/settings.rs`, `src/config/theme.rs`, `src/config/keymap/section.rs` and `src/config/keymap/chord.rs`; verify `grep -rn deny_unknown_fields src` (excluding schemars attributes) finds nothing serde-side
- [x] 1.2 Replace the unknown-key error tests (`unknown_config_key_errors`, `unknown_switch_session_key_errors`, any theme/keymap equivalents, and `tests/config_load.rs` cases) with tests asserting that unknown sections, unknown keys in known sections, unknown theme keys and unknown binding fields load successfully with the other values applied; verify `cargo test` passes
- [x] 1.3 Add tests that wrong types (`exclude_from_recency = "_popup"`, `[picker] border = "gray"`), invalid values (`preview_ratio = 150`), unknown `on` key names and unknown `run` commands still exit with code 2 and name the file and the key; verify `cargo test` passes
- [x] 1.4 Verify that the uncommitted `[floating-pane]` section in `src/config/preset/config-default.toml` no longer stops the preset from loading (`cargo run -- picker` starts; `just check` passes)

## 2. Schema generation

- [x] 2.1 Add `schemars` (with draft-07 settings available) and `jsonschema` to `[dev-dependencies]`; verify `cargo test --no-run` builds and that `cargo tree -e normal` does not list either crate
- [x] 2.2 Add `#[cfg_attr(test, derive(schemars::JsonSchema))]` and `#[cfg_attr(test, schemars(deny_unknown_fields))]` to the config, keymap and theme types, and add a short comment in `src/config/mod.rs` explaining the pattern; verify `cargo test` compiles
- [x] 2.3 Add a `COMMAND_NAMES` constant next to `parse_run` in `src/config/keymap/run.rs`, with a unit test that every listed name is recognized by `parse_run` and every command in the default keymap is listed; verify the test passes
- [x] 2.4 Write manual `JsonSchema` impls for colours (named, `#rrggbb`, `reset`), `on` (string or string array) and `run` (string or array, pattern built from `COMMAND_NAMES`), with a unit test that every named colour in the schema parses; verify the tests pass
- [x] 2.5 Create `src/config/schema.rs` (`#[cfg(test)]`) that generates draft-07 schemas for the user-file shapes of `config.toml`, `keymap.toml` and `theme.toml`, applying a transform that strips `required` from section objects but keeps `on` and `run` required on bindings; verify with a unit test that a partial user file (e.g. only `[picker] preview_position`) validates

## 3. Committed schemas and checks

- [x] 3.1 Add the stale-check test: compare each generated schema with `schemas/config.json`, `schemas/keymap.json` and `schemas/theme.json`, fail with the file name and regeneration command on mismatch, and rewrite the files when `PANE_MUX_UPDATE_SCHEMAS=1` is set; add a `just schemas` recipe; verify by generating the files, committing them, and confirming `cargo test` passes and fails after a deliberate edit to one file
- [x] 3.2 Add the preset-validation test: each built-in preset (TOML converted to JSON) validates against its schema with `jsonschema`; verify it passes, and that adding a bogus key to a preset copy makes it fail
- [x] 3.3 Add schema tests for the spec scenarios: `preveiw_ratio` rejected, `preview_position = "left"` rejected, `run = "pste"` rejected, unknown section rejected; verify `cargo test` passes

## 4. Directives and docs

- [x] 4.1 Add a `#:schema https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/<file>.json` line, with a one-line comment naming Tombi/taplo, to the top of `config-default.toml`, `keymap-default.toml` and `theme-default.toml`; verify the presets still load (`cargo test`) and the schema tests pass
- [x] 4.2 Update `docs/configuration.md` and the README configuration section: unknown keys are ignored, how to enable editor validation with the `#:schema` lines for each file, and `just schemas` for contributors; verify the documented URLs match the committed file paths
- [x] 4.3 Manually verify in an editor with Tombi or taplo that a user `config.toml` with the `#:schema` line flags `preveiw_ratio` and accepts the default keys

## 5. Integration

- [x] 5.1 Run `just ci`, and run `openspec validate lenient-config-schemas --strict`; verify both pass
