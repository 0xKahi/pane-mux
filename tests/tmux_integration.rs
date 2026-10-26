//! Opt-in tests use only a private tmux server, never the invoking server.

use pane_mux::tmux::{Tmux, TmuxCli};

struct ServerGuard;

impl Drop for ServerGuard {
    fn drop(&mut self) {
        // Cleanup also runs during an assertion panic.
        let _ = TmuxCli::new(Some("pane-mux-test")).run(&["kill-server"]);
    }
}

/// Query the private server, not an unrelated tmux binary/server version.
fn supports_floating(client: &TmuxCli) -> bool {
    let version = client
        .run(&["display-message", "-p", "#{version}"])
        .unwrap();
    let (major, minor) = version
        .trim()
        .split_once('.')
        .expect("tmux major.minor version");
    let major: u32 = major.parse().expect("tmux major version");
    // Release suffixes such as 3.8a do not change the capability boundary.
    let minor: u32 = minor
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("tmux minor version");
    (major, minor) >= (3, 8)
}

#[test]
fn private_server_tree_capture_and_mutations() {
    if std::env::var("PANE_MUX_TMUX_TESTS").as_deref() != Ok("1") {
        return;
    }
    let client = TmuxCli::new(Some("pane-mux-test"));
    let _guard = ServerGuard;
    let session = client.new_session(Some("work")).unwrap();
    let other = client.new_session(Some("notes")).unwrap();
    let window = client
        .new_window(&session, Some("name with spaces"))
        .unwrap();
    let tree = client.tree().unwrap();
    assert_eq!(tree.sessions.len(), 2);
    let work = tree.sessions.iter().find(|s| s.id == session).unwrap();
    assert_eq!(work.last_attached, 0);
    let logs = work.windows.iter().find(|w| w.id == window).unwrap();
    assert_eq!(logs.name, "name with spaces");
    let pane = &logs.panes[0].id;
    // Verify the command's pre-terminal path against the private server.
    let options =
        pane_mux::commands::move_pane::options(Some("work:name with spaces.0"), &client, false)
            .unwrap();
    assert_eq!(options.cut.as_deref(), Some(pane.as_str()));
    assert!(options.cut_locked);
    assert_eq!(options.header, "[Move]");
    let state = pane_mux::picker::State::load(options, &client, None).unwrap();
    assert_eq!(state.cut.as_deref(), Some(pane.as_str()));
    assert_eq!(state.focus, pane_mux::tmux::Level::Session);
    assert_eq!(state.session.as_deref(), Some(session.as_str()));
    assert_eq!(state.hovered_session().unwrap().name, "work");
    assert_ne!(
        state.sessions()[0].id,
        session,
        "source is not the top session"
    );
    assert!(state.cut_target().is_some());
    // A display message targets the private pane; capture only needs to succeed.
    client.capture(pane).unwrap();
    assert!(
        client
            .current_with_env(Some("private-server"))
            .unwrap()
            .is_some()
    );
    assert!(client.current_with_env(None).unwrap().is_none());
    let error = client.new_session(Some("work")).unwrap_err().to_string();
    assert!(
        error.contains("tmux new-session: duplicate session: work"),
        "{error}"
    );
    client.rename_session(&other, "renamed").unwrap();
    client.rename_window(&window, "renamed window").unwrap();
    let tree = client.tree().unwrap();
    assert!(tree.sessions.iter().any(|s| s.name == "renamed"));
    assert!(
        tree.sessions
            .iter()
            .flat_map(|s| &s.windows)
            .any(|w| w.id == window && w.name == "renamed window")
    );
    client.kill_pane(pane).unwrap();
    assert!(
        !client
            .tree()
            .unwrap()
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .flat_map(|w| &w.panes)
            .any(|p| &p.id == pane)
    );
    let extra = client.new_window(&session, None).unwrap();
    client.kill_window(&extra).unwrap();
    client.kill_session(&other).unwrap();
    assert_eq!(client.tree().unwrap().sessions.len(), 1);
    let floating_supported = supports_floating(&client);
    pane_moves(&client, &session, floating_supported);
    if floating_supported {
        floating_source_moves(&client, &session);
        float_swap_regressions(&client, &session);
        float_stash_round_trip(&client, &session);
    }
}

