//! Backend-neutral multiplexer types and operations.

pub mod backend;
pub mod model;

pub use backend::Mux;

pub use model::{Current, Level, Pane, Session, Target, Tree, Window};

use crate::tmux::{FloatMux, TmuxCli};

/// Open the active backend. The add-rex-backend change will extend this with detection.
pub fn open() -> anyhow::Result<Box<dyn FloatMux>> {
    Ok(Box::new(TmuxCli::default()))
}
