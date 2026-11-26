# Spec Delta

## MODIFIED Requirements

### Requirement: Rex switch-session
On the Rex backend, `pane-mux switch-session` SHALL, on `<Enter>`, make the Rex app chosen by Rex switch-session app selection show the picked session and focus the picked window or block at the focused column's level, then exit with code 0.

#### Scenario: Switch to a block in another session
- **WHEN** the user runs `pane-mux switch-session` under Rex and picks block `B` in window `W` of session `sidecar`
- **THEN** the Rex app shows session `sidecar` with window `W` active and block `B` focused, and the exit code is 0

## ADDED Requirements

### Requirement: Rex switch-session app selection
Under `switch-session` on Rex, the app whose view changes SHALL be chosen in this order: the client named by `REX_CLIENT` when set; otherwise the connected app client that owns the block named by `REX_BLOCK`; otherwise the only connected app client attached to the session being left, or, when no connected app client is attached to it, the only connected app client. At most one app's view SHALL change.

#### Scenario: Two devices connected, picker opened from one
- **GIVEN** apps `jojo-mac` and `shuffle-mac-m4` are both connected to the Rex server and attached to the same sessions
- **AND** pane-mux runs in a block owned by `jojo-mac`
- **WHEN** the user picks session `sidecar` under `switch-session`
- **THEN** `jojo-mac` shows session `sidecar` and the view of `shuffle-mac-m4` is unchanged

#### Scenario: REX_CLIENT overrides the block owner
- **GIVEN** `REX_CLIENT` names app `shuffle-mac-m4` and pane-mux runs in a block owned by `jojo-mac`
- **WHEN** the user picks session `sidecar` under `switch-session`
- **THEN** `shuffle-mac-m4` shows session `sidecar` and the view of `jojo-mac` is unchanged

#### Scenario: Block owner is not a connected app
- **GIVEN** the block pane-mux runs in has no owner, or its owner is not a connected app client
- **AND** exactly one connected app client is attached to the session being left
- **WHEN** the user picks session `sidecar` under `switch-session`
- **THEN** that app shows session `sidecar`

#### Scenario: Single connected app
- **GIVEN** exactly one app client is connected and the block pane-mux runs in has no owner
- **WHEN** the user picks session `sidecar` under `switch-session`
- **THEN** that app shows session `sidecar`

### Requirement: Ambiguous Rex app fallback
When `switch-session` on Rex cannot choose an app because more than one connected app client is still a candidate, it SHALL change no app's view. It SHALL still focus the picked window or block inside its session and record the switch in the recency history. It SHALL print to stderr a hint that names the target session, says pane-mux could not tell which Rex app to switch, and says `REX_CLIENT` can name one. It SHALL exit with code 0.

#### Scenario: Two candidate apps and no usable owner
- **GIVEN** apps `jojo-mac` and `shuffle-mac-m4` are connected and both attached to the session being left
- **AND** `REX_CLIENT` is unset and the block pane-mux runs in is not owned by a connected app client
- **WHEN** the user picks window `W` of session `sidecar` under `switch-session`
- **THEN** window `W` becomes the active window of `sidecar`, neither app's view changes, stderr names `sidecar` and mentions `REX_CLIENT`, the switch is recorded in the recency history, and the exit code is 0
