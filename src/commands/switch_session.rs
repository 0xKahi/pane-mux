//! Switch the invoking multiplexer client.

use super::Outcome;
use crate::{
    config::Loaded,
    mux::{BackendKind, Mux},
    picker::{self, Exit, PickerOptions, SessionOrder},
};
use anyhow::{Result, ensure};

/// Require a supported client before opening the picker; cancellation never switches clients.
pub fn run(tmux: &dyn Mux, loaded: &Loaded) -> Result<Outcome> {
    ensure!(
        tmux.in_client(),
        match tmux.kind() {
            BackendKind::Rex => "switch-session must run inside a Rex block",
            BackendKind::Tmux => "switch-session must run inside tmux or Rex",
        }
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, tmux::FakeTmux};

    #[test]
    fn rex_requires_a_block_before_opening_picker() {
        let fake = FakeTmux::new(Default::default()).with_kind(BackendKind::Rex);
        let loaded = config::load_from(None).unwrap();
        let error = run(&fake, &loaded).err().expect("outside Rex must fail");
        assert_eq!(
            error.to_string(),
            "switch-session must run inside a Rex block"
        );
        assert!(fake.calls().is_empty());
    }
}
