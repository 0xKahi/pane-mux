//! Opt-in live test. All mutations are scoped to sessions created by this test.

use std::{
    thread,
    time::{Duration, Instant},
};

use pane_mux::{
    config::settings::{FloatingPaneConfig, FloatingPosition},
    mux::{Level, Mux, Session},
    picker::{PickerOptions, State, action::Action},
    rex::{ProcessRunner, RexCli},
};
use serde_json::Value;

const UNSCOPED: &[&str] = &["REX_SESSION", "REX_BLOCK", "REX_CLIENT"];

struct Sessions {
    rex: RexCli,
    ids: Vec<String>,
}

impl Sessions {
    fn create(&mut self, label: &str) -> String {
        let id = self.rex.new_session(Some(label)).unwrap();
        self.ids.push(id.clone());
        id
    }

    fn destroy_all(&mut self) {
        self.ids.retain(|id| match self.rex.kill_session(id) {
            Ok(()) => false,
            Err(err) => {
                eprintln!("failed to destroy Rex test session {id}: {err:#}");
                true
            }
        });
    }
}

impl Drop for Sessions {
    fn drop(&mut self) {
        self.destroy_all();
    }
}

fn json(rex: &RexCli, args: &[&str]) -> Value {
    let output = rex.run(args, UNSCOPED).unwrap();
    serde_json::from_str(&output).unwrap()
}

fn session(rex: &RexCli, id: &str) -> Session {
    rex.tree()
        .unwrap()
        .sessions
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
}

fn inspect(rex: &RexCli, id: &str) -> Value {
    let mut view = json(rex, &["session", "inspect", id, "--json"]);
    // Each inspection includes its own transient CLI connection.
    view.as_object_mut().unwrap().remove("clients");
    view
}

fn foreground(rex: &RexCli, id: &str, block: &str) -> Value {
    json(rex, &["block", "call", "-s", id, "-b", block, "process"])["foreground"].clone()
}

