//! All tmux access goes through this module.

pub mod client;
pub mod fake;
pub mod format;
pub mod model;

pub use client::{Tmux, TmuxCli, suppressed_pane, suppressed_pane_from_env};
pub use fake::{FakeTmux, sample_tree};
pub use model::{Current, Level, Pane, Session, Target, Tree, Window};
