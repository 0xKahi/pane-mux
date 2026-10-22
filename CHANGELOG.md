# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0]

### Added

- `pane-mux picker`: a yazi-style three-column picker for tmux sessions, windows
  and panes with a pane preview. It prints exactly one tmux target to stdout and
  supports `--session`, `--window` and `--pane` level flags.
- Find action (`<Space> /`): regex search over the focused column with smart,
  case-sensitive and case-insensitive modes, match highlighting, an `[X/Y]`
  position badge, and `n`/`N` to jump between matches.
- `pane-mux switch-session`: switches the current tmux client to the picked
  session, window or pane. Sessions are ordered current first, then by most
  recent attach.
- Configuration through `config.toml` in `$PANE_MUX_CONFIG_HOME`,
  `$XDG_CONFIG_HOME/pane-mux` or `~/.config/pane-mux`, with preview position and
  ratio settings.
- Configurable key bindings in `keymap.toml` with `keymap`, `prepend_keymap` and
  `append_keymap` layers for the picker, input, confirm and help modes.
- Themes in `theme.toml` covering rows, indicators, icons, preview, status,
  which-key, input, confirm, help and find highlights.

[Unreleased]: https://github.com/0xKahi/pane-mux/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/0xKahi/pane-mux/releases/tag/v0.1.0
