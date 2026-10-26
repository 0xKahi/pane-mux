
# Design

## Context

See proposal.md for motivation and `specs/floating-pane-stash/spec.md` for behavior.

All tmux access goes through the `Tmux` trait (`src/tmux/client.rs`), with `FakeTmux` (`src/tmux/fake.rs`) for unit tests and a private-socket server in `tests/tmux_integration.rs`. Commands live in `src/commands/`, return `Outcome`, and errors exit 2 (`src/main.rs`). `move-pane` sets the convention for targeting: an optional positional target, otherwise `Tmux::current()`, which needs `$TMUX`. `run-shell` jobs do get `$TMUX`, so key bindings work.

tmux 3.8 has no way to hide a pane. Spikes on a private server established these facts.

Moving panes:
- `join-pane` always makes a moved float tiled by splitting the target pane. Moving a float with `join-pane` and then `break-pane -W` therefore shrinks the target pane and resizes the float to the full split, then reverses both. That is two resizes for each program, and it caused visible redraw glitches in the destination's main pane (a Neovim dashboard lost its image and stayed re-anchored).
- `new-pane -d -x -y -X -Y` creates a floating pane directly in any window, without changing the active pane. `-E` creates it with no process, `-T` sets its title at creation, and `-P -F '#{pane_id}'` prints its id. It also works in a window whose only panes are floating, which `join-pane` refuses ("can't split a floating pane").
- `swap-pane -d -s A -t B` between two floats works across windows and sessions. Each pane takes the other's slot: floating state, geometry, stacking position (`pane_z`) and, if the slot was active, the active flag. Measured: zero `SIGWINCH` for every tiled pane, and zero for the moved float when both slots have the same size. When the sizes differ, the moved float gets exactly one resize, to its final size.
- Pane id, process, pane user options (`@...`) and the "visible above zoom" attribute (`new-pane -A`, shown as `A` in `#{pane_flags}`) all travel with the pane through `swap-pane`. `-A` can only be set when a pane is created.
- `-x/-y/-X/-Y` include the border, while `#{pane_width}/#{pane_height}/#{pane_left}/#{pane_top}` do not: `x = width + 2`, `y = height + 2`, `X = left - 1`, `Y = top - 1`. This holds for `new-pane` and `break-pane -W`.
- `new-pane` rejects a position that starts outside the window ("position too large"), but silently accepts oversized sizes and negative positions.

Zoom:
- In a zoomed window, floats without `-A` are hidden. They report `pane_floating_flag=0` and lose `F` in `#{pane_flags}`, and they are missing from `#{window_visible_layout}`. They still appear in `#{window_layout}`.
- A plain `new-pane` or `swap-pane` touching a zoomed window unzooms it. `new-pane -A` and `swap-pane -Z` keep the zoom. Killing a hidden float unzooms the window. Killing a visible `-A` float keeps it zoomed while other visible floats remain; the spike was inconsistent about killing the last visible float.

Other:
- When the active pane leaves a window, tmux makes the last-active tiled pane active.
- Killing the last pane of a window removes the window, and removing the last window ends the session.

## Goals / Non-Goals

**Goals:**
- Stash and unstash are a lossless round trip for geometry, stacking order and focus when the window size hasn't changed.
- Moves never split or resize a tiled pane, and never resize a float whose window size is unchanged. The only exception is the unzoom that unstash performs on a zoomed window.
- Every step is safe to re-run after a partial failure, and leaves no temporary pane behind.

**Non-Goals:**
- Surviving a tmux server restart (panes die with the server anyway).
- Stashing individual floats; the unit is the whole window.
- Configurable stash session name, a picker view of stashes, or orphan cleanup.
- Changing a float's "visible above zoom" attribute; it is kept as created.
- Supporting tmux older than 3.8 beyond letting its errors surface.

## Decisions

### Swap move through a dummy float
Every move of a float into a window, for stash, unstash and the picker's floating paste, uses one helper:

```
new-pane -d -A -E -T pane-mux-dummy -t <destination window> -x X -y Y -X PX -Y PY -P -F '#{pane_id}'   -> %D
swap-pane -d -Z -s %P -t %D \; kill-pane -t %D
```

