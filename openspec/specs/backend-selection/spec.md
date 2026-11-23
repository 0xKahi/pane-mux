# backend-selection Specification

## Purpose

Defines how pane-mux decides whether to drive tmux or Rex for a given invocation, how users override that choice, and which commands each backend supports.

## Requirements

### Requirement: Automatic backend detection
pane-mux SHALL choose its backend from the environment in this order: tmux when `TMUX` is set, otherwise Rex when `REX_BLOCK` is set and not blank, otherwise tmux. tmux SHALL win when both are present, because any process started from a Rex block inherits `REX_BLOCK`, including a tmux server running inside it.

#### Scenario: Inside tmux
- **WHEN** pane-mux runs with `TMUX` set and no `REX_BLOCK`
- **THEN** it uses the tmux backend

#### Scenario: Inside a Rex block
- **WHEN** pane-mux runs with `REX_BLOCK=block:01a2…` set and no `TMUX`
- **THEN** it uses the Rex backend

#### Scenario: tmux nested in a Rex block
- **WHEN** pane-mux runs in a tmux pane whose tmux server was started from a Rex block, so both `TMUX` and `REX_BLOCK` are set
- **THEN** it uses the tmux backend

#### Scenario: Blank REX_BLOCK
- **WHEN** `REX_BLOCK` is set to an empty string and `TMUX` is unset
- **THEN** it uses the tmux backend

#### Scenario: Neither present
- **WHEN** neither `TMUX` nor `REX_BLOCK` is set
- **THEN** it uses the tmux backend, and tmux-specific errors (such as "no tmux server") are reported as today

### Requirement: Backend override
The `PANE_MUX_BACKEND` environment variable SHALL force the backend when set to `tmux` or `rex`, ignoring automatic detection. An empty value SHALL mean automatic detection. Any other value SHALL make pane-mux print an error naming the accepted values to stderr and exit with code 2, without opening the picker.

#### Scenario: Force Rex
- **WHEN** `PANE_MUX_BACKEND=rex` is set inside a tmux pane
- **THEN** pane-mux uses the Rex backend

#### Scenario: Force tmux
- **WHEN** `PANE_MUX_BACKEND=tmux` is set inside a Rex block
- **THEN** pane-mux uses the tmux backend

#### Scenario: Invalid override
- **WHEN** `PANE_MUX_BACKEND=zellij` is set
- **THEN** stderr says the value must be `tmux` or `rex` and the exit code is 2

### Requirement: Float commands are tmux-only
`pane-mux float stash`, `float unstash` and `float toggle` SHALL run only on the tmux backend. On the Rex backend they SHALL print an error to stderr saying floating panes are managed by Rex's built-in minimize, and exit with code 2 without changing anything.

#### Scenario: Float under Rex
- **WHEN** the user runs `pane-mux float toggle` and the Rex backend is active
- **THEN** stderr explains that `float` is tmux-only and Rex's minimize should be used instead, nothing changes in Rex, and the exit code is 2

#### Scenario: Float under tmux
- **WHEN** the user runs `pane-mux float toggle` and the tmux backend is active
- **THEN** it behaves as specified by `floating-pane-stash`
