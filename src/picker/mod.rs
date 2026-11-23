//! Reusable TUI picker.

use std::time::Duration;

const REFRESH_TICK: Duration = Duration::from_millis(300);

pub mod action;
pub mod app;
pub mod finder;
pub mod options;
pub mod popup;
pub mod state;
pub mod terminal;
pub mod ui;
pub mod which;

#[cfg(test)]
mod test_fixtures;

pub use app::run;
pub use options::{PickerOptions, SessionOrder};
pub use popup::{InputPurpose, Popup};
pub use state::{Exit, Preview, State};
pub use which::Which;
