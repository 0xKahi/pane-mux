use std::{path::PathBuf, time::Duration};

use serde_json::{Value, json};

use super::{
    FixtureRunner, RexCli, RunnerError,
    client::bounds,
    history,
    snapshot::{SCRIPT, UNSCOPED},
};
use crate::{
    commands::move_pane,
    config::{
        keymap::Keymap,
        preset,
        settings::{FloatingPaneConfig, FloatingPosition},
    },
    mux::{Current, Level, Mux, Session, Target, Tree},
    picker::{PickerOptions, SessionOrder, State},
};

const REAL: &str = include_str!("fixtures/snapshot.json");
const KAHI: &str = "session:01a1fa96-984e-7cf4-987f-c3267e96063a";
const HERE: &str = "block:01a20544-961c-77fa-a9ed-4987cd6e22b7";
const WINDOW: &str = "window:01a20544-6ba1-7562-90a1-a210ecad287e";
const APP: &str = "client:01a200d8-c685-7641-8339-76f0f2a2c748";

fn setup(snapshot: &str, block: Option<&str>) -> (RexCli, FixtureRunner) {
    let runner = FixtureRunner::default();
    reply(&runner, &["do", "-e", SCRIPT, "--json"], Ok(snapshot));
    (
        RexCli::with_runner(None, block.map(str::to_owned), runner.clone()),
        runner,
    )
}

fn reply(r: &FixtureRunner, args: &[&str], result: Result<&str, &str>) {
    let args = [&["--autostart=false"][..], args].concat();
    r.reply(
        &args,
        result
            .map(str::to_owned)
            .map_err(|s| RunnerError::Exit(s.into())),
    );
}

fn api(
    r: &FixtureRunner,
    session: Option<&str>,
    method: &str,
    params: Value,
    result: Result<&str, &str>,
) {
    let mut args = vec!["api", "call"];
    if let Some(id) = session {
        args.extend(["-s", id]);
    }
    let params = params.to_string();
    args.extend([method, &params]);
    reply(r, &args, result);
}

fn snapshot(sessions: &[Value]) -> String {
    json!({"sessions":sessions}).to_string()
}

#[test]
fn real_tree_preserves_rex_order_and_marks_current_minimized_float() {
    let (rex, runner) = setup(REAL, Some(HERE));
    let tree = rex.tree().unwrap();
    assert_eq!(
        tree.sessions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["kahi", "shuffle", "shuffle-be"]
    );
    let windows = &tree.sessions[0].windows;
    assert_eq!(
        windows
            .iter()
            .take(3)
            .map(|w| (w.index, w.name.as_str()))
            .collect::<Vec<_>>(),
        [(1, "config"), (2, "pi"), (3, "pane-mux")]
    );
    assert_eq!(windows[0].panes[0].command, "nvim");
    let w = windows.iter().find(|w| w.active).unwrap();
    assert_eq!(w.id, WINDOW);
    assert_eq!(
        w.panes
            .iter()
            .map(|p| (p.index, p.floating, p.active))
            .collect::<Vec<_>>(),
        [(1, false, true), (2, true, false)]
    );
    assert_eq!(
        rex.current().unwrap(),
        Some(Current {
            session_id: KAHI.into(),
            window_id: WINDOW.into(),
            pane_id: HERE.into()
        })
    );
    let state = State::new(
        PickerOptions::default(),
        tree.clone(),
        rex.current().unwrap(),
        None,
    );
    for id in [KAHI, WINDOW, HERE] {
        assert!(state.is_current(id));
    }
    assert!(tree.stash_counts().is_empty());
    assert!(
        windows
            .iter()
            .all(|w| !w.zoomed && w.panes.iter().all(|p| p.mode.is_empty()))
    );
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0].env_remove, UNSCOPED);
}

