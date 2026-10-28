# Design

## Context

- The picker's tree comes from a single `list-panes -a -F PANE_FORMAT` call (`src/tmux/format.rs`, `TmuxCli::tree` in `src/tmux/client.rs`), parsed by `parse_tree` into `Tree` / `Session` / `Window` / `Pane` (`src/tmux/model.rs`). The picker re-reads it about once a second (Live refresh).
- Stashes (`src/commands/float.rs`) live in a session named `pmux` marked with session option `@pmux_stash = 1`. Each stash window carries window option `@pmux_source = <source window id>` and holds a tiled placeholder pane plus the stashed panes, which stay floating. The placeholder may have been killed.
- Rows are drawn in `src/picker/ui/columns.rs::column`. The find `[X/Y]` badge already sits inline after the label, and its width is reserved before the label is clipped. On "filled" hovered rows (`bg` or `reversed`) icon and marker spans drop their colours.
- `FakeTmux` (`src/tmux/fake.rs`) stores a `Tree` and a separate option map. `tree()` returns the stored tree as is.

## Goals / Non-Goals

**Goals:**
- Get both indicators from the existing snapshot query, with no extra tmux calls per refresh.
- Keep the stash count correct whether or not the placeholder pane still exists.

**Non-Goals:**
- Refactoring the row layout code beyond what the indicator needs.
- Exposing pane mode anywhere other than the pane row (no status messages, no keymap conditions).

## Decisions

### 1. Extend `PANE_FORMAT` with three fields

Append `#{pane_mode}`, `#{@pmux_stash}` and `#{@pmux_source}` to `PANE_FORMAT` (16 fields). tmux resolves a `#{@option}` in pane context through pane → window → session → global options, so a pane row picks up its window's `@pmux_source` and its session's `@pmux_stash`. Older tmux versions and unset options expand to empty strings, which parse as "no mode" / "not a stash".

- *Alternative:* a separate `show-options` per `pmux` window, as `float.rs` does. Rejected: it adds N tmux calls to every refresh.
- *Alternative:* `#{pane_in_mode}`. Rejected: it does not tell copy and view mode apart from tree, clock and other modes.

### 2. The model carries raw facts; counts are derived

- `Pane` gets `mode: String` (raw `pane_mode`) plus a helper `in_copy_mode()` that is true for `copy-mode` and `view-mode`.
- `Session` gets `stash: bool` (`@pmux_stash == "1"`). `Window` gets `stash_source: Option<String>` (non-empty `@pmux_source`).
- A pure function, `Tree::stash_counts() -> HashMap<window_id, usize>`, looks only at sessions named `pmux` with `stash == true`. For each window with a `stash_source`, it adds that window's floating panes to the source window's count. Counting floating panes leaves out the placeholder (tiled), whether or not it still exists, matching how `float.rs` treats stash contents.
- The picker computes the counts once per tree load/refresh (stored on `State`, or computed from `state.tree` during render). Rendering looks them up by window id.

*Alternative:* a precomputed `Window.stashed: usize` filled in by `parse_tree`. Rejected: the fake tmux and tests would then have to reproduce the cross-session join by hand. Keeping raw fields plus one derivation function keeps both real and fake trees honest.

### 3. FakeTmux overlays options in `tree()`

`FakeTmux::tree()` fills `Session.stash` and `Window.stash_source` from its option map before returning, so float-command tests and picker tests share one source of truth. Pane mode stays a plain field that tests set directly on the stored tree.

### 4. Rendering inline, reserved like the find badge

In `column()`, build an optional indicator span per row: the stash text with `$count` replaced for window rows, the mode text for pane rows, `None` when the theme text is empty or the state does not apply. The row becomes

```
label (clipped)  |  "  " + indicator  |  " [X/Y]" badge
```

The widths of the indicator (gap included) and the badge are subtracted from `content_area.width` before the label is drawn, the same way the badge is today. The indicator is drawn right after the label's actual width. The indicator gets `fg` from its `Icon`, or the default style when `filled`, following the same rule as the level icons. Width is measured with `Span::width()`, so wide Nerd Font glyphs are handled.

The existing picker-find requirement ("an `[X/Y]` badge SHALL follow its name") still holds: the badge still follows the name, now after the indicator.

### 5. Theme keys under `[icons]`

Add `stashed: Icon` and `copy_mode: Icon` to `Icons`, reusing the existing `Icon { text, fg }` type. `$count` substitution is a plain string replace at render time. Defaults go in `theme-default.toml` as Unicode escapes (`\U000F09A4 $count`, `\U000F18F1`). The key keeps the name `copy_mode` even though it also covers view mode: it is the name users will look for, and view mode is a read-only relative of copy mode.

## Risks / Trade-offs

- [A pane-level `@pmux_source` would shadow the window option in format resolution] → pane-mux sets these only at window/session scope. The integration test checks the real query against a real stash.
- [The `pmux` session's own stash windows also carry `@pmux_source`] → counts are keyed by the *source* window id, so stash windows never count themselves. They show no indicator unless something were stashed *from* them, which `float stash` refuses.
- [A user session named `pmux` without `@pmux_stash`] → ignored, matching `float.rs`'s ownership check.
- [`PANE_FORMAT` field count changes] → `parse_row`'s field-count check and its tests are updated together. Tolerating empty values keeps older tmux working.
- [Narrow columns] → the indicator is reserved before the name is clipped. In extremely narrow columns it gets clipped like the badge, never panicking (covered by the existing tiny-terminal test pattern).
