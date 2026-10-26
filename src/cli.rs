//! Command-line interface definitions.

use crate::tmux::Level;
use clap::{Args, Parser, Subcommand};

/// A yazi-style picker for tmux sessions, windows and panes.
#[derive(Debug, Parser)]
#[command(name = "pane-mux", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Available pane-mux operations.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Pick a tmux target and print it to stdout.
    Picker(PickerArgs),
    /// Pick a target and switch the current tmux client to it.
    SwitchSession,
    /// Move a pane using a locked cut-and-paste picker.
    MovePane { target: Option<String> },
    /// Stash or restore a window's floating panes (tmux 3.8).
    Float {
        #[command(subcommand)]
        action: FloatAction,
    },
}

/// Window-wide floating pane operations.
#[derive(Debug, Subcommand)]
pub enum FloatAction {
    /// Stash all floating panes, including those hidden by zoom.
    Stash { target: Option<String> },
    /// Restore stashed floating panes.
    Unstash { target: Option<String> },
    /// Stash when any floats exist, otherwise unstash.
    Toggle { target: Option<String> },
}

/// Optional output level; without a flag the focused column determines it.
#[derive(Debug, Args)]
#[group(id = "level", multiple = false)]
pub struct PickerArgs {
    #[arg(long)]
    pub session: bool,
    #[arg(long)]
    pub window: bool,
    #[arg(long)]
    pub pane: bool,
}

impl PickerArgs {
    /// Convert CLI switches into the picker model's output level.
    pub fn level(&self) -> Option<Level> {
        if self.session {
            Some(Level::Session)
        } else if self.window {
            Some(Level::Window)
        } else if self.pane {
            Some(Level::Pane)
        } else {
            None
        }
    }
}
