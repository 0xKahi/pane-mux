//! Editable presets in debug builds; embedded assets in release builds.

use std::borrow::Cow;

macro_rules! config_preset {
    ($name:literal) => {{
        #[cfg(debug_assertions)]
        {
            Cow::Owned(
                std::fs::read_to_string(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/config/preset/",
                    $name,
                    "-default.toml"
                ))
                .expect(concat!("Failed to read ", $name, " preset")),
            )
        }
        #[cfg(not(debug_assertions))]
        {
            Cow::Borrowed(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/config/preset/",
                $name,
                "-default.toml"
            )))
        }
    }};
}

/// Built-in picker settings.
pub fn config() -> Cow<'static, str> {
    config_preset!("config")
}

/// Built-in key bindings.
pub fn keymap() -> Cow<'static, str> {
    config_preset!("keymap")
}

/// Built-in appearance settings.
pub fn theme() -> Cow<'static, str> {
    config_preset!("theme")
}
