//! Loader checks use injected paths, never process-wide environment mutation.

use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use pane_mux::config::{self, Config, PreviewPosition, Theme, keymap::Keymap, paths, preset};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pane-mux-config-{}-{timestamp}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
    }
}

// Drop runs when the test leaves scope, including during a failed assertion.
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn invalid_keymap_toml_names_full_path_and_parse_problem() {
    let dir = TempDir::new();
    dir.write("keymap.toml", "[picker\n");
    let error = config::load_from(Some(&dir.0)).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains(&dir.0.join("keymap.toml").display().to_string()));
    assert!(message.contains("TOML parse error"), "{message}");
}

#[test]
fn missing_directory_uses_defaults() {
    let dir = TempDir::new();
    let loaded = config::load_from(Some(&dir.0.join("missing"))).unwrap();
    assert_eq!(
        loaded.config,
        Config::from_toml(&preset::config(), None).unwrap()
    );
    assert_eq!(
        loaded.theme,
        Theme::from_toml(&preset::theme(), None).unwrap()
    );
    assert!(!loaded.keymap.picker.is_empty());
    assert!(!loaded.keymap.input.is_empty());
    assert!(!loaded.keymap.confirm.is_empty());
    assert!(!loaded.keymap.help.is_empty());
}

#[test]
fn invalid_ratio_names_config_file_and_setting() {
    let dir = TempDir::new();
    dir.write("config.toml", "[picker]\npreview_ratio = 150");
    let error = config::load_from(Some(&dir.0)).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains(&dir.0.join("config.toml").display().to_string()));
    assert!(message.contains("preview_ratio"));
}

#[test]
fn invalid_floating_pane_names_config_file_and_setting() {
    for (setting, value) in [("position", "'middle'"), ("width", "0")] {
        let dir = TempDir::new();
        dir.write(
            "config.toml",
            &format!("[floating-pane]\n{setting} = {value}"),
        );
        let message = format!("{:#}", config::load_from(Some(&dir.0)).unwrap_err());
        assert!(message.contains(&dir.0.join("config.toml").display().to_string()));
        assert!(message.contains(setting), "{message}");
    }
}

#[test]
fn invalid_switch_session_exclusions_name_config_file_and_setting() {
    let dir = TempDir::new();
    dir.write(
        "config.toml",
        "[switch-session]\nexclude_from_recency = \"_popup\"",
    );
    let error = config::load_from(Some(&dir.0)).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains(&dir.0.join("config.toml").display().to_string()));
    assert!(message.contains("exclude_from_recency"));
}

#[test]
fn unknown_key_names_file_and_key() {
    let dir = TempDir::new();
    dir.write(
        "keymap.toml",
        "[picker]\nkeymap = [{ on = '<Foo>', run = 'quit' }]",
    );
    let message = format!("{:#}", config::load_from(Some(&dir.0)).unwrap_err());
    assert!(message.contains("keymap.toml"));
    assert!(message.contains("<Foo>"));
}

#[test]
fn unknown_fields_in_all_files_preserve_known_values() {
    let dir = TempDir::new();
    dir.write("config.toml", "[future-feature]\nenabled = true\n[picker]\nremoved_setting = true\npreview_ratio = 70\n[switch-session]\nsurprise = true\nexclude_from_recency = ['_popup']");
    dir.write("theme.toml", "[future-feature]\nenabled = true\n[picker]\nsparkle = { fg = 'red' }\nborder = { fg = 'green', future = true }");
    dir.write("keymap.toml", "[future-feature]\nenabled = true\n[picker]\nfuture = true\nkeymap = [{ on = 'q', run = 'quit', note = 'x' }]");
    let loaded = config::load_from(Some(&dir.0)).unwrap();
    assert_eq!(loaded.config.picker.preview_ratio, 70);
    assert_eq!(
        loaded.config.switch_session.exclude_from_recency,
        ["_popup"]
    );
    let mut expected = Theme::from_toml(&preset::theme(), None).unwrap();
    expected.picker.border.fg = Some(ratatui::style::Color::Green);
    assert_eq!(loaded.theme, expected);
    assert_eq!(loaded.keymap.picker.len(), 1);
    assert_eq!(
        loaded.keymap.picker[0].run,
        vec![pane_mux::picker::action::Action::Quit]
    );
}

