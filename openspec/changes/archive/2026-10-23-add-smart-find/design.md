# Design

## Context

- **Cursor model:** each column keeps a stable id cursor (`State.session` / `window` / `pane`) and `focus: Level`. `ids(level)` returns the visible row order, and `select(level, id)` moves a cursor and resets the child columns.
- **Text entry:** the only text entry is `Popup::Input { title, value, purpose }`.
  - Printable characters are appended in `handle_key`.
  - `backspace` / `kill-word` / `kill-bol` edit the value in `apply`.
  - The value is only acted on when `close --submit` runs (`submit`).
- **Change detection:** `State` derives `Clone + PartialEq + Eq`. `refresh` compares against a clone to decide whether to redraw.
- **Command parsing:** commands are parsed by `config/keymap/run.rs` from whitespace-split words.
- **Themes:** themes are deep-merged TOML deserialized into `deny_unknown_fields` structs. Every new style key therefore needs a preset default.
- **Yazi reference:** `yazi-actor/src/mgr/{find,find_do,find_arrow}.rs` and `yazi-core/src/tab/finder.rs`. Behaviour is in the `picker-find` spec. This document covers only how to build it.

## Goals / Non-Goals

**Goals:**
- Match yazi's finder semantics: regex, smart case, include-hovered on typing, exclude-hovered on `n`/`N`, wrap, no restore on cancel.
- Reuse the existing input popup and input keymap layer; add no new keymap layer.

**Non-Goals:**
- Filtering or hiding rows, which is yazi's `filter`, a different action.
- Find history, a cursor that can move inside the input, or moving the prompt to the top-center position yazi uses. The prompt stays centred like the create and rename prompts.
- A debounce. Lists are small and searched synchronously.

## Decisions

### D1. Finder lives on `State`, not in the popup
Add `State.finder: Option<Finder>` with `Finder { query: String, case: Case, regex: Regex }`.

- The finder must outlive the prompt (spec: Enter/Esc keep it) and the refresh, so it cannot live in `Popup`.
- `regex::Regex` is not `PartialEq`. Implement `PartialEq`/`Eq` (and `Debug`) by hand, comparing `query` and `case`. This keeps `State`'s derives and refresh-diffing intact.
- Alternative considered: store only the query and recompile on each use. It's simpler, but render would recompile on every frame. Rejected.

### D2. Prompt is `InputPurpose::Find { previous, case }`
`find` opens `Popup::Input { title: "Find next:" | "Find previous:", value: "", purpose: Find { .. } }`.

- After every edit to the value, `State` runs `find_do(value, previous, case)`. Edits are character insertion in `handle_key` plus Backspace, KillWord and KillBol in `apply`. A small `input_changed()` helper called from both places avoids duplicating the check.
- `close` and `close --submit` both just drop the popup for this purpose. `submit` does nothing for `Find`.
- `find_do` mirrors yazi:
  - An empty value clears the finder.
  - A regex compile error returns without changes.
  - The same `(query, case)` as the current finder does nothing.
  - Otherwise it searches from the cursor with the hovered row included, calls `select(focus, id)`, refreshes the preview, and stores the finder.
- Alternative considered: a separate `Popup::Find`. It would mean adding a variant to every exhaustive `Popup` match and duplicating the input rendering. Rejected.

### D3. Matching by names in the focused column
Add `State::names(level) -> Vec<(String /*id*/, &str /*name*/)>`, built from the same tree slices as `ids`. It returns the session `name`, window `name`, or pane `command`.

- `Finder::next(names, cursor, include)` and `prev` work like yazi: they walk `(cursor + i) % len` and return the matching id.
- `find_arrow` uses `include = false` and the current `focus`, so the finder follows focus (decision (a)).

### D4. Case handling
`Case::{Sensitive, Insensitive, Smart}` is parsed from `--insensitive` / `--smart`. If both flags are given, the last one wins. With `Smart`, matching is case-insensitive unless `query.chars().any(char::is_uppercase)`, checked on the raw query like yazi does. Matching uses the `regex` crate on `&str`, since names are already UTF-8.

### D5. Command parsing
- `find` accepts any order and combination of `--previous`, `--smart` and `--insensitive`.
- `find_arrow` accepts nothing or `--previous`.
- Anything else is a bad-arguments error (exit 2), consistent with the existing commands.
- New actions: `Action::Find { previous, case }` and `Action::FindArrow { previous }`.
- The command keeps yazi's `find_arrow` spelling so yazi users' keymaps carry over, even though pane-mux's own commands use kebab-case (`kill-word`).

### D6. Escape order
`Action::Escape` clears, in order and stopping at the first that applies: which-key, popup, finder, message. Today the order is which-key, popup, message. The finder goes after the popup because the picker-layer `escape` only runs when no popup is open.

### D7. Rendering
- **Highlights (`ui/columns.rs`):** when `state.finder` is set and `level == state.focus`, split each row's name into before / match / after using the first regex match. The match is drawn as `row_style.patch(find_keyword)`. Because the name follows the `"N: "` index prefix in window and pane labels, offsets are computed on the name part only.
- **Badge:** on the hovered row, if it matches, append `" [X/Y]"` in `find_position` after the name. X and Y are computed from `names(focus)` during render. With the fake tmux and real tmux lists, the full scan is cheap. There's no 99+ cap.
- **Text cursor:** `ui/popup.rs` `input()` replaces the hard-coded `value + REVERSED` cursor cell with `Style::from(&theme.input.selected)`.

### D8. Theme and preset
- **`InputTheme`:** add `selected`. The preset sets `value = { fg = "white" }` and `selected = { reversed = true }`.
- **`PickerTheme`:** add `find_keyword` and `find_position`. The preset defaults are the user's yazi values:
  - `find_keyword`: yellow, bold, italic, underline.
  - `find_position`: magenta on `reset`, bold, italic.
- **Default keymap:** add `<Space> /`, `n` and `N` to `[picker]`, with descriptions.

## Risks / Trade-offs

- [`<Space>` as a prefix: a user binding a lone `<Space>` in `[picker]` shadows `<Space> /`, because exact single-key matches win.] → This is existing documented behaviour. Note it in the README keymap section.
- [Smart case treats regex escapes like `\S` as uppercase.] → This is yazi's behaviour, kept for parity. Users can force case-insensitive matching with `--insensitive`.
- [Jumping in the sessions or windows column resets the child cursors to their active items on every keystroke.] → This is the expected effect of `select`. The preview follows the jump, like it does for arrow moves.
- [Regex compile on each keystroke.] → Queries are tiny and compile cost is negligible at typing speed.
- [Default `[input] value` changes from `#c8d3f5` to `white`.] → Visual only. Users can override it in `theme.toml`.

## Migration Plan

No migration is needed. All keymap and theme additions merge in from the presets, and existing user files stay valid. To roll back, revert the change. No persisted state exists.
