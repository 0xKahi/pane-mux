# Picker actions

## cut and paste

Run `pane-mux picker`, focus the panes column, and press `x` to cut the hovered pane. Cutting another pane replaces the buffer; cutting the same pane again clears it. A left-edge marker and a `cut <target>` header badge identify the source. Cutting does not move anything. Floating panes can also be cut; when pasted as a new session or window, they become tiled panes.

Navigate to a destination and press `p`:

| Focused column | Destination |
| --- | --- |
| Sessions | New session: enter a session name, then a window name |
| Windows | New window in the hovered session: enter a window name |
| Panes | Floating pane in the hovered window, sized and placed by `[floating-pane]` |

Empty names use tmux defaults. Cancelling either naming prompt changes nothing. Floating paste requires tmux 3.8; pasting into the source window does nothing, and a zoomed target window is refused. With nothing cut, paste only shows a message. tmux errors are shown in the picker and keep the cut buffer.

After a successful paste, `pane-mux picker` stays open, selects the result and clears the buffer. `pane-mux move-pane` uses the same destinations but starts with the cursor on the session that holds the cut pane, closes after a successful paste and does not allow changing the source with `x`.

Moves are detached: they do not switch the client to the destination. Empty source windows and sessions close normally. Moving the last pane of the attached session destroys it; with `detach-on-destroy on`, the client detaches and the popup closes.

See [configuration](configuration.md#floating-pane-settings) for floating size and position, and `[marker]` theme styles for the cut indicator.
