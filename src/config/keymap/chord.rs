//! A binding is a key sequence followed by one or more actions.

use serde::{Deserialize, Deserializer, de};

use super::{Key, run::parse_run};
use crate::picker::action::Action;

/// A configured key sequence and the actions it dispatches in order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub struct Chord {
    #[serde(deserialize_with = "deserialize_on")]
    #[cfg_attr(test, schemars(with = "crate::config::schema::On"))]
    pub on: Vec<Key>,
    #[serde(deserialize_with = "deserialize_run")]
    #[cfg_attr(test, schemars(with = "crate::config::schema::Run"))]
    pub run: Vec<Action>,
    pub desc: Option<String>,
}

// An untagged enum lets serde accept either TOML shape without an extra dependency.
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

fn deserialize_on<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Key>, D::Error> {
    let strings = OneOrMany::deserialize(deserializer)?.into_vec();
    if strings.is_empty() {
        return Err(de::Error::custom("'on' cannot be empty"));
    }
    strings
        .into_iter()
        .map(|key| key.parse().map_err(de::Error::custom))
        .collect()
}

fn deserialize_run<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Action>, D::Error> {
    let strings = OneOrMany::deserialize(deserializer)?.into_vec();
    strings
        .into_iter()
        .map(|run| parse_run(&run).map_err(de::Error::custom))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::action::Step;

    #[test]
    fn parses_sequence() {
        let chord: Chord = toml::from_str("on = ['g', 'g']\nrun = 'arrow top'").unwrap();
        assert_eq!(chord.on, vec!["g".parse::<Key>().unwrap(); 2]);
        assert_eq!(chord.run, vec![Action::Arrow(Step::Top)]);
        assert_eq!(chord.desc, None);
    }

    #[test]
    fn parses_run_list_in_order() {
        let chord: Chord =
            toml::from_str("on = 'x'\nrun = ['escape', 'quit']\ndesc = 'Exit'").unwrap();
        assert_eq!(chord.run, vec![Action::Escape, Action::Quit]);
        assert_eq!(chord.desc.as_deref(), Some("Exit"));
    }

    #[test]
    fn ignores_unknown_fields_but_rejects_empty_sequences() {
        let chord: Chord = toml::from_str("on = 'q'\nrun = 'quit'\nnote = 'x'").unwrap();
        assert_eq!(chord.on, vec!["q".parse::<Key>().unwrap()]);
        assert_eq!(chord.run, vec![Action::Quit]);
        assert!(toml::from_str::<Chord>("on = []\nrun = 'quit'").is_err());
    }

    #[test]
    fn serde_errors_preserve_invalid_key_and_command() {
        for (text, expected) in [
            ("on = '<Foo>'\nrun = 'quit'", "invalid key \"<Foo>\""),
            ("on = 'x'\nrun = 'explode'", "unknown command \"explode\""),
        ] {
            assert!(
                toml::from_str::<Chord>(text)
                    .unwrap_err()
                    .to_string()
                    .contains(expected)
            );
        }
    }
}
