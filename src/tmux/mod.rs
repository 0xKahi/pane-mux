//! All tmux access goes through this module.

pub mod client;
pub mod fake;
pub mod format;
pub mod model;

pub use client::{FloatMux, TmuxCli, suppressed_pane};
pub use fake::{FakeTmux, sample_tree};
