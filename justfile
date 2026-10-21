# Run `just` to list recipes.
default:
    @just --list

# Format all code
fmt:
    cargo fmt

# Lint with clippy, treating warnings as errors
lint:
    cargo clippy --all-targets -- -D warnings

# Run tests
test:
    cargo test

# Everything CI would run
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Opt-in tests against a private tmux server (tmux -L pane-mux-test)
test-tmux:
    PANE_MUX_TMUX_TESTS=1 cargo test -- --test-threads=1
