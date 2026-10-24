# Proposal

## Why

Rearranging tmux panes across windows and sessions takes several tmux commands or a separate fzf script (`move-pane-to-session.sh`). pane-mux already shows the whole session/window/pane tree, so moving a pane should be a yazi-style cut and paste inside the picker, plus a one-shot `pane-mux move-pane` command that can replace the script.

## What Changes

- New picker actions `cut` and `paste`, bound by default to `x` and `p`.
  - `x` on a pane (panes column focused) puts it in a single-pane cut buffer. `x` on another pane moves the cut there, and `x` on the cut pane clears the buffer.
  - `p` pastes based on the focused column:
    - Sessions column: prompt for a session name, then a window name. A new session is created with the pane as its only window.
    - Windows column: prompt for a window name. The pane becomes a new window in the hovered session.
    - Panes column: the pane joins the hovered window as a floating pane, sized and placed by `[floating-pane]`. Pasting into the pane's own window does nothing.
  - Moves never change the client's current session, window or pane (detached moves, as in the script). After a paste the cursor follows the moved pane, as it does after create, and the buffer is cleared.
  - Short on-screen messages when cut or paste can't apply (no pane focused, nothing cut, zoomed target window, tmux error).
  - Floating panes can be cut and pasted like tiled panes. Pasted as a new session or window, they become tiled, as with the script.
- A yazi-style cut indicator: a coloured bar on the cut pane's row and a header badge naming the cut pane's target.
- A new command `pane-mux move-pane [TARGET]`. It opens the picker with `TARGET` (default: the current pane) already cut and locked, so it can't be uncut or changed. The picker closes after a successful paste.
- Config: the `[floating-pane]` section (`width`, `height`, `position`) becomes a supported setting. The uncommitted `always_on_top` key is removed, because tmux cannot apply it to a moved pane.
- Theme: a new `[marker]` section with `cut` (default `{ fg = "lightred", bg = "lightred" }`), `symbol` (default `"│"`) and `badge_cut` (default `{ fg = "white", bg = "red" }`).

## Capabilities

### New Capabilities
- `pane-moving`: Cutting a pane in the picker, pasting it into a new session, a new window or an existing window as a floating pane, the cut indicator, and the `pane-mux move-pane` command.

### Modified Capabilities
- `configuration`: Adds `[floating-pane]` settings and extends the theme requirement with the `[marker]` section.

## Impact

- Code: `src/picker/action.rs`, `src/picker/state.rs`, `src/picker/popup.rs`, `src/picker/options.rs`, `src/picker/ui/columns.rs`, `src/picker/ui/mod.rs`; `src/tmux/client.rs`, `src/tmux/format.rs`, `src/tmux/model.rs`, `src/tmux/fake.rs`; `src/config/settings.rs`, `src/config/theme.rs`, `src/config/keymap/run.rs` and the three presets; `src/cli.rs`, `src/main.rs`, and a new `src/commands/move_pane.rs`.
- tmux: requires floating-pane support (`break-pane -W`, `move-pane -P`, tmux 3.8). Other picker features keep working on older tmux, where only the floating paste fails, with tmux's error shown on screen.
- Depends on `lenient-config-schemas`, which lands first on the same branch: the new config and theme keys go into the generated schemas, and the preset no longer fails on unknown sections.
- Docs: `docs/configuration.md`, `docs/usage-examples.md`, README commands section.
