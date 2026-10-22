# Spec Delta

## ADDED Requirements

### Requirement: Sessions ordered by name
Unless a command specifies another session order, the picker's sessions column SHALL list sessions in tmux's session order, which is by name. The cursor SHALL open on the first row. Windows SHALL be listed by window index and panes by pane index.

#### Scenario: Plain picker order
- **WHEN** sessions `work`, `dotfiles` and `kahi` exist, the client is on `work`, and the user runs `pane-mux picker`
- **THEN** the sessions column lists `dotfiles`, `kahi`, `work` in that order, with the cursor on `dotfiles`

#### Scenario: Recency does not affect the plain picker
- **WHEN** the client most recently switched from `work` to `kahi` and the user runs `pane-mux picker`
- **THEN** the sessions column still lists sessions by name
