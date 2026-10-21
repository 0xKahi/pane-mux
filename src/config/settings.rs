//! Typed picker settings layered over the preset.

use anyhow::{Context, ensure};
use serde::Deserialize;

use super::merge;

/// Application settings from config.toml.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub picker: PickerConfig,
}

/// Preview sizing and placement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickerConfig {
    pub preview_ratio: u16,
    pub preview_position: PreviewPosition,
}

/// Which side of the columns contains the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewPosition {
    Top,
    Bottom,
}

impl Config {
    /// Deep-merge TOML and validate the resulting settings.
    pub fn from_toml(preset: &str, user: Option<&str>) -> anyhow::Result<Self> {
        let config: Self = merge::from_toml(preset, user)?
            .try_into()
            .context("invalid config settings")?;
        config.validate()?;
        Ok(config)
    }

    /// Ensure preview sizing leaves room for both the columns and preview.
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            (1..=99).contains(&self.picker.preview_ratio),
            "preview_ratio must be between 1 and 99"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::preset;

    #[test]
    fn partial_override_keeps_default_ratio() {
        let config = Config::from_toml(
            &preset::config(),
            Some("[picker]\npreview_position = 'top'"),
        )
        .unwrap();
        assert_eq!(config.picker.preview_ratio, 50);
        assert_eq!(config.picker.preview_position, PreviewPosition::Top);
    }

    #[test]
    fn invalid_ratio_names_preview_ratio() {
        for ratio in [0, 150] {
            let user = format!("[picker]\npreview_ratio = {ratio}");
            let error = Config::from_toml(&preset::config(), Some(&user)).unwrap_err();
            assert!(format!("{error:#}").contains("preview_ratio"));
        }
    }

    #[test]
    fn unknown_config_key_errors() {
        let error =
            Config::from_toml(&preset::config(), Some("[picker]\nsurprise = true")).unwrap_err();
        assert!(format!("{error:#}").contains("surprise"));
    }
}
