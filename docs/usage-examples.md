# Examples

## open session picker in tmux popup 

```tmux
bind o popup -w 75% -h 75% -x C -y C -E 'pane-mux switch-session'
```

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

