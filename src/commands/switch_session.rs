//! Switch the invoking tmux client with one switch-client call.

use super::Outcome;
use crate::{
    config::Loaded,
    picker::{self, Exit, PickerOptions, SessionOrder},
    tmux::Tmux,
};
use anyhow::{Result, ensure};

/// Require tmux before opening the picker; cancellation never switches clients.
pub fn run(tmux: &dyn Tmux, loaded: &Loaded) -> Result<Outcome> {
    ensure!(
        std::env::var_os("TMUX").is_some(),
        "switch-session must run inside tmux"
    );
    let options = PickerOptions {
        level: None,
        header: "[Switch]".into(),
        order: SessionOrder::Recent,
        recency_exclude: loaded.config.switch_session.exclude_from_recency.clone(),
        ..PickerOptions::default()
    };
    match picker::run(options, tmux, loaded)? {
        Exit::Pick(target) => {
            tmux.switch_client(&target)?;
            Ok(Outcome::Done)
        }
        Exit::Cancel => Ok(Outcome::Cancelled),
        Exit::Moved => Ok(Outcome::Done),
    }
}
