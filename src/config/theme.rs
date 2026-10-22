//! Theme styles and icons, layered over the built-in TOML theme.

use anyhow::Context;
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

use super::merge;

/// Optional style components; omitted components leave the terminal style unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleDef {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub reversed: Option<bool>,
    pub dim: Option<bool>,
}

impl From<&StyleDef> for Style {
    fn from(def: &StyleDef) -> Self {
        let mut style = Style::default();
        if let Some(fg) = def.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = def.bg {
            style = style.bg(bg);
        }
        for (enabled, modifier) in [
            (def.bold, Modifier::BOLD),
            (def.italic, Modifier::ITALIC),
            (def.underline, Modifier::UNDERLINED),
            (def.reversed, Modifier::REVERSED),
            (def.dim, Modifier::DIM),
        ] {
            // Explicit false removes an inherited modifier; None leaves it alone.
            match enabled {
                Some(true) => style = style.add_modifier(modifier),
                Some(false) => style = style.remove_modifier(modifier),
                None => {}
            }
        }
        style
    }
}

/// All appearance settings from theme.toml.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub picker: PickerTheme,
    pub preview: PreviewTheme,
    pub icons: Icons,
    pub indicator: IndicatorTheme,
    pub mode: ModeTheme,
    pub status: StatusTheme,
    pub which: WhichTheme,
    pub confirm: ConfirmTheme,
    pub input: InputTheme,
    pub help: HelpTheme,
    pub notify: NotifyTheme,
}

impl Theme {
    /// Deep-merge the optional user theme and deserialize all styles.
    pub fn from_toml(preset: &str, user: Option<&str>) -> anyhow::Result<Self> {
        merge::from_toml(preset, user)?
            .try_into()
            .context("invalid theme settings")
    }
}

/// Tree column appearance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickerTheme {
    pub border: StyleDef,
    pub session: StyleDef,
    pub window: StyleDef,
    pub pane: StyleDef,
    pub header: StyleDef,
}

/// Pane preview frame appearance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewTheme {
    pub border: StyleDef,
    pub title: StyleDef,
}

/// Glyph and colour for a tree level. Empty text deliberately hides the icon.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icon {
    pub text: String,
    pub fg: Option<Color>,
}

/// Icons for the three tmux tree levels.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icons {
    pub session: Icon,
    pub window: Icon,
    pub pane: Icon,
}

/// Hovered-row caps and the current-item marker.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndicatorTheme {
    pub parent: StyleDef,
    pub current: StyleDef,
    pub preview: StyleDef,
    pub padding: IndicatorPadding,
    pub active: ActiveIndicator,
}

/// Cap text; either omitted side retains its default glyph.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndicatorPadding {
    pub open: String,
    pub close: String,
}

impl Default for IndicatorPadding {
    fn default() -> Self {
        Self {
            open: "\u{e0b6}".into(),
            close: "\u{e0b4}".into(),
        }
    }
}

/// Marker text with the same optional colour/modifier fields as other styles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveIndicator {
    pub text: String,
    #[serde(flatten)]
    pub style: StyleDef,
}

/// Active layer label in the status line.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeTheme {
    pub normal: StyleDef,
    pub input: StyleDef,
    pub confirm: StyleDef,
}

/// Status line background and informational text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusTheme {
    pub overall: StyleDef,
    pub label: StyleDef,
    pub message: StyleDef,
}

/// Multi-key continuation popup.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhichTheme {
    pub border: StyleDef,
    pub key: StyleDef,
    pub desc: StyleDef,
    pub separator: StyleDef,
}

/// Destructive-action confirmation popup.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmTheme {
    pub border: StyleDef,
    pub title: StyleDef,
    pub body: StyleDef,
}

/// Name entry popup.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputTheme {
    pub border: StyleDef,
    pub title: StyleDef,
    pub value: StyleDef,
}

