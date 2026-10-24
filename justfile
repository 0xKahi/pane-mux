# Run `just` to list recipes.
default:
    @just --list

# Format all code
fmt:
    cargo fmt

# Lint with clippy, treating warnings as errors
lint:
    cargo clippy --all-targets -- -D warnings

# Regenerate the committed editor schemas
schemas:
    PANE_MUX_UPDATE_SCHEMAS=1 cargo test config::schema::committed_schemas

# Run tests
test:
    cargo test

# Everything CI would run
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Exactly what the GitHub Actions CI workflow runs (requires tmux)
ci:
    cargo fmt --check
    cargo clippy --all-targets --locked -- -D warnings
    PANE_MUX_TMUX_TESTS=1 cargo test --locked -- --test-threads=1

# Opt-in tests against a private tmux server (tmux -L pane-mux-test)
test-tmux:
    PANE_MUX_TMUX_TESTS=1 cargo test -- --test-threads=1
