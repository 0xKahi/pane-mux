# Proposal

## Why

The picker shows nothing about two common pieces of hidden pane state. A window whose floats were stashed with `pane-mux float stash` looks the same as a window that never had floats, so it is easy to forget the stash exists. A pane left in copy mode (or tmux's view mode) is frozen and does not take input, and from the picker it looks like any other pane. Both are visible only after switching to the item.

## What Changes

- Window rows in the picker show a stash indicator when the window has stashed floats: an icon followed by the number of stashed floats (default `󰦤 <count>`, `lightcyan`).
- Pane rows in the picker show a mode indicator when the pane is in `copy-mode` or `view-mode` (default `󱣱`, `lightyellow`).
- Both indicators sit inline, two spaces after the row's name, before the find `[X/Y]` badge. A long name is clipped before the indicator or badge is. On hovered rows filled by `bg` or `reversed`, indicators drop their colour, as icons and markers already do.
- The theme gains `[icons] stashed` and `[icons] copy_mode` with `text` and optional `fg`, like the existing icons. In `stashed.text`, `$count` is replaced by the number of stashed floats. Empty `text` hides the indicator.
- The indicators update with the picker's existing live refresh.
- Not included: windows do not show their panes' modes, sessions do not show stash totals, and the `pmux` stash session's own rows get no special marking.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tmux-picker`: adds the stash indicator on window rows, the mode indicator on pane rows, and how row indicators are laid out next to the name and the find badge.
- `configuration`: the theme's `[icons]` section gains `stashed` and `copy_mode` entries with their defaults and the `$count` placeholder.

## Impact

- `src/tmux/format.rs`, `src/tmux/model.rs`: the snapshot query reads each pane's mode and the stash markers (`@pmux_stash`, `@pmux_source`) in the same `list-panes -a` call. Window and pane models carry the new data.
- `src/tmux/fake.rs`: the fake tmux reflects stash options and pane modes in its tree so picker tests can exercise them.
- `src/picker/ui/columns.rs`: row rendering adds the indicators and reserves their width.
- `src/config/theme.rs`, `src/config/preset/theme-default.toml`, `schemas/theme.json`: new icon keys and defaults.
- `docs/configuration.md`: documents the new icons.
- No new dependencies, no new tmux round-trips, no breaking config changes: existing theme files keep working and get the new defaults.
