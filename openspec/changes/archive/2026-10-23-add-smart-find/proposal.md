# Proposal

## Why

The picker can only move between rows one step, one page, or to the top or bottom. With many sessions, windows or panes, reaching a known name is slow. Yazi users already know `find --smart` and `n` / `N` for jumping to a name. pane-mux is meant to feel like yazi, and V1 explicitly deferred search, so this is the next gap to close.

## What Changes

- New picker action `find --smart` (plus `find --previous` and plain `find`). It opens a "Find next:" prompt. As the user types, the focused column's cursor jumps to the next row whose name matches.
  - The query is a regex.
  - `--smart` makes matching case-insensitive unless the query contains an uppercase character.
- New picker action `find_arrow [--previous]`. It jumps to the next or previous match after the cursor, wrapping around.
- The finder searches whichever column is focused. Names only are matched: the session name, the window name, and the pane command.
- Closing the prompt keeps the finder, whether with `<Enter>` or `<Esc>`, and does not restore the cursor. `escape` in the picker clears an active finder.
- In the focused column, matches are highlighted and the hovered match shows an `[X/Y]` position badge.
- New default bindings: `<Space> /` runs `find --smart`, `n` runs `find_arrow`, and `N` runs `find_arrow --previous`.
- Theme changes:
  - New `[input] selected` style for the prompt's text cursor, which is currently hard-coded as reversed.
  - New `[picker] find_keyword` and `[picker] find_position` styles.
  - The `[input]` defaults become: border yellow, title red, value white, selected reversed.
  - `find_keyword` and `find_position` take their defaults from the user's yazi theme.

## Capabilities

### New Capabilities
- `picker-find`: incremental, regex-based find within the picker's focused column. Covers the find prompt, smart-case matching, next/previous navigation, finder lifetime, match highlighting and the position badge.

### Modified Capabilities
- `configuration`: the Theme styles requirement gains the `[input]` section, including the new `selected` style, and the `[picker] find_keyword` / `find_position` styles.

## Impact

- Code:
  - `src/picker/action.rs` and `src/config/keymap/run.rs`: new actions and argument parsing.
  - `src/picker/state.rs`: finder state, realtime prompt, `escape`.
  - `src/picker/popup.rs`: find input purpose.
  - `src/picker/ui/{popup,columns}.rs`: cursor style, highlights, badge.
  - `src/config/theme.rs`: new style fields.
  - Presets `src/config/preset/{keymap,theme}-default.toml`.
- Dependencies: adds the `regex` crate.
- Compatibility:
  - The keymap and theme changes add to what exists; nothing is removed.
  - `n`, `N` and `<Space>` were unbound before.
  - User themes that set `[input]` keep working, because the new keys merge in from the defaults.
  - The default `[input] value` colour changes from `#c8d3f5` to `white`.
