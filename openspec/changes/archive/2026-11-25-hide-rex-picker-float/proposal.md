# Proposal

## Why

On tmux, pane-mux runs in a `display-popup`, which is not a pane, so the picker never shows up in its own tree. Rex has no equivalent. The documented key bindings open pane-mux in a new floating layer, and that layer's block shows up in the panes column as an extra `pane-mux` row. The extra row shifts the numbering and can be hovered, picked, cut or deleted. The Rex docs confirm that blocks have no hidden or ephemeral flag. A tmux-style popup is an accepted Rex feature request but hasn't shipped, so pane-mux has to filter the block out itself.

## What Changes

- On the Rex backend, the block the picker runs in (`REX_BLOCK`) is left out of the panes column when it sits in a floating layer, whether minimized or not. This applies to `picker`, `switch-session` and `move-pane`, and holds across refreshes.
- A self block in a tiled layout, such as pane-mux typed into an ordinary shell block, stays listed as today.
- Pane row numbers count only the listed blocks, so hiding the float leaves no gap.
- The current session and window are still found from `REX_BLOCK`. When the self block is hidden, no pane is marked current.
- The self-pane preview placeholder now applies only when the self block is listed, which means it is tiled.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rex-backend`: "Rex tree mapping" leaves out the picker's own floating block. "Rex row numbering and pane names" counts only listed blocks. "Rex current-item markers" keeps the session and window markers but marks no pane when the self block is hidden. "Rex preview and self-block suppression" is narrowed to a tiled self block.

## Impact

- `src/rex/snapshot.rs`: building the tree from the snapshot, including block filtering and pane index assignment.
- `src/rex/client.rs`: `current()` and the tree it uses to find the self block.
- `src/rex/tests.rs` and fixtures: cases for a floating self block, a minimized one, a tiled one, and renumbering.
- `docs/usage-examples.md` and `README.md` (Rex section): the line that says the picker shows a placeholder when previewing its own block.
- No change to the tmux backend, the CLI, config or the Rex API revision requirement.
