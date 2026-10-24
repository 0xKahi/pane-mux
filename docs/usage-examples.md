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

