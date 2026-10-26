//! Owned snapshots of tmux sessions, windows, and panes.

/// A consistent snapshot from one tmux query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    pub sessions: Vec<Session>,
}

/// A tmux session, addressed internally by its stable `$` id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub last_attached: u64,
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
