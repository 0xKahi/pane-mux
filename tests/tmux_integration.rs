//! Opt-in tests use only a private tmux server, never the invoking server.

use pane_mux::tmux::{Tmux, TmuxCli};

struct ServerGuard;

impl Drop for ServerGuard {
    fn drop(&mut self) {
        // Cleanup also runs during an assertion panic.
        let _ = TmuxCli::new(Some("pane-mux-test")).run(&["kill-server"]);
    }
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
}