#[test]
fn live_rex_tree_targets_moves_and_cross_session_refusal() {
    if std::env::var("PANE_MUX_REX_TESTS").as_deref() != Ok("1") {
        return;
    }
    let server = std::env::var("REX_SERVER").expect("set REX_SERVER to a running Rex server");
    assert!(!server.trim().is_empty(), "REX_SERVER must not be blank");
    let mut sessions = Sessions {
        rex: RexCli::with_runner(Some(server.clone()), None, ProcessRunner),
        ids: Vec::new(),
    };
    sessions.rex.check_api_revision().unwrap();
    let label = format!("pane-mux-test-{}", std::process::id());
    let a = sessions.create(&label);
    let b = sessions.create(&format!("{label}-b"));
    let rex = &sessions.rex;
    let initial = session(rex, &a);
    let source = &initial.windows[0];
    let anchor = &source.panes[0].id;
    let dest = rex.new_window(&a, Some("Destination")).unwrap();
    let created = json(
        rex,
        &[
            "split",
            "-s",
            &a,
            "-b",
            anchor,
            "--focus=false",
            "--shell=none",
            "--label",
            "Move me",
            "--json",
            "--",
            "sleep",
            "300",
        ],
    );
    let block = created["block_ids"][0].as_str().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let pid = loop {
        let process = foreground(rex, &a, block);
        if process["name"] == "sleep" {
            break process["pid"].as_u64().unwrap();
        }
        assert!(Instant::now() < deadline, "sleep did not start: {process}");
        thread::sleep(Duration::from_millis(50));
    };

    // The invoking block is in our own session and is not the block being moved.
    // No user history is read or written, and no app actions are issued.
    let rex = RexCli::with_runner(Some(server), Some(anchor.clone()), ProcessRunner);
    let tree = rex.tree().unwrap();
    let listed = tree.sessions.iter().find(|s| s.id == a).unwrap();
    assert_eq!(listed.name, label);
    assert_eq!(listed.windows.len(), 2);
    assert_eq!(listed.windows[1].id, dest);
    assert_eq!(listed.windows[1].name, "Destination");
    assert_eq!(listed.windows[0].panes.len(), 2);
    let pane = &listed.windows[0].panes[1];
    assert_eq!(pane.id, block);
    assert_eq!(pane.command, "sleep");
    assert_eq!(rex.current().unwrap().unwrap().pane_id, *anchor);

    let opts = PickerOptions {
        cut: Some(block.into()),
        initial_pane: Some(block.into()),
        ..PickerOptions::default()
    };
    let state = State::new(
        opts.clone(),
        tree.clone(),
        rex.current().unwrap(),
        rex.suppressed_pane(),
    );
    let target = state.cut_target().unwrap();
    for (level, prefix, id, display) in [
        (Level::Session, "session:", a.as_str(), label.clone()),
        (
            Level::Window,
            "window:",
            source.id.as_str(),
            format!("{label}:1"),
        ),
        (Level::Pane, "block:", block, format!("{label}:1.2")),
    ] {
        let output = rex.target_string(&tree, &target, level);
        assert!(output.starts_with(prefix));
        assert_eq!(output, id);
        assert_eq!(rex.display_target(&tree, &target, level), display);
    }

    let moved_window = rex
        .move_pane_to_new_window(block, &a, Some("Moved"))
        .unwrap();
    let after = session(&rex, &a);
    let moved = after.windows.iter().find(|w| w.id == moved_window).unwrap();
    assert_eq!(moved.name, "Moved");
    assert_eq!(moved.panes.len(), 1, "placeholder must be gone");
    assert_eq!(moved.panes[0].id, block);
    assert_eq!(foreground(&rex, &a, block)["pid"], pid);
    let blocks = json(&rex, &["block", "ls", "-s", &a, "--json"]);
    assert_eq!(blocks["blocks"].as_array().unwrap().len(), 3);

    // Exercise the actual picker paste path, with a deterministic float config.
    let mut state = State::new(
        opts.clone(),
        rex.tree().unwrap(),
        rex.current().unwrap(),
        rex.suppressed_pane(),
    );
    state.focus = Level::Pane;
    state.session = Some(a.clone());
    state.window = Some(dest.clone());
    state.floating_pane = FloatingPaneConfig {
        width: 50,
        height: 50,
        position: FloatingPosition::Centre,
    };
    assert_eq!(state.apply(&Action::Paste, &rex), None);
    assert_eq!(state.message, None);
    assert_eq!(state.cut, None);
    let after = session(&rex, &a);
    assert!(after.windows.iter().all(|w| w.id != moved_window));
    let target_window = after.windows.iter().find(|w| w.id == dest).unwrap();
    assert!(
        target_window
            .panes
            .iter()
            .any(|p| p.id == block && p.floating)
    );
    let after_a = inspect(&rex, &a);
    assert_eq!(after_a["active_window_id"], source.id);
    let win = after_a["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["window_id"] == dest)
        .unwrap();
    let layer = win["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| {
            l["kind"] == "floating"
                && l["blocks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["block_id"] == block)
        })
        .unwrap();
    assert_eq!(layer["bounds"], serde_json::json!([0.25, 0.25, 0.5, 0.5]));
    assert_eq!(foreground(&rex, &a, block)["pid"], pid);

    let before_b = inspect(&rex, &b);
    let b_window = before_b["windows"][0]["window_id"].as_str().unwrap();
    let mut state = State::new(
        opts,
        rex.tree().unwrap(),
        rex.current().unwrap(),
        rex.suppressed_pane(),
    );
    state.session = Some(b.clone());
    state.window = Some(b_window.into());
    state.focus = Level::Pane;
    assert_eq!(state.apply(&Action::Paste, &rex), None);
    assert_eq!(
        state.message.as_deref(),
        Some("rex does not allow moving panes to different session")
    );
    assert_eq!(state.cut.as_deref(), Some(block));
    assert!(state.popup.is_none());
    assert_eq!(inspect(&rex, &a), after_a);
    assert_eq!(inspect(&rex, &b), before_b);

    sessions.destroy_all();
    assert!(sessions.ids.is_empty(), "test sessions must be destroyed");
}
