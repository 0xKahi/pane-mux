# Tasks

## 1. Filter the self float while building the Rex tree

- [x] 1.1 Add a snapshot fixture under `src/rex/fixtures/` with a window that has two tiled blocks, the self block in a floating layer, and another floating block. Add a variant where the self layer is `minimized`, and one where the self block is tiled. Verify each fixture parses with the existing `Snapshot` deserializer in a test.
- [x] 1.2 Change the snapshot-to-`Tree` conversion in `src/rex/snapshot.rs` so it takes the self block ID. It skips that block only when its layer kind is `floating`, assigns `Pane.index` after skipping, and returns the self block's session and window IDs whether or not the block was skipped. Verify unit tests show: the float is hidden with rows numbered `1..n` and no gap; a minimized self float is hidden; other floats stay listed; a tiled self block is listed; with no self block, the output is unchanged.
- [x] 1.3 Make `RexClient::tree()` in `src/rex/client.rs` pass `self.block` into the conversion and store the recorded self location next to `last_tree` on every build. Verify a test where a second `tree()` call (a refresh) still leaves out the self float.

## 2. Keep current-item and recency behaviour

- [x] 2.1 Make `RexClient::current()` return the recorded self location as `Current { session_id, window_id, pane_id: REX_BLOCK }`, building the tree first when none is cached, and returning `None` when `REX_BLOCK` is unset or not found. Verify tests show: with a floating self block, `current()` gives the right session and window; with a tiled self block, as before; with an unknown block, `None`.
- [x] 2.2 Verify with a picker-state test on the floating-self fixture that the session and window carry the current marker and no pane row does. Also verify that `switch-session` ranking still lists the current session first and that the recorded history entry names it as the session left.
- [x] 2.3 Add a test recording what `move-pane` without an explicit target does when run from the floating self block, as noted in design.md Risks. It must not panic, and its result is asserted as it is today.

## 3. Docs

- [x] 3.1 Update the Rex section of `README.md` and the "open pane-mux in a Rex floating block" section of `docs/usage-examples.md` to say the picker's own floating block is not listed, and that the self-preview placeholder appears only when pane-mux runs in a tiled block. Verify by reading the rendered sections.

## 4. Integration check

- [x] 4.1 Run `cargo fmt --check`, `just lint` and `just test` and confirm they pass. Then, in a live Rex session, open the picker with the `Ctrl-b o` binding and with the `Ctrl-b m` binding, and confirm the float is not listed, numbering has no gap, and the session and window markers are present.

## Workflow follow-up

- Run `openspec validate hide-rex-picker-float --strict` before review.
- Archive the change after review so the `rex-backend` delta is merged into `openspec/specs/rex-backend/spec.md`.
