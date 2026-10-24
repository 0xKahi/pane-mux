//! Typed picker settings layered over the preset.

use anyhow::{Context, ensure};
use serde::Deserialize;

use super::merge;

/// Application settings from config.toml.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct Config {
    pub picker: PickerConfig,
    #[serde(rename = "switch-session")]
    pub switch_session: SwitchSessionConfig,
    #[serde(rename = "floating-pane")]
    pub floating_pane: FloatingPaneConfig,
}

/// Floating pane dimensions as percentages of the target window.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct FloatingPaneConfig {
    #[cfg_attr(test, schemars(range(min = 1, max = 100)))]
    pub width: u8,
    #[cfg_attr(test, schemars(range(min = 1, max = 100)))]
    pub height: u8,
    pub position: FloatingPosition,
}

/// Named positions accepted by tmux's move-pane -P option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FloatingPosition {
    TopLeft,
    TopCentre,
    TopRight,
    CentreLeft,
    Centre,
    CentreRight,
    BottomLeft,
    BottomCentre,
    BottomRight,
    TopLeftCentre,
    TopRightCentre,
    BottomLeftCentre,
    BottomRightCentre,
}

impl std::fmt::Display for FloatingPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TopLeft => "top-left",
            Self::TopCentre => "top-centre",
            Self::TopRight => "top-right",
            Self::CentreLeft => "centre-left",
            Self::Centre => "centre",
            Self::CentreRight => "centre-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomCentre => "bottom-centre",
            Self::BottomRight => "bottom-right",
            Self::TopLeftCentre => "top-left-centre",
            Self::TopRightCentre => "top-right-centre",
            Self::BottomLeftCentre => "bottom-left-centre",
            Self::BottomRightCentre => "bottom-right-centre",
        })
    }
}

/// Session recency ranking settings for switch-session.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct SwitchSessionConfig {
    pub exclude_from_recency: Vec<String>,
}

/// Preview sizing and placement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct PickerConfig {
    #[cfg_attr(test, schemars(range(min = 1, max = 99)))]
    pub preview_ratio: u16,
    pub preview_position: PreviewPosition,
}

/// Which side of the columns contains the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
        ensure!(
            (1..=100).contains(&self.floating_pane.width),
            "floating-pane width must be between 1 and 100"
        );
        ensure!(
            (1..=100).contains(&self.floating_pane.height),
            "floating-pane height must be between 1 and 100"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::preset;

    #[test]
    fn floating_pane_defaults_and_custom_values() {
        let default = Config::from_toml(&preset::config(), None).unwrap();
        assert_eq!(
            default.floating_pane,
            FloatingPaneConfig {
                width: 50,
                height: 50,
                position: FloatingPosition::Centre,
            }
        );
        let custom = Config::from_toml(
            &preset::config(),
            Some("[floating-pane]\nwidth = 80\nheight = 30\nposition = 'bottom-centre'"),
        )
        .unwrap();
        assert_eq!(
            custom.floating_pane,
            FloatingPaneConfig {
                width: 80,
                height: 30,
                position: FloatingPosition::BottomCentre,
            }
        );
    }

    #[test]
    fn floating_positions_round_trip_to_tmux_arguments() {
        for position in [
            "top-left",
            "top-centre",
            "top-right",
            "centre-left",
            "centre",
            "centre-right",
            "bottom-left",
            "bottom-centre",
            "bottom-right",
            "top-left-centre",
            "top-right-centre",
            "bottom-left-centre",
            "bottom-right-centre",
        ] {
            let user = format!("[floating-pane]\nposition = '{position}'");
            let config = Config::from_toml(&preset::config(), Some(&user)).unwrap();
            assert_eq!(config.floating_pane.position.to_string(), position);
        }
    }

    #[test]
    fn floating_pane_sizes_validate_bounds() {
        for key in ["width", "height"] {
            for value in [0, 101, 255] {
                let user = format!("[floating-pane]\n{key} = {value}");
                let error = Config::from_toml(&preset::config(), Some(&user)).unwrap_err();
                assert!(format!("{error:#}").contains(key));
            }
            for value in [1, 100] {
                let user = format!("[floating-pane]\n{key} = {value}");
                Config::from_toml(&preset::config(), Some(&user)).unwrap();
            }
        }
    }

    #[test]
    fn invalid_floating_position_names_setting() {
        let error = Config::from_toml(
            &preset::config(),
            Some("[floating-pane]\nposition = 'middle'"),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("position"));
    }

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
    fn unknown_switch_session_key_is_ignored() {
        let config = Config::from_toml(
            &preset::config(),
            Some("[switch-session]\nsurprise = true\nexclude_from_recency = ['_popup']"),
        )
        .unwrap();
        assert_eq!(config.switch_session.exclude_from_recency, ["_popup"]);
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
    fn unknown_config_sections_and_keys_are_ignored() {
        let config = Config::from_toml(
            &preset::config(),
            Some("[future-feature]\nenabled = true\n[picker]\nsurprise = true\npreview_ratio = 70"),
        )
        .unwrap();
        assert_eq!(config.picker.preview_ratio, 70);
    }
}
