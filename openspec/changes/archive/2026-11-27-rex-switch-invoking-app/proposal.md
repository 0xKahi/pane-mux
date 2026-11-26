# Proposal

## Why

When more than one Rex app is connected to the same Rex server, `pane-mux switch-session` can switch the wrong device's screen. For example, jojo-mac and shuffle-mac-m4 are both connected to shuffle-mac-m4's server, and you open the picker from jojo-mac. pane-mux picks the app client by "attached to the session being left, else the first app". Every app attaches to every session, so that rule always picks the first connected app. Rex never sets `REX_CLIENT` in terminals, so the explicit override never applies either. The visible result: the switch happens on the other machine and your own view stays where it was.

## What Changes

- `switch-session` on Rex picks the app to switch in this order:
  1. `REX_CLIENT` when set (unchanged).
  2. The connected app client that owns the block pane-mux runs in (`REX_BLOCK`). Rex makes the client that created a block its owner, so a picker block opened by an app's key binding is owned by that app.
  3. The single remaining candidate: the only connected app attached to the session being left, or, when none is attached, the only connected app.
- When those steps leave more than one candidate, pane-mux SHALL NOT switch any app. It still focuses the picked window or block inside its session, prints a hint saying it could not tell which Rex app to switch, and exits 0. Previously it picked the first app, which could be another person's or another device's screen.
- The Rex snapshot also reads each block's `owner`, so finding the owner costs no extra `rex` invocation.
- README and usage docs explain switching when several devices are connected, and that `REX_CLIENT` (client ID or label) forces a choice.

No breaking changes for a single connected app: it is still the only candidate and is still switched.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `rex-backend`: "Rex switch-session" defines which app's view changes (the app that owns the picker's block, with the order above). "Remote Control fallback" adds the ambiguous-app case: focus inside the session, print a hint naming the target session, change no app's view, exit 0.

## Impact

- `src/rex/snapshot.lua`, `src/rex/snapshot.rs`: carry the block `owner` and record the owner of the `REX_BLOCK` block.
- `src/rex/client.rs`: `select_app` uses the new order and the ambiguity refusal. The hint text distinguishes "could not tell which app" from "Remote Control off / no app".
- `src/rex/tests.rs`, `src/rex/fixtures/`: snapshot fixtures with `owner`, plus multi-app `client ls` cases.
- `README.md`, `docs/usage-examples.md`: multi-device note.
- No new dependencies. No change to tmux, to `picker` output, or to recency history.
