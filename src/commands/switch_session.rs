//! Switch the invoking tmux client with one switch-client call.

use super::Outcome;
use crate::{
    config::Loaded,
    picker::{self, PickerOptions, SessionOrder},
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
    };
    match picker::run(options, tmux, loaded)? {
        Some(target) => {
            tmux.switch_client(&target)?;
            Ok(Outcome::Done)
        }
        None => Ok(Outcome::Cancelled),
    }
}
