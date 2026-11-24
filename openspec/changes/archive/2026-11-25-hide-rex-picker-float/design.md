# Design

## Context

See proposal.md for why this change is needed. On the Rex backend, data flows like this:

```
rex do -e snapshot.lua --json
        |
        v
snapshot.rs  From<SessionView> for Session
  - layers sorted tiled first, then floating
  - blocks flattened; Pane.index = position + 1
  - Pane.floating = layer.kind == "floating"
        |
        v
client.rs  tree()  -> history::rank -> last_tree (cache) -> picker
client.rs  current() = pane_location(last_tree or tree(), REX_BLOCK)
```

`current()` finds the session and window by searching the tree for `REX_BLOCK`. Switch-session recency also depends on it: `current()?` gives the session left (client.rs ~285) and drives the "current first" ranking. Picker state uses `Current` for current-item markers and for deletion protection (`State::is_current`).

Facts from the Rex docs:
- `session.view` lists every block that is in a window, in tiled and floating layers.
- A layer's `kind` is `tiled` or `floating`.
- `state` is `minimized` or absent.
- Blocks have no hidden or ephemeral flag.
- `session.new_layer` always creates real blocks.
- A tmux-style popup is an accepted feature request but hasn't shipped.

## Goals / Non-Goals

**Goals:**
- Filter the self floating block in one place, so every Rex command and every refresh agrees.
- Keep `current()` (session and window) and the recency features working when the self block is filtered out.

**Non-Goals:**
- Hiding other pane-mux blocks, such as stale picker layers or pickers started from other blocks. Only `REX_BLOCK` is hidden.
- Marking the block that was focused before the float opened. `focus_history.reverse` or a launch-time env var could provide it later.
- Any change to the tmux backend or the shared picker state.

## Decisions

### Filter while building the tree, and record the self location as we go

When a snapshot is turned into a `Tree`, the conversion receives the self block ID. It skips that block only if its layer is `floating`, and assigns `Pane.index` after skipping, so numbering has no gap. While it walks the snapshot, it records where the self block was found (session ID, window ID) whether or not the block is skipped. The client stores that location next to `last_tree`, and `current()` returns it: `Current { session_id, window_id, pane_id: REX_BLOCK }`.

- `pane_id` stays `REX_BLOCK` even when the block is hidden. No listed row matches it, so no pane gets the marker. Session and window markers and deletion protection are unchanged.
- One tree serves the picker, `lookup()` and the history ranking, and every refresh goes through `tree()`, so the filter survives refreshes for free.

Alternatives considered:
- **Keep the unfiltered tree in `last_tree` and filter only the copy returned to the picker.** This needs no new state, but it leaves two trees with different numbering, and `lookup()` callers could see blocks the user cannot. It's easy to get wrong.
- **Filter in the picker (shared `State`) using `suppressed_pane()`.** This would also hide a tiled self block under tmux-like semantics, and the picker can't tell whether the block is floating without a Rex-specific field. It would put backend detail into shared code.
- **Filter in `snapshot.lua`.** That keeps Rust simpler, but it loses the self location needed by `current()`, and the Lua script is deliberately read-only and unscoped (it runs with `REX_BLOCK` unset).

### Decide "floating" by the self block's layer kind

The self block is hidden when the layer that contains it has `kind == "floating"`, whatever its `state` (normal or minimized). This matches the existing `Pane.floating` flag and covers the documented `new_layer` key bindings. Detecting pane-mux by process name or label was rejected: it's fragile, and it would hide a real shell the user typed `pane-mux` into.

### `current()` falls back to a fresh tree as today

If no tree has been built yet, `current()` builds one, which records the self location, then returns that location. If `REX_BLOCK` is unset or not found, it returns `None`, as now.

## Risks / Trade-offs

- [`pane-mux move-pane` with no target, run inside the float, uses the current pane, which is now the hidden float itself.] The documented binding always passes `ctx.block_id`, and moving the picker's own float was never meaningful. Behaviour is left as it is, and a test records it so it doesn't regress silently. If it becomes a problem, refuse it in a follow-up.
- [The self float in a window with no other blocks gives an empty panes column.] This only happens if the user has floated every block of that window. An empty column is already handled.
- [`lookup()` for the self block fails, because the block isn't in the tree.] No current flow looks up the hidden self block except the degenerate `move-pane` case above.
- [Rex later ships real popups.] The filter is harmless if pane-mux no longer runs in a block layer. The key binding docs can switch to popups then.

## Migration Plan

No data or config migration. Update the Rex section of the README and `docs/usage-examples.md` to say the picker's float is not listed. To roll back, revert the change.
