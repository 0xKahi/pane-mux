use std::{path::PathBuf, time::Duration};

use serde_json::{Value, json};

use super::{
    FixtureRunner, RexCli, RunnerError,
    client::bounds,
    history,
    snapshot::{CAPTURE, SCRIPT, Snapshot, UNSCOPED},
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

const SNAPSHOT: &str = include_str!("fixtures/snapshot.json");
const SELF_FLOATING: &str = include_str!("fixtures/snapshot_self_floating.json");
const SELF_MINIMIZED: &str = include_str!("fixtures/snapshot_self_minimized.json");
const SELF_TILED: &str = include_str!("fixtures/snapshot_self_tiled.json");
const HOME: &str = "session:home";
const SIDECAR: &str = "session:sidecar";
const PI: &str = "window:pi";
const PICKER: &str = "block:picker";
const WORK: &str = "session:00000000-0000-7000-8000-000000000001";
const HERE: &str = "block:00000000-0000-7000-8000-0000006e22b7";
const WINDOW: &str = "window:00000000-0000-7000-8000-000000000025";
const APP: &str = "client:00000000-0000-7000-8000-000000000022";

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

fn capture_reply(r: &FixtureRunner, session: &str, block: &str, output: &str) {
    let args = json!({"session_id": session, "block_id": block, "budget": 131_072}).to_string();
    reply(
        r,
        &["do", "-e", CAPTURE, "--json", "--args", &args],
        Ok(output),
    );
}

fn snapshot(sessions: &[Value]) -> String {
    json!({"sessions":sessions}).to_string()
}

#[test]
fn mock_tree_preserves_rex_order_and_lists_other_minimized_floats() {
    let (rex, runner) = setup(SNAPSHOT, None);
    let tree = rex.tree().unwrap();
    assert_eq!(
        tree.sessions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["work", "notes", "scratch"]
    );
    let windows = &tree.sessions[0].windows;
    assert_eq!(
        windows
            .iter()
            .take(3)
            .map(|w| (w.index, w.name.as_str()))
            .collect::<Vec<_>>(),
        [(1, "editor"), (2, "logs"), (3, "build")]
    );
    assert_eq!(windows[0].panes[0].command, "vim");
    let w = windows.iter().find(|w| w.active).unwrap();
    assert_eq!(w.id, WINDOW);
    assert_eq!(
        w.panes
            .iter()
            .map(|p| (p.index, p.floating, p.active))
            .collect::<Vec<_>>(),
        [(1, false, true), (2, true, false)]
    );
    assert_eq!(rex.current().unwrap(), None);
    assert!(tree.stash_counts().is_empty());
    assert!(
        windows
            .iter()
            .all(|w| !w.zoomed && w.panes.iter().all(|p| p.mode.is_empty()))
    );
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0].env_remove, UNSCOPED);
}

fn picker_current() -> Current {
    Current {
        session_id: HOME.into(),
        window_id: PI.into(),
        pane_id: PICKER.into(),
    }
}

#[test]
fn snapshot_filters_only_the_floating_self_block_before_numbering() {
    for (fixture, tiled) in [
        (SELF_FLOATING, false),
        (SELF_MINIMIZED, false),
        (SELF_TILED, true),
    ] {
        let parse = || serde_json::from_str::<Snapshot>(fixture).unwrap();
        let (tree, current) = parse().into_tree(Some(PICKER));
        assert_eq!(current, Some(picker_current()));
        let rows = |tree: &Tree| {
            tree.sessions[0].windows[0]
                .panes
                .iter()
                .map(|p| (p.index, p.command.clone(), p.floating))
                .collect::<Vec<_>>()
        };
        let (unfiltered, no_current) = parse().into_tree(None);
        assert_eq!(no_current, None);
        assert_eq!(
            rows(&unfiltered),
            [
                (1, "nvim".into(), false),
                (2, "zsh".into(), false),
                (3, "pane-mux".into(), !tiled),
                (4, "htop".into(), true),
            ]
        );
        if tiled {
            assert_eq!(tree, unfiltered);
        } else {
            assert_eq!(
                rows(&tree),
                [
                    (1, "nvim".into(), false),
                    (2, "zsh".into(), false),
                    (3, "htop".into(), true),
                ]
            );
        }
    }
}

#[test]
fn refresh_keeps_self_float_hidden_and_clears_stale_current() {
    let (rex, r) = setup(SELF_FLOATING, Some(PICKER));
    reply(&r, &["do", "-e", SCRIPT, "--json"], Ok(SELF_FLOATING));
    for _ in 0..2 {
        let tree = rex.tree().unwrap();
        let panes = &tree.sessions[0].windows[0].panes;
        assert_eq!(panes.len(), 3);
        assert!(panes.iter().all(|p| p.id != PICKER));
        assert_eq!(rex.current().unwrap(), Some(picker_current()));
    }
    reply(&r, &["do", "-e", SCRIPT, "--json"], Ok(SNAPSHOT));
    rex.tree().unwrap();
    assert_eq!(rex.current().unwrap(), None);
    assert_eq!(r.calls().len(), 3);
}

