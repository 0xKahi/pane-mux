//! Editor-only schemas: runtime deserialization deliberately remains lenient.

use schemars::{JsonSchema, r#gen::SchemaSettings, schema::Schema};
use serde_json::{Value, json};

use super::{Config, Theme, keymap::run::COMMAND_NAMES, keymap::section::KeymapFile, preset};

const COLOUR_NAMES: &[&str] = &[
    "reset",
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "gray",
    "darkgray",
    "lightred",
    "lightgreen",
    "lightyellow",
    "lightblue",
    "lightmagenta",
    "lightcyan",
    "white",
];

pub struct Colour;
pub struct On;
pub struct Run;

impl JsonSchema for Colour {
    fn schema_name() -> String {
        "Colour".into()
    }
    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        serde_json::from_value(json!({
            "type": "string",
            "pattern": format!("^({}|#[0-9a-fA-F]{{6}})$", COLOUR_NAMES.join("|")),
            "description": "A named terminal colour, reset, or #rrggbb."
        }))
        .unwrap()
    }
}

fn one_or_many(item: Value, description: &str) -> Schema {
    serde_json::from_value(json!({
        "description": description,
        "anyOf": [item, {"type": "array", "items": item}]
    }))
    .unwrap()
}

impl JsonSchema for On {
    fn schema_name() -> String {
        "On".into()
    }
    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        serde_json::from_value(json!({
            "description": "A key or sequence of keys; supports key names and modifiers such as <C-a>.",
            "anyOf": [
                {"type": "string"},
                {"type": "array", "items": {"type": "string"}, "minItems": 1}
            ]
        })).unwrap()
    }
}

impl JsonSchema for Run {
    fn schema_name() -> String {
        "Run".into()
    }
    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        one_or_many(
            json!({"type":"string", "pattern": format!("^({})(\\s.*)?$", COMMAND_NAMES.join("|"))}),
            "One command or an ordered list of commands, with optional arguments.",
        )
    }
}

// Partial overlays need no section fields, but bindings must remain executable.
fn strip_required(value: &mut Value, binding: bool) {
    match value {
        Value::Object(object) => {
            if binding {
                object.insert("required".into(), json!(["on", "run"]));
            } else {
                object.remove("required");
            }
            for (name, child) in object.iter_mut() {
                strip_required(child, name == "Chord");
            }
        }
        Value::Array(array) => {
            for child in array {
                strip_required(child, false);
            }
        }
        _ => {}
    }
}

fn generate<T: JsonSchema>(name: &str) -> Value {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<T>();
    let mut value = serde_json::to_value(schema).unwrap();
    strip_required(&mut value, false);
    value["$id"] = json!(format!(
        "https://raw.githubusercontent.com/0xKahi/pane-mux/main/schemas/{name}.json"
    ));
    value
}

fn schemas() -> [(String, Value, String); 3] {
    [
        (
            "config".into(),
            generate::<Config>("config"),
            preset::config().into_owned(),
        ),
        (
            "keymap".into(),
            generate::<KeymapFile>("keymap"),
            preset::keymap().into_owned(),
        ),
        (
            "theme".into(),
            generate::<Theme>("theme"),
            preset::theme().into_owned(),
        ),
    ]
}

fn toml_json(text: &str) -> Value {
    serde_json::to_value(toml::from_str::<toml::Value>(text).unwrap()).unwrap()
}

#[test]
fn committed_schemas() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    for (name, schema, _) in schemas() {
        let path = directory.join(format!("{name}.json"));
        let expected = format!("{}\n", serde_json::to_string_pretty(&schema).unwrap());
        if std::env::var("PANE_MUX_UPDATE_SCHEMAS").as_deref() == Ok("1") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(&path, expected).unwrap();
        } else {
            assert_eq!(
                std::fs::read_to_string(&path).unwrap_or_default(),
                expected,
                "{} is stale; run `just schemas`",
                path.display()
            );
        }
    }
}

#[test]
fn built_in_presets_validate() {
    let mut errors = Vec::new();
    for (name, schema, text) in schemas() {
        let validator = jsonschema::validator_for(&schema).unwrap();
        let value = toml_json(&text);
        for error in validator.iter_errors(&value) {
            errors.push(format!(
                "{name}-default.toml {}: {error}",
                error.instance_path
            ));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn partial_files_and_strict_scenarios() {
    for (_, schema, _) in schemas() {
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&json!({}))
        );
    }
    let config = jsonschema::validator_for(&generate::<Config>("config")).unwrap();
    assert!(config.is_valid(&toml_json("[picker]\npreview_position = 'top'")));
    for text in [
        "[picker]\npreveiw_ratio = 70",
        "[picker]\npreview_position = 'left'",
        "[future-feature]\nenabled = true",
        "[picker]\npreview_ratio = 150",
        "[floating-pane]\nposition = 'middle'",
        "[floating-pane]\nwidth = 0",
        "[floating-pane]\nheight = 101",
    ] {
        assert!(!config.is_valid(&toml_json(text)), "{text}");
    }
    let keymap = jsonschema::validator_for(&generate::<KeymapFile>("keymap")).unwrap();
    for binding in [
        json!({"on":"q", "run":"pste"}),
        json!({"on":"q"}),
        json!({"run":"quit"}),
        json!({"on":"q", "run":"quit", "note":"x"}),
    ] {
        assert!(!keymap.is_valid(&json!({"picker":{"keymap":[binding]}})));
    }
    assert!(keymap.is_valid(
        &json!({"picker":{"prepend_keymap":[{"on":["g","g"],"run":["arrow top","quit"]}]}})
    ));
    let theme = jsonschema::validator_for(&generate::<Theme>("theme")).unwrap();
    assert!(theme.is_valid(&json!({"help":{"desc":{"fg":"red"}}})));
    assert!(theme.is_valid(&json!({"marker":{"cut":{"fg":"yellow"}}})));
    assert!(config.is_valid(&toml_json(
        "[floating-pane]\nwidth = 80\nheight = 30\nposition = 'bottom-centre'"
    )));
    assert!(!theme.is_valid(&json!({"picker":{"sparkle":{"fg":"red"}}})));
    assert!(!theme.is_valid(&json!({"indicator":{"active":{"typo":true}}})));
}

#[test]
fn bogus_key_in_preset_copy_is_rejected() {
    for (_, schema, text) in schemas() {
        let mut value = toml_json(&text);
        value
            .as_object_mut()
            .unwrap()
            .insert("bogus-key".into(), json!(true));
        assert!(!jsonschema::validator_for(&schema).unwrap().is_valid(&value));
    }
}

#[test]
fn schema_colour_names_parse() {
    let schema = serde_json::to_value(schemars::schema_for!(Colour)).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for name in COLOUR_NAMES {
        assert!(name.parse::<ratatui::style::Color>().is_ok(), "{name}");
        assert!(validator.is_valid(&json!(name)));
    }
    assert!(validator.is_valid(&json!("#aBc123")));
    assert!(!validator.is_valid(&json!("not-a-colour")));
}
