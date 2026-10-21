//! Rendering. Pure functions of `&State`; no tmux calls, no mutation.

use ratatui::{Frame, layout::Rect};

use super::state::State;
use crate::config::{PickerConfig, Theme};

/// Heights the state needs for paging/scrolling, computed from the same layout as `render`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heights {
    /// Rows available for items in one column (inside borders).
    pub list: u16,
    /// Rows available for preview text (inside borders).
    pub preview: u16,
}

/// Compute `Heights` for a terminal area. The event loop copies these into `State` before drawing.
pub fn heights(area: Rect, config: &PickerConfig) -> Heights {
    let _ = (area, config);
    Heights { list: 10, preview: 10 }
}

/// Draw the whole picker.
pub fn render(frame: &mut Frame, state: &State, theme: &Theme, config: &PickerConfig) {
    let _ = (frame, state, theme, config);
}
