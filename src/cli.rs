//! Command-line interface definitions.

use clap::Parser;

/// A yazi-style picker for tmux sessions, windows and panes.
#[derive(Debug, Parser)]
#[command(name = "pane-mux", version, about)]
pub struct Cli {}
