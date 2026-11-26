use crate::mux::{Current, Pane, Session, Tree, Window};
use serde::{Deserialize, Deserializer, de};

pub(super) const SCRIPT: &str = include_str!("snapshot.lua");
pub(super) const CAPTURE: &str = include_str!("capture.lua");
// REX_CLIENT otherwise makes even `do -e` target the app rather than the server.
pub(super) const UNSCOPED: &[&str] = &["REX_SESSION", "REX_BLOCK", "REX_CLIENT"];

pub(super) fn list<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum List<T> {
        Array(Vec<T>),
        Object(Empty),
    }
    match List::deserialize(d).map_err(de::Error::custom)? {
        List::Array(items) => Ok(items),
        List::Object(_) => Ok(Vec::new()),
    }
}

#[derive(Deserialize)]
pub(super) struct Snapshot {
    #[serde(deserialize_with = "list")]
    pub sessions: Vec<SessionView>,
}

#[derive(Deserialize)]
pub(super) struct SessionView {
    session_id: String,
    #[serde(default)]
    label: String,
    active_window_id: Option<String>,
    #[serde(default, deserialize_with = "list")]
    windows: Vec<WindowView>,
}

#[derive(Deserialize)]
struct WindowView {
    window_id: String,
    #[serde(default)]
    label: String,
    focused_block_id: Option<String>,
    #[serde(default, deserialize_with = "list")]
    layers: Vec<Layer>,
}

#[derive(Deserialize)]
struct Layer {
    kind: String,
    #[serde(default, deserialize_with = "list")]
    blocks: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    block_id: String,
    #[serde(default)]
    label: String,
    process_name: Option<String>,
    owner: Option<String>,
}

pub(super) struct SelfBlock {
    pub current: Current,
    pub owner: Option<String>,
}

impl Snapshot {
    pub(super) fn into_tree(self, self_block: Option<&str>) -> (Tree, Option<SelfBlock>) {
        let mut found = None;
        let sessions = self
            .sessions
            .into_iter()
            .map(|view| {
                let (session, location) = view.into_session(self_block);
                if found.is_none() {
                    found = location;
                }
                session
            })
            .collect();
        (Tree { sessions }, found)
    }
}

impl SessionView {
    pub(super) fn into_session(self, self_block: Option<&str>) -> (Session, Option<SelfBlock>) {
        let mut found = None;
        let windows = self
            .windows
            .into_iter()
            .enumerate()
            .map(|(i, w)| {
                let active = self.active_window_id.as_ref() == Some(&w.window_id);
                let mut layers = w.layers;
                layers.sort_by_key(|layer| layer.kind == "floating");
                let panes = layers
                    .into_iter()
                    .flat_map(|layer| {
                        let floating = layer.kind == "floating";
                        layer.blocks.into_iter().map(move |block| (block, floating))
                    })
                    .filter_map(|(mut block, floating)| {
                        if self_block == Some(block.block_id.as_str()) {
                            found.get_or_insert_with(|| SelfBlock {
                                current: Current {
                                    session_id: self.session_id.clone(),
                                    window_id: w.window_id.clone(),
                                    pane_id: block.block_id.clone(),
                                },
                                owner: block.owner.take(),
                            });
                            if floating {
                                return None;
                            }
                        }
                        Some((block, floating))
                    })
                    .enumerate()
                    .map(|(i, (b, floating))| Pane {
                        index: (i + 1) as u32,
                        active: w.focused_block_id.as_ref() == Some(&b.block_id),
                        id: b.block_id,
                        command: b.process_name.filter(|s| !s.is_empty()).unwrap_or(b.label),
                        floating,
                        ..Pane::default()
                    })
                    .collect();
                Window {
                    id: w.window_id,
                    index: (i + 1) as u32,
                    name: w.label,
                    active,
                    panes,
                    ..Window::default()
                }
            })
            .collect();
        (
            Session {
                id: self.session_id,
                name: self.label,
                windows,
                ..Session::default()
            },
            found,
        )
    }
}

pub(super) fn pane_location(tree: &Tree, pane: &str) -> Option<Current> {
    for s in &tree.sessions {
        for w in &s.windows {
            if w.panes.iter().any(|p| p.id == pane) {
                return Some(Current {
                    session_id: s.id.clone(),
                    window_id: w.id.clone(),
                    pane_id: pane.into(),
                });
            }
        }
    }
    None
}
