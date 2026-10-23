# Design

## Context

See proposal.md (Why) for the motivation and `specs/session-switching/spec.md` for the required order.

Current state:
- `Config` (`src/config/settings.rs`) has only `[picker]`, with `deny_unknown_fields` on every struct. Config is the preset TOML with the user TOML deep-merged over it (`src/config/merge.rs`). Tables merge; arrays replace.
- `PickerOptions` (`src/picker/options.rs`) is built by each command and carries `order: SessionOrder` (`Name` | `Recent`). `switch-session` uses `Recent`.
- `State::sort_sessions` (`src/picker/state.rs`) sorts by `(not_current, Reverse(last_attached), name)` under `Recent`. It runs on construction and on every `refresh`. `initial_session` opens on the first non-current session.
- The working tree has an uncommitted preset key `[switch-session] recency_exclusion = []`, which this change renames.

## Goals / Non-Goals

**Goals:**
- Excluded sessions form their own bucket, sorted by name, after all ranked sessions. This includes never-attached ones.
- No change to the cursor logic, the refresh logic, or `SessionOrder::Name` behaviour.

**Non-Goals:**
- Glob or regex matching, and matching by session id.
- Exclusion in `pane-mux picker`, or an exclusion setting for the windows/panes columns.
- Warning about configured names that match no session.

## Decisions

### 1. Config: a new `SwitchSessionConfig` section, renamed to `switch-session`
Add `#[serde(rename = "switch-session")] pub switch_session: SwitchSessionConfig` to `Config`, with `exclude_from_recency: Vec<String>` and `deny_unknown_fields`. The preset ships `exclude_from_recency = []`, so the field is always present after merging, in line with how `[picker]` keys are required and come from the preset.
- *Why kebab-case for the section:* it matches the `pane-mux switch-session` command name users already type.
- *Alternative:* putting the key under `[picker]`. Rejected because recency order only exists in switch-session mode, so the key would mean nothing to `pane-mux picker`.
- No validation is needed beyond the types. Any string is a valid name, and duplicates are harmless.

### 2. Carry the list in `PickerOptions`, not in `SessionOrder`
Add `recency_exclude: Vec<String>` to `PickerOptions` (default empty). `switch_session::run` fills it from `loaded.config.switch_session.exclude_from_recency`, and `pane-mux picker` leaves it empty.
- *Alternative:* `SessionOrder::Recent { exclude: Vec<String> }`. Rejected because `SessionOrder` is `Copy` and is compared with `==` in several places. A data-carrying variant would ripple through all of them.
- *Alternative:* having `State` read `Loaded` directly. Rejected because `State` is built from `PickerOptions` plus the tmux snapshot, and keeping config out of it keeps the state tests simple (existing tests use `..PickerOptions::default()`).

### 3. One extra sort-key element
Under `Recent`, the key becomes:

```
(not_current, is_excluded, if excluded { 0 } else { Reverse(last_attached) }, name)
```

Neutralising `last_attached` for excluded sessions sorts them by name alone. Putting `is_excluded` before it places the whole bucket after the never-attached ranked sessions. The current session is never moved because `not_current` comes first. Use the same comparable type in both branches, e.g. `Reverse(if excluded { 0 } else { last_attached })`.
- `initial_session` stays as it is. Because the first non-current row is the most recent non-excluded session when one exists, the "skip excluded" scenarios hold without new logic.
- `refresh` already calls `sort_sessions` and keeps the cursor by id, so an excluded session that gets re-attached stays put.
- Matching compares `session.name` exactly against the list. A linear `contains` on a short `Vec` is fine; a `HashSet` isn't worth adding.

## Risks / Trade-offs

- [A renamed session drops out of exclusion] → This is expected with exact-name matching. It's documented in `docs/configuration.md`.
- [The uncommitted `recency_exclusion` preset key would fail `deny_unknown_fields` once the struct exists] → Replace it in the same task that adds the struct. Preset loading is covered by the existing config tests.
- [Sessions are named, not identified by id, so two servers can share a name] → Not an issue: the list applies to whichever server the command talks to.

## Migration Plan

None needed. The default `[]` keeps today's behaviour, and the old `recency_exclusion` key was never released.
