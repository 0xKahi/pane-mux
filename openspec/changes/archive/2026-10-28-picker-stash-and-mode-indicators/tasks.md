# Tasks

## 1. Snapshot data (tmux layer)

- [x] 1.1 Add `Pane.mode: String` with an `in_copy_mode()` helper (true for `copy-mode` and `view-mode`), `Session.stash: bool` and `Window.stash_source: Option<String>` in `src/tmux/model.rs`. Verify with unit tests for `in_copy_mode()` covering copy, view, clock and empty modes.
- [x] 1.2 Append `#{pane_mode}`, `#{@pmux_stash}` and `#{@pmux_source}` to `PANE_FORMAT` and parse them in `parse_row` (empty values allowed for older tmux and unset options). Verify by updating the `src/tmux/format.rs` tests to the new field count and asserting mode, stash and source parsing, including empty fields.
- [x] 1.3 Add `Tree::stash_counts()`: count floating panes per `stash_source` across windows of the `pmux` session whose `stash` is true. Verify with unit tests: two floats counted, placeholder (tiled) excluded, killed placeholder still counts floats, unowned `pmux` session ignored, windows of other sessions with `@pmux_source` ignored.
- [x] 1.4 Make `FakeTmux::tree()` fill `Session.stash` and `Window.stash_source` from its option map. Verify with a fake test where `float stash` on the fake yields a non-empty `stash_counts()` for the source window.

## 2. Theme keys

- [x] 2.1 Add `stashed` and `copy_mode` (`Icon`) to `Icons` in `src/config/theme.rs` and defaults to `src/config/preset/theme-default.toml` (`"\U000F09A4 $count"` / `lightcyan`, `"\U000F18F1"` / `lightyellow`). Verify with theme tests asserting defaults and a partial override (`stashed = { text = "S" }` keeps `lightcyan`).
- [x] 2.2 Regenerate `schemas/theme.json` with `just schemas` and verify the schema test (shipped defaults validate, committed schema matches) passes.
- [x] 2.3 Document `[icons] stashed` (including `$count`) and `copy_mode` (copy and view mode) in `docs/configuration.md`, including the `[icons]` paragraph and the full-defaults block. Verify the documented defaults match `theme-default.toml`.

## 3. Picker rendering

- [x] 3.1 Make stash counts available to rendering, recomputed on initial load and every tree refresh (`src/picker/state.rs`). Verify with a state test that the count updates after the fake's stash options change and a refresh runs.
- [x] 3.2 In `src/picker/ui/columns.rs`, build the per-row indicator (`$count` substituted for window rows, mode icon for pane rows in copy or view mode, none when text is empty), draw it two spaces after the label, place the find badge after it, and reserve both widths before clipping the label. Verify with a snapshot test of a windows column showing `1: editor  󰦤 2` and a panes column showing `1: zsh  󱣱`.
- [x] 3.3 Apply hover colour rules to indicators: own `fg` on unhovered and underline rows, no colour on filled (`bg` / reversed) rows. Verify with buffer-style tests asserting the indicator cell fg/bg on a reversed hovered row and on an underlined hovered row.
- [x] 3.4 Cover layout edges: indicator plus find badge order (`1: editor  󰦤 2 [1/1]`), long name clipped while indicator stays visible before the end cap, empty theme text leaves no trailing gap, tiny terminals do not panic. Verify with snapshot/buffer tests in `src/picker/ui/tests.rs`.

## 4. Integration

- [x] 4.1 Extend `tests/tmux_integration.rs` (inside the floating-support guard): stash floats of a window on a private server, read `tree()`, and assert `stash_counts()` reports the right count for the source window by id. Put a pane in copy mode (`copy-mode -t`) and assert its `in_copy_mode()` is true. Verify with `cargo test --test tmux_integration`.
- [x] 4.2 Run `just check` and verify it passes with updated snapshots reviewed.
