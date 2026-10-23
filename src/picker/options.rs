//! Options shared by picker commands and state.

use crate::tmux::Level;

/// Session ordering chosen by the command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SessionOrder {
    #[default]
    Name,
    Recent,
}

/// Output specificity and the label displayed above the picker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickerOptions {
    pub level: Option<Level>,
    pub header: String,
    pub order: SessionOrder,
    pub recency_exclude: Vec<String>,
}
