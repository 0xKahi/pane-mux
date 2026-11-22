//! Detailed tmux state for floating-pane commands.

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