fn float_stash_round_trip(client: &TmuxCli, session: &str) {
    use pane_mux::tmux::model::FloatGeometry;
    let window = client.new_window(session, Some("float stash")).unwrap();
    client.resize_window(&window, 200, 50).unwrap();
    let tiled = client.float_window(&window).unwrap().panes[0].id.clone();
    let mut originals = Vec::new();
    for g in [
        FloatGeometry {
            x: 119,
            y: 4,
            width: 62,
            height: 17,
        },
        FloatGeometry {
            x: 20,
            y: 10,
            width: 42,
            height: 12,
        },
    ] {
        let id = client
            .run(&[
                "new-pane",
                "-d",
                "-t",
                &window,
                "-x",
                &g.width.to_string(),
                "-y",
                &g.height.to_string(),
                "-X",
                &g.x.to_string(),
                "-Y",
                &g.y.to_string(),
                "-P",
                "-F",
                "#{pane_id}",
            ])
            .unwrap()
            .trim()
            .to_owned();
        originals.push((id, g));
    }
    client.select_pane(&originals[0].0).unwrap();
    let before = client.float_window(&window).unwrap();
    pane_mux::commands::float::run_stash(Some(&tiled), client).unwrap();
    assert_eq!(client.float_window(&window).unwrap().panes.len(), 1);
    let tree = client.tree().unwrap();
    let stash = tree.sessions.iter().find(|s| s.name == "pmux").unwrap();
    let stash_window = stash
        .windows
        .iter()
        .find(|w| client.option("-w", &w.id, "@pmux_source").unwrap() == window)
        .unwrap();
    let stashed = client.float_window(&stash_window.id).unwrap();
    assert_eq!(stashed.panes.len(), 3);
    assert_eq!(stashed.panes.iter().filter(|p| !p.floating).count(), 1);
    for (id, g) in &originals {
        let p = stashed.panes.iter().find(|p| &p.id == id).unwrap();
        assert!(p.floating);
        assert_eq!(FloatGeometry::capture(p), *g);
    }
    // Stash resizing must not change the restore metadata.
    client.resize_window(&stash_window.id, 100, 30).unwrap();
    pane_mux::commands::float::run_unstash(Some(&tiled), client).unwrap();
    let after = client.float_window(&window).unwrap();
    for p in before.panes.iter().filter(|p| p.floating) {
        let restored = after.panes.iter().find(|r| r.id == p.id).unwrap();
        assert_eq!(FloatGeometry::capture(restored), FloatGeometry::capture(p));
        assert_eq!(restored.z, p.z);
        assert_eq!(restored.active, p.active);
    }
    assert!(
        !client
            .tree()
            .unwrap()
            .sessions
            .iter()
            .any(|s| s.name == "pmux")
    );
    pane_mux::commands::float::run_stash(Some(&tiled), client).unwrap();
    client.resize_window(&window, 100, 25).unwrap();
    pane_mux::commands::float::run_unstash(Some(&tiled), client).unwrap();
    let after = client.float_window(&window).unwrap();
    for (id, g) in originals {
        let p = after.panes.iter().find(|p| p.id == id).unwrap();
        let scaled = FloatGeometry::capture(p);
        assert_eq!(scaled, g.scaled((200, 50), (100, 25)));
        assert!(scaled.x >= 0 && scaled.y >= 0);
        assert!(scaled.x + scaled.width <= 100 && scaled.y + scaled.height <= 25);
    }
}

