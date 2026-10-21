//! Config discovery, presets and merging.

pub mod keymap;
pub mod loader;
pub mod merge;
pub mod paths;
pub mod preset;
pub mod settings;
pub mod theme;

pub use loader::{Loaded, load, load_from};
pub use settings::{Config, PickerConfig, PreviewPosition};
pub use theme::{StyleDef, Theme};
