//! Options shared by picker commands and state.

use crate::tmux::Level;

/// Output specificity and the label displayed above the picker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickerOptions {
    pub level: Option<Level>,
    pub header: String,
}