/// Regression coverage is called only after checking the private server's version.
fn float_swap_regressions(client: &TmuxCli, session: &str) {
    use pane_mux::commands::float::{run_stash, run_toggle, run_unstash};
    use pane_mux::tmux::model::FloatGeometry;
    use std::{
        fs,
        thread::sleep,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    struct Logs(std::path::PathBuf);
    impl Drop for Logs {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
    let logs = Logs(std::env::temp_dir().join(format!(
            "pane-mux-winch-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir(&logs.0).unwrap();
    let logger = |name: &str| {
        let log = quote(logs.0.join(format!("{name}.log")).to_str().unwrap());
        let ready = quote(logs.0.join(format!("{name}.ready")).to_str().unwrap());
        let script = format!(
            "trap {} WINCH; touch {ready}; while :; do sleep 1; done",
            quote(&format!("echo WINCH >> {log}"))
        );
        format!("exec sh -c {}", quote(&script))
    };
    let window = client
        .new_window(session, Some("swap regressions"))
        .unwrap();
    client.resize_window(&window, 200, 50).unwrap();
    let tiled = client.float_window(&window).unwrap().panes[0].id.clone();
    client
        .run(&["respawn-pane", "-k", "-t", &tiled, &logger("main")])
        .unwrap();
    let float = client
        .run(&[
            "new-pane",
            "-d",
            "-t",
            &window,
            "-x",
            "62",
            "-y",
            "17",
            "-X",
            "119",
            "-Y",
            "4",
            "-P",
            "-F",
            "#{pane_id}",
            &logger("float"),
        ])
        .unwrap()
        .trim()
        .to_owned();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !["main", "float"]
        .iter()
        .all(|name| logs.0.join(format!("{name}.ready")).exists())
    {
        assert!(
            Instant::now() < deadline,
            "SIGWINCH loggers failed to start"
        );
        sleep(Duration::from_millis(20));
    }
    // Settle setup signals before measuring only the stash/unstash round trip.
    sleep(Duration::from_millis(1200));
    for name in ["main", "float"] {
        fs::write(logs.0.join(format!("{name}.log")), "").unwrap();
    }
    let original = client
        .float_window(&window)
        .unwrap()
        .panes
        .into_iter()
        .find(|p| p.id == float)
        .unwrap();
    run_stash(Some(&tiled), client).unwrap();
    run_unstash(Some(&tiled), client).unwrap();
    sleep(Duration::from_millis(1200));
    for name in ["main", "float"] {
        let text = fs::read_to_string(logs.0.join(format!("{name}.log"))).unwrap();
        assert!(
            text.is_empty(),
            "{name} resized during stash/unstash: {text}"
        );
    }
    let restored = client
        .float_window(&window)
        .unwrap()
        .panes
        .into_iter()
        .find(|p| p.id == float)
        .unwrap();
    assert_eq!(
        FloatGeometry::capture(&restored),
        FloatGeometry::capture(&original)
    );

    client
        .run(&[
            "new-pane", "-d", "-t", &window, "-x", "42", "-y", "12", "-X", "30", "-Y", "10",
        ])
        .unwrap();
    let floats_before_zoom = client
        .float_window(&window)
        .unwrap()
        .panes
        .into_iter()
        .filter(|p| p.floating)
        .collect::<Vec<_>>();
    assert_eq!(floats_before_zoom.len(), 2);
    // Two tiled panes make preservation of the same zoomed slot observable.
    client.run(&["split-window", "-d", "-t", &tiled]).unwrap();
    client.select_pane(&tiled).unwrap();
    client.toggle_zoom(&tiled).unwrap();
    let before = client.float_window(&window).unwrap();
    for original in &floats_before_zoom {
        let hidden = before.panes.iter().find(|p| p.id == original.id).unwrap();
        assert!(hidden.floating && !hidden.flags.contains('F'));
        assert_eq!(
            FloatGeometry::capture(hidden),
            FloatGeometry::capture(original)
        );
        assert_eq!(hidden.z, original.z);
    }
    run_toggle(Some(&tiled), client).unwrap();
    let after = client.float_window(&window).unwrap();
    assert!(after.zoomed);
    assert_eq!(after.panes.iter().find(|p| p.active).unwrap().id, tiled);
    assert!(!after.panes.iter().any(|p| p.id == float));
    let stash_window = client
        .tree()
        .unwrap()
        .sessions
        .into_iter()
        .find(|s| s.name == "pmux")
        .unwrap()
        .windows
        .into_iter()
        .find(|w| client.option("-w", &w.id, "@pmux_source").unwrap() == window)
        .unwrap()
        .id;
    let stashed = client.float_window(&stash_window).unwrap();
    for original in &floats_before_zoom {
        let pane = stashed.panes.iter().find(|p| p.id == original.id).unwrap();
        assert_eq!(
            FloatGeometry::capture(pane),
            FloatGeometry::capture(original)
        );
        assert_eq!(pane.options["@pmux_z"], original.z.to_string());
    }

    // A stash containing only floats remains reusable after losing its label.
    let placeholder = stashed
        .panes
        .iter()
        .find(|p| !p.floating)
        .unwrap()
        .id
        .clone();
    client.kill_pane(&placeholder).unwrap();
    client.toggle_zoom(&tiled).unwrap();
    let second = client
        .run(&[
            "new-pane",
            "-d",
            "-t",
            &window,
            "-x",
            "42",
            "-y",
            "12",
            "-X",
            "20",
            "-Y",
            "10",
            "-P",
            "-F",
            "#{pane_id}",
        ])
        .unwrap()
        .trim()
        .to_owned();
    let dummy_source = client
        .new_dummy_float(&window, "20", "10", None, None)
        .unwrap();
    let dummy_stash = client
        .new_dummy_float(&stash_window, "20", "10", None, None)
        .unwrap();
    run_stash(Some(&tiled), client).unwrap();
    let stashed = client.float_window(&stash_window).unwrap();
    assert_eq!(stashed.panes.len(), 3);
    assert!(
        stashed
            .panes
            .iter()
            .all(|p| p.floating && p.title != "pane-mux-dummy")
    );
    assert!(stashed.panes.iter().any(|p| p.id == second));
    let all = client.tree().unwrap();
    assert!(
        !all.sessions
            .iter()
            .flat_map(|s| &s.windows)
            .flat_map(|w| &w.panes)
            .any(|p| p.id == dummy_source || p.id == dummy_stash)
    );
    run_unstash(Some(&tiled), client).unwrap();
    let after = client.float_window(&window).unwrap();
    assert!(after.panes.iter().any(|p| p.id == float && p.floating));
    assert!(after.panes.iter().any(|p| p.id == second && p.floating));
    assert_eq!(after.panes.iter().filter(|p| p.floating).count(), 3);
    assert!(
        !client
            .tree()
            .unwrap()
            .sessions
            .iter()
            .any(|s| s.name == "pmux")
    );
    // Stop loggers before removing their files.
    client.kill_window(&window).unwrap();
}

fn floating_source_moves(client: &TmuxCli, source_session: &str) {
    use pane_mux::config::{FloatingPaneConfig, FloatingPosition};

    let source = client
        .new_window(source_session, Some("floating source"))
        .unwrap();
    let destination = client
        .new_window(source_session, Some("floating destination"))
        .unwrap();
    let floating = FloatingPaneConfig {
        width: 80,
        height: 30,
        position: FloatingPosition::BottomCentre,
    };
    for kind in ["session", "window", "pane"] {
        let before = client.tree().unwrap();
        let remaining = before
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.id == source)
            .unwrap()
            .panes
            .clone();
        let pane = client
            .run(&["new-pane", "-d", "-P", "-F", "#{pane_id}", "-t", &source])
            .unwrap()
            .trim()
            .to_owned();
        let tree = client.tree().unwrap();
        let source_window = tree
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.id == source)
            .unwrap();
        assert!(
            source_window
                .panes
                .iter()
                .find(|p| p.id == pane)
                .unwrap()
                .floating,
            "{kind}: source must be floating"
        );

        let (session, window) = match kind {
            "session" => client
                .move_pane_to_new_session(
                    &pane,
                    Some("floating-session"),
                    Some("floating-to-tiled"),
                )
                .unwrap(),
            "window" => (
                source_session.to_owned(),
                client
                    .move_pane_to_new_window(&pane, source_session, Some("floating-to-tiled"))
                    .unwrap(),
            ),
            "pane" => {
                client
                    .float_pane_in_window(&pane, &destination, &floating)
                    .unwrap();
                (source_session.to_owned(), destination.clone())
            }
            _ => unreachable!(),
        };
        let tree = client.tree().unwrap();
        let target_session = tree.sessions.iter().find(|s| s.id == session).unwrap();
        if kind == "session" {
            assert_eq!(target_session.windows.len(), 1);
        }
        let target = target_session
            .windows
            .iter()
            .find(|w| w.id == window)
            .unwrap();
        let moved = target.panes.iter().find(|p| p.id == pane).unwrap();
        assert_eq!(
            moved.floating,
            kind == "pane",
            "{kind}: resulting floating flag"
        );
        assert_eq!(target.panes.len(), if kind == "pane" { 2 } else { 1 });
        let source_window = tree
            .sessions
            .iter()
            .find(|s| s.id == source_session)
            .unwrap()
            .windows
            .iter()
            .find(|w| w.id == source)
            .unwrap();
        assert_eq!(
            source_window.panes, remaining,
            "{kind}: source keeps its other panes"
        );
    }
}

fn pane_moves(client: &TmuxCli, source_session: &str, floating_supported: bool) {
    use pane_mux::config::settings::{FloatingPaneConfig, FloatingPosition};

    // Keep a separate source window alive throughout detached moves.
    client
        .run(&["set-option", "-g", "base-index", "1"])
        .unwrap();
    let source = client
        .new_window(source_session, Some("move source"))
        .unwrap();
    let pane = client
        .run(&["display-message", "-p", "-t", &source, "#{pane_id}"])
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(client.resolve_pane(&pane).unwrap(), pane);
    assert!(client.resolve_pane("nosuch:9.9").is_err());

    let before = client.tree().unwrap();
    assert!(
        client
            .move_pane_to_new_session(&pane, Some("work"), None)
            .is_err()
    );
    assert_eq!(client.tree().unwrap(), before);
    // A placeholder created before a failing break-pane must be rolled back.
    assert!(
        client
            .move_pane_to_new_session("%999999", Some("rollback"), None)
            .is_err()
    );
    assert!(
        !client
            .tree()
            .unwrap()
            .sessions
            .iter()
            .any(|s| s.name == "rollback")
    );

    let active = || {
        client
            .run(&[
                "display-message",
                "-p",
                "-t",
                source_session,
                "#{window_id} #{pane_id}",
            ])
            .unwrap()
    };
    let current = active();
    let (session, window) = client
        .move_pane_to_new_session(&pane, Some("moved"), Some("tail"))
        .unwrap();
    let tree = client.tree().unwrap();
    let moved = tree.sessions.iter().find(|s| s.id == session).unwrap();
    assert_eq!(moved.windows.len(), 1);
    assert_eq!(moved.windows[0].id, window);
    assert_eq!(moved.windows[0].index, 1);
    assert_eq!(moved.windows[0].name, "tail");
    assert_eq!(moved.windows[0].panes[0].id, pane);
    assert!(
        !tree
            .sessions
            .iter()
            .filter(|s| s.id == source_session)
            .flat_map(|s| &s.windows)
            .any(|w| w.id == source)
    );
    assert_eq!(active(), current);

    let new_window = client
        .move_pane_to_new_window(&pane, source_session, Some("scratch"))
        .unwrap();
    let tree = client.tree().unwrap();
    assert!(!tree.sessions.iter().any(|s| s.id == session));
    let target = tree
        .sessions
        .iter()
        .find(|s| s.id == source_session)
        .unwrap();
    let moved = target.windows.iter().find(|w| w.id == new_window).unwrap();
    assert_eq!(moved.name, "scratch");
    assert_eq!(moved.panes.len(), 1);
    assert_eq!(moved.panes[0].id, pane);
    let destination = target
        .windows
        .iter()
        .find(|w| w.id != new_window)
        .unwrap()
        .id
        .clone();
    // Make the geometry deterministic, independent of the terminal running tests.
    client
        .run(&[
            "resize-window",
            "-t",
            &destination,
            "-x",
            "200",
            "-y",
            "100",
        ])
        .unwrap();
    let floating = FloatingPaneConfig {
        width: 80,
        height: 30,
        position: FloatingPosition::BottomCentre,
    };
    let result = client.float_pane_in_window(&pane, &destination, &floating);
    if floating_supported {
        result.unwrap();
    } else {
        let error = result.unwrap_err().to_string();
        assert!(error.contains("tmux new-pane:"), "{error}");
        // Unsupported floating fails before moving the source pane.
        assert_eq!(client.resolve_pane(&pane).unwrap(), pane);
        assert!(
            client
                .tree()
                .unwrap()
                .sessions
                .iter()
                .flat_map(|s| &s.windows)
                .any(|w| w.id == new_window && w.panes.iter().any(|p| p.id == pane))
        );
        return;
    }
    let tree = client.tree().unwrap();
    let windows = &tree
        .sessions
        .iter()
        .find(|s| s.id == source_session)
        .unwrap()
        .windows;
    assert!(!windows.iter().any(|w| w.id == new_window));
    let target = windows.iter().find(|w| w.id == destination).unwrap();
    assert_eq!(
        target.panes.iter().find(|p| p.id == pane).unwrap().floating,
        floating_supported
    );
    if floating_supported {
        let geometry = client
            .run(&[
                "display-message",
                "-p",
                "-t",
                &pane,
                "#{pane_width} #{pane_height} #{pane_left} #{pane_top}",
            ])
            .unwrap();
        let geometry: Vec<u32> = geometry
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        // Requested percentages include the two border cells.
        assert_eq!(geometry[0] + 2, 160);
        assert_eq!(geometry[1] + 2, 30);
        // Floating borders occupy one cell on each side.
        assert!(geometry[2].abs_diff(20) <= 1, "{geometry:?}");
        assert!(geometry[3].abs_diff(70) <= 2, "{geometry:?}");
    }
    assert_eq!(active(), current);

    // Exercise break-pane's split-pane path too (not just whole-window moves).
    let split = client
        .run(&[
            "split-window",
            "-d",
            "-P",
            "-F",
            "#{pane_id}",
            "-t",
            &destination,
        ])
        .unwrap()
        .trim()
        .to_owned();
    let (split_session, split_window) = client
        .move_pane_to_new_session(&split, Some("split-moved"), None)
        .unwrap();
    assert_ne!(split_window, destination);
    let tree = client.tree().unwrap();
    let split_target = tree
        .sessions
        .iter()
        .find(|s| s.id == split_session)
        .unwrap();
    assert_eq!(split_target.windows.len(), 1);
    assert_eq!(split_target.windows[0].panes[0].id, split);
    assert!(
        tree.sessions
            .iter()
            .flat_map(|s| &s.windows)
            .any(|w| w.id == destination)
    );
}
