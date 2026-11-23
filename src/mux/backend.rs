//! Core operations shared by multiplexer backends.

use std::time::Duration;

use anyhow::Result;

use super::{
    BackendKind,
    model::{Current, Level, Target, Tree},
};

/// Operations needed by the picker, switch-session, and move-pane commands.
/// Mutations address items by id.
pub trait Mux {
    fn tree(&self) -> Result<Tree>;
    /// Format a target for command output and client switching.
    fn target_string(&self, tree: &Tree, target: &Target, level: Level) -> String;
    /// Format a target for display in the picker.
    fn display_target(&self, tree: &Tree, target: &Target, level: Level) -> String;
    /// Which multiplexer this backend drives, for backend-specific wording.
    fn kind(&self) -> BackendKind;
    /// How often the picker re-captures an unchanged preview.
    fn preview_interval(&self) -> Duration {
        Duration::from_millis(300)
    }
    /// Whether panes can be moved between sessions.
    fn cross_session_moves(&self) -> bool {
        true
    }
    fn in_client(&self) -> bool;
    fn suppressed_pane(&self) -> Option<String>;
    fn capture(&self, pane_id: &str) -> Result<String>;
    fn current(&self) -> Result<Option<Current>>;
    fn resolve_pane(&self, target: &str) -> Result<String>;
    fn move_pane_to_new_session(
        &self,
        pane_id: &str,
        session_name: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<(String, String)>;
    fn move_pane_to_new_window(
        &self,
        pane_id: &str,
        session_id: &str,
        name: Option<&str>,
    ) -> Result<String>;
    fn float_pane_in_window(
        &self,
        pane_id: &str,
        window_id: &str,
        floating: &crate::config::settings::FloatingPaneConfig,
    ) -> Result<()>;
    fn new_session(&self, name: Option<&str>) -> Result<String>;
    fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String>;
    fn rename_session(&self, session_id: &str, name: &str) -> Result<()>;
    fn rename_window(&self, window_id: &str, name: &str) -> Result<()>;
    fn kill_session(&self, session_id: &str) -> Result<()>;
    fn kill_window(&self, window_id: &str) -> Result<()>;
    fn kill_pane(&self, pane_id: &str) -> Result<()>;
    fn switch_client(&self, target: &str) -> Result<()>;
}
