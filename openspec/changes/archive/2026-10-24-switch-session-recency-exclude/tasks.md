# Tasks

## 1. Config setting

- [x] 1.1 Add `SwitchSessionConfig { exclude_from_recency: Vec<String> }` (with `deny_unknown_fields`) to `src/config/settings.rs` as `Config.switch_session` renamed to `switch-session`, export it from `src/config/mod.rs`, and verify that `cargo build` succeeds
- [x] 1.2 In `src/config/preset/config-default.toml`, replace the uncommitted `recency_exclusion` key with `[switch-session] exclude_from_recency = []` and a short comment, and verify that `cargo test config` loads the preset without errors (the preset currently fails `deny_unknown_fields`)
- [x] 1.3 Add unit tests in `src/config/settings.rs` for the default being empty, a user list replacing the default, the string-instead-of-list error naming `exclude_from_recency`, and an unknown `[switch-session]` key error naming `surprise`, and verify that `cargo test settings` passes
- [x] 1.4 Add a `tests/config_load.rs` case where a `config.toml` with `exclude_from_recency = "_popup"` produces an error naming the file path, and verify that `cargo test --test config_load` passes
- [x] 1.5 Document `[switch-session] exclude_from_recency` (exact names, current session stays first, excluded sessions listed last by name) in `docs/configuration.md` with a TOML example, and verify that the example parses by pasting it into a settings test or checking it against 1.3

## 2. Recency ordering with exclusions

- [x] 2.1 Add `recency_exclude: Vec<String>` to `PickerOptions` in `src/picker/options.rs` (default empty) and update the explicit constructions in `src/commands/picker.rs` and `src/picker/ui/tests.rs`, then verify that `cargo build --all-targets` succeeds
- [x] 2.2 Extend `State::sort_sessions` in `src/picker/state.rs` with the excluded bucket (key `(not_current, is_excluded, Reverse(last_attached or 0 when excluded), name)`), leaving `initial_session` and `refresh` unchanged, and verify that the existing recency tests still pass with `cargo test state`
- [x] 2.3 Add tests in `src/picker/state/tests.rs` for these spec scenarios: excluded listed after ranked (`session1`, `session3`, `session4`, `session2`, cursor on `session3`); excluded ordered by name despite recency; excluded after never-attached; current excluded stays first with the cursor on the next ranked session; only excluded besides current (cursor on first by name); unknown name ignored; refresh after another client attaches an excluded session keeps its position and the cursor. Verify that `cargo test state` passes
- [x] 2.4 Pass `loaded.config.switch_session.exclude_from_recency` into `PickerOptions` in `src/commands/switch_session.rs`, and verify with `cargo build` plus a manual run: with `exclude_from_recency = ["<recent session>"]`, `pane-mux switch-session` in tmux lists that session last and opens the cursor on the next most recent one

## 3. Integration check

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`, and verify that all pass
- [x] 3.2 Run `openspec validate switch-session-recency-exclude --strict` and verify that it reports the change as valid
