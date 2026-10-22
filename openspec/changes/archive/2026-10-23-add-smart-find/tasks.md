# Tasks

## 1. Theme styles

- [x] 1.1 Add `selected` to `InputTheme` and `find_keyword` / `find_position` to `PickerTheme` in `src/config/theme.rs`. Set the defaults in `src/config/preset/theme-default.toml` per design D8: `[input]` border yellow, title red, value white, selected reversed; find styles from the user's yazi theme. Verify with a theme test that a partial `[input] border` override keeps the other three defaults and `cargo test` passes.
- [x] 1.2 Use `theme.input.selected` for the prompt's text cursor in `src/picker/ui/popup.rs` `input()`, replacing the hard-coded `REVERSED`. Verify that the `input_popup_with_cursor` snapshot is unchanged, or accept it only for the value colour change, and that a test with `selected = { bg = "blue", reversed = false }` renders a blue, non-reversed cursor cell.

## 2. Commands and parsing

- [x] 2.1 Add `regex` to `Cargo.toml`. Add `Case`, `Action::Find { previous, case }` and `Action::FindArrow { previous }` to `src/picker/action.rs`. Verify with `cargo build`.
- [x] 2.2 Parse `find [--previous] [--smart|--insensitive]` (any order) and `find_arrow [--previous]` in `src/config/keymap/run.rs`, and add both names to the bad-arguments list. Verify with `run.rs` unit tests for valid combinations, an unknown flag on each command (error), and the existing tests still passing.

## 3. Finder core

- [x] 3.1 Implement `Finder { query, case, regex }` with hand-written `PartialEq`/`Eq`/`Debug` on `(query, case)`, smart-case compile (D4), and `next`/`prev(names, cursor, include)` with wrap-around (D1, D3), in a new `src/picker/finder.rs`. Verify with unit tests: smart lowercase and uppercase queries, `--insensitive`, an invalid regex returning an error, include versus exclude of the hovered row, wrap in both directions, and no match.
- [x] 3.2 Add `State.finder` and `State::names(level)` returning session name, window name or pane command. Verify that a state test shows window indexes are never matched.

## 4. Picker behaviour

- [x] 4.1 Add `InputPurpose::Find { previous, case }` and the `find` action opening `Find next:` / `Find previous:`. Make `close` and `close --submit` keep the finder and run no submit side effects. Verify with state tests in `src/picker/state/tests.rs`: Enter and Esc both close the prompt, leave the cursor where it jumped, and keep the finder.
- [x] 4.2 Run `find_do` after every input edit (character insert, backspace, kill-word, kill-bol) through one `input_changed()` hook (D2). It clears the finder on an empty query, ignores an invalid regex, skips an unchanged query, and otherwise searches with the hovered row included, then calls `select` and refreshes the preview. Verify with state tests covering the `Incremental jump while typing` and `Regex query with smart case` scenarios in `specs/picker-find/spec.md`, including the sessions-column jump that updates child cursors and the preview.
- [x] 4.3 Implement `find_arrow [--previous]` on the focused column, excluding the hovered row. Verify with state tests for next, previous with wrap, no finder (no-op), and the finder following focus after `h`/`l`.
- [x] 4.4 Extend `Action::Escape` to clear the finder after which-key and popup but before the message (D6), and confirm the finder survives `refresh`. Verify with state tests for `Escape clears`, `Escape with a message shown` and `Survives refresh`.

## 5. Rendering

- [x] 5.1 Highlight the first regex match in each matching row of the focused column with `find_keyword` patched over the row style. Offsets apply to the name part after the `"N: "` prefix. Verify with an insta snapshot (and a style assertion) in `src/picker/ui/tests.rs` showing highlighted rows and no highlights in unfocused columns.
- [x] 5.2 Append the ` [X/Y]` badge in `find_position` to the hovered row when it matches. Verify with a snapshot of the `Highlight and badge` scenario (`[2/2]` on `blog`), plus a check that long names keep the right edge and caps intact.

## 6. Defaults and docs

- [x] 6.1 Add `<Space> /` → `find --smart`, `n` → `find_arrow` and `N` → `find_arrow --previous` with descriptions to `[picker]` in `src/config/preset/keymap-default.toml`. Verify with a test that the default keymap resolves these chords, and that pressing `<Space>` shows the which-key popup listing `/`.
- [x] 6.2 Update `README.md`:
  - Document `find` and `find_arrow` with their flags and default keys.
  - Document the regex and smart-case rules.
  - Add the `[input] selected` and `[picker] find_*` theme keys.
  - Note that binding a lone `<Space>` shadows `<Space> /`.

  Verify that the documented examples parse, by loading them in a config test or with a manual `pane-mux picker` run.

## 7. Integration

- [x] 7.1 Run `just check` (fmt, clippy `-D warnings`, tests). Then run `just test-tmux` and a manual `pane-mux picker` session: `<Space> /` with `log`, `n`/`N`, `<Esc>`, and switching focus. Verify that all commands pass and the behaviour matches `specs/picker-find/spec.md`.

## Workflow follow-up

- Archive the change after review, which syncs `picker-find` and the `configuration` delta into the main specs.
