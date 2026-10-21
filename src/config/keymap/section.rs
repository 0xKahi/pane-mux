//! Merge the four keymap layers independently.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{Chord, Key};

/// Fully merged bindings for each picker mode.
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    pub picker: Vec<Chord>,
    pub input: Vec<Chord>,
    pub confirm: Vec<Chord>,
    pub help: Vec<Chord>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct KeymapFile {
    picker: Layer,
    input: Layer,
    confirm: Layer,
    help: Layer,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Layer {
    keymap: Option<Vec<Chord>>,
    prepend_keymap: Option<Vec<Chord>>,
    append_keymap: Option<Vec<Chord>>,
}

impl Keymap {
    /// Parse preset and optional user TOML, then apply per-layer precedence.
    pub fn from_toml(preset: &str, user: Option<&str>) -> Result<Self> {
        let preset: KeymapFile = toml::from_str(preset).context("invalid preset keymap.toml")?;
        let user: KeymapFile = match user {
            Some(text) => toml::from_str(text).context("invalid user keymap.toml")?,
            None => KeymapFile::default(),
        };
        Ok(Self {
            picker: merge_layer(preset.picker, user.picker),
            input: merge_layer(preset.input, user.input),
            confirm: merge_layer(preset.confirm, user.confirm),
            help: merge_layer(preset.help, user.help),
        })
    }
}

// Yazi identifies a binding by its first two keys, including a missing second key.
fn prefix(chord: &Chord) -> (Option<Key>, Option<Key>) {
    (chord.on.first().copied(), chord.on.get(1).copied())
}

fn merge_layer(preset: Layer, user: Layer) -> Vec<Chord> {
    let base = user.keymap.or(preset.keymap).unwrap_or_default();
    let prepend = user
        .prepend_keymap
        .or(preset.prepend_keymap)
        .unwrap_or_default();
    let append = user
        .append_keymap
        .or(preset.append_keymap)
        .unwrap_or_default();
    let prepended: Vec<_> = prepend.iter().map(prefix).collect();
    let bound: Vec<_> = base.iter().chain(&prepend).map(prefix).collect();

    let mut result = prepend;
    result.extend(
        base.into_iter()
            .filter(|chord| !prepended.contains(&prefix(chord))),
    );
    result.extend(
        append
            .into_iter()
            .filter(|chord| !bound.contains(&prefix(chord))),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::action::{Action, Step};

    const PRESET: &str = include_str!("../preset/keymap-default.toml");

    fn binding<'a>(chords: &'a [Chord], on: &[&str]) -> &'a Chord {
        let keys: Vec<Key> = on.iter().map(|text| text.parse().unwrap()).collect();
        chords.iter().find(|chord| chord.on == keys).unwrap()
    }

    #[test]
    fn preset_parses_and_every_binding_has_a_description() {
        let keymap = Keymap::from_toml(PRESET, None).unwrap();
        for layer in [&keymap.picker, &keymap.input, &keymap.confirm, &keymap.help] {
            assert!(!layer.is_empty());
            assert!(
                layer
                    .iter()
                    .all(|chord| chord.desc.as_ref().is_some_and(|desc| !desc.is_empty()))
            );
        }
        assert!(
            !keymap
                .picker
                .iter()
                .any(|chord| chord.on == vec!["K".parse().unwrap()]
                    || chord.on == vec!["J".parse().unwrap()])
        );
    }

    #[test]
    fn prepend_overrides_default() {
        let keymap = Keymap::from_toml(
            PRESET,
            Some("[picker]\nprepend_keymap = [{ on = 'q', run = 'escape' }]"),
        )
        .unwrap();
        assert_eq!(binding(&keymap.picker, &["q"]).run, vec![Action::Escape]);
        assert_eq!(
            keymap
                .picker
                .iter()
                .filter(|chord| chord.on == vec!["q".parse().unwrap()])
                .count(),
            1
        );
    }

    #[test]
    fn append_does_not_override_default() {
        let keymap = Keymap::from_toml(
            PRESET,
            Some("[picker]\nappend_keymap = [{ on = 'j', run = 'arrow prev' }]"),
        )
        .unwrap();
        assert_eq!(
            binding(&keymap.picker, &["j"]).run,
            vec![Action::Arrow(Step::Next)]
        );
        assert_eq!(
            keymap
                .picker
                .iter()
                .filter(|chord| chord.on == vec!["j".parse().unwrap()])
                .count(),
            1
        );
    }

    #[test]
    fn keymap_replaces_only_its_layer() {
        let keymap = Keymap::from_toml(
            PRESET,
            Some("[picker]\nkeymap = [{ on = 'x', run = 'quit' }]"),
        )
        .unwrap();
        assert_eq!(keymap.picker.len(), 1);
        assert_eq!(binding(&keymap.picker, &["x"]).run, vec![Action::Quit]);
        assert!(!keymap.input.is_empty());
    }

    #[test]
    fn append_cannot_override_prepend_and_keeps_unbound_keys() {
        let keymap = Keymap::from_toml("", Some("[picker]\nprepend_keymap = [{ on = 'x', run = 'quit' }]\nappend_keymap = [{ on = 'x', run = 'escape' }, { on = 'z', run = 'help' }]")).unwrap();
        assert_eq!(keymap.picker.len(), 2);
        assert_eq!(binding(&keymap.picker, &["x"]).run, vec![Action::Quit]);
        assert_eq!(binding(&keymap.picker, &["z"]).run, vec![Action::Help]);
    }

    #[test]
    fn merge_compares_first_two_keys_like_yazi() {
        let preset = "[picker]\nkeymap = [{ on = ['g', 'g', 'a'], run = 'quit' }, { on = ['g', 'h'], run = 'help' }, { on = 'g', run = 'pick' }]";
        let user = "[picker]\nprepend_keymap = [{ on = ['g', 'g', 'b'], run = 'escape' }]";
        let keymap = Keymap::from_toml(preset, Some(user)).unwrap();
        assert_eq!(keymap.picker.len(), 3);
        assert_eq!(keymap.picker[0].run, vec![Action::Escape]);
        assert_eq!(binding(&keymap.picker, &["g", "h"]).run, vec![Action::Help]);
        assert_eq!(binding(&keymap.picker, &["g"]).run, vec![Action::Pick]);
    }

    #[test]
    fn unknown_layers_and_fields_are_errors() {
        assert!(Keymap::from_toml("", Some("[typo]\nkeymap = []")).is_err());
        assert!(Keymap::from_toml("", Some("[picker]\ntypo = []")).is_err());
    }
}
