//! Configuration directory resolution without platform-specific directory conventions.

use std::path::PathBuf;

/// Read the process environment to locate the configuration directory.
pub fn config_dir() -> Option<PathBuf> {
    config_dir_from(|name| std::env::var(name).ok())
}

/// Resolve paths using an injected lookup, so tests never mutate the environment.
pub fn config_dir_from(get: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(path) = get("PANE_MUX_CONFIG_HOME").map(PathBuf::from)
        && path.is_absolute()
    {
        return Some(path);
    }
    if let Some(path) = get("XDG_CONFIG_HOME").map(PathBuf::from)
        && path.is_absolute()
    {
        return Some(path.join("pane-mux"));
    }
    get("HOME").map(|home| PathBuf::from(home).join(".config/pane-mux"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(entries: &[(&str, &str)]) -> Option<PathBuf> {
        config_dir_from(|name| {
            entries
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn absolute_override_wins() {
        assert_eq!(
            resolve(&[
                ("PANE_MUX_CONFIG_HOME", "/tmp/pm"),
                ("XDG_CONFIG_HOME", "/xdg"),
                ("HOME", "/home/me")
            ]),
            Some(PathBuf::from("/tmp/pm"))
        );
    }

    #[test]
    fn xdg_fallback() {
        assert_eq!(
            resolve(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/me")]),
            Some(PathBuf::from("/xdg/pane-mux"))
        );
    }

    #[test]
    fn home_fallback() {
        assert_eq!(
            resolve(&[("HOME", "/home/me")]),
            Some(PathBuf::from("/home/me/.config/pane-mux"))
        );
    }

    #[test]
    fn relative_override_is_ignored() {
        assert_eq!(
            resolve(&[
                ("PANE_MUX_CONFIG_HOME", "relative"),
                ("XDG_CONFIG_HOME", "/xdg")
            ]),
            Some(PathBuf::from("/xdg/pane-mux"))
        );
    }

    #[test]
    fn relative_xdg_is_ignored() {
        assert_eq!(
            resolve(&[("XDG_CONFIG_HOME", "relative"), ("HOME", "/home/me")]),
            Some(PathBuf::from("/home/me/.config/pane-mux"))
        );
    }

    #[test]
    fn missing_environment_has_no_directory() {
        assert_eq!(resolve(&[]), None);
    }
}
