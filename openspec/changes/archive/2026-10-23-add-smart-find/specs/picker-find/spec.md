# Spec Delta

## Purpose

Lets users jump the picker's cursor to rows whose names match a query, yazi `find` style, without filtering any rows out of the columns.

## ADDED Requirements

### Requirement: Find prompt
The `find` action SHALL open a single-line input prompt titled `Find next:`, or `Find previous:` with `--previous`. The prompt SHALL use the input keymap layer. Closing it with `<Enter>` or `<Esc>` SHALL keep the finder and SHALL NOT restore the cursor to where it was before the prompt opened.

#### Scenario: Open with the default binding
- **WHEN** the picker is focused on a column and the user presses `<Space>` then `/`
- **THEN** a prompt titled `Find next:` opens

#### Scenario: Cancel keeps the jumped cursor
- **WHEN** the user opens the find prompt with the cursor on `alpha`, types `log` so the cursor jumps to `logs`, and presses `<Esc>`
- **THEN** the prompt closes and the cursor stays on `logs`

### Requirement: Incremental jump while typing
Every edit to the find query SHALL immediately search the focused column and move its cursor to the first match. The search SHALL start at the hovered row and include it, go forward (backward for `find --previous`), and wrap around. When nothing matches, the cursor SHALL NOT move.

#### Scenario: Jump forward with wrap
- **WHEN** the focused sessions column lists `api`, `logs`, `work`, the cursor is on `work`, and the user types `a` in the find prompt
- **THEN** the cursor wraps to `api`

#### Scenario: Hovered row can match
- **WHEN** the cursor is on `logs` and the user types `lo`
- **THEN** the cursor stays on `logs`

#### Scenario: No match
- **WHEN** the user types `zzz` and no row in the focused column matches
- **THEN** the cursor does not move

### Requirement: Match on names in the focused column
The finder SHALL search only the focused column. It SHALL match against the session name, the window name, or the pane command, and never against indexes, icons or markers. When focus moves to another column, the same finder SHALL apply to the newly focused column.

#### Scenario: Window index is not matched
- **WHEN** the windows column lists `1: vim` and `2: logs` and the user finds `1`
- **THEN** neither row matches

#### Scenario: Finder follows focus
- **WHEN** a finder for `log` is active on the windows column and the user moves focus to the sessions column and presses `n`
- **THEN** the cursor moves to the next session whose name matches `log`

### Requirement: Regex query with smart case
The find query SHALL be a regular expression. `--smart` SHALL make matching case-insensitive unless the query contains an uppercase character, `--insensitive` SHALL always ignore case, and without either flag matching SHALL be case-sensitive. An invalid expression SHALL leave the cursor and the previous finder unchanged and SHALL NOT show an error. An empty query SHALL clear the finder.

#### Scenario: Smart case lowercase query
- **WHEN** the user runs `find --smart` and types `work` with a session named `Work`
- **THEN** the cursor moves to `Work`

#### Scenario: Smart case uppercase query
- **WHEN** the user runs `find --smart` and types `Work` with only a session named `work`
- **THEN** nothing matches and the cursor does not move

#### Scenario: Regex query
- **WHEN** the user types `^d.t` with sessions `api` and `dotfiles`
- **THEN** the cursor moves to `dotfiles`

#### Scenario: Incomplete regex
- **WHEN** the user has typed `lo` (cursor on `logs`) and then types `(`
- **THEN** the cursor stays on `logs` and no error is shown

### Requirement: Next and previous match
`find_arrow` SHALL move the cursor to the next row after the hovered row that matches the active finder. `find_arrow --previous` SHALL move it to the previous one. Both SHALL wrap around and SHALL do nothing when no finder is active or nothing else matches.

#### Scenario: Next match
- **WHEN** the finder `log` is active, the column lists `logs`, `api`, `blog`, the cursor is on `logs`, and the user presses `n`
- **THEN** the cursor moves to `blog`

#### Scenario: Previous match wraps
- **WHEN** the same finder is active with the cursor on `logs` and the user presses `N`
- **THEN** the cursor moves to `blog`

#### Scenario: No finder
- **WHEN** no finder is active and the user presses `n`
- **THEN** nothing happens

### Requirement: Escape clears the finder
When a finder is active, the picker's `escape` action SHALL clear it, removing its highlights and position badge, and do nothing else. A later `escape` keeps its existing dismiss behaviour. The finder SHALL survive live refreshes of the tmux tree.

#### Scenario: Escape clears
- **WHEN** a finder is active and the user presses `<Esc>` in the picker
- **THEN** match highlights disappear and `n` no longer moves the cursor

#### Scenario: Escape with a message shown
- **WHEN** a finder is active, an error message is shown, and the user presses `<Esc>` twice
- **THEN** the first press clears the finder and the second clears the message

#### Scenario: Survives refresh
- **WHEN** a finder is active and the tmux tree refreshes
- **THEN** `n` still jumps to matches of the same query

### Requirement: Match highlighting and position badge
While a finder is active, the first match in each matching row of the focused column SHALL be drawn with `[picker] find_keyword` over the row style. When the hovered row matches, an `[X/Y]` badge SHALL follow its name, drawn with `[picker] find_position`. X is the row's 1-based position among the matches and Y is the total number of matches. Other columns SHALL show no highlights.

#### Scenario: Highlight and badge
- **WHEN** the finder `log` is active on a column listing `logs`, `api`, `blog` with the cursor on `blog`
- **THEN** `log` is highlighted in `logs` and `blog`, and the `blog` row shows `[2/2]`

#### Scenario: Unfocused columns
- **WHEN** a finder is active and the panes column contains a matching command but is not focused
- **THEN** the panes column shows no highlights

### Requirement: Default find bindings
The default picker keymap SHALL bind `<Space> /` to `find --smart`, `n` to `find_arrow` and `N` to `find_arrow --previous`.

#### Scenario: Default next binding
- **WHEN** no user keymap is set, a finder is active and the user presses `n`
- **THEN** `find_arrow` runs
