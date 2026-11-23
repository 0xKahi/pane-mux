# Proposal

## Why

Rex is a terminal app with the same session > window (tab) > pane (block) structure as tmux and a scriptable `rex` CLI. Users who live in Rex can't use pane-mux today, because every command assumes a tmux server. With the backend seam from `backend-neutral-core` in place, pane-mux can drive Rex for its picker, session switching and pane moving. Rex's native minimize already covers what `pane-mux float` does in tmux.

## What Changes

- **Backend selection:** pane-mux picks tmux or Rex automatically from the environment (`PANE_MUX_BACKEND` override, then `$TMUX`, then a non-empty `$REX_BLOCK`, otherwise tmux), so tmux nested inside a Rex block still resolves to tmux.
- **Rex backend for `picker`:** lists Rex sessions, windows and blocks (tiled and floating), previews block output with colours, suppresses the picker's own block, and supports create, rename and delete. `<Enter>` prints the picked item's full Rex ID (`session:…`, `window:…`, `block:…`), which every `rex` command accepts.
- **Rex backend for `switch-session`:** switches the Rex app to the picked session, window or block. When the app refuses because Remote Control is off, it prints a hint naming the target instead of failing. Sessions are ranked current first, then by pane-mux's own most-recently-switched list, then in Rex's session order. `exclude_from_recency` still applies.
- **Rex backend for `move-pane` and picker cut/paste:** moves within a session (into a new window, or into an existing window as a floating pane sized by `[floating-pane]`). Any paste that would cross sessions, including "paste into a new session", fails with `rex does not allow moving panes to different session`, because Rex cannot move blocks between sessions.
- `pane-mux float stash|unstash|toggle` stays tmux-only. Under Rex it exits 2 and points to Rex's built-in minimize.
- Copy-mode and stashed-float indicators are not shown under Rex, because Rex exposes no such state.

## Capabilities

### New Capabilities
- `backend-selection`: how pane-mux chooses between tmux and Rex, the override variable, nested-multiplexer behavior, and which commands each backend supports.
- `rex-backend`: how the picker, `switch-session` and `move-pane` behave against Rex: tree mapping, pick output, switching and its Remote Control fallback, recency ranking, same-session moves, the cross-session error, and the indicators Rex doesn't support.

### Modified Capabilities
- `tmux-picker`: "Pick and print target" now prints the active backend's target format (unchanged for tmux).
- `session-switching`: "Requires tmux client" becomes "requires a supported multiplexer client", and "Sessions ordered by recency" is scoped to tmux, with Rex ranking defined in `rex-backend`.
- `pane-moving`: "Move-pane command" accepts Rex block targets and uses `$REX_BLOCK` as the current pane under Rex.

## Impact

- Depends on `backend-neutral-core` (the core trait, backend-owned targets, single construction point).
- New code: a Rex backend module implementing the core trait by running the `rex` CLI (JSON output, `-S $REX_SERVER`, `--autostart=false`), a test fake, and a small state file for switch recency.
- External: requires a Rex install with a `rex` CLI new enough to provide `session inspect --json`, `capture --format vt` and the `session.*` API methods (developed against Rex 0.1.0-preview.21, API revision 18). Switching the app view needs Rex's **Remote Control** setting turned on.
- Docs: README and `docs/usage-examples.md` gain a Rex section (launching pane-mux from a Rex key binding or custom action in a floating layer).
- Tests: a new opt-in Rex integration tier (`PANE_MUX_REX_TESTS=1`) alongside the tmux tier.