#[test]
fn current_builds_tree_once_and_marks_only_listed_rows() {
    for (fixture, tiled) in [(SELF_FLOATING, false), (SELF_TILED, true)] {
        let (rex, r) = setup(fixture, Some(PICKER));
        assert_eq!(rex.current().unwrap(), Some(picker_current()));
        reply(&r, &["do", "-e", SCRIPT, "--json"], Ok(fixture));
        let state = State::new(
            PickerOptions::default(),
            rex.tree().unwrap(),
            rex.current().unwrap(),
            rex.suppressed_pane(),
        );
        assert!(state.is_current(&state.sessions()[0].id));
        assert!(state.is_current(&state.windows()[0].id));
        assert_eq!(
            state
                .panes()
                .iter()
                .filter(|p| state.is_current(&p.id))
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            if tiled { &[PICKER][..] } else { &[] }
        );
        assert_eq!(state.preview_suppressed(), tiled);
        assert_eq!(r.calls().len(), 2);
    }
    let (rex, r) = setup(SELF_FLOATING, Some("block:unknown"));
    assert_eq!(rex.current().unwrap(), None);
    assert_eq!(rex.current().unwrap(), None);
    assert_eq!(r.calls().len(), 1);
    let (rex, r) = setup(SELF_FLOATING, None);
    assert_eq!(rex.current().unwrap(), None);
    assert!(r.calls().is_empty());
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
    let (rex, r) = setup(SNAPSHOT, Some(HERE));
    let tree = rex.tree().unwrap();
    let target = Target {
        session_id: WORK.into(),
        window_id: Some(WINDOW.into()),
        pane_id: Some(HERE.into()),
        session: "work".into(),
        window_index: 4,
        pane_index: 2,
    };
    for (level, id, display) in [
        (Level::Session, WORK, "work"),
        (Level::Window, WINDOW, "work:4"),
        (Level::Pane, HERE, "work:4.2"),
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
fn move_pane_without_target_from_self_float_cannot_paste() {
    let (rex, r) = setup(SELF_FLOATING, Some(PICKER));
    let options = move_pane::options(None, &rex, rex.in_client()).unwrap();
    assert_eq!(options.cut.as_deref(), Some(PICKER));
    assert_eq!(options.initial_pane.as_deref(), Some(PICKER));
    assert!(options.cut_locked);

    for _ in 0..2 {
        reply(&r, &["do", "-e", SCRIPT, "--json"], Ok(SELF_FLOATING));
    }
    capture_reply(
        &r,
        HOME,
        "block:nvim",
        include_str!("fixtures/capture_alt_screen.json"),
    );
    let mut state = State::load(options, &rex, rex.suppressed_pane()).unwrap();
    let keymap = Keymap::from_toml(&preset::keymap(), None).unwrap();
    assert!(
        state
            .handle_key("p".parse().unwrap(), &keymap, &rex)
            .is_none()
    );
    assert_eq!(state.cut, None);
    assert_eq!(state.message.as_deref(), Some("Cut pane no longer exists"));
}

#[test]
fn capture_uses_one_unscoped_call_and_keeps_colored_screen_rows() {
    let (rex, r) = setup(SNAPSHOT, None);
    capture_reply(
        &r,
        WORK,
        HERE,
        include_str!("fixtures/capture_history.json"),
    );
    assert_eq!(
        rex.capture(HERE).unwrap(),
        "\x1b[31mscreen 1\x1b[0m\nscreen 2\n"
    );

    let calls = r.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].args,
        ["--autostart=false", "do", "-e", SCRIPT, "--json"]
    );
    let capture = &calls[1];
    assert_eq!(capture.env_remove, UNSCOPED);
    assert_eq!(capture.args.len(), 7);
    assert_eq!(
        capture.args[..6],
        ["--autostart=false", "do", "-e", CAPTURE, "--json", "--args"]
    );
    assert_eq!(
        serde_json::from_str::<Value>(&capture.args[6]).unwrap(),
        json!({"session_id": WORK, "block_id": HERE, "budget": 131_072})
    );

    assert_eq!(rex.preview_interval(), Duration::from_secs(1));
    assert!(!rex.cross_session_moves());
}

