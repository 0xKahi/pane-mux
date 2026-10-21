# pane-mux

A yazi-style terminal picker for tmux sessions, windows, and panes, written in Rust. It provides a three-column tree, a pane preview, configurable key bindings, and themes, with picked tmux targets intended for shell scripting.

## TODO: commands

## Configuration

The configuration directory is the first available of:

1. `$PANE_MUX_CONFIG_HOME`, if absolute.
2. `$XDG_CONFIG_HOME/pane-mux`, if `XDG_CONFIG_HOME` is absolute.
3. `$HOME/.config/pane-mux` (also on macOS).

Relative overrides are ignored. If no directory can be resolved, built-in defaults are used.

The directory may contain `config.toml`, `keymap.toml`, and `theme.toml`. Each file is optional; a missing directory is fine. Invalid TOML, unknown fields, invalid commands, and invalid values are errors naming the file. The loader is implemented; command integration is still pending.

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

- `[picker]`: `border`, `hovered`, `hovered_unfocused`, `current`, `header`
- `[preview]`: `border`, `title`
- `[mode]`: `normal`, `input`, `confirm`
- `[status]`: `overall`, `label`, `message`
- `[which]`: `border`, `key`, `desc`, `separator`
- `[confirm]`: `border`, `title`, `body`
- `[input]`: `border`, `title`, `value`
- `[help]`: `border`, `title`, `on`, `run`, `desc`, `hovered`
- `[notify]`: `error`

`[icons]` has `session`, `window`, and `pane` tables with `text` and optional `fg`. Override just `text` to preserve the preset colour. Empty text hides an icon. Default glyphs require a Nerd Font; plain text alternatives work too.

<!-- theme.toml -->
```toml
[preview]
border = { fg = "#82aaff" }

[picker]
hovered = { bold = true, reversed = false }

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