/// Key-binding help popup.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelpTheme {
    pub border: StyleDef,
    pub title: StyleDef,
    pub on: StyleDef,
    pub run: StyleDef,
    pub desc: StyleDef,
    pub hovered: StyleDef,
}

/// Non-fatal error messages.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotifyTheme {
    pub error: StyleDef,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::preset;

    #[test]
    fn partial_preview_override_changes_only_border_colour() {
        let mut expected = Theme::from_toml(&preset::theme(), None).unwrap();
        expected.preview.border.fg = Some(Color::Rgb(0x12, 0x34, 0x56));
        let actual = Theme::from_toml(
            &preset::theme(),
            Some("[preview]\nborder = { fg = '#123456' }"),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn custom_icon_keeps_colour_and_other_icons() {
        let mut expected = Theme::from_toml(&preset::theme(), None).unwrap();
        expected.icons.session.text = "S".to_owned();
        let actual =
            Theme::from_toml(&preset::theme(), Some("[icons]\nsession = { text = 'S' }")).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn partial_indicator_override_preserves_other_defaults() {
        let mut expected = Theme::from_toml(&preset::theme(), None).unwrap();
        expected.indicator.padding.open = "<".into();
        expected.indicator.active.style.bold = Some(true);
        let actual = Theme::from_toml(
            &preset::theme(),
            Some("[indicator]\npadding = { open = '<' }\nactive = { bold = true }"),
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.indicator.padding.close, "\u{e0b4}");
        assert_eq!(actual.indicator.active.text, "*");
        assert_eq!(actual.indicator.active.style.fg, Some(Color::Magenta));
        let padding: IndicatorPadding = toml::from_str("open = '<'").unwrap();
        assert_eq!(padding.close, "\u{e0b4}");
    }

    #[test]
    fn partial_indicator_preview_override_keeps_other_roles() {
        let mut expected = Theme::from_toml(&preset::theme(), None).unwrap();
        expected.indicator.preview.fg = Some(Color::Yellow);
        let actual = Theme::from_toml(
            &preset::theme(),
            Some("[indicator]\npreview = { fg = 'yellow' }"),
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.indicator.preview.underline, Some(true));
        assert_eq!(actual.indicator.parent.reversed, Some(true));
        assert_eq!(actual.indicator.current.reversed, Some(true));
    }

    #[test]
    fn invalid_colour_errors() {
        assert!(
            Theme::from_toml(
                &preset::theme(),
                Some("[preview]\nborder = { fg = 'not-a-colour' }")
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_theme_key_errors() {
        let error = Theme::from_toml(&preset::theme(), Some("[input]\nunknown = {} ")).unwrap_err();
        assert!(format!("{error:#}").contains("unknown"));
    }

    #[test]
    fn named_hex_and_reset_colours_deserialize() {
        let def: StyleDef = toml::from_str("fg = 'lightgreen'\nbg = 'reset'").unwrap();
        assert_eq!(def.fg, Some(Color::LightGreen));
        assert_eq!(def.bg, Some(Color::Reset));
        let def: StyleDef = toml::from_str("fg = '#abcdef'").unwrap();
        assert_eq!(def.fg, Some(Color::Rgb(0xab, 0xcd, 0xef)));
    }

    #[test]
    fn modifiers_preserve_explicit_false() {
        let def = StyleDef {
            bold: Some(true),
            italic: Some(true),
            underline: Some(true),
            reversed: Some(true),
            dim: Some(false),
            ..StyleDef::default()
        };
        let style = Style::from(&def);
        assert!(style.add_modifier.contains(
            Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::REVERSED
        ));
        assert!(style.sub_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn default_icons_are_nonempty() {
        let theme = Theme::from_toml(&preset::theme(), None).unwrap();
        assert!(!theme.icons.session.text.is_empty());
        assert_eq!(theme.icons.window.text, "󱂬");
        assert!(!theme.icons.pane.text.is_empty());
    }
}
