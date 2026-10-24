# Design

## Context

Loading works in two steps (`src/config/merge.rs`, `src/config/loader.rs`): the preset TOML and the user TOML are deep-merged as `toml::Value`, and the merged table is then deserialized into typed structs. Because the merged table always holds every preset key, the typed structs can keep all fields required. Strictness comes only from `#[serde(deny_unknown_fields)]` (22 uses across `settings.rs`, `theme.rs`, `keymap/section.rs`, `keymap/chord.rs`). Value checks such as `preview_ratio` range, key names in `on` and command names in `run` live in custom deserializers and `validate()`, and those stay as they are.

yazi, the model here, silently ignores unknown keys at runtime and catches typos in the editor through `#:schema`-linked JSON schemas. yazi maintains those schemas by hand in a separate repo, with no sync check.

## Goals / Non-Goals

**Goals:**
- No unknown key or section can stop pane-mux from starting.
- Strict, editor-consumable schemas that cannot silently drift from the code.
- No new runtime dependency in the release binary.

**Non-Goals:**
- Runtime warnings for unknown keys, in the status line or on stderr. The editor schema replaces them.
- A SchemaStore catalog entry. It can be added later without spec changes.
- Aliases or migration for renamed keys.

## Decisions

### D1. Remove `deny_unknown_fields` everywhere
Drop the attribute from every config struct, so serde's default (ignore) applies. Keep custom deserializers (`deserialize_on`, `deserialize_run`) and `Config::validate()` unchanged, so bad values still exit with code 2.
- *Alternative:* collect ignored paths with `serde_ignored` and warn. Rejected for now: stderr is hidden behind the alternate screen, and a status-line notice adds a new message path. The editor schema covers the typo case.
- Side benefit: `ActiveIndicator` combines `#[serde(flatten)]` with `deny_unknown_fields`, a pair serde handles poorly. That interaction goes away.

### D2. Generate schemas with `schemars`, and enable it only in tests
Derive `schemars::JsonSchema` through `#[cfg_attr(test, derive(schemars::JsonSchema))]` and list `schemars` under `[dev-dependencies]`. Schema generation and the stale check live in a unit-test module inside the library (e.g. `src/config/schema.rs`, `#[cfg(test)]`), so the derives exist exactly where they're used, and the release binary doesn't carry schemars.
- Strictness lives in the schema only: add `#[cfg_attr(test, schemars(deny_unknown_fields))]` to each struct, so the schema emits `additionalProperties: false` while serde stays lenient.
- *Alternative:* hand-written schemas, as yazi does. Rejected because they drift. This change already adds `[marker]` and edits `[floating-pane]` right away.
- *Alternative:* a regular dependency plus a hidden `pane-mux schema` subcommand. Rejected: it adds user-visible surface and runtime weight for a developer-only task.

### D3. Partial-file shape: strip `required`
User files are partial overlays, but the typed structs require every field. A schemars transform applied during generation recursively removes `required` from section objects. A binding object (`Chord`) is the one exception: it keeps `on` and `run` as required, because the runtime requires them too.

### D4. Custom types get hand-written `JsonSchema` impls
- Colours (`fg`/`bg`): a string matching a named colour, `#rrggbb` or `reset`. A unit test checks that every name in the schema's list parses as a colour, so the list can't promise colours the parser rejects.
- `on`: a string or an array of strings. Key-name syntax stays free-form in the schema, with a description. Its grammar is too rich for a useful pattern, and the runtime already validates it.
- `run`: a string or an array of strings matching `^(cmd1|cmd2|...)(\s.*)?$`. `parse_run` in `src/config/keymap/run.rs` is a single `match` with no name table today, so add a `COMMAND_NAMES` constant next to it. A unit test checks both directions: every name in `COMMAND_NAMES` is recognized by `parse_run` (it fails, if at all, only on arguments, never as an unknown command), and every command in the default keymap appears in `COMMAND_NAMES`.

### D5. Draft-07 output, committed under `schemas/`, served from raw GitHub
Generate with `SchemaSettings::draft07()` for the broadest language-server support (Tombi, taplo). Commit the results as pretty-printed JSON in `schemas/config.json`, `schemas/keymap.json` and `schemas/theme.json`. The published URL is `https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/<file>.json`.
- *Alternative:* GitHub Pages. Rejected for now because it needs a deploy pipeline. The URL can move later.

### D6. Stale check and regeneration
The schema test generates each schema and compares it with the committed file. It fails with the file name and the regeneration command when they differ. Running with `PANE_MUX_UPDATE_SCHEMAS=1` rewrites the files instead. A `just schemas` recipe wraps that command.

### D7. Preset validation test
A test converts each built-in preset from TOML to JSON and validates it against its generated schema with the `jsonschema` crate (dev-dependency). This catches a preset key that isn't in the types, which the lenient loader would now ignore silently.

## Risks / Trade-offs

- [Typos in user config now fail silently at runtime] → Editor schema plus the docs' `#:schema` setup. The preset-validation test protects the presets themselves.
- [A `main`-branch URL can describe keys newer than the user's installed version] → Acceptable: unknown keys are harmless at runtime. Versioned URLs (tags) can come later.
- [Language-server draft support varies] → Draft-07 output. Check with Tombi during implementation.
- [The `cfg_attr(test)` derives make the struct definitions noisier] → Contained to config types. A short comment in `src/config/mod.rs` explains the pattern.

## Migration Plan

No user action is required. Configs that loaded before still load. Configs that failed only because of unknown keys now load. Rollback means re-adding the attributes.
