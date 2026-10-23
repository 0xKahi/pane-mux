//! Typed picker settings layered over the preset.

use anyhow::{Context, ensure};
use serde::Deserialize;

use super::merge;

/// Application settings from config.toml.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub picker: PickerConfig,
    #[serde(rename = "switch-session")]
    pub switch_session: SwitchSessionConfig,
}

/// Session recency ranking settings for switch-session.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwitchSessionConfig {
    pub exclude_from_recency: Vec<String>,
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
    fn default_switch_session_exclusions_are_empty() {
        let config = Config::from_toml(&preset::config(), None).unwrap();
        assert!(config.switch_session.exclude_from_recency.is_empty());
    }

    #[test]
    fn user_switch_session_exclusions_replace_default() {
        let config = Config::from_toml(
            &preset::config(),
            Some("[switch-session]\nexclude_from_recency = [\"_popup\", \"scratch\"]"),
        )
        .unwrap();
        assert_eq!(
            config.switch_session.exclude_from_recency,
            ["_popup", "scratch"]
        );
    }

    #[test]
    fn invalid_switch_session_exclusions_name_setting() {
        let error = Config::from_toml(
            &preset::config(),
            Some("[switch-session]\nexclude_from_recency = \"_popup\""),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("exclude_from_recency"));
    }

    #[test]
    fn unknown_switch_session_key_errors() {
        let error = Config::from_toml(&preset::config(), Some("[switch-session]\nsurprise = true"))
            .unwrap_err();
        assert!(format!("{error:#}").contains("surprise"));
    }

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
