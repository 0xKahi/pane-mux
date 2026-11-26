# Design

## Context

`RexCli::select_app` (`src/rex/client.rs`) picks the app client that `rex -C <id> do session.select` targets. With `REX_CLIENT` unset, which is always the case because Rex never sets it in terminals, it runs `rex client ls --json` and keeps connected clients with `info.kind == "app"`. It prefers one whose `session_ids` contains the session being left, and otherwise takes the first. A Rex app attaches to every session on a server it is connected to, so with two apps on one server the "attached to the left session" test matches both, and the first-connected app wins. See proposal.md (Why).

Rex records an `owner` client per block in `session.view` / `session.attach` views. Per the Rex API docs, a block created by a client attached to its session is owned by that client. Ownership can move later: `session.own_block` with mode `take`, or the owner disconnecting and the block passing to the next client in line. On a live server the owner of a tiled block opened from `jojo-mac` was `jojo-mac`'s app client, and a block opened from the other device was owned by that device's app.

`switch` always loads the tree (snapshot Lua script, one `rex do -e` call) before calling `select_app`, so data from the snapshot is free to use.

## Goals / Non-Goals

**Goals:**
- Switch the view of the device the user invoked pane-mux from, using no extra `rex` invocation beyond today's `client ls`.
- Never switch a screen pane-mux cannot attribute to the user. Ambiguity ends in the hint path (spec: Ambiguous Rex app fallback).

**Non-Goals:**
- Listing or switching sessions across several Rex servers or hosts. That is a separate future change.
- Changing the shipped `init.lua` binding examples to pass a client ID (see Decisions, alternatives).
- Claiming block ownership (`session.own_block`) from pane-mux.

## Decisions

### 1. Read the owner in the existing snapshot script
Add `owner` to each block entry in `src/rex/snapshot.lua` (omitted when absent, like `process_name`). Deserialize it as `Option<String>` in `snapshot.rs`. While building the tree, keep the owner of the block whose ID equals `REX_BLOCK`, wherever it is listed, including floating and minimized layers. Store it on `RexCli` next to `last_current` (for example `self_owner: RefCell<Option<String>>`, refreshed on every `tree()`).

- Keep it out of the shared `mux::Current`, which is backend-neutral and also used by tmux.
- Alternative: a separate `rex block inspect "$REX_BLOCK"` or `session.view` call at switch time. Rejected because it adds a `rex` round trip, which costs real time over Tailscale, for data the snapshot already returns.

### 2. Selection order, with the owner checked against `client ls`
`select_app` resolves in this order:
1. `REX_CLIENT` set → use it directly, with no `client ls` call (unchanged).
2. Run `client ls --json` and take the connected `kind == "app"` clients. If the self-block owner's client ID is among them → use it.
3. Candidates = connected apps whose `session_ids` contain the session being left. If that set is empty, use all connected apps instead.
   - Exactly one candidate → use it.
   - Zero candidates → the existing "no connected Rex app client" error (Remote Control hint).
   - More than one → a distinct ambiguity error.

The owner has to be checked against the app list. The snapshot runs `session.attach` on the CLI's own connection, so an unowned block could be claimed by a non-app CLI connection. A disconnected owner also must not be targeted. When the session being left is unknown (no `REX_BLOCK` or block not found), step 3 starts from all connected apps.

- Alternative: keep "first app" as the last resort. Rejected by the user, because switching another device's screen is worse than not switching.

### 3. A distinct hint for ambiguity
`switch` currently maps every `select_app` error to the Remote Control hint. Return a typed or tagged error for ambiguity and format a different hint, for example:

`Could not tell which Rex app should show session "sidecar": 2 apps are connected. Set REX_CLIENT to an app's label or client ID to choose one.`

`REX_CLIENT` is worth mentioning because `rex -C` accepts a client label (for example `jojo-mac`), so it is usable by hand. The Remote Control hint stays for the zero-app and refused-select cases. In both cases focus inside the session still happens first and history is still recorded (existing code order in `switch`), and the exit code stays 0.

### Alternatives considered for identifying the device
- **Pass the client from the key binding** (`env REX_CLIENT=<ctx client>` in `init.lua`). Would be exact, but it is not confirmed that the binding context exposes a client ID, and it requires every user to edit their config. It could be added later on top of this change, because `REX_CLIENT` already wins.
- **Most recent input or focus per client.** `client ls` exposes no such field, and `focus_history.reverse` is per principal, not per client, and it mutates.

## Risks / Trade-offs

- **[Ownership moved to another device]** If the other device took ownership of the block (`take`), or the user runs `switch-session` by hand in an old tiled block the other device created, that device's view changes. → Documented. The usual path (a fresh floating block from the key binding) is owned by the invoking app. `REX_CLIENT` overrides.
- **[Key-binding blocks may not be owned by the invoking app]** If Rex runs `init.lua` bindings so that `rex.session.new_layer` is not attributed to the app, the picker block has no app owner and the user lands in the ambiguity hint whenever two apps are connected. → A task verifies this on a real two-device setup before release. The fallback is safe (nothing is wrongly switched).
- **[Snapshot JSON grows]** One short string per block. → Negligible.
- **[Behaviour change for multi-app users without owner data]** Users who relied on "first app" now get a hint instead. → Intended. The hint names `REX_CLIENT`.

## Migration Plan

No migration. Single-app setups behave as before. Rollback is reverting the change.