#[test]
fn capture_keeps_all_rows_on_a_partly_filled_screen() {
    let (rex, r) = setup(SNAPSHOT, None);
    capture_reply(&r, WORK, HERE, include_str!("fixtures/capture_short.json"));
    assert_eq!(rex.capture(HERE).unwrap(), "first\nsecond");
}

#[test]
fn capture_drops_the_cut_line_from_partial_output() {
    let (rex, r) = setup(SNAPSHOT, None);
    for (output, expected) in [
        (
            include_str!("fixtures/capture_partial.json"),
            "screen 1\nscreen 2",
        ),
        (
            include_str!("fixtures/capture_partial_short.json"),
            "screen\n",
        ),
        (include_str!("fixtures/capture_partial_no_newline.json"), ""),
    ] {
        capture_reply(&r, WORK, HERE, output);
        assert_eq!(rex.capture(HERE).unwrap(), expected);
    }
}

#[test]
fn capture_leaves_alternate_screen_unchanged() {
    let (rex, r) = setup(SNAPSHOT, None);
    capture_reply(
        &r,
        WORK,
        HERE,
        include_str!("fixtures/capture_alt_screen.json"),
    );
    assert_eq!(rex.capture(HERE).unwrap(), "nvim\n~\n");
}

#[test]
fn capture_keeps_all_output_if_row_count_is_zero() {
    let (rex, r) = setup(SNAPSHOT, None);
    capture_reply(
        &r,
        WORK,
        HERE,
        include_str!("fixtures/capture_zero_rows.json"),
    );
    assert_eq!(rex.capture(HERE).unwrap(), "first\nsecond\n");
}

#[test]
fn capture_reports_missing_pane_after_refresh() {
    let (rex, r) = setup(SNAPSHOT, None);
    rex.tree().unwrap();
    reply(&r, &["do", "-e", SCRIPT, "--json"], Ok(SNAPSHOT));
    assert_eq!(
        rex.capture("block:missing").unwrap_err().to_string(),
        "pane not found: block:missing"
    );
    let calls = r.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|call| {
        call.args == ["--autostart=false", "do", "-e", SCRIPT, "--json"]
            && call.env_remove == UNSCOPED
    }));
}

const LAYOUT: &str = "com.superlogical.terminal.shell";

#[test]
fn creates_renames_and_deletes_with_exact_rex_api_calls() {
    let (rex, r) = setup(SNAPSHOT, None);
    rex.tree().unwrap();
    api(
        &r,
        None,
        "session.create",
        json!({"label":"notes","initial_windows":[{"layout":{"block":{"flavor":LAYOUT}}}]}),
        Ok(include_str!("fixtures/api_create.json")),
    );
    assert_eq!(rex.new_session(Some("notes")).unwrap(), SPIKE);
    new_window(&r, WORK, "logs");
    assert_eq!(rex.new_window(WORK, Some("logs")).unwrap(), NEW);
    api(
        &r,
        Some(WORK),
        "session.set_label",
        json!({"label":"new"}),
        Ok(include_str!("fixtures/api_set_label.json")),
    );
    rex.rename_session(WORK, "new").unwrap();
    api(
        &r,
        Some(WORK),
        "session.set_window_label",
        json!({"window_id":WINDOW,"label":"new"}),
        Ok(include_str!("fixtures/api_set_window_label.json")),
    );
    rex.rename_window(WINDOW, "new").unwrap();
    api(
        &r,
        Some(WORK),
        "block.close",
        json!({"block_id":HERE}),
        Ok(include_str!("fixtures/api_block_close.json")),
    );
    rex.kill_pane(HERE).unwrap();
    api(
        &r,
        Some(WORK),
        "session.close_window",
        json!({"window_id":WINDOW}),
        Ok(include_str!("fixtures/api_close_window.json")),
    );
    rex.kill_window(WINDOW).unwrap();
    api(
        &r,
        Some(WORK),
        "session.destroy",
        json!({}),
        Ok(include_str!("fixtures/api_destroy.json")),
    );
    rex.kill_session(WORK).unwrap();
    assert_eq!(r.calls().len(), 8);
}

