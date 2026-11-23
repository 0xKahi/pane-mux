# Design

## Context

See proposal.md for motivation and the specs for behavior. This change builds on `backend-neutral-core`: a `Mux` core trait, a tmux-only `FloatMux`, backend-owned `target_string` / `display_target`, `in_client()` / `suppressed_pane()`, and a single `mux::open()` factory.

Rex facts this design relies on, checked against Rex 0.1.0-preview.21 (API revision 18) with read-only commands:

- IDs are typed UUIDs: `session:…`, `window:…`, `block:…`. Full IDs and hex suffixes resolve without `-s`. Block labels are not unique (the default is `shell`). There is no positional block target and no `s:w.p` syntax.
- `rex session inspect S --json` returns `active_window_id` and `windows[].{window_id,label,focused_block_id,layers[].{layer_id,kind:"tiled"|"floating",state,bounds,blocks[].{block_id,label,flavor}}}`. `state` is `"minimized"` or null.
- `rex whoami --json` returns `session_id`, `window_id`, `block_id`, `endpoint` and `api_revision`. Environment: `REX_SESSION`, `REX_BLOCK`, `REX_SERVER`, `REX_CLIENT`.
- Block process: `rex block call -s S -b B process` gives `foreground.name`. Preview: `rex capture -b B --format vt --unwrap [--trim]`, where scrollback can be large.
- API (`rex api call [-s S] <method> <json>`): `session.view`, `session.new_window`, `session.move_block` (block placed in any window or detached; anchor must be placed; same session), `session.float_block` (tiled block into a new floating layer at `bounds` `[x,y,w,h]` fractions), `session.focus_block`, `session.focus_window`, `session.close_window`, `block.close`, `session.set_label`, `session.set_window_label`, `session.destroy`.
- Changing what the app shows needs a client action, `rex -C <client> do session.select session_id=… [window_id=…]`, and the app accepts it only when **Remote Control** is on. Server-side `focus_window` does not change the app's view.
- There is no session recency, no copy-mode state and no zoom flag in the JSON. Cross-session block moves don't exist, and upstream feedback marks them deferred.
- workmux (a sibling project) ships a Rex backend. We reuse its lessons: always pass `-S $REX_SERVER`, use `--autostart=false`, strip `REX_SESSION`/`REX_BLOCK` for `rex do` scripts, Lua empty tables serialize as `{}`, and a `rex do` script only sees sessions it has attached to.

## Goals / Non-Goals

**Goals:**
- `picker`, `switch-session` and `move-pane` work in Rex with the behavior in `rex-backend`, while the tmux behavior stays byte-for-byte unchanged.
- The picker feels as responsive on Rex as on tmux, with each 300 ms tick bounded to a couple of process spawns.
- Rex-specific logic lives in the Rex backend module. The picker gains only capability checks.