The dummy is created at the pane's target geometry, so `%P` arrives floating exactly there. The dummy then sits in `%P`'s old slot and is killed in the same tmux invocation, which keeps its brief visibility to a few milliseconds.
- `-A` keeps a zoomed destination zoomed and makes killing the dummy zoom-safe.
- `-Z` keeps a zoomed source or destination zoomed during the swap.
- `-E` gives the dummy no process; `-T` marks it as a dummy at creation, with no window for a race.
- *Alternative, `join-pane` then `break-pane -W`:* this was the first implementation. It splits the destination's tiled pane and resizes both programs twice per move (see Context).
- *Alternative, `join-pane -l 1`:* smaller resizes, but still two per move, and it squeezes the float to one row in between.

### Stash window: placeholder pane as a label
Each stash window is created with a tiled placeholder pane and sized to the source window (`resize-window -x -y`). Stashed panes float above it at their original geometry. The placeholder only labels the window: it prints the source `session:window` and `pane-mux float unstash`, then blocks (`printf ...; exec tail -f /dev/null`). Because `new-pane` works in a window holding only floats, nothing depends on the placeholder. If the user kills it, the window keeps its floats and later stashes add to it.
- *Alternative, no placeholder:* stash windows would hold only floats. This removes a little code, but the window would lose its restore hint and need a new-window-then-kill-the-initial-pane step.

### Pane options are the source of truth for geometry
At stash time each float gets `@pmux_x/@pmux_y/@pmux_w/@pmux_h` (border-inclusive values, ready for the flags), `@pmux_z`, `@pmux_win_w/@pmux_win_h` (source window size) and `@pmux_active`. Unstash reads only these options and never the pane's current geometry in the stash. Resizing the stash window, or attaching to `pmux` from a small terminal, therefore can't distort the restore. A float that is already stashed keeps its options when more floats are merged in.

### Scaling on unstash
If the source window is still `@pmux_win_w x @pmux_win_h`, the stored values are used as-is. Otherwise each value is scaled by the size ratio (rounded), then clamped: size to at least 3x3 and at most the window, position so the pane fits. Clamping is required: `new-pane` only rejects positions that start outside the window and silently accepts oversized floats.

### Stash window size
Before stashing, the stash window is resized to fit both the source window and the stored geometry of every float already stashed for that source, so the dummy never hits "position too large".

### Stacking order
Floats are moved back to front (descending `pane_z` on stash, descending `@pmux_z` on unstash). Each dummy is created at the front, and the moved pane takes its slot, so the last pane moved ends up at the front. This keeps the original relative order, and restored floats sit above any floats already visible. No `move-pane -z` is needed.

### Finding stash windows
The `pmux` session is tagged with `@pmux_stash=1`. Each stash window is tagged with `@pmux_source=<window_id>` and named `session:window` at stash time; the name is only for display. Lookup filters `pmux` windows on `@pmux_source`. If a session named `pmux` exists without the tag, the command fails.
- *Alternative, matching by window name:* breaks on rename or renumbering.

The first stash creates the session with `new-session -d -s pmux -x W -y H -n <name> <placeholder>`, so the placeholder is the initial window and nothing needs deleting. Later stash windows use `new-window -d -t pmux: -n <name> <placeholder>` and `resize-window`.

### Finding floats, including in zoomed windows
Floats and their geometry come from the per-pane formats when the window isn't zoomed. In a zoomed window, hidden floats report as tiled, so membership, geometry and `z` come from `#{window_layout}`, which still lists them. The implementation confirms the layout's coordinate convention (inner or border-inclusive) on a private server and converts to the same values as `capture`. Unzooming to read the formats is not an option, because it would resize the zoomed pane twice.

### Focus and zoom
- Stash: if the stashed float held the active slot, the dummy inherits it, and killing the dummy makes the last-active tiled pane active. `@pmux_active=1` records which float it was. `-A` and `-Z` keep a zoomed window zoomed. As a fallback, if the window is no longer zoomed after the moves, `resize-pane -Z` re-zooms the same pane.
- Unstash: if the window is zoomed, `resize-pane -Z` unzooms it before restoring, so restored floats without `-A` are visible. After restoring, `select-pane` selects the pane with `@pmux_active=1`, if any.

