# Examples

## open session picker in tmux popup 

```tmux
bind o popup -w 75% -h 75% -x C -y C -E 'pane-mux switch-session'
```

## move a pane from a tmux popup

Add to `~/.tmux.conf`, then reload tmux (`tmux source-file ~/.tmux.conf`) and press `<prefix> M`:

```tmux
bind-key M display-popup -E "pane-mux move-pane"
```

`pane-mux move-pane [TARGET]` opens with `[Move]` and the source pane already cut and locked. Without a target it uses the invoking client's current pane; explicit targets accept tmux syntax:

```sh
pane-mux move-pane work:1.2
pane-mux move-pane '%17'
```

It starts with the cursor on the session that holds the cut pane. Press `p` to paste using the focused column as described in [picker actions](picker-actions.md#cut-and-paste). A successful paste closes the picker with exit 0 and no target output; `q` without pasting exits 130. Unknown targets, or omitting the target outside tmux, exit 2. Cancelled prompts, failed moves and same-window paste keep the picker open. Prefer the popup binding: running directly in the source pane moves the shell running the picker too.

## stash and unstash floating panes

Requires tmux 3.8. Add to `~/.tmux.conf`, then reload tmux (`tmux source-file ~/.tmux.conf`) and press `<prefix> f`:

```tmux
bind f run-shell 'pane-mux float toggle'
```

`pane-mux float stash|unstash|toggle [TARGET]` acts on the window containing the optional target, or the current client's window when omitted:

```sh
pane-mux float stash work:1.2
pane-mux float unstash work:1.2
pane-mux float toggle
```

- `stash` stashes every floating pane, keeping it floating at its original size and position and leaving tiled panes in place. New floats merge into any existing stash.
- `unstash` restores all stashed floats with their saved geometry and stacking order, alongside any visible floats. Geometry scales proportionally to fit if the source window changed size.
- `toggle` stashes when any float exists (including floats hidden by zoom); otherwise it unstashes. With both visible and stashed floats, two toggles stash and then unstash them all.

Stashing an active float returns focus to the last-active tiled pane; unstashing refocuses the float active at stash time. Stash preserves zoom; unstash unzooms first. Nothing to stash or unstash produces a tmux message and exits 0. An unknown target, no target outside tmux, or targeting a stash window exits 2.

### manage the stash session

Floats live in `pmux`, an ordinary tmux session tagged with `@pmux_stash=1`, created on first stash. Each source window gets one stash window named after its `session:window`, with a tiled placeholder pane explaining how to restore. Renaming or renumbering the source does not lose its stash. If you already have an untagged session named `pmux`, stash fails rather than using it.

The stash appears in the picker and `switch-session`. To exclude it from recency ranking and list it last, add to `config.toml`:

```toml
[switch-session]
exclude_from_recency = ["pmux"]
```

This does not hide `pmux`. Unstashing all floats removes their stash window; the last restored stash also removes the session. Closing a source window leaves its orphaned stash alone. Open `pane-mux picker` and kill the orphaned window, or cut its panes with `x` and paste them elsewhere with `p` to recover them. See [picker actions](picker-actions.md) for kill and cut/paste controls.

## script to open picker and attach tmux session 

```bash
#!/bin/sh

# Check if currently in tmux session 
if [ -n "$TMUX" ]; then
    printf "\e[101;30mERROR\e[0m attach cant be run inside of tmux session\n"
    exit 0 
fi

# Let user select from available files
selected=$(pane-mux picker --session)

if [ "$selected" ]; then
   tmux a -t "$selected"
fi
```

