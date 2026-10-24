# pane-mux

<p align="center">
  <a href="#installation">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#configuration">Configuration</a> ·
  <a href="#commands">Commands</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

A yazi-style terminal picker for tmux sessions, windows, and panes, written in Rust. It provides a three-column tree, a pane preview, configurable key bindings, and themes, with picked tmux targets intended for shell scripting.

![pane-mux demo](assets/pmux-demo.gif)

## Installation

Install from crates.io:

```sh
cargo install pane-mux --locked
```

Or build from a source checkout:

```sh
cargo install --path . --locked
```

## Quick start

Add this binding to `~/.tmux.conf` to open the session switcher in a popup:

```tmux
bind o popup -w 75% -h 75% -x C -y C -E 'pane-mux switch-session'
```

Reload tmux (`tmux source-file ~/.tmux.conf`), then press `<prefix> o`.

See [docs/usage-examples.md](docs/usage-examples.md) for more examples.

## Configuration

The configuration directory is the first available of:

1. `$PANE_MUX_CONFIG_HOME`, if absolute.
2. `$XDG_CONFIG_HOME/pane-mux`, if `XDG_CONFIG_HOME` is absolute.
3. `$HOME/.config/pane-mux` (also on macOS).

Relative overrides are ignored. If no directory can be resolved, built-in defaults are used.

The directory may contain `config.toml`, `keymap.toml`, and `theme.toml`. Each file is optional; a missing directory is fine. Unknown keys and sections are ignored. Invalid TOML, wrong types, invalid values, unknown key names in `on`, and unknown commands in `run` are errors naming the file; pane-mux then exits with code 2.

For editor validation with Tombi or taplo, put the matching line at the top of each file:

<!-- config.toml -->
```toml
#:schema https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/config.json
```

<!-- keymap.toml -->
```toml
#:schema https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/keymap.json
```

<!-- theme.toml -->
```toml
#:schema https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/theme.json
```

The schemas flag unknown keys and sections even though pane-mux ignores them. For contributors, `just schemas` regenerates `schemas/`; a test fails when the committed schemas are stale.

See [docs/configuration.md](docs/configuration.md) for picker settings, key bindings, find, theme, and the full defaults.

## Commands

- [`picker`](#pane-mux-picker---session--window--pane) - Pick a tmux target and print it to stdout
- [`switch-session`](#pane-mux-switch-session) - Pick a tmux target and switch the current client to it
- [`move-pane`](#pane-mux-move-pane-target) - Move a pane to a picked destination

For command help, run `pane-mux --help` or `pane-mux picker --help`.

### `pane-mux picker [--session|--window|--pane]`

Prints exactly one tmux target after closing the UI: a session name,
`session:window_index`, or `session:window_index.pane_index`. Without a level
flag, the focused column determines the output. The UI uses stderr (or
`/dev/tty` when stderr is redirected), leaving stdout available for scripts.

Sessions are listed by name. Level flags are mutually exclusive:
`pane-mux picker --session --pane` prints a usage error without opening the UI.

With the panes column focused, `x` cuts the hovered pane; `x` again clears it, or cuts a different pane instead. Press `p` to paste:

- Sessions column: prompt for a new session name, then its window name.
- Windows column: prompt for a new window name in the hovered session.
- Panes column: float the pane in the hovered window using `[floating-pane]` (requires tmux 3.8).

Moves are detached. After a successful paste, the picker stays open, clears the cut buffer and selects the result. See [picker actions](docs/picker-actions.md#cut-and-paste) for restrictions and the last-pane edge case.

#### Options

- `--session`: Print the picked session name.
- `--window`: Print the picked window as `session:window_index`.
- `--pane`: Print the picked pane as `session:window_index.pane_index`.

#### Examples

```sh
# Capture a session and switch to it from a shell inside tmux
selected=$(pane-mux picker --session) && tmux switch-client -t "$selected"
```

### `pane-mux switch-session`

Requires tmux and switches the current client to the picked target, including
its window and pane when selected. It prints no target.

Sessions are listed current first, then most recently attached on the tmux
server. Ties are ordered by name; never-attached sessions come last, also by
name. The cursor opens on the session below the current one (the most recently
used other session), so `<Enter>` right away switches back to it; with no other
session it opens on the current one. The cursor stays on its session when the
list refreshes. Windows and panes keep their existing order.

#### Examples

```tmux
# ~/.tmux.conf
bind o popup -w 75% -h 75% -x C -y C -E 'pane-mux switch-session'
```

### `pane-mux move-pane [TARGET]`

Opens the picker with `[Move]` and the specified pane already cut and locked. `TARGET` accepts tmux pane targets such as `work:1.2` or `%17`; omitting it uses the current pane and requires running inside tmux. An unknown target or no target outside tmux is an error (exit 2).

Use `p` with the same destinations as picker cut/paste above. A successful paste closes the picker and exits 0 without printing a target; `q` without pasting exits 130. Cancelled prompts, same-window paste and failed moves keep the picker open.

#### Examples

Prefer a popup so the pane running the picker is not itself moved:

```tmux
# ~/.tmux.conf
bind-key M display-popup -E "pane-mux move-pane"
```

Reload tmux (`tmux source-file ~/.tmux.conf`), then press `<prefix> M`.

```sh
pane-mux move-pane work:1.2
```

Moves are detached, not followed by a client switch. Empty source windows and sessions close normally: moving the last pane of the attached session destroys it, and with `detach-on-destroy on` the client detaches and the popup closes. Floating paste requires tmux 3.8; other paste destinations do not.

### Exit codes

| Exit code | Meaning |
| --- | --- |
| 0 | Picked successfully (and switched for `switch-session`), or moved with `move-pane` |
| 130 | Cancelled without picking or pasting in `move-pane` |
| 2 | Error or invalid command-line usage |


## Releasing

Releases are automated with [release-plz](https://release-plz.dev):

1. Squash-merge pull requests into `main` with a
   [conventional commit](https://www.conventionalcommits.org) title such as
   `feat: add pane filter` or `fix(picker): keep cursor on refresh`. The PR
   title check enforces this; use `!` (for example `feat!:`) for breaking changes.
2. On each push to `main`, release-plz opens or updates a release PR that bumps
   the version in `Cargo.toml`/`Cargo.lock` and updates `CHANGELOG.md`.
3. Merging the release PR publishes the new version to crates.io, tags it
   `vX.Y.Z`, and creates a GitHub Release.
