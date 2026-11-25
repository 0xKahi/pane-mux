# Design

## Context

See proposal.md for why. Today's preview path on Rex:

```
picker update_preview  --(blocks input loop)-->  RexCli::capture(block)
    rex capture -b BLOCK --format vt --unwrap --trim     (whole scrollback, ~64 MiB max)
    keep last 500 lines, CRLF -> LF
    picker renders from scroll 0 (top of those 500 lines)
```

Measurements from a live Rex server (API revision 19) while exploring:

| block | `rex capture` | output |
|---|---|---|
| typical shell (2–60 KB) | 25–30 ms | full |
| 190 KB history | ~30 ms | full |
| 10 MB history | 260–340 ms | 10 MB piped to pane-mux |

Rex facts the design relies on, all checked against the live server:
- `com.superlogical.terminal.size` returns `rows` and `columns`.
- `com.superlogical.terminal.format` takes `format`, `trim`, `unwrap` and `max_bytes`. On the primary screen it returns scrollback followed by the screen. Blank rows below the last line of text are already left out: a 79-row block with 2 lines of text returned 2 lines. On the alternate screen it returns exactly `rows` lines and no history.
- With `max_bytes` set, a reply that doesn't hold the whole output has an `id` and a `total`. `com.superlogical.terminal.blob_read(id, offset, max_bytes)` reads any range of that result. `max_bytes` is measured after JSON encoding, so `ESC` counts as six bytes (`\u001b`).
- Inside `rex do -e`, creator-method calls (`rex.call("com.superlogical.terminal.…", {session_id, block_id, args = {…}})`) return `nil` unless the script first calls `session.attach` for that session. `snapshot.lua` already attaches for the same reason.
- `rex do -e CODE --args JSON` hands the object to the script as `rex.args`.
- One `rex do` running attach, size, format and blob_read on the 190 KB block took 33–41 ms in total.

## Goals / Non-Goals

**Goals:**
- One `rex` process per preview capture, with output limited to a fixed number of bytes.
- The preview lines are the block's screen rows.

**Non-Goals:**
- Making capture non-blocking, or debouncing captures while the cursor moves. These would be a separate change to the picker loop that also affects tmux.
- Changing `preview_interval` (1 s on Rex) or the tree refresh tick.
- Getting the visible screen exactly right when it is only partly filled and the block also has scrollback (see Risks).
- Keeping preview scroll-back through history.

## Decisions

### D1. Capture through a Lua script run by `rex do`, not `rex capture`

`src/rex/capture.lua` is embedded with `include_str!` like `snapshot.lua`. It runs as `rex do -e <CAPTURE> --json --args <json>` with `UNSCOPED` removed from the environment. The JSON arguments are `{session_id, block_id, budget}`. The script:

```
attach(session_id)
rows  = terminal.size().rows
first = terminal.format{format="vt", trim=true, max_bytes=budget}
if first.id == nil:                       -- whole output fit
    return {rows, content = first.content, partial = false}
offset = max(0, first.total - budget)
repeat                                    -- until eof
    chunk  = terminal.blob_read{id = first.id, offset, max_bytes = budget}
    offset = offset + #chunk.data
return {rows, content = concat(chunks), partial = true}
```

`total` and `offset` count raw bytes, but `blob_read`'s `max_bytes` counts JSON-encoded bytes, and its schema says a read "can be shorter than max_bytes even when more text remains". Found during implementation: on a styled 20,000-line block a single read from `total - budget` stopped about 29 KB short of the end (`eof: false`), so the script keeps reading until `eof`. Chunks never split a character and splice losslessly.

Alternatives considered:
- **Keep `rex capture` and cut to the screen afterwards.** This gives the right rows but still pipes the whole scrollback. It fixes correctness only.
- **Two `rex api call` processes (format, then blob_read).** Measured at about 115 ms on the 10 MB block, and it costs about 25 ms extra on every capture.
- **Ask for a 1-byte reply first to learn `total`, then read the end.** That always takes two server calls. Fitting the whole output in the first call is just as cheap for small blocks, and the server calls run inside one process anyway.

### D2. Visible screen = the last `rows` lines of the output without `unwrap`

Without `unwrap`, each line of the output is one screen row, so the screen is the last `rows` lines. On the alternate screen this changes nothing, because the output is already the screen. `trim` (spaces at line ends) stays on to save bytes.

Alternatives considered:
- **Keep `unwrap`.** Joined lines no longer match screen rows, so the row count overshoots into history.
- **Use the cursor position (`extra_cursor`) to find the screen bottom.** This needs escape-sequence parsing and still misses text below the cursor. Not worth it for the partly-filled edge case.

### D3. Byte budget is 128 KiB, filled from the whole output first

`budget = 131072` is a constant in `src/rex/client.rs`. It caps the JSON size of a whole-output reply and the raw bytes of a partial tail, so a partial reply's JSON is somewhat larger (168 KB measured for a styled tail) but still fixed. It's well above the screens measured: the largest alternate-screen capture was 23 KB for a 77×252 block, and every block under the budget is captured whole in one server call. A heavily styled, very large screen could exceed it. In that case the preview shows the rows that fit, ending at the screen bottom (see Risks).

When `partial` is true, pane-mux drops everything up to and including the first `\n`. That removes a line cut at the chunk start, including any multi-byte character or escape sequence split by `offset`.

### D4. Rust-side processing

`RexCli::capture(pane_id)`:
1. Finds the block's session through the existing `lookup()` cache. This normally costs no spawn and refreshes the tree once if the block is missing. If the block is still missing, it fails with `pane not found: <id>`.
2. Runs the script and deserializes `{rows: u32, content: String, partial: bool}`.
3. If `partial`, drops the first line. Converts `\r\n` to `\n` as today. Keeps the last `rows` lines, ignoring the one trailing newline.

The 500-line cap goes away, since `rows` replaces it.

### D5. Errors

Script or parse failures go through `RexCli::run` and the existing picker message path (`update_preview` errors become `state.message`). There's no fallback to `rex capture`, so slow captures can't come back unnoticed.

## Risks / Trade-offs

- [Partly filled screen with scrollback] Rex leaves out the blank rows below the text, so the last `rows` lines can include a few history lines above the screen. → Accepted. It's harmless and only happens until the screen fills. `clear` usually wipes history too.
- [Styling started above the first kept line] Escape sequences that set a colour before the cut are lost, so the first lines may render without it. → Accepted for now. In the VT samples seen while exploring, Rex reset the style at the end of each line and set it again on the next. Revisit if it's visible.
- [Screen larger than the budget] The top rows of a huge, heavily styled screen are dropped. → The 128 KiB budget is about 5× the largest screen measured. Raise the constant if it happens.
- [`session.attach` on every capture] The script attaches its own connection, the same as `snapshot.lua` does on every tree refresh. The cost was measured at a few ms.
- [Older Rex servers] The methods are confirmed on API revision 19 and the minimum is 18. → Checked in tasks: either confirm 18 has them, or raise `MIN_API_REVISION` to 19 so older servers get the existing "Rex too old" error instead of a confusing preview error.
- [Lost history scrolling] `<C-u>`/`<C-d>` no longer reach earlier output on Rex. → Intended. This matches tmux and the existing spec.

## Migration Plan

None. Behaviour changes on upgrade, and there is no config or data. Rollback is a revert.
