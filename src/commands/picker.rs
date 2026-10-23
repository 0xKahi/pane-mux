//! Pick a target, then print only its final value on stdout.

use super::Outcome;
use crate::{
    config::Loaded,
    picker::{self, PickerOptions, SessionOrder},
    tmux::{Level, Tmux},
};
use anyhow::Result;

/// Open the picker and print the result after terminal cleanup.
pub fn run(level: Option<Level>, tmux: &dyn Tmux, loaded: &Loaded) -> Result<Outcome> {
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
            recency_exclude: Vec::new(),
        },
        tmux,
        loaded,
    )? {
        Some(target) => {
            println!("{target}");
            Ok(Outcome::Done)
        }
        None => Ok(Outcome::Cancelled),
    }
}
