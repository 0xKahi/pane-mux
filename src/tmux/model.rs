//! Owned snapshots of tmux sessions, windows, and panes.

use std::collections::HashMap;

/// A consistent snapshot from one tmux query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    pub sessions: Vec<Session>,
}

impl Tree {
    /// Count floating stash contents by the stable id of their source window.
    pub fn stash_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for session in &self.sessions {
            if session.name != "pmux" || !session.stash {
                continue;
            }
            for window in &session.windows {
                if let Some(source) = &window.stash_source {
                    let count = window.panes.iter().filter(|pane| pane.floating).count();
                    if count > 0 {
                        *counts.entry(source.clone()).or_default() += count;
                    }
                }
            }
        }
        counts
    }
}

/// A tmux session, addressed internally by its stable `$` id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub last_attached: u64,
    pub stash: bool,
    pub windows: Vec<Window>,
}

impl Session {
    /// Return the active window, falling back to the first window.
    pub fn active_window(&self) -> Option<&Window> {
        self.windows
            .iter()
            .find(|window| window.active)
            .or(self.windows.first())
    }
}

/// A window within a session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Window {
    pub id: String,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub zoomed: bool,
    pub stash_source: Option<String>,
    pub panes: Vec<Pane>,
}

impl Window {
    /// Return the active pane, falling back to the first pane.
    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes
            .iter()
            .find(|pane| pane.active)
            .or(self.panes.first())
    }
}

/// A pane and its foreground command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pane {
    pub id: String,
    pub index: u32,
    pub active: bool,
    pub command: String,
    pub floating: bool,
    pub mode: String,
}

impl Pane {
    /// Copy and read-only view modes share the picker's copy-mode indicator.
    pub fn in_copy_mode(&self) -> bool {
        matches!(self.mode.as_str(), "copy-mode" | "view-mode")
    }
}

/// Detailed pane state for floating-pane commands. Options travel with the pane.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FloatPane {
    pub id: String,
    pub title: String,
    pub flags: String,
    pub floating: bool,
    pub active: bool,
    pub z: i32,
    pub width: i32,
    pub height: i32,
    pub left: i32,
    pub top: i32,
    pub options: std::collections::BTreeMap<String, String>,
}

/// Detailed snapshot of a single window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FloatWindow {
    pub width: i32,
    pub height: i32,
    pub zoomed: bool,
    pub panes: Vec<FloatPane>,
}

/// Border-inclusive floating geometry accepted by new-pane.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FloatGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The specificity of a printed tmux target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Session,
    Window,
    Pane,
}

/// A human-readable target suitable for tmux's `-t` argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub session: String,
    pub window_index: u32,
    pub pane_index: u32,
}

impl Target {
    /// Format the target at the requested level.
    pub fn at(&self, level: Level) -> String {
        match level {
            Level::Session => self.session.clone(),
            Level::Window => format!("{}:{}", self.session, self.window_index),
            Level::Pane => format!("{}:{}.{}", self.session, self.window_index, self.pane_index),
        }
    }
}

/// Stable ids of the invoking tmux client's current items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Current {
    pub session_id: String,
    pub window_id: String,
    pub pane_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_and_view_modes_only() {
        for (mode, expected) in [
            ("copy-mode", true),
            ("view-mode", true),
            ("clock-mode", false),
            ("", false),
        ] {
            let pane = Pane {
                mode: mode.into(),
                ..Default::default()
            };
            assert_eq!(pane.in_copy_mode(), expected, "{mode}");
        }
    }

    #[test]
    fn stash_counts_only_owned_stash_floats() {
        let float = Pane {
            floating: true,
            ..Default::default()
        };
        let window = Window {
            stash_source: Some("@3".into()),
            panes: vec![Pane::default(), float.clone(), float],
            ..Default::default()
        };
        let mut tree = Tree {
            sessions: vec![Session {
                name: "pmux".into(),
                stash: true,
                windows: vec![window.clone()],
                ..Default::default()
            }],
        };
        assert_eq!(tree.stash_counts().get("@3"), Some(&2));
        tree.sessions[0].windows[0].panes.remove(0);
        assert_eq!(tree.stash_counts().get("@3"), Some(&2));
        tree.sessions[0].windows.push(window);
        assert_eq!(tree.stash_counts().get("@3"), Some(&4));
        tree.sessions[0].stash = false;
        assert!(tree.stash_counts().is_empty());
        tree.sessions[0].stash = true;
        tree.sessions[0].name = "work".into();
        assert!(tree.stash_counts().is_empty());
    }

    #[test]
    fn target_formatting() {
        let target = Target {
            session: "work".into(),
            window_index: 1,
            pane_index: 0,
        };
        assert_eq!(target.at(Level::Session), "work");
        assert_eq!(target.at(Level::Window), "work:1");
        assert_eq!(target.at(Level::Pane), "work:1.0");
    }
}
