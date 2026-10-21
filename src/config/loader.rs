//! Optional user files layered over validated built-in presets.

use std::{io::ErrorKind, path::Path};

use anyhow::{Context, Result};

use super::{Config, Theme, keymap::Keymap, paths, preset};

/// Fully loaded settings, bindings, and appearance for the picker.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub config: Config,
    pub keymap: Keymap,
    pub theme: Theme,
}

/// Load user files from the environment-selected configuration directory.
pub fn load() -> Result<Loaded> {
    load_from(paths::config_dir().as_deref())
}

/// Load optional files from a directory, or only presets when no directory is given.
pub fn load_from(dir: Option<&Path>) -> Result<Loaded> {
    let config_preset = preset::config();
    let keymap_preset = preset::keymap();
    let theme_preset = preset::theme();

    // Validate presets separately so a broken preset is never blamed on a user file.
    let mut config =
        Config::from_toml(&config_preset, None).context("preset config-default.toml")?;
    let mut keymap =
        Keymap::from_toml(&keymap_preset, None).context("preset keymap-default.toml")?;
    let mut theme = Theme::from_toml(&theme_preset, None).context("preset theme-default.toml")?;

    if let Some(dir) = dir {
        let dir = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            std::env::current_dir()
                .context("cannot resolve configuration directory")?
                .join(dir)
        };
        let path = dir.join("config.toml");
        if let Some(text) = read_optional(&path)? {
            config = Config::from_toml(&config_preset, Some(&text))
                .with_context(|| path.display().to_string())?;
        }
        let path = dir.join("keymap.toml");
        if let Some(text) = read_optional(&path)? {
            keymap = Keymap::from_toml(&keymap_preset, Some(&text))
                .with_context(|| path.display().to_string())?;
        }
        let path = dir.join("theme.toml");
        if let Some(text) = read_optional(&path)? {
            theme = Theme::from_toml(&theme_preset, Some(&text))
                .with_context(|| path.display().to_string())?;
        }
    }
    Ok(Loaded {
        config,
        keymap,
        theme,
    })
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| path.display().to_string()),
    }
}