#[test]
fn injected_config_home_feeds_loader() {
    let dir = TempDir::new();
    dir.write("config.toml", "[picker]\npreview_position = 'top'");
    let resolved = paths::config_dir_from(|key| {
        (key == "PANE_MUX_CONFIG_HOME").then(|| dir.0.to_str().unwrap().to_owned())
    });
    let loaded = config::load_from(resolved.as_deref()).unwrap();
    assert_eq!(loaded.config.picker.preview_position, PreviewPosition::Top);
    assert_eq!(loaded.config.picker.preview_ratio, 50);
}

#[test]
fn unreadable_file_names_full_path() {
    let dir = TempDir::new();
    // A directory in place of a file is a portable way to trigger a read error.
    std::fs::create_dir(dir.0.join("theme.toml")).unwrap();
    let message = format!("{:#}", config::load_from(Some(&dir.0)).unwrap_err());
    assert!(message.contains(&dir.0.join("theme.toml").display().to_string()));
}

#[test]
fn invalid_theme_names_file_and_problem() {
    let dir = TempDir::new();
    dir.write("theme.toml", "[preview]\nborder = { fg = 'not-a-colour' }");
    let message = format!("{:#}", config::load_from(Some(&dir.0)).unwrap_err());
    assert!(message.contains("theme.toml"));
    assert!(
        message.to_lowercase().contains("color") || message.to_lowercase().contains("colour"),
        "{message}"
    );
}

#[test]
fn presets_load_without_user_directory() {
    let config_text = preset::config();
    let keymap_text = preset::keymap();
    let theme_text = preset::theme();
    assert!(!config_text.is_empty());
    assert!(!keymap_text.is_empty());
    assert!(!theme_text.is_empty());
    // Release presets must be borrowed static strings, not runtime file reads.
    #[cfg(not(debug_assertions))]
    {
        assert!(matches!(config_text, std::borrow::Cow::Borrowed(_)));
        assert!(matches!(keymap_text, std::borrow::Cow::Borrowed(_)));
        assert!(matches!(theme_text, std::borrow::Cow::Borrowed(_)));
    }
    let loaded = config::load_from(None).unwrap();
    assert_eq!(loaded.config.picker.preview_ratio, 50);
    assert!(!loaded.keymap.picker.is_empty());
    assert!(!loaded.theme.icons.window.text.is_empty());
}

#[test]
fn documented_toml_examples_parse_over_presets() {
    let mut kind = None;
    let mut block = None;
    let mut count = 0;
    for line in include_str!("../docs/configuration.md").lines() {
        if let Some(text) = block.as_mut() {
            if line == "```" {
                let text: String = block.take().unwrap();
                match kind.take().expect("TOML example needs a file marker") {
                    "config.toml" => {
                        Config::from_toml(&preset::config(), Some(&text)).unwrap();
                    }
                    "keymap.toml" => {
                        Keymap::from_toml(&preset::keymap(), Some(&text)).unwrap();
                    }
                    "theme.toml" => {
                        Theme::from_toml(&preset::theme(), Some(&text)).unwrap();
                    }
                    _ => unreachable!(),
                }
                count += 1;
            } else {
                text.push_str(line);
                text.push('\n');
            }
        } else if line == "```toml" {
            block = Some(String::new());
        } else if let Some(name) = line
            .strip_prefix("<!-- ")
            .and_then(|s| s.strip_suffix(" -->"))
        {
            kind = Some(name);
        }
    }
    assert!(block.is_none(), "unclosed TOML fence");
    assert_eq!(count, 10, "expected all ten documented file examples");
}
