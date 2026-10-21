//! Config discovery, presets and merging.

pub mod keymap;
pub mod merge;
pub mod paths;
pub mod preset;
pub mod settings;
pub mod theme;

pub use settings::{Config, PickerConfig, PreviewPosition};
pub use theme::{StyleDef, Theme};
