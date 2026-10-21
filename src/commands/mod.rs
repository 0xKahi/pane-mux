//! One module per subcommand.

pub mod picker;
pub mod switch_session;

/// Process outcome after the terminal has been restored.
pub enum Outcome {
    Done,
    Cancelled,
}
