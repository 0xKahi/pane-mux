# Tasks

## 1. Visible-screen capture on Rex

- [x] 1.1 Add `src/rex/capture.lua` as in design.md D1: attach the session, read `rows` from `com.superlogical.terminal.size`, then call `format` (`vt`, `trim`, `max_bytes = budget`, no `unwrap`). If the reply has an `id`, follow with `blob_read` at `max(0, total - budget)`. Return `{rows, content, partial}`. Embed it with `include_str!` next to `SCRIPT` in `src/rex/snapshot.rs` or a new `src/rex/capture.rs`. Verify by running it by hand with `rex do -e "$(cat src/rex/capture.lua)" --json --args '{"session_id":…,"block_id":…,"budget":131072}'` against a small block (`partial: false`) and a block with more than 128 KiB of history (`partial: true`).
- [x] 1.2 Rewrite `RexCli::capture` in `src/rex/client.rs` as in design.md D4: find the session through `lookup()`, run the script once through `run(…, UNSCOPED)` with `--args` holding `session_id`, `block_id` and `budget = 131072`, drop the first line when `partial`, turn CRLF into LF, and keep the last `rows` lines. Delete the 500-line cap. Verify with `cargo test` passing new tests in `src/rex/tests.rs` that replace `capture_uses_unscoped_vt_and_keeps_last_500_lines`. Use JSON reply fixtures under `src/rex/fixtures/`:
  - Whole output fits, longer than `rows`: only the last `rows` lines remain, in order.
  - Whole output shorter than `rows`: every line is kept.
  - `partial: true`: the cut first line is dropped, then the last `rows` lines are kept.
  - Alternate-screen reply with exactly `rows` lines: returned unchanged.
  - CRLF is normalised, and colour escapes survive.
  - Exactly one runner call, with `UNSCOPED` removed and the expected argv and `--args` JSON.
  - A block missing from the tree after a refresh returns `pane not found: <id>`.
- [x] 1.3 Find out whether `com.superlogical.terminal.size`, `.format` with `max_bytes`, and `.blob_read` exist at Rex API revision 18, using the Rex changelog and docs (the `mcp__superlogical` tools in codemode). If they don't, raise `MIN_API_REVISION` in `src/rex/client.rs` to the first revision that has them and update the existing "Rex too old" test and its fixture. Verify that the outcome is recorded in this task's notes or the PR description, and that `cargo test` passes.

## 2. Docs

- [x] 2.1 In the Rex section of `README.md`, say that the preview shows the block's visible screen, like tmux, not its scrollback. Verify by reading the section alongside the existing floating-block paragraph.

## 3. Integration check

- [x] 3.1 Add an opt-in live test to `tests/rex_integration.rs`, behind `PANE_MUX_REX_TESTS=1` like the others. It creates a session, prints 2,000 numbered lines in a block, captures the preview, and asserts that the line count is at most the block's `rows`, the last line is `2000`, and no line below `2000 - rows + 1` appears. Verify with `just test-rex`.
- [x] 3.2 Run `cargo fmt --check`, `just lint` and `just test` and confirm they pass. Then put a `rex` wrapper that logs each call on `PATH` (as in the exploration: argv, duration, output bytes) and run `pane-mux switch-session` over a block with a large history. Confirm each preview capture is a single `rex do` call, its output is at most about 128 KiB, it takes about the same time as a small block's capture, and the preview opens on the block's current screen.

## Implementation notes

- 1.1: A single `blob_read` from `total - budget` can stop short of the end, because `max_bytes` counts JSON-encoded bytes while offsets count raw bytes. The script loops `blob_read` until `eof` (see design.md D1). Verified live: a small block returned `partial: false`, and a 20,000-line styled block returned `partial: true` ending at line 20000 and the prompt, in 20–40 ms.
- 1.3: `MIN_API_REVISION` stays at 18. The docs don't list revisions per block method, but the changelog dates them: `rex capture` started handling large scrollback without response-size failures (the `format` `max_bytes` + `blob_read` path) in server/0.1.0-bd45734 on 2026-10-22, and terminal sizing goes back to mac/0.1.0-preview.5. API revision 17 (client prompts, `rex.ui.input`) shipped with mac/0.1.0-preview.32 on 2026-11-18, so revision 18 came later and has all three methods. They are confirmed live on revision 19.
- 3.2: With a logging `rex` wrapper, `switch-session` over the 20,000-line block ran one `rex do -e <CAPTURE>` per preview tick, 49–55 ms with 168,297 bytes of output each time, against 41 ms for a small block. The preview opened on the block's current screen. The output is 128 KiB of content plus JSON escaping (see design.md D3).

## Workflow follow-up

- Run `openspec validate rex-visible-screen-preview --strict` before review.
- Archive the change after review so the `rex-backend` delta is merged into `openspec/specs/rex-backend/spec.md`.
