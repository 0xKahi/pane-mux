# Configuration

See the [README](../README.md#configuration) for where configuration files are loaded from.

## Picker settings

`config.toml` and `theme.toml` recursively merge tables over the defaults. Other values, including arrays, replace defaults; omitted keys retain defaults.

`preview_ratio` is an integer from 1 through 99 (default 50). `preview_position` is `"top"` or `"bottom"` (default `"bottom"`).

<!-- config.toml -->
```toml
[picker]
preview_position = "top"
preview_ratio = 75
```

## Key bindings

`keymap.toml` has four independent layers: `[picker]`, `[input]`, `[confirm]`, and `[help]`. Within each:

- `keymap` replaces the preset bindings for that layer, including when set to `[]`.
- `prepend_keymap` comes before the base and shadows matching base bindings.
- `append_keymap` comes last and cannot override base or prepended bindings.

Binding conflicts compare the first two keys of the sequence (including a missing second key), following yazi's rule. Untouched layers keep their presets. Binding a lone `<Space>` in `[picker]` shadows the default `<Space> /` find sequence.

`on` accepts one key or a list forming a sequence. Keys include literal characters (`j`, `?`), modifiers (`<C-u>`, `<S-Tab>`, `<A-x>`), and named keys (`<Enter>`, `<Esc>`, `<Space>`, `<PageUp>`, `<PageDown>`, `<F1>`). `run` accepts a command string or a list run in order. `desc` is optional help text.

<!-- keymap.toml -->
```toml
[picker]
prepend_keymap = [
    { on = "x", run = "quit", desc = "Quit" },
    { on = ["g", "g"], run = "arrow top", desc = "First item" },
    { on = ["<Space>", "/"], run = "find --smart", desc = "Find" },
    { on = "n", run = "find_arrow", desc = "Next match" },
    { on = "N", run = "find_arrow --previous", desc = "Previous match" },
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
| picker | `escape`, `quit`, `arrow next`, `arrow prev`, `arrow top`, `arrow bot`, `arrow N`, `arrow N%`, `focus prev`, `focus next`, `seek N%`, `pick`, `create`, `rename`, `remove`, `find [--previous] [--smart|--insensitive]`, `find_arrow [--previous]`, `help` |
| input | `close`, `close --submit`, `backspace`, `kill-word`, `kill-bol` |
| confirm | `close`, `close --submit` |
| help | `arrow` with the same arguments as above, `close` |

`N` is a signed integer: for example, `arrow -3`, `arrow -100%`, or `seek 50%`. Input character insertion is implicit, not a run command. The parser recognises commands globally; use them in the appropriate layer.

## Find

Press `<Space>` then `/` to open `find --smart`. The query is a regular expression, not a filter: all rows stay visible. Each edit searches from the hovered row (including it), wraps around, and jumps to the first match. With no match, the cursor stays put. `find --previous` searches backward. After closing the prompt, `n` (`find_arrow`) and `N` (`find_arrow --previous`) jump to the next or previous matching row, skipping the hovered row and wrapping. Without an active finder, they do nothing.

`find` without a case flag is case-sensitive. `--smart` ignores case unless the raw query contains an uppercase character; regex escapes such as `\S` count as uppercase too. `--insensitive` always ignores case. Flags may appear in any order; if both case flags are supplied, the last one wins. Invalid regexes are silently ignored, leaving the cursor and previous finder unchanged. An empty query clears the finder.

Only names in the focused column are searched: session names, window names, or pane commands, never indexes, icons, or markers. The finder follows focus to another column and survives tree refreshes. The first match in each matching row is highlighted; a matching hovered row shows `[X/Y]`, its 1-based position among the matches and the total match count. Other columns show no find highlights.

`<Enter>` and `<Esc>` close the find prompt but keep the finder and the jumped cursor; neither restores the original cursor. In the picker, `<Esc>` clears the finder and its highlights and badge. The picker `escape` action dismisses which-key, a popup, the finder, then a message, stopping at the first that applies.

## Theme

Styles accept optional `fg`, `bg`, `bold`, `italic`, `underline`, `reversed`, and `dim` fields. Colours use ratatui names (such as `green`, `lightcyan`, `darkgray`), `"#rrggbb"`, or `"reset"`. Modifiers are booleans; explicit `false` removes the modifier. Each style table merges independently with the preset.

Theme sections and style keys:

- `[picker]`: `border`, `header`, `session`, `window`, `pane` (row text styles), `find_keyword`, `find_position`
- `[indicator]`: `parent`, `current`, `preview`, `padding = { open, close }`, `active = { text, fg, ... }`
- `[preview]`: `border`, `title`
- `[mode]`: `normal`, `input`, `confirm`
- `[status]`: `overall`, `label`, `message`
- `[which]`: `border`, `key`, `desc`, `separator`
- `[confirm]`: `border`, `title`, `body`
- `[input]`: `border`, `title`, `value`, `selected`
- `[help]`: `border`, `title`, `on`, `run`, `desc`, `hovered`
- `[notify]`: `error`

`[icons]` has `session`, `window`, and `pane` tables with `text` and optional `fg`. Override just `text` to preserve the preset colour. Empty text hides an icon. Default glyphs require a Nerd Font; plain text alternatives work too.

`[indicator] padding` sets the hovered-row caps (defaults `"\uE0B6"` and `"\uE0B4"`, matching yazi). Either side can be overridden independently. Caps use the highlight colour against the terminal background; styles without a background or reversal use spaces instead. `active` sets the current-item marker (default `*`, magenta) and accepts the same style fields listed above. Non-current rows reserve the marker's display width. Names are not restyled for current items.

`[picker] session`, `window`, and `pane` set the base row text styles. Session names default to `#82AAFF`, window names to `lightcyan` (matching their default icons), and pane names to `white` (an explicit colour keeps the hover underline one colour). Unhovered icons and markers retain their own colours.

`[input]` defaults are yellow `border`, red `title`, white `value`, and reversed `selected` (the text cursor). `[picker] find_keyword` styles matched text over the row style (default yellow, bold, italic, underline); `find_position` styles the `[X/Y]` badge (default magenta on `reset`, bold, italic).

Hovered rows patch an `[indicator]` style over the base style: columns left of focus use `parent` (default reversed), the focused column uses `current` (default reversed), and columns right of focus use `preview` (default underline). Marker, icon, name, and trailing fill inherit this uniform style across the entire inner width. Underlined rows have spaces instead of caps, including underlined edge spaces. Reversed caps use the patched foreground colour as their highlight colour. A default row reads `cap + space + marker + space + icon + space + name + fill + cap`; unhovered caps are spaces.

<!-- theme.toml -->
```toml
[preview]
border = { fg = "#82aaff" }

[picker]
session = { fg = "#82AAFF" }
window = { fg = "lightcyan" }
pane = { fg = "white" }
find_keyword = { fg = "yellow", bold = true, italic = true, underline = true }
find_position = { fg = "magenta", bg = "reset", bold = true, italic = true }

[input]
border = { fg = "yellow" }
title = { fg = "red" }
value = { fg = "white" }
selected = { reversed = true }

[indicator]
current = { bold = true, reversed = false, bg = "blue" }
preview = { underline = true }
padding = { open = "<", close = ">" }
active = { text = "*", fg = "magenta" }

[icons]
session = { text = "S" }
window = { text = "W" }
pane = { text = "P" }
```

## Full defaults

The complete, authoritative defaults are:

- [config-default.toml](../src/config/preset/config-default.toml)
- [keymap-default.toml](../src/config/preset/keymap-default.toml)
- [theme-default.toml](../src/config/preset/theme-default.toml)

Debug builds read these preset files from the source tree at runtime. Release builds embed them in the binary, so the source tree is not needed.
