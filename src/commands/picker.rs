//! Pick a target, then print only its final value on stdout.

use super::Outcome;
use crate::{
    config::Loaded,
    mux::{Level, Mux},
    picker::{self, Exit, PickerOptions, SessionOrder},
};
use anyhow::Result;

/// Open the picker and print the result after terminal cleanup.
pub fn run(level: Option<Level>, tmux: &dyn Mux, loaded: &Loaded) -> Result<Outcome> {
    let header = match level {
        Some(Level::Session) => "[Pick session]",
        Some(Level::Window) => "[Pick window]",
        Some(Level::Pane) => "[Pick pane]",
        None => "[Pick]",
    };
    match picker::run(
        PickerOptions {
            level,
            header: header.into(),
            order: SessionOrder::Name,
            ..PickerOptions::default()
        },
        tmux,
        loaded,
    )? {
        Exit::Pick {
            target,
            level,
            tree,
        } => {
            println!("{}", tmux.target_string(&tree, &target, level));
            Ok(Outcome::Done)
        }
        Exit::Cancel => Ok(Outcome::Cancelled),
        Exit::Moved => Ok(Outcome::Done),
    }
}
