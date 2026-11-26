# Tasks

## 1. Self-block owner from the snapshot

- [x] 1.1 In `src/rex/snapshot.lua`, add `owner = block.owner` to each block entry (omitted when absent) and update the header comment's schema. Verify with `just test-rex` or a manual `rex do -e "$(cat src/rex/snapshot.lua)" --json` showing `owner` on blocks.
- [x] 1.2 In `src/rex/snapshot.rs`, deserialize `owner: Option<String>` on `Block` and return the owner of the `REX_BLOCK` block (tiled, floating or minimized) alongside the existing `Current`. Add `owner` to `fixtures/snapshot_self_tiled.json`, `snapshot_self_floating.json` and `snapshot_self_minimized.json`. Verify with unit tests asserting the owner is found for each, and is `None` when the block has no owner or `REX_BLOCK` is unset or not found.
- [x] 1.3 In `src/rex/client.rs`, store the self-block owner on `RexCli` (refreshed on every `tree()`, next to `last_current`). Verify `cargo test` passes and `mux::Current` is unchanged.

## 2. App selection and ambiguity hint

- [x] 2.1 Rewrite `select_app` with the order in design.md, Decision 2: `REX_CLIENT` → owner if it is a connected app in `client ls` → the single candidate among apps attached to the left session (or all connected apps when none are attached) → zero candidates gives the existing "no connected Rex app client" error, more than one gives a distinct ambiguity error. Verify with `src/rex/tests.rs` cases: two apps where the owner selects the second; `REX_CLIENT` (`with_app_client`) beats the owner without calling `client ls`; owner disconnected or not an app falls back to the single attached app; single app with no owner is selected.
- [x] 2.2 Update `switching_focuses_block_then_selects_app_attached_to_left_session`, which currently expects the second attached app to lose to `APP` through first-app ordering, so that it reflects the new rules (owner-based or single-candidate). Verify `cargo test` passes.
- [x] 2.3 In `switch`, map the ambiguity error to its own hint naming the target session label, saying pane-mux could not tell which Rex app to switch and that `REX_CLIENT` (label or client ID) chooses one. Keep the Remote Control hint for the other errors. Verify with a test where two connected apps are attached to the left session and there is no usable owner: focus call made, no `session.select` call, the hint contains the session label and `REX_CLIENT`, recency history records target and left (as in `app_failure_is_nonfatal_and_records_target_then_left_by_whoami_endpoint`), and `switch` returns `Ok(Some(_))`.
- [x] 2.4 Update `README.md` (Rex section near the Remote Control note) and `docs/usage-examples.md` (Remote Control paragraph): with several devices connected to one Rex server, pane-mux switches the app that opened the picker block, and `REX_CLIENT` forces a choice. Verify the docs read correctly and `just check` passes.

## 3. Integration check

- [x] 3.1 Run `just check` and verify fmt, clippy and all tests pass.
- [x] 3.2 Manual two-device check: with `jojo-mac` and `shuffle-mac-m4` both connected to `shuffle-mac-m4`'s server, open `switch-session` through the `init.lua` binding from `jojo-mac` and confirm `rex api call session.view -s "$REX_BLOCK"` shows the picker block owned by `jojo-mac`'s app client. Pick another session and confirm only `jojo-mac`'s view changes. Repeat from `shuffle-mac-m4`. If the binding-created block has no app owner, record that in design.md Risks and confirm the ambiguity hint appears instead of a wrong-device switch.

## Workflow follow-up

- Archive the change once the manual two-device check passes.