#[test]
fn failed_rename_is_displayed_without_closing_picker() {
    let (rex, r) = setup(SNAPSHOT, Some(HERE));
    let mut state = State::new(
        PickerOptions::default(),
        rex.tree().unwrap(),
        rex.current().unwrap(),
        Some(HERE.into()),
    );
    let error = "Error: label is not allowed";
    api(
        &r,
        Some(WORK),
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
    let (rex, r) = setup(SNAPSHOT, Some(HERE));
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
    let (rex, r) = setup(SNAPSHOT, Some(HERE));
    api(
        &r,
        Some(WORK),
        "session.focus_window",
        json!({"window_id":WINDOW}),
        Ok("{}"),
    );
    reply(&r, &["client", "ls", "--json"], Ok(r#"{"clients":{}}"#));
    let hint = rex.switch(WINDOW).unwrap().unwrap();
    assert!(hint.contains("work") && hint.ends_with("no connected Rex app client"));
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
    let (rex, r) = setup(SNAPSHOT, Some(HERE));
    let rex = rex
        .with_history_path(path.0.clone())
        .with_app_client(APP.into());
    reply(
        &r,
        &["whoami", "--json"],
        Ok(r#"{"api_revision":18,"endpoint":"endpoint"}"#),
    );
    rex.check_api_revision().unwrap();
    let target = "session:00000000-0000-7000-8000-000000000005";
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
        hint.contains("notes")
            && hint.contains("Turn on Remote Control in Rex settings")
            && hint.ends_with("Error: Remote Control is disabled")
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(&path.0).unwrap()).unwrap();
    assert_eq!(stored, json!({"endpoint":[target,WORK]}));
    let (next, r) = setup(SNAPSHOT, None);
    let next = next.with_history_path(path.0.clone());
    reply(
        &r,
        &["whoami", "--json"],
        Ok(r#"{"api_revision":18,"endpoint":"endpoint"}"#),
    );
    next.check_api_revision().unwrap();
    let state = recent(next.tree().unwrap(), target, &[]);
    assert_eq!(state.sessions()[1].id, WORK);
    assert_eq!(state.session.as_deref(), Some(WORK));
}

#[test]
fn switching_from_hidden_float_ranks_and_records_the_session_left() {
    let path = TempPath::new();
    let (rex, r) = setup(SELF_FLOATING, Some(PICKER));
    let rex = rex
        .with_history_path(path.0.clone())
        .with_app_client(APP.into());
    reply(
        &r,
        &["whoami", "--json"],
        Ok(r#"{"api_revision":18,"endpoint":"endpoint"}"#),
    );
    rex.check_api_revision().unwrap();
    history::record(&path.0, "endpoint", None, SIDECAR).unwrap();
    let tree = rex.tree().unwrap();
    assert!(tree.sessions[1].last_attached > tree.sessions[0].last_attached);
    let state = State::new(
        PickerOptions {
            order: SessionOrder::Recent,
            ..PickerOptions::default()
        },
        tree,
        rex.current().unwrap(),
        rex.suppressed_pane(),
    );
    assert_eq!(state.sessions()[0].id, HOME);
    assert_eq!(state.session.as_deref(), Some(SIDECAR));

    reply(
        &r,
        &[
            "-C",
            APP,
            "do",
            "session.select",
            "session_id=session:sidecar",
            "--json",
        ],
        Ok(include_str!("fixtures/select_success.json")),
    );
    assert_eq!(rex.switch(SIDECAR).unwrap(), None);
    let stored: Value = serde_json::from_slice(&std::fs::read(&path.0).unwrap()).unwrap();
    assert_eq!(stored, json!({"endpoint":[SIDECAR,HOME]}));
}

#[test]
fn recent_order_uses_history_then_rex_order_and_existing_exclusions() {
    let path = TempPath::new();
    let mut tree = Tree {
        sessions: ["work", "inbox", "notes", "scratch"]
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
        "inbox",
        &[],
        &["inbox", "work", "notes", "scratch"],
    );
    history::record(&path.0, "a", Some("work"), "notes").unwrap();
    history::record(&path.0, "a", None, "deleted").unwrap();
    check(
        &mut tree,
        "a",
        "work",
        &[],
        &["work", "notes", "inbox", "scratch"],
    );
    history::record(&path.0, "a", None, "scratch").unwrap();
    check(
        &mut tree,
        "a",
        "work",
        &["scratch", "work"],
        &["work", "notes", "inbox", "scratch"],
    );
    check(
        &mut tree,
        "other",
        "work",
        &[],
        &["work", "inbox", "notes", "scratch"],
    );
    for i in 0..55 {
        history::record(&path.0, "cap", None, &i.to_string()).unwrap();
    }
    let all: Value = serde_json::from_slice(&std::fs::read(&path.0).unwrap()).unwrap();
    assert_eq!(all["cap"].as_array().unwrap().len(), 50);
    assert_eq!(all["cap"][0], "54");
    assert_eq!(all["cap"][49], "5");
}

const SPIKE: &str = "session:00000000-0000-7000-8000-000000000041";
const NEW: &str = "window:00000000-0000-7000-8000-000000000048";
const PLACEHOLDER: &str = "block:00000000-0000-7000-8000-000000000047";
const PANE: &str = "block:00000000-0000-7000-8000-000000000045";
const SOURCE: &str = "window:00000000-0000-7000-8000-000000000043";
const FOCUSED: &str = "block:00000000-0000-7000-8000-000000000042";

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
