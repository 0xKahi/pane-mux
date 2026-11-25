# Proposal

## Why

On the Rex backend, the preview runs `rex capture`. That returns a block's whole scrollback, up to about 64 MiB, and pane-mux keeps only the last 500 lines. A block with 10 MB of history took about 300 ms per capture. The picker's input loop waits on each capture, so moving the cursor onto that block froze the picker, and it froze again every second while the cursor stayed there. The preview also starts at the top of the content (scroll 0). Rex previews therefore open about 500 lines back in history instead of on the block's screen. The tmux preview shows only the visible screen (`capture-pane` without `-S`), and the spec already says the preview shows the block's "visible contents".

## What Changes

- On Rex, the preview shows only the rows currently on the block's screen, not its scrollback. This matches the tmux backend.
- A block running a full-screen program, which uses the alternate screen, previews that program's screen, as it does today.
- The time to capture a preview no longer grows with a block's history. pane-mux receives at most a fixed number of bytes per capture and asks Rex for one in a single `rex` call.
- Long lines show the way they're wrapped on the block's screen, one preview line per screen row. Rex no longer joins wrapped lines (`--unwrap`) for the preview.
- Preview scrolling (`<C-u>`/`<C-d>`) on Rex moves within the screen rows only, as on tmux. You can no longer scroll back through the previous 500 lines of history.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rex-backend`: "Rex preview and self-block suppression" changes "visible contents" to mean the block's screen rows only, with no scrollback. A new requirement says the cost of capturing a preview doesn't depend on how much scrollback a block has.

## Impact

- `src/rex/client.rs`: `RexCli::capture` stops running `rex capture` and runs a capture script through `rex do -e … --json` instead, then keeps only the screen rows.
- `src/rex/`: a new `capture.lua` next to `snapshot.lua`, plus new fixtures and tests in `src/rex/tests.rs`. The existing `capture_uses_unscoped_vt_and_keeps_last_500_lines` test is replaced.
- `tests/rex_integration.rs`: an opt-in live check that the preview contains no history lines.
- The Rex section of `README.md`: one line saying the preview shows the block's visible screen.
- No change to the tmux backend, the picker loop and state, the config, the CLI or `preview_interval`. The Rex methods used are `com.superlogical.terminal.size`, `.format` and `.blob_read`. They are confirmed on API revision 19 but not yet on 18, the current minimum. design.md covers what happens if they're missing.
