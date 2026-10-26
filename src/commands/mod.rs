//! One module per subcommand.

pub mod float;
pub mod move_pane;
pub mod picker;
pub mod switch_session;

/// Process outcome after the terminal has been restored.
pub enum Outcome {
    Done,
    Cancelled,
}