### Leftover dummies
A dummy only outlives a command if the process dies between `new-pane` and the chained `swap-pane \; kill-pane`. Every float command starts by killing panes titled `pane-mux-dummy` in the source window and in its stash windows. A dummy has no process, so its title never changes.

### Unstash cleanup
After an unstash, a stash window with no stashed float left in it is removed with `kill-window` if it still holds the placeholder. Without the placeholder, tmux removes it when its last dummy is killed. A window that still holds stashed floats after a partial failure is kept. When the last stash window goes, the `pmux` session ends on its own.

### Picker floating paste
`float_pane_in_window` (pasting a cut pane into a window from the picker's panes column) uses the same helper. The dummy is created with `-x W% -y H%` from `[floating-pane]` and moved to its `position` with `move-pane -P` before the swap. Its behavior in `openspec/specs/pane-moving` is unchanged; the destination window's panes are just no longer split during the paste.

### Locking: file lock, not `tmux wait-for`
Each command holds an exclusive `std::fs::File::lock` on `<socket_path>.pane-mux-float.lock`, where `socket_path` is `#{socket_path}`: one lock per server, in tmux's private per-user directory. The OS releases the lock if the process dies. `File::lock` requires Rust 1.89, so `rust-version` is 1.89.
- *Alternative, `wait-for -L/-U`:* uses only tmux, but a crash between lock and unlock leaves the channel locked forever, which would freeze every later float command.

### Best-effort execution and messages
Per-pane failures are collected, not returned right away. The command continues with the remaining panes, then returns an error listing the failed pane ids (exit 2). Already-moved panes stay put, and re-running converges because every pane carries its own options and each move is a single swap. "No floating panes" and "nothing stashed" are shown with `display-message` and exit 0, since `run-shell` output would otherwise open tmux's view mode.

### Command and trait shape
In `src/cli.rs`, `Command::Float { action: FloatAction }` with `Stash`, `Unstash` and `Toggle`, each taking `target: Option<String>`. `src/commands/float.rs` holds the stash, unstash and toggle logic, written against `Tmux` so it can be unit-tested with `FakeTmux`. The trait provides small primitives: list a window's panes with geometry, z-order, flags, title and options; read a window's layout; set and read pane and window options; create a dummy float; swap and kill; resize-window; zoom toggle; select-pane; kill-pane and kill-window; display-message; socket path. `FakeTmux` models swaps slot by slot and records calls in the existing `calls()` style.

## Risks / Trade-offs

- [A dummy is briefly visible in the slot the moved pane left] → It has no process, so it's blank, and `swap-pane \; kill-pane` runs in one invocation. Its lifetime is a few milliseconds.
- [The process dies between creating and killing a dummy] → The next float command removes panes titled `pane-mux-dummy`.
- [Killing the last visible dummy might unzoom a zoomed window] → The spike was inconsistent here. The re-zoom fallback restores the zoom, at the cost of one resize of the zoomed pane in that case only.
- [`#{pane_flags}` letters and the `#{window_layout}` format are not fully documented] → They are parsed defensively and covered by integration tests, which should be rerun on tmux upgrades.
- [The placeholder is an extra process per stash window] → It's tiny and blocks forever. A stash window exists only while something is stashed.
- [Rounding when scaling] → Restores are exact when the size is unchanged; scaled restores may be one cell off. That's acceptable.
- [A float closes its program while stashed] → It disappears like any pane; unstash restores whatever is left.
- [The user moves a stashed pane by hand within `pmux`] → Lookup follows `@pmux_source` on the window, so a pane moved to another stash window is restored with that window's source. A pane moved out of `pmux` keeps stale `@pmux_*` options, which nothing reads unless it is stashed again, and stashing overwrites them.
- [`resize-window` on a stash window sets `window-size manual` for that window] → It's intended. It stops attached `pmux` clients from shrinking the stash, and restores don't depend on it anyway.
- [The `pmux` name collides with a user session] → Detected by the missing tag; the command fails instead of using it.
- [tmux < 3.8] → `new-pane` doesn't exist and `pane_floating_flag` expands empty. These surface as exit 2, consistent with floating paste.

## Migration Plan

This is an additive CLI; there is no migration. Users opt in by binding keys, for example `bind f run-shell 'pane-mux float toggle'`.
