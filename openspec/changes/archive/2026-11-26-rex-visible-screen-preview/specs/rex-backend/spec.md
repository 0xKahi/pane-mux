# Spec Delta

## MODIFIED Requirements

### Requirement: Rex preview and self-block suppression
On the Rex backend the preview SHALL show the previewed block's visible contents with colours. Visible contents are the rows currently on the block's screen, one preview line per screen row, and SHALL NOT include the block's scrollback history. When the previewed block is the block the picker runs in, which can only happen when that block is tiled, the preview SHALL show the self-pane placeholder instead.

#### Scenario: Colours preserved
- **WHEN** the previewed block shows coloured `ls` output
- **THEN** the preview renders those colours

#### Scenario: Own block
- **WHEN** the picker runs in tiled block `B` and `B` is the previewed block
- **THEN** the preview shows the placeholder message

#### Scenario: Long history shows only the screen
- **WHEN** the previewed block is 40 rows tall, has printed 2,000 numbered lines, and its screen shows lines 1961 to 2000
- **THEN** the preview's first line is line 1961 and its last line is line 2000
- **AND** no line before 1961 appears in the preview

#### Scenario: Full-screen program
- **WHEN** the previewed block is running a full-screen program, such as an editor on the alternate screen
- **THEN** the preview shows that program's screen and none of the shell history behind it

#### Scenario: Wrapped line
- **WHEN** a line on the previewed block's screen is wider than the block and wraps onto a second screen row
- **THEN** the preview shows it as two lines, the same as the screen

#### Scenario: Partly filled screen
- **WHEN** the previewed block has no scrollback and only its first 3 screen rows hold text
- **THEN** the preview shows those 3 lines

## ADDED Requirements

### Requirement: Rex preview capture cost bounded
On the Rex backend, capturing a preview SHALL use a single `rex` invocation and SHALL transfer at most a fixed amount of block content to pane-mux. That amount SHALL NOT depend on the size of the block's scrollback, so a block with a long history previews about as quickly as one without.

#### Scenario: Large scrollback
- **WHEN** the previewed block holds 10 MB of scrollback
- **THEN** pane-mux runs one `rex` command to capture the preview
- **AND** that command's output to pane-mux is no larger than for a block whose history fits within that fixed amount

#### Scenario: Small block
- **WHEN** the previewed block holds 700 bytes of output
- **THEN** pane-mux runs one `rex` command to capture the preview and shows all of it
