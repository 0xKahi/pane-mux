//! Resolve a source pane before opening the locked move picker.

use super::Outcome;
use crate::{
    config::Loaded,
    picker::{self, Exit, PickerOptions},
    tmux::Tmux,
};
use anyhow::{Context, Result, ensure};

/// Resolve the source and construct options without touching the terminal.
pub fn options(target: Option<&str>, tmux: &dyn Tmux, inside_tmux: bool) -> Result<PickerOptions> {
    let pane_id = match target {
        Some(target) => tmux
            .resolve_pane(target)
            .with_context(|| format!("pane not found: {target}"))?,
        None => {
            ensure!(inside_tmux, "move-pane requires a target outside tmux");
            tmux.current()?
                .context("could not determine current pane")?
                .pane_id
        }
    };
    Ok(PickerOptions {
        header: "[Move]".into(),
        initial_pane: Some(pane_id.clone()),
        cut: Some(pane_id),
        cut_locked: true,
        ..PickerOptions::default()
    })
}

/// Only a completed paste is successful; cancelling never moves anything.
pub fn run(target: Option<&str>, tmux: &dyn Tmux, loaded: &Loaded) -> Result<Outcome> {
    let options = options(target, tmux, std::env::var_os("TMUX").is_some())?;
    Ok(match picker::run(options, tmux, loaded)? {
        Exit::Moved => Outcome::Done,
        Exit::Cancel | Exit::Pick(_) => Outcome::Cancelled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{Current, FakeTmux, sample_tree};

    #[test]
    fn resolves_explicit_or_current_pane_before_opening_picker() {
        let fake = FakeTmux::new(sample_tree(&["work"])).with_current(Current {
            session_id: "$0".into(),
            window_id: "@0".into(),
            pane_id: "%0".into(),
        });
        let explicit = options(Some("work:0.0"), &fake, false).unwrap();
        assert_eq!(explicit.cut.as_deref(), Some("%0"));
        assert_eq!(explicit.initial_pane.as_deref(), Some("%0"));
        assert!(explicit.cut_locked);
        assert_eq!(explicit.header, "[Move]");
        assert_eq!(explicit.order, picker::SessionOrder::Name);
        assert_eq!(options(None, &fake, true).unwrap(), explicit);
        assert!(
            options(None, &fake, false)
                .unwrap_err()
                .to_string()
                .contains("target outside tmux")
        );
        assert!(
            options(Some("missing:9.9"), &fake, false)
                .unwrap_err()
                .to_string()
                .contains("pane not found")
        );
    }
}
