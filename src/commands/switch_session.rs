//! Switch the invoking tmux client with one switch-client call.

use super::Outcome;
use crate::{
    config::Loaded,
    mux::Mux,
    picker::{self, Exit, PickerOptions, SessionOrder},
};
use anyhow::{Result, ensure};

/// Require tmux before opening the picker; cancellation never switches clients.
pub fn run(tmux: &dyn Mux, loaded: &Loaded) -> Result<Outcome> {
    ensure!(tmux.in_client(), "switch-session must run inside tmux");
    let options = PickerOptions {
        level: None,
        header: "[Switch]".into(),
        order: SessionOrder::Recent,
        recency_exclude: loaded.config.switch_session.exclude_from_recency.clone(),
        ..PickerOptions::default()
    };
    match picker::run(options, tmux, loaded)? {
        Exit::Pick {
            target,
            level,
            tree,
        } => {
            tmux.switch_client(&tmux.target_string(&tree, &target, level))?;
            Ok(Outcome::Done)
        }
        Exit::Cancel => Ok(Outcome::Cancelled),
        Exit::Moved => Ok(Outcome::Done),
    }
}
