//! Recursive TOML merging shared by settings and themes.

use anyhow::Context;

/// Merge tables recursively; all other user values replace the preset value.
pub fn deep_merge(base: &mut toml::Table, user: toml::Table) {
    for (key, value) in user {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(base_table)), toml::Value::Table(user_table)) => {
                deep_merge(base_table, user_table);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// Parse and combine preset and optional user TOML before typed deserialization.
pub fn from_toml(preset: &str, user: Option<&str>) -> anyhow::Result<toml::Table> {
    let mut table = toml::from_str(preset).context("invalid preset TOML")?;
    if let Some(user) = user {
        deep_merge(
            &mut table,
            toml::from_str(user).context("invalid user TOML")?,
        );
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_merge_and_other_values_replace() {
        let merged = from_toml(
            "items = [1, 2]\n[nested]\na = 1\nb = 2",
            Some("items = [3]\n[nested]\na = 4"),
        )
        .unwrap();
        assert_eq!(merged["items"].as_array().unwrap().len(), 1);
        assert_eq!(merged["nested"]["a"].as_integer(), Some(4));
        assert_eq!(merged["nested"]["b"].as_integer(), Some(2));
        let replaced = from_toml("[nested]\na = 1", Some("nested = false")).unwrap();
        assert_eq!(replaced["nested"].as_bool(), Some(false));
    }
}
