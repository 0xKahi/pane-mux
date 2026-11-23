# Spec Delta

## MODIFIED Requirements

### Requirement: Move-pane command
`pane-mux move-pane [TARGET]` SHALL resolve `TARGET` to a pane, or use the invoking client's current pane when `TARGET` is omitted, and open the picker with that pane cut and the header `[Move]`, with the sessions column focused and the cursor on the session that holds the cut pane. On the tmux backend `TARGET` is any tmux pane target (e.g. `test:1.2` or `%17`) and the current pane is the tmux client's. On the Rex backend `TARGET` is a Rex block ID or ID suffix and the current pane is the block named by `REX_BLOCK`. An unresolvable target, or an omitted target outside tmux and outside a Rex block, SHALL print an error to stderr and exit with code 2 without opening the picker.

#### Scenario: Cursor starts on the source session
- **WHEN** the user runs `pane-mux move-pane notes:2.1` and sessions are listed `alpha`, `notes`, `work`
- **THEN** the sessions column is focused with the cursor on `notes`

#### Scenario: Explicit target
- **WHEN** the user runs `pane-mux move-pane test:1.2`
- **THEN** the picker opens with header `[Move]` and `test:1.2` cut

#### Scenario: Current pane
- **WHEN** the user runs `pane-mux move-pane` from a popup opened on pane `work:0.1`
- **THEN** the picker opens with `work:0.1` cut

#### Scenario: Unknown target
- **WHEN** the user runs `pane-mux move-pane nosuch:9.9`
- **THEN** stderr explains the target could not be found and the exit code is 2

#### Scenario: Outside tmux without a target
- **WHEN** the user runs `pane-mux move-pane` with no `TMUX` and no `REX_BLOCK` environment
- **THEN** stderr explains that a target is required outside tmux or Rex and the exit code is 2

#### Scenario: Rex block target
- **WHEN** the Rex backend is active and the user runs `pane-mux move-pane 6e22b7`
- **THEN** the picker opens with header `[Move]` and the block whose ID ends in `6e22b7` cut

#### Scenario: Rex current block
- **WHEN** the Rex backend is active and the user runs `pane-mux move-pane` with no target
- **THEN** the picker opens with the block named by `REX_BLOCK` cut
