# Tasks

## 1. Package metadata and contents

- [x] 1.1 Find the lowest Rust toolchain (at least 1.85) that passes `cargo +<ver> check --locked` with the current lockfile, by trying candidate toolchains; record the version for 1.2
- [x] 1.2 Add `license = "MIT"`, `repository`/`homepage` = `https://github.com/0xKahi/pane-mux`, `readme = "README.md"`, up to 5 `keywords`, `categories = ["command-line-utilities"]` and `rust-version` (from 1.1) to `[package]` in `Cargo.toml`; verify `cargo metadata --no-deps --format-version=1` shows the fields and `cargo build --locked` still succeeds
- [x] 1.3 Add `include = ["src/**/*", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "CHANGELOG.md"]` to `Cargo.toml`; verify `cargo package --list --allow-dirty` lists only those paths (no `openspec/`, `tests/`, `.pi/`, `justfile`, `target/`) and includes every `src/config/preset/*` file used by `include_str!`
- [x] 1.4 Create `CHANGELOG.md` in Keep a Changelog style with a `## [0.1.0]` entry summarising the picker, find action, `switch-session` with recency order, configuration, key bindings and themes; verify it is listed by `cargo package --list --allow-dirty`

## 2. CI workflow

- [x] 2.1 Add a `ci` recipe to `justfile` running `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `PANE_MUX_TMUX_TESTS=1 cargo test --locked -- --test-threads=1`; verify `just ci` passes locally with tmux installed
- [x] 2.2 Add `.github/workflows/ci.yml` (triggers: `pull_request`, `push` to `main`; `permissions: contents: read`; `ubuntu-latest`) with checkout, `dtolnay/rust-toolchain@stable` (components rustfmt, clippy), `Swatinem/rust-cache@v2`, `sudo apt-get update && sudo apt-get install -y tmux`, and the same three commands as `just ci`; verify the YAML parses (for example `actionlint` if available, or `python3 -c 'import yaml,sys; yaml.safe_load(open(sys.argv[1]))' .github/workflows/ci.yml`)

## 3. PR title convention

- [x] 3.1 Add `.github/workflows/pr-title.yml` using `amannn/action-semantic-pull-request@v5` on `pull_request_target` (types `opened`, `edited`, `synchronize`, `reopened`) with `permissions: pull-requests: read` and allowed types `feat, fix, perf, refactor, docs, test, ci, build, chore, style, revert`; verify the YAML parses as in 2.2

## 4. Release workflow

- [x] 4.1 Add `release-plz.toml` with `[workspace]` settings `semver_check = false`, `git_release_enable = true`, and changelog generation on (default tag `v{{ version }}`); verify `release-plz release-pr --dry-run` (or `release-plz update` in a scratch clone) runs without config errors if release-plz is installed locally, otherwise verify the keys against the release-plz configuration docs
- [x] 4.2 Add `.github/workflows/release-plz.yml` triggered on `push` to `main`, with two jobs that each check out with `fetch-depth: 0`, install stable Rust, and run `release-plz/action` (pinned major version): `release-plz-release` (`command: release`, `permissions: contents: write`, env `GITHUB_TOKEN` and `CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`) and `release-plz-pr` (`command: release-pr`, `permissions: contents: write, pull-requests: write`, a `concurrency` group without `cancel-in-progress`, env `GITHUB_TOKEN`); verify the YAML parses as in 2.2 and that both jobs are skipped on forks via `if: github.repository_owner == '0xKahi'`

## 5. Documentation

- [x] 5.1 Change the README install section to `cargo install pane-mux --locked` (keep `cargo install --path .` as the from-source option) and add a short "Releasing" section: squash-merge PRs with conventional titles, release-plz opens a release PR, and merging it publishes to crates.io; verify the README renders and the commands are correct

## 6. Pre-merge integration check

- [x] 6.1 Run `just ci`, then `cargo publish --dry-run --locked --allow-dirty` (and again without `--allow-dirty` once committed); verify both succeed and the packaged crate builds from the `.crate` tarball in `target/package/`
- [ ] 6.2 Push the branch and open a PR titled `ci: add CI and release-plz publishing`; verify the `ci` and `pr-title` workflows pass on the PR

## Workflow follow-up

- After merging, confirm the `release-plz-release` job published `pane-mux 0.1.0`, pushed tag `v0.1.0` and created a GitHub Release, then run `cargo install pane-mux --locked` from a clean `CARGO_HOME` and check `pane-mux --version` prints `0.1.0`.
- Once the crate exists, optionally narrow the `CARGO_REGISTRY_TOKEN` scope to publish-update for `pane-mux`, and decide on a branch protection rule requiring CI (see the PAT note in design.md if required checks block release PRs).
- Archive the change after the release is verified.
