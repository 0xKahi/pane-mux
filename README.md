# pane-mux

A yazi-style terminal picker for tmux sessions, windows, and panes, written in Rust. It provides a three-column tree, a pane preview, configurable key bindings, and themes, with picked tmux targets intended for shell scripting.

## Commands

Install from the project directory:

```sh
cargo install --path .
```

```text
pane-mux picker [--session|--window|--pane]
pane-mux switch-session
```

`picker` prints exactly one tmux target after closing the UI: a session name,
`session:window_index`, or `session:window_index.pane_index`. Without a level
flag, the focused column determines the output. The UI uses stderr (or
`/dev/tty` when stderr is redirected), leaving stdout available for scripts.

`switch-session` requires tmux and switches the current client to the picked
target, including its window and pane when selected. It prints no target.

| Exit code | Meaning |
| --- | --- |
| 0 | Picked successfully (and switched for `switch-session`) |
| 130 | Cancelled without picking |
| 2 | Error or invalid command-line usage |

Capture a session and switch from a shell inside tmux:

```sh
selected=$(pane-mux picker --session) && tmux switch-client -t "$selected"
```

Add this binding to `~/.tmux.conf` to open the switcher in a popup:

```tmux
bind-key s display-popup -E -w 90% -h 90% "pane-mux switch-session"
```

For command help, run `pane-mux --help` or `pane-mux picker --help`.
Level flags are mutually exclusive: `pane-mux picker --session --pane`
prints a usage error without opening the UI.

## Configuration

The configuration directory is the first available of:

1. `$PANE_MUX_CONFIG_HOME`, if absolute.
2. `$XDG_CONFIG_HOME/pane-mux`, if `XDG_CONFIG_HOME` is absolute.
3. `$HOME/.config/pane-mux` (also on macOS).

Relative overrides are ignored. If no directory can be resolved, built-in defaults are used.

The directory may contain `config.toml`, `keymap.toml`, and `theme.toml`. Each file is optional; a missing directory is fine. Invalid TOML, unknown fields, invalid commands, and invalid values are errors naming the file; pane-mux then exits with code 2.

### Picker settings

`config.toml` and `theme.toml` recursively merge tables over the defaults. Other values, including arrays, replace defaults; omitted keys retain defaults.

`preview_ratio` is an integer from 1 through 99 (default 50). `preview_position` is `"top"` or `"bottom"` (default `"bottom"`).

<!-- config.toml -->
```toml
[picker]
preview_position = "top"
preview_ratio = 75
```

### Key bindings

`keymap.toml` has four independent layers: `[picker]`, `[input]`, `[confirm]`, and `[help]`. Within each:

- `keymap` replaces the preset bindings for that layer, including when set to `[]`.
- `prepend_keymap` comes before the base and shadows matching base bindings.
- `append_keymap` comes last and cannot override base or prepended bindings.

Binding conflicts compare the first two keys of the sequence (including a missing second key), following yazi's rule. Untouched layers keep their presets.

`on` accepts one key or a list forming a sequence. Keys include literal characters (`j`, `?`), modifiers (`<C-u>`, `<S-Tab>`, `<A-x>`), and named keys (`<Enter>`, `<Esc>`, `<Space>`, `<PageUp>`, `<PageDown>`, `<F1>`). `run` accepts a command string or a list run in order. `desc` is optional help text.

<!-- keymap.toml -->
```toml
[picker]
prepend_keymap = [
    { on = "x", run = "quit", desc = "Quit" },
    { on = ["g", "g"], run = "arrow top", desc = "First item" },
]
append_keymap = [
    { on = "z", run = ["escape", "help"], desc = "Show help" },
]

[input]
append_keymap = [
    { on = "<C-w>", run = "kill-word", desc = "Delete previous word" },
]
```

Supported commands (arguments are whitespace-separated):

| Layer | Commands |
| --- | --- |
| picker | `escape`, `quit`, `arrow next`, `arrow prev`, `arrow top`, `arrow bot`, `arrow N`, `arrow N%`, `focus prev`, `focus next`, `seek N%`, `pick`, `create`, `rename`, `remove`, `help` |
| input | `close`, `close --submit`, `backspace`, `kill-word`, `kill-bol` |
| confirm | `close`, `close --submit` |
| help | `arrow` with the same arguments as above, `close` |

`N` is a signed integer: for example, `arrow -3`, `arrow -100%`, or `seek 50%`. Input character insertion is implicit, not a run command. The parser recognises commands globally; use them in the appropriate layer.

### Theme

Styles accept optional `fg`, `bg`, `bold`, `italic`, `underline`, `reversed`, and `dim` fields. Colours use ratatui names (such as `green`, `lightcyan`, `darkgray`), `"#rrggbb"`, or `"reset"`. Modifiers are booleans; explicit `false` removes the modifier. Each style table merges independently with the preset.

Theme sections and style keys:

- `[picker]`: `border`, `hovered`, `hovered_unfocused`, `header`
- `[indicator]`: `padding = { open, close }`, `active = { text, fg, ... }`
- `[preview]`: `border`, `title`
- `[mode]`: `normal`, `input`, `confirm`
- `[status]`: `overall`, `label`, `message`
- `[which]`: `border`, `key`, `desc`, `separator`
- `[confirm]`: `border`, `title`, `body`
- `[input]`: `border`, `title`, `value`
- `[help]`: `border`, `title`, `on`, `run`, `desc`, `hovered`
- `[notify]`: `error`

`[icons]` has `session`, `window`, and `pane` tables with `text` and optional `fg`. Override just `text` to preserve the preset colour. Empty text hides an icon. Default glyphs require a Nerd Font; plain text alternatives work too.

`[indicator] padding` sets the hovered-row caps (defaults `"\uE0B6"` and `"\uE0B4"`, matching yazi). Either side can be overridden independently. Caps use the highlight colour against the terminal background; styles without a background or reversal use spaces instead. `active` sets the current-item marker (default `*`, magenta) and accepts the same style fields listed above. Non-current rows reserve the marker's display width. Names are not restyled for current items.

Hovered rows use one uniform style for marker, icon, name, and fill. The focused default is reversed and bold; other columns use reversed without bold. Unhovered icons and markers retain their own colours. A default row reads `cap + space + marker + space + icon + space + name + fill + cap`; unhovered caps are spaces.

<!-- theme.toml -->
```toml
[preview]
border = { fg = "#82aaff" }

[picker]
hovered = { bold = true, reversed = false, bg = "blue" }

[indicator]
padding = { open = "<", close = ">" }
active = { text = "*", fg = "magenta" }

[icons]
session = { text = "S" }
window = { text = "W" }
pane = { text = "P" }
```

### Full defaults

The complete, authoritative defaults are:

- [config-default.toml](src/config/preset/config-default.toml)
- [keymap-default.toml](src/config/preset/keymap-default.toml)
- [theme-default.toml](src/config/preset/theme-default.toml)

Debug builds read these preset files from the source tree at runtime. Release builds embed them in the binary, so the source tree is not needed.
