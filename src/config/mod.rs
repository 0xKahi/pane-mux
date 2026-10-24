//! Config discovery, presets and merging.

pub mod keymap;
pub mod loader;
pub mod merge;
pub mod paths;
pub mod preset;
pub mod settings;
pub mod theme;

// Test-only derives keep editor schemas strict without adding runtime dependencies.
#[cfg(test)]
mod schema;

pub use loader::{Loaded, load, load_from};
pub use settings::{
    Config, FloatingPaneConfig, FloatingPosition, PickerConfig, PreviewPosition,
    SwitchSessionConfig,
};
pub use theme::{StyleDef, Theme};