**Non-Goals:**
- `pane-mux float` on Rex, minimized-float indicators, or a stash concept for Rex.
- Remote Rex hosts beyond honoring `REX_SERVER`.
- A long-running daemon or event follower (recency comes from pane-mux's own history).

## Decisions

### D1. Detection lives in `mux::open()`
`mux::open()` reads `PANE_MUX_BACKEND`, then `TMUX`, then non-blank `REX_BLOCK`, otherwise tmux, and returns an enum `Backend { Tmux(TmuxCli), Rex(RexCli) }` with `as_mux()` and `as_float() -> Option<&dyn FloatMux>`. The `float` subcommand calls `as_float()` and prints the tmux-only error when it gets `None`. Detection is a pure function of an env snapshot, so it is unit-tested without touching the process env.
*Alternative:* a config key. Rejected because the backend is a property of where pane-mux runs, not a user preference, and workmux uses the same env-only approach.

### D2. `RexCli` runs the `rex` CLI through an injectable runner
`RexCli` holds a `Runner` (a trait that executes argv and returns stdout or an error) plus the resolved server flag. Every call prepends `--autostart=false` and `-S $REX_SERVER` when that is set. A spawn `NotFound` error maps to "`rex` command not found". A connection failure maps to "Rex server unreachable". Other non-zero exits surface Rex's stderr, which the picker shows on screen. Tests inject a fake runner that replays JSON fixtures recorded from a real server (`src/rex/fixtures/*.json`).
*Alternative:* talk to the server socket directly. Rejected because the Rex skill explicitly says to use the CLI, and the socket protocol is not a stable contract.

### D3. One `rex do` Lua snapshot per tree refresh
An embedded Lua script (`include_str!("snapshot.lua")`), run via `rex do -e`, lists sessions, attaches to and views each one, walks windows, layers and blocks, and calls each terminal block's `process` method for `foreground.name`. It returns one JSON document, so one tick costs one process spawn. A custom deserializer accepts `{}` as an empty list. `REX_SESSION`/`REX_BLOCK` are removed from that child's env.
*Fallback (if the spike shows `rex do` can't do this on the target version):* `rex ls --json` plus one `rex api call session.view` per session, with process names fetched only for the hovered window.
*Alternative:* `session inspect` per session plus `block call process` per block. Rejected because it costs N+1 spawns per 300 ms tick.

*Spike result (preview.21, API 18):* `env -u REX_SESSION -u REX_BLOCK rex --autostart=false -S "$REX_SERVER" do -e "$(< src/rex/snapshot.lua)" --json` took 20.5–21.6 ms (20.7 ms median, 7 runs; 3 sessions, 9 windows, 11 terminal blocks). `session.attach` returns the view directly and only attaches the temporary CLI connection. Before/after `whoami` location, app client membership and `session inspect` view data matched (ignoring each inspector's transient CLI connection). `com.superlogical.terminal.process` takes `session_id` and `block_id`; omit `process_name` when there is no foreground process. Empty lists serialize as `{}`. Normal layers omit `state`; minimized floats carry `state: "minimized"`. `active_window_id` and `focused_block_id` may also be absent.

### D4. Tree mapping onto the neutral model
- Sessions come in Rex order and carry the label as `name`.
- Windows come in Rex order with `index` = 1-based position.
- Panes: tiled-layer blocks in layout order, then floating-layer blocks. `index` = 1-based position, `floating` = layer kind is floating (minimized included), `command` = `foreground.name`, falling back to the block label.
- `zoomed = false`, `mode = None`, no stash.
- `Current` comes from looking up `REX_BLOCK` in the tree. `suppressed_pane()` = `REX_BLOCK`, and `in_client()` = non-blank `REX_BLOCK`.
- The picker's name ordering (`SessionOrder::Name`) sorts by label, with ties broken by Rex order.

### D5. Targets
`target_string` returns the full ID of the item at the requested level. `display_target` returns `label:window_pos.pane_pos` for the UI. `resolve_pane(TARGET)` runs `rex block inspect TARGET --json`, so full IDs, suffixes and unique labels all resolve the way Rex resolves them.

### D6. Preview capture cadence
`capture()` runs `rex capture -b B --format vt --unwrap --trim` and keeps only the last N lines needed for the preview height. On Rex, the picker re-captures when the previewed block changes and otherwise at most every ~1 s instead of every tick. This uses a per-backend `preview_interval()` on `Mux`, which tmux keeps at the current tick, so tmux is unchanged.

*Spike result (preview.21):* Keep `--format vt --unwrap --trim` and take the tail in pane-mux: `capture` has no visible-only or line-count option. `--unwrap` joins soft-wrapped lines; `--trim` trims blank space, not scrollback. On a throwaway block with `seq 1 20000` plus a completion marker, plain VT was 128,910 bytes; `--unwrap --trim` was 128,909 bytes. Both had 20,001 newline characters. `capture.vt` is 52 bytes of real colored output and contains CRLF line separators plus a final LF. For cross-session targets, pass `-s S` when known, or run unscoped block commands with `REX_SESSION`/`REX_BLOCK` removed: with the inherited environment, both `capture -b` and D5's `block inspect` failed for blocks outside the current session even with a full ID or unique suffix, respectively.

### D7. Switching and the Remote Control fallback
For the picked target:
1. Server side (always): `session.focus_block` for a block, or `session.focus_window` for a window, so the target session lands on the right window or block.
2. App side: pick the client from `REX_CLIENT` if set, otherwise from `rex client ls --json` the connected `info.kind == "app"` client whose `session_ids` includes the current session (else the first app client). Then run `rex -C <client> do session.select session_id=S window_id=W`.
3. If step 2 fails because Remote Control is off or there's no app client (classified from Rex's error text, which the spike records), print the hint to stderr and still exit 0.

*Spike result (preview.21):* With Remote Control on, `rex -C <app client> do session.select session_id=S window_id=W --json` exits 0 and prints `null` (`select_success.json`). A nonexistent client exits 3 with `Error: client target "client:…" not found` (`select_no_client.txt`). The Remote Control-off error text is not recorded yet, so per the risk note any app-side failure after the server-side focus is treated as the fallback.

### D8. Recency history and ranking
Store `$XDG_STATE_HOME/pane-mux/rex-recent.json`, falling back to `~/.local/state/pane-mux/`, keyed by server identity (`REX_SERVER` if set, else `whoami.endpoint`). Each entry is a most-recent-first list of session IDs, capped at 50, written atomically (temp file + rename). After a switch, pane-mux moves the left session to the front and then the target session, so the list reads target, left, …. IDs are stored rather than labels because labels change and aren't unique.
Ranking reuses the existing `SessionOrder::Recent` sort unchanged. The Rex backend fills `Session.last_attached` with synthetic, strictly decreasing values: history sessions first in history order, then every other session in Rex order. Every value is non-`None`, so the current-first, `exclude_from_recency`-by-label and cursor rules apply as they are. History IDs that no longer exist are skipped.
*Alternative:* a separate Rex-only ordering in `picker/state.rs`. Rejected because it duplicates the exclusion and cursor logic.

### D9. Moves
- Add `Mux::cross_session_moves() -> bool` (true for tmux, false for Rex). In `State::paste`, when it's false, the sessions column focus, or a hovered window or session different from the cut pane's session, sets `message = "rex does not allow moving panes to different session"` before any prompt and returns `Ok(None)`, which keeps the buffer and the picker open, including under `move-pane`.
- `move_pane_to_new_window(pane, session, name)`: `session.new_window{focus:false, window_label}` with a placeholder shell block, then `session.move_block{block_id: pane, anchor_block_id: placeholder}`, then `block.close{placeholder}`. Return the new window ID.
- `float_pane_in_window(pane, window, cfg)`: pick an anchor (the window's `focused_block_id` if it's tiled, else its first tiled block), then `session.move_block{pane, anchor}`, then `session.float_block{pane, bounds}`.
- Bounds: `w = width/100`, `h = height/100`, `x ∈ {0, (1-w)/4, (1-w)/2, 3(1-w)/4, 1-w}` for left, left-centre, centre, right-centre, right, and `y` likewise for top, centre and bottom, following the `FloatingPosition` names.
- After a move, if the source window has no blocks left and Rex hasn't closed it, `session.close_window` it.
- `move_pane_to_new_session` returns an error and is unreachable behind the capability check.

*Spike result (preview.21):* `session.new_window` requires an explicit `layout` (`{"block":{"flavor":"com.superlogical.terminal.shell"}}`); Rex does not add a placeholder shell on its own. The placeholder flow and `move_block` + `float_block` work as described, and the moved process keeps its PID. Rex auto-closes a window as soon as its last block moves out, so the explicit `close_window` is only a guard. A `move_block` of a block that is focused in its source window makes the destination window active and focuses the block (even when the source window was inactive), so moves must restore the previous `active_window_id` / focused block afterwards. Sessions are created with `session.create {label, initial_windows:[{window_label, layout}]}` (there is no `session.new`, and it does not change the app view). Fixtures live in `src/rex/fixtures/api_*.json` and `inspect_*.json`.

### D10. Indicators
The Rex backend never sets `mode` or stash counts, so the existing UI code shows no copy-mode or stashed indicator. The zoomed-target refusal reads `window.zoomed`, which is always false on Rex.

## Risks / Trade-offs

- [Rex is pre-release and API names or fields may change] → Check `whoami.api_revision >= 18` on startup and print a clear "Rex too old" error otherwise. Fixtures are recorded from real output and the opt-in integration tier catches drift.
- [Snapshot latency on many sessions] → One spawn per tick (D3). Measure in the spike, and raise the Rex tick interval if one snapshot takes more than ~150 ms.
- [Preview capture of huge scrollback] → Trim to the preview height, re-capture on a slower cadence (D6), and prefer a visible-only capture if Rex offers one.
- [The placeholder-window dance could leave a stray placeholder on partial failure] → Close the placeholder on every error path and report the failure on screen.
- [`move_block` might change the app's focused block, against "moves keep the view"] → The spike verifies this. If it does, restore focus with `session.focus_block` on the previously focused block.
- [Remote Control error classification depends on Rex's wording] → Treat any app-side failure after the server-side focus succeeded as the fallback path, and print Rex's message after the hint.

## Migration Plan

Additive. Under tmux nothing changes. Rex users opt in just by running pane-mux inside Rex. Rollback is reverting the change. The history file is disposable.

## Open Questions

- The exact Lua API surface for attaching to other sessions inside `rex do`, and the exact Remote Control error text. Both are resolved by the spike tasks and change neither the specs nor the task breakdown.