#[test]
fn handles_lua_empty_lists_missing_focus_and_process_label_fallback() {
    let (rex, _) = setup(r#"{"sessions":{}}"#, None);
    assert!(rex.tree().unwrap().sessions.is_empty());
    let (rex, _) = setup(
        r#"{"sessions":[{"session_id":"s","windows":[{"window_id":"w","layers":[{"kind":"floating","blocks":[{"block_id":"float","label":"log"}]},{"kind":"tiled","blocks":[{"block_id":"tile","label":"shell"}]},{"kind":"floating","blocks":{}}]}]},{"session_id":"empty","windows":{}}]}"#,
        Some("detached"),
    );
    let tree = rex.tree().unwrap();
    assert!(tree.sessions[1].windows.is_empty());
    let window = &tree.sessions[0].windows[0];
    assert!(!window.active);
    assert_eq!(
        window
            .panes
            .iter()
            .map(|p| (p.id.as_str(), p.command.as_str()))
            .collect::<Vec<_>>(),
        [("tile", "shell"), ("float", "log")]
    );
    assert!(window.panes.iter().all(|p| !p.active));
    assert_eq!(rex.current().unwrap(), None);
}

#[test]
fn backend_target_strings_and_move_pane_options() {
    let (rex, r) = setup(REAL, Some(HERE));
    let tree = rex.tree().unwrap();
    let target = Target {
        session_id: KAHI.into(),
        window_id: Some(WINDOW.into()),
        pane_id: Some(HERE.into()),
        session: "kahi".into(),
        window_index: 4,
        pane_index: 2,
    };
    for (level, id, display) in [
        (Level::Session, KAHI, "kahi"),
        (Level::Window, WINDOW, "kahi:4"),
        (Level::Pane, HERE, "kahi:4.2"),
    ] {
        assert_eq!(rex.target_string(&tree, &target, level), id);
        assert_eq!(rex.display_target(&tree, &target, level), display);
    }
    for arg in [HERE, "6e22b7"] {
        reply(
            &r,
            &["block", "inspect", "--json", "--", arg],
            Ok(include_str!("fixtures/block_inspect.json")),
        );
        let opts = move_pane::options(Some(arg), &rex, true).unwrap();
        assert_eq!(opts.cut.as_deref(), Some(HERE));
        assert_eq!(opts.initial_pane.as_deref(), Some(HERE));
        assert!(opts.cut_locked);
    }
    assert_eq!(
        move_pane::options(None, &rex, true).unwrap().cut.as_deref(),
        Some(HERE)
    );
    reply(
        &r,
        &["block", "inspect", "--json", "--", "nosuchblock"],
        Err(include_str!("fixtures/block_inspect_missing.txt")),
    );
    assert_eq!(
        rex.resolve_pane("nosuchblock").unwrap_err().to_string(),
        "pane not found: nosuchblock"
    );
    assert!(r.calls().iter().all(|call| call.env_remove == UNSCOPED));
}

#[test]
fn capture_uses_unscoped_vt_and_keeps_last_500_lines() {
    let (rex, r) = setup(REAL, None);
    let args = [
        "capture", "-b", HERE, "--format", "vt", "--unwrap", "--trim",
    ];
    let capture = include_str!("fixtures/capture.vt");
    reply(&r, &args, Ok(capture));
    assert_eq!(rex.capture(HERE).unwrap(), capture.replace("\r\n", "\n"));
    let long = (0..505).map(|i| format!("{i}\r\n")).collect::<String>();
    reply(&r, &args, Ok(&long));
    let tail = rex.capture(HERE).unwrap();
    assert_eq!(tail.lines().count(), 500);
    assert!(tail.starts_with("5\n") && tail.ends_with("504\n"));
    assert!(r.calls().iter().all(|call| call.env_remove == UNSCOPED));
    assert_eq!(rex.preview_interval(), Duration::from_secs(1));
    assert!(!rex.cross_session_moves());
}

const LAYOUT: &str = "com.superlogical.terminal.shell";

#[test]
fn creates_renames_and_deletes_with_exact_rex_api_calls() {
    let (rex, r) = setup(REAL, None);
    rex.tree().unwrap();
    api(
        &r,
        None,
        "session.create",
        json!({"label":"notes","initial_windows":[{"layout":{"block":{"flavor":LAYOUT}}}]}),
        Ok(include_str!("fixtures/api_create.json")),
    );
    assert_eq!(rex.new_session(Some("notes")).unwrap(), SPIKE);
    new_window(&r, KAHI, "logs");
    assert_eq!(rex.new_window(KAHI, Some("logs")).unwrap(), NEW);
    api(
        &r,
        Some(KAHI),
        "session.set_label",
        json!({"label":"new"}),
        Ok(include_str!("fixtures/api_set_label.json")),
    );
    rex.rename_session(KAHI, "new").unwrap();
    api(
        &r,
        Some(KAHI),
        "session.set_window_label",
        json!({"window_id":WINDOW,"label":"new"}),
        Ok(include_str!("fixtures/api_set_window_label.json")),
    );
    rex.rename_window(WINDOW, "new").unwrap();
    api(
        &r,
        Some(KAHI),
        "block.close",
        json!({"block_id":HERE}),
        Ok(include_str!("fixtures/api_block_close.json")),
    );
    rex.kill_pane(HERE).unwrap();
    api(
        &r,
        Some(KAHI),
        "session.close_window",
        json!({"window_id":WINDOW}),
        Ok(include_str!("fixtures/api_close_window.json")),
    );
    rex.kill_window(WINDOW).unwrap();
    api(
        &r,
        Some(KAHI),
        "session.destroy",
        json!({}),
        Ok(include_str!("fixtures/api_destroy.json")),
    );
    rex.kill_session(KAHI).unwrap();
    assert_eq!(r.calls().len(), 8);
}

#[test]
fn failed_rename_is_displayed_without_closing_picker() {
    let (rex, r) = setup(REAL, Some(HERE));
    let mut state = State::new(
        PickerOptions::default(),
        rex.tree().unwrap(),
        rex.current().unwrap(),
        Some(HERE.into()),
    );
    let error = "Error: label is not allowed";
    api(
        &r,
        Some(KAHI),
        "session.set_label",
        json!({"label":"x"}),
        Err(error),
    );
    let keymap = Keymap::from_toml(&preset::keymap(), None).unwrap();
    for key in ["r", "<C-u>", "x", "<Enter>"] {
        assert!(
            state
                .handle_key(key.parse().unwrap(), &keymap, &rex)
                .is_none()
        );
    }
    assert_eq!(state.message.as_deref(), Some(error));
}

#[test]
fn switching_focuses_block_then_selects_app_attached_to_left_session() {
    let (rex, r) = setup(REAL, Some(HERE));
    let tree = rex.tree().unwrap();
    let target = &tree.sessions[1];
    let win = &target.windows[0];
    let pane = &win.panes[0];
    api(
        &r,
        Some(&target.id),
        "session.focus_block",
        json!({"block_id":pane.id}),
        Ok(include_str!("fixtures/api_focus_block.json")),
    );
    let mut clients: Value = serde_json::from_str(include_str!("fixtures/client_ls.json")).unwrap();
    clients["clients"].as_array_mut().unwrap().insert(
        0,
        json!({
            "client_id":"client:unrelated","info":{"kind":"app"},
            "connection_state":"connected","session_ids":[target.id],
        }),
    );
    reply(&r, &["client", "ls", "--json"], Ok(&clients.to_string()));
    reply(
        &r,
        &[
            "-C",
            APP,
            "do",
            "session.select",
            &format!("session_id={}", target.id),
            &format!("window_id={}", win.id),
            "--json",
        ],
        Ok(include_str!("fixtures/select_success.json")),
    );
    assert_eq!(rex.switch(&pane.id).unwrap(), None);
    let calls = r.calls();
    assert_eq!(calls[1].args[5], "session.focus_block");
    assert_eq!(calls[3].args[4], "session.select");
}

#[test]
fn missing_app_still_focuses_target_window() {
    let (rex, r) = setup(REAL, Some(HERE));
    api(
        &r,
        Some(KAHI),
        "session.focus_window",
        json!({"window_id":WINDOW}),
        Ok("{}"),
    );
    reply(&r, &["client", "ls", "--json"], Ok(r#"{"clients":{}}"#));
    let hint = rex.switch(WINDOW).unwrap().unwrap();
    assert!(hint.contains("kahi") && hint.ends_with("no connected Rex app client"));
}

struct TempPath(PathBuf);
impl TempPath {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(
            std::env::temp_dir()
                .join(format!(
                    "pane-mux-rex-history-{}-{stamp}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ))
                .join("rex-recent.json"),
        )
    }
}
impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn recent(tree: Tree, current: &str, excluded: &[&str]) -> State {
    State::new(
        PickerOptions {
            order: SessionOrder::Recent,
            recency_exclude: excluded.iter().map(|s| (*s).into()).collect(),
            ..PickerOptions::default()
        },
        tree,
        Some(Current {
            session_id: current.into(),
            window_id: "w".into(),
            pane_id: "p".into(),
        }),
        None,
    )
}

#[test]
fn app_failure_is_nonfatal_and_records_target_then_left_by_whoami_endpoint() {
    let path = TempPath::new();
    let (rex, r) = setup(REAL, Some(HERE));
    let rex = rex
        .with_history_path(path.0.clone())
        .with_app_client(APP.into());
    reply(
        &r,
        &["whoami", "--json"],
        Ok(r#"{"api_revision":18,"endpoint":"endpoint"}"#),
    );
    rex.check_api_revision().unwrap();
    let target = "session:01a1ff68-9419-7376-b92e-3b7fce14a5c8";
    reply(
        &r,
        &[
            "-C",
            APP,
            "do",
            "session.select",
            &format!("session_id={target}"),
            "--json",
        ],
        Err("Error: Remote Control is disabled"),
    );
    let hint = rex.switch(target).unwrap().unwrap();
    assert!(
        hint.contains("shuffle")
            && hint.contains("Turn on Remote Control in Rex settings")
            && hint.ends_with("Error: Remote Control is disabled")
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(&path.0).unwrap()).unwrap();
    assert_eq!(stored, json!({"endpoint":[target,KAHI]}));
    let (next, r) = setup(REAL, None);
    let next = next.with_history_path(path.0.clone());
    reply(
        &r,
        &["whoami", "--json"],
        Ok(r#"{"api_revision":18,"endpoint":"endpoint"}"#),
    );
    next.check_api_revision().unwrap();
    let state = recent(next.tree().unwrap(), target, &[]);
    assert_eq!(state.sessions()[1].id, KAHI);
    assert_eq!(state.session.as_deref(), Some(KAHI));
}

#[test]
fn recent_order_uses_history_then_rex_order_and_existing_exclusions() {
    let path = TempPath::new();
    let mut tree = Tree {
        sessions: ["kahi", "shuffle", "notes", "scratch"]
            .map(|name| Session {
                id: name.into(),
                name: name.into(),
                ..Session::default()
            })
            .into(),
    };
    let check = |tree: &mut Tree, server, current, excluded: &[&str], expected: &[&str]| {
        history::rank(tree, Some(&path.0), Some(server));
        let state = recent(tree.clone(), current, excluded);
        assert_eq!(
            state
                .sessions()
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(state.session.as_deref(), Some(expected[1]));
    };
    check(
        &mut tree,
        "a",
        "shuffle",
        &[],
        &["shuffle", "kahi", "notes", "scratch"],
    );
    history::record(&path.0, "a", Some("kahi"), "notes").unwrap();
    history::record(&path.0, "a", None, "deleted").unwrap();
    check(
        &mut tree,
        "a",
        "kahi",
        &[],
        &["kahi", "notes", "shuffle", "scratch"],
    );
    history::record(&path.0, "a", None, "scratch").unwrap();
    check(
        &mut tree,
        "a",
        "kahi",
        &["scratch", "kahi"],
        &["kahi", "notes", "shuffle", "scratch"],
    );
    check(
        &mut tree,
        "other",
        "kahi",
        &[],
        &["kahi", "shuffle", "notes", "scratch"],
    );
    for i in 0..55 {
        history::record(&path.0, "cap", None, &i.to_string()).unwrap();
    }
    let all: Value = serde_json::from_slice(&std::fs::read(&path.0).unwrap()).unwrap();
    assert_eq!(all["cap"].as_array().unwrap().len(), 50);
    assert_eq!(all["cap"][0], "54");
    assert_eq!(all["cap"][49], "5");
}

const SPIKE: &str = "session:01a20593-ba47-7659-b62a-f4f49c2e8b97";
const NEW: &str = "window:01a20593-ba93-7153-8be7-390ea5ba7a9d";
const PLACEHOLDER: &str = "block:01a20593-ba89-71dc-a359-32c788833b9f";
const PANE: &str = "block:01a20593-ba69-7e57-b192-6ad9078f63a0";
const SOURCE: &str = "window:01a20593-ba54-7a87-a1bc-16ec6653ac73";
const FOCUSED: &str = "block:01a20593-ba47-76e5-8989-ac9c1a88acb4";

fn new_window(r: &FixtureRunner, session: &str, name: &str) {
    api(
        r,
        Some(session),
        "session.new_window",
        json!({"focus":false,"window_label":name,"layout":{"block":{"flavor":LAYOUT}}}),
        Ok(include_str!("fixtures/api_new_window.json")),
    );
}
fn inspect(r: &FixtureRunner, view: &str) {
    reply(r, &["session", "inspect", SPIKE, "--json"], Ok(view));
}
fn spike(view: &str) -> (RexCli, FixtureRunner) {
    setup(&snapshot(&[serde_json::from_str(view).unwrap()]), None)
}

#[test]
fn placeholder_move_closes_shell_on_success_and_midflow_failure() {
    const BEFORE: &str = include_str!("fixtures/inspect_focus_before_placeholder_move.json");
    for fail in [false, true] {
        let (rex, r) = spike(BEFORE);
        inspect(&r, BEFORE);
        new_window(&r, SPIKE, "mon");
        api(
            &r,
            Some(SPIKE),
            "session.move_block",
            json!({"block_id":PANE,"anchor_block_id":PLACEHOLDER}),
            if fail {
                Err("move denied")
            } else {
                Ok(include_str!("fixtures/api_move_block.json"))
            },
        );
        api(
            &r,
            Some(SPIKE),
            "block.close",
            json!({"block_id":PLACEHOLDER}),
            Ok(include_str!("fixtures/api_block_close.json")),
        );
        if fail {
            let mut after: Value = serde_json::from_str(BEFORE).unwrap();
            after["windows"]
                .as_array_mut()
                .unwrap()
                .push(json!({"window_id":NEW,"layers":{}}));
            inspect(&r, &after.to_string());
            api(
                &r,
                Some(SPIKE),
                "session.close_window",
                json!({"window_id":NEW}),
                Ok("{}"),
            );
        } else {
            inspect(
                &r,
                include_str!("fixtures/inspect_after_placeholder_close.json"),
            );
        }
        let result = rex.move_pane_to_new_window(PANE, SPIKE, Some("mon"));
        if fail {
            assert_eq!(result.unwrap_err().to_string(), "move denied");
        } else {
            assert_eq!(result.unwrap(), NEW);
        }
        assert_eq!(r.calls().len(), if fail { 7 } else { 6 });
    }
}

#[test]
fn float_move_restores_focus_and_only_closes_source_if_still_empty() {
    const BEFORE: &str = include_str!("fixtures/inspect_window_autoclose_before.json");
    const AFTER: &str = include_str!("fixtures/inspect_window_autoclose_after.json");
    let before: Value = serde_json::from_str(BEFORE).unwrap();
    let after: Value = serde_json::from_str(AFTER).unwrap();
    let dest = after["active_window_id"].as_str().unwrap();
    let target = before["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["window_id"] == dest)
        .unwrap();
    let anchor = target["focused_block_id"].as_str().unwrap();
    let source = before["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["focused_block_id"] == PANE)
        .unwrap()["window_id"]
        .as_str()
        .unwrap();
    for source_auto_closed in [true, false] {
        let mut before = before.clone();
        before["active_window_id"] = json!(SOURCE);
        let mut after = after.clone();
        // Simulate the alternate server behavior and a changed focus in the previous window.
        if !source_auto_closed {
            after["windows"]
                .as_array_mut()
                .unwrap()
                .push(json!({"window_id":source,"layers":{}}));
        }
        let (rex, r) = spike(&before.to_string());
        inspect(&r, &before.to_string());
        api(
            &r,
            Some(SPIKE),
            "session.move_block",
            json!({"block_id":PANE,"anchor_block_id":anchor}),
            Ok(include_str!("fixtures/api_move_block_float_target.json")),
        );
        api(
            &r,
            Some(SPIKE),
            "session.float_block",
            json!({"block_id":PANE,"bounds":[0.25,0.25,0.5,0.5]}),
            Ok(include_str!("fixtures/api_float_block.json")),
        );
        inspect(&r, &after.to_string());
        if !source_auto_closed {
            api(
                &r,
                Some(SPIKE),
                "session.close_window",
                json!({"window_id":source}),
                Ok(include_str!("fixtures/api_close_window.json")),
            );
        }
        api(
            &r,
            Some(SPIKE),
            "session.focus_window",
            json!({"window_id":SOURCE}),
            Ok(include_str!("fixtures/api_focus_window.json")),
        );
        rex.float_pane_in_window(
            PANE,
            dest,
            &FloatingPaneConfig {
                width: 50,
                height: 50,
                position: FloatingPosition::Centre,
            },
        )
        .unwrap();
        assert_eq!(after["windows"][0]["focused_block_id"], FOCUSED);
        assert_eq!(r.calls().len(), if source_auto_closed { 6 } else { 7 });
    }
}

#[test]
fn float_move_uses_tiled_anchor_if_floating_is_focused_and_restores_block() {
    let before = json!({"session_id":SPIKE,"active_window_id":"source","windows":[
        {"window_id":"source","focused_block_id":"keep","layers":[{"kind":"tiled","blocks":[{"block_id":"keep"},{"block_id":"moving"},{"block_id":"other"}]}]},
        {"window_id":"dest","focused_block_id":"floating","layers":[{"kind":"floating","blocks":[{"block_id":"floating"}]},{"kind":"tiled","blocks":[{"block_id":"anchor"}]}]},
    ]});
    let mut after = before.clone();
    after["active_window_id"] = json!("dest");
    after["windows"][0]["focused_block_id"] = json!("other");
    after["windows"][0]["layers"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let (rex, r) = spike(&before.to_string());
    inspect(&r, &before.to_string());
    api(
        &r,
        Some(SPIKE),
        "session.move_block",
        json!({"block_id":"moving","anchor_block_id":"anchor"}),
        Ok("{}"),
    );
    api(
        &r,
        Some(SPIKE),
        "session.float_block",
        json!({"block_id":"moving","bounds":[0.25,0.25,0.5,0.5]}),
        Ok("{}"),
    );
    inspect(&r, &after.to_string());
    api(
        &r,
        Some(SPIKE),
        "session.focus_block",
        json!({"block_id":"keep"}),
        Ok("{}"),
    );
    let cfg = FloatingPaneConfig {
        width: 50,
        height: 50,
        position: FloatingPosition::Centre,
    };
    rex.float_pane_in_window("moving", "dest", &cfg).unwrap();
    assert_eq!(r.calls().len(), 6);
    rex.float_pane_in_window("anchor", "dest", &cfg).unwrap();
    assert_eq!(r.calls().len(), 6);
}

#[test]
fn bounds_cover_all_thirteen_positions() {
    use FloatingPosition::*;
    for (position, x, y) in [
        (TopLeft, 0., 0.),
        (TopCentre, 0.25, 0.),
        (TopRight, 0.5, 0.),
        (CentreLeft, 0., 0.375),
        (Centre, 0.25, 0.375),
        (CentreRight, 0.5, 0.375),
        (BottomLeft, 0., 0.75),
        (BottomCentre, 0.25, 0.75),
        (BottomRight, 0.5, 0.75),
        (TopLeftCentre, 0.125, 0.),
        (TopRightCentre, 0.375, 0.),
        (BottomLeftCentre, 0.125, 0.75),
        (BottomRightCentre, 0.375, 0.75),
    ] {
        assert_eq!(
            bounds(&FloatingPaneConfig {
                width: 50,
                height: 25,
                position
            }),
            [x, y, 0.5, 0.25]
        );
    }
}
