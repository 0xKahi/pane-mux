# Design

## Context

- See proposal.md (Why) for motivation. The repo has no `.github/` directory, no tags, and `Cargo.toml` is at `version = "0.1.0"` with only `name`, `version`, `edition` and `description` set. A root `LICENSE` file (MIT) exists, but the manifest has no `license` field, and crates.io rejects a publish without one.
- The crate is both a library (`src/lib.rs`) and a binary (`src/main.rs`). Built-in presets are embedded with `include_str!` from `src/config/preset/`, so all runtime assets live under `src/`.
- `just check` runs fmt, clippy and tests. The tmux integration test (`tests/tmux_integration.rs`) is skipped unless `PANE_MUX_TMUX_TESTS=1`. It uses `tmux -L pane-mux-test -f /dev/null`, strips `TMUX`/`TMUX_PANE`, and creates sessions detached, so it needs only a `tmux` binary, with no TTY.
- `.pi/` is gitignored. `openspec/`, `tests/`, `justfile`, `rustfmt.toml` and `rust-toolchain.toml` are tracked, so cargo would package them by default.
- Maintainer workflow: PRs into `main`, squash-only, default commit message "Pull request title and commit details". `CARGO_REGISTRY_TOKEN` is already stored as a repository secret. Actions have write permissions and may create PRs.

## Goals / Non-Goals

**Goals:**
- A merge to `main` is the only step needed to publish, after the release PR is reviewed.
- Version bumps and changelog entries come from squash-merged PR titles, not from hand edits.
- CI exercises the real tmux code path, not only the fake.

**Non-Goals:**
- Cross-platform binary artifacts, installers, or package managers other than crates.io.
- macOS CI runners. Linux covers the tmux integration, and macOS users install from source through cargo.
- Keyless (OIDC) publishing. A later change can move from the token to trusted publishing.

## Decisions

### Use release-plz for versioning and publishing
release-plz runs as two jobs on every push to `main`:
- `release-pr` opens or updates a "chore: release vX.Y.Z" PR that bumps `Cargo.toml`/`Cargo.lock` and prepends to `CHANGELOG.md` using git-cliff over conventional commits.
- `release` publishes any version in `Cargo.toml` that is not on crates.io, pushes tag `vX.Y.Z`, and creates a GitHub Release whose body is that version's changelog section.

This matches the maintainer's changesets mental model (bot PR, then merge, then publish) without extra files per PR.
- Alternatives: **knope** keeps `.changeset/*.md` files but is less common in Rust. **cargo-release** and the tag-triggered scripts used by projects like workmux have no release PR and need local tooling. A **hand-written `cargo publish` workflow** would mean reimplementing idempotency, tagging and changelog generation.

### Squash-merged PR titles are the commit convention
Only the squash commit's summary line (the PR title) is parsed. Branch commits listed in the body are ignored because git-cliff `split_commits` stays off. A `pr-title.yml` workflow (`amannn/action-semantic-pull-request`) enforces `type(scope)?: subject` with the types `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `ci`, `build`, `chore`, `style`, `revert`. Breaking changes use `!` in the title.
- Alternative: rely on discipline alone. Rejected because a single malformed title silently drops a changelog entry and the version bump.

### release-plz configuration (`release-plz.toml`)
- `semver_check = false`. The library target exists to support the binary and integration tests, not as a public API promise. Running cargo-semver-checks would flag internal refactors as breaking and force minor bumps.
- Keep release-plz's 0.x defaults: a breaking change bumps minor, and `feat`/`fix` bump patch. The maintainer can turn on `features_always_increment_minor` later without changing this design.
- `git_release_enable = true` and the default tag name `v{{ version }}`.
- `publish_timeout` is left at the default. `cargo publish` runs with `--locked` behaviour via release-plz's default lockfile handling, and CI builds with `--locked`.

