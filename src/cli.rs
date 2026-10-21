//! Command-line interface definitions.

use clap::{Parser, Subcommand};

/// A yazi-style picker for tmux sessions, windows and panes.
#[derive(Debug, Parser)]
#[command(name = "pane-mux", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Pick a tmux target and print it to stdout.
    Picker,
}