### First release publishes 0.1.0 directly
The `release` job publishes the manifest version when it is missing from the registry, so the merge that adds the workflow publishes `0.1.0` with no release PR. Because of this, metadata, the `include` list and the README all land in the **same PR** as the workflow. A hand-written `CHANGELOG.md` with a `0.1.0` entry also ships in that PR, so the first GitHub Release has notes and git-cliff has a file to prepend to.
- Alternative: set the manifest to `0.0.0` and let release-plz propose `0.1.0`. Rejected because it adds a round trip and the version is already the intended one.

### Package contents via `include`
`include = ["src/**/*", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "CHANGELOG.md"]`. An allow-list is safer than an `exclude` list as new tooling directories appear. Tests are not shipped. `cargo package --list` is the acceptance check.

### Manifest metadata
Add `license = "MIT"`, `repository` and `homepage` = `https://github.com/0xKahi/pane-mux`, `readme = "README.md"`, `keywords` (at most 5, for example `tmux`, `picker`, `tui`, `terminal`, `cli`), `categories = ["command-line-utilities"]`, and `rust-version`. `rust-version` is the lowest toolchain that passes `cargo check --locked` with the locked dependencies. It must be at least 1.85 for edition 2024. Determine it during implementation by checking candidate toolchains, not by guessing.

### CI: one Linux job running everything
`ci.yml` runs on `pull_request` and on `push` to `main`, on `ubuntu-latest`: checkout, `dtolnay/rust-toolchain@stable` with rustfmt and clippy (the stable `rust-toolchain.toml` agrees), `Swatinem/rust-cache@v2`, `sudo apt-get install -y tmux`, then:
1. `cargo fmt --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `PANE_MUX_TMUX_TESTS=1 cargo test --locked -- --test-threads=1`

This runs the tests once, including tmux, and avoids doing `just check` and `just test-tmux` back to back. `just` is not installed in CI to keep it dependency-free. A `just ci` recipe mirrors the same three commands so that local and CI runs match.
- Alternative: two parallel jobs (checks and tmux tests) as workmux does. Not worth it at the current suite size, and easy to split later.

## Risks / Trade-offs

- [Release PRs opened with `GITHUB_TOKEN` do not trigger other workflows, so CI and the PR-title check will not run on them. A required-status-check rule would block merging.] → Accept for now: release PRs only touch the version, the lockfile and the changelog. If branch protection with required checks is turned on, either let the maintainer bypass it for release PRs, or switch the `release-pr` job to a fine-grained PAT or a GitHub App token stored as a secret.
- [A broken first publish cannot be undone. Versions can be yanked but never replaced.] → `cargo publish --dry-run --locked` and `cargo package --list` must pass locally before the PR is merged.
- [A malformed PR title on a squash merge loses its changelog line or bump.] → The PR-title check blocks it. The release PR's changelog can also be edited before merging.
- [`CARGO_REGISTRY_TOKEN` is a long-lived secret.] → Scope it to publish-update for `pane-mux` only once the crate exists (the first publish needs publish-new). Migrate to trusted publishing later.
- [The Ubuntu tmux version differs from the maintainer's local tmux.] → The integration test uses only stable commands and formats. Acceptable.
- [Unconventional historical commits such as `added find action` predate the convention.] → They fall before `v0.1.0`, so they never reach a generated changelog.

## Migration Plan

1. Land everything in one PR from `feat/deployment-v0.1.0` with a conventional title (for example `ci: add CI and release-plz publishing`), after the local dry run passes.
2. On merge, the `release` job publishes `0.1.0`, tags `v0.1.0`, and creates the GitHub Release. The `release-pr` job finds no unreleased changes.
3. Verify with `cargo install pane-mux --locked` on a clean machine or in a clean `CARGO_HOME`.
4. Rollback: if the publish fails, nothing is released. Fix and push again. If a bad `0.1.0` is published, `cargo yank --version 0.1.0` and release `0.1.1`.

## Open Questions

- Whether to turn on a branch protection ruleset that requires CI on `main` now or later. This changes only a repository setting and the PAT note above, not the workflows.
