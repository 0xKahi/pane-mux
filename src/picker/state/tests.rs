use super::*;
use crate::{
    config::preset,
    tmux::{FakeTmux, sample_tree},
};

fn fixture() -> FakeTmux {
    let mut tree = sample_tree(&["work", "dotfiles", "extra"]);
    tree.sessions[0].windows[0].active = false;
    tree.sessions[0].windows.push(Window {
        id: "@10".into(),
        index: 1,
        name: "editor".into(),
        active: true,
        panes: vec![
            Pane {
                id: "%10".into(),
                index: 0,
                active: true,
                command: "nvim".into(),
            },
            Pane {
                id: "%11".into(),
                index: 2,
                active: false,
                command: "sh".into(),
            },
        ],
    });
    tree.sessions[1].windows.push(Window {
        id: "@20".into(),
        index: 2,
        name: "logs".into(),
        active: false,
        panes: vec![Pane {
            id: "%20".into(),
            index: 0,
            active: true,
            command: "tail".into(),
        }],
    });
    FakeTmux::new(tree)
        .with_current(Current {
            session_id: "$0".into(),
            window_id: "@10".into(),
            pane_id: "%10".into(),
        })
        .with_capture(
            "%10",
            &(0..50).map(|i| format!("line {i}\n")).collect::<String>(),
        )
        .with_capture("%11", "\u{1b}[31msecond pane\u{1b}[0m\n")
}

fn state(fake: &FakeTmux) -> State {
    State::load(PickerOptions::default(), fake, None).unwrap()
}

fn key(state: &mut State, fake: &FakeTmux, text: &str) -> Option<Exit> {
    let keymap = Keymap::from_toml(&preset::keymap(), None).unwrap();
    state.handle_key(text.parse().unwrap(), &keymap, fake)
}

fn type_text(state: &mut State, fake: &FakeTmux, text: &str) {
    for c in text.chars() {
        key(state, fake, &c.to_string());
    }
}

fn find(state: &mut State, fake: &FakeTmux, query: &str, previous: bool, case: Case) {
    state.apply(&Action::Find { previous, case }, fake);
    type_text(state, fake, query);
}

#[test]
fn default_find_bindings_and_which_listing() {
    let fake = fixture();
    let mut state = state(&fake);
    let map = Keymap::from_toml(&preset::keymap(), None).unwrap();
    for (keys, action) in [
        (
            vec!["<Space>", "/"],
            Action::Find {
                previous: false,
                case: Case::Smart,
            },
        ),
        (vec!["n"], Action::FindArrow { previous: false }),
        (vec!["N"], Action::FindArrow { previous: true }),
    ] {
        let keys: Vec<Key> = keys.into_iter().map(|k| k.parse().unwrap()).collect();
        let chord = map.picker.iter().find(|c| c.on == keys).unwrap();
        assert_eq!(chord.run, vec![action]);
        assert!(chord.desc.is_some());
    }
    key(&mut state, &fake, "<Space>");
    let which = state.which.as_ref().unwrap();
    assert_eq!(which.candidates[0].on[1], "/".parse().unwrap());
    key(&mut state, &fake, "/");
    assert!(state.which.is_none());
    assert!(
        matches!(state.popup, Some(Popup::Input { ref title, ref value, purpose: InputPurpose::Find { previous: false, case: Case::Smart } }) if title == "Find next:" && value.is_empty())
    );
}

#[test]
fn find_prompt_enter_and_escape_keep_finder_and_cursor_without_submit_effects() {
    for close in ["<Enter>", "<Esc>"] {
        let fake = fixture();
        let mut state = state(&fake);
        find(&mut state, &fake, "dot", false, Case::Smart);
        assert_eq!(state.session.as_deref(), Some("$1"));
        let finder = state.finder.clone();
        let calls = fake.calls();
        key(&mut state, &fake, close);
        assert!(state.popup.is_none());
        assert_eq!(state.finder, finder);
        assert_eq!(state.session.as_deref(), Some("$1"));
        assert_eq!(fake.calls(), calls);
    }
}

#[test]
fn find_incremental_wrap_include_no_match_and_child_preview() {
    let fake =
        FakeTmux::new(sample_tree(&["api", "logs", "work"])).with_capture("%0", "api preview");
    let mut state = state(&fake);
    key(&mut state, &fake, "G");
    find(&mut state, &fake, "a", false, Case::Smart);
    assert_eq!(state.session.as_deref(), Some("$0"));
    assert_eq!(state.window.as_deref(), Some("@0"));
    assert_eq!(state.pane.as_deref(), Some("%0"));
    assert_eq!(state.preview.pane_id.as_deref(), Some("%0"));
    assert_eq!(state.preview.content, "api preview");
    key(&mut state, &fake, "<C-u>");
    assert!(state.finder.is_none());
    type_text(&mut state, &fake, "lo");
    assert_eq!(state.session.as_deref(), Some("$1"));
    type_text(&mut state, &fake, "g");
    assert_eq!(state.session.as_deref(), Some("$1"));
    key(&mut state, &fake, "<C-u>");
    type_text(&mut state, &fake, "zzz");
    assert_eq!(state.session.as_deref(), Some("$1"));
    assert_eq!(state.finder.as_ref().unwrap().query, "zzz");
    key(&mut state, &fake, "<Esc>");
    find(&mut state, &fake, "a", true, Case::Smart);
    assert_eq!(state.session.as_deref(), Some("$0"));
    assert!(
        matches!(state.popup, Some(Popup::Input { ref title, .. }) if title == "Find previous:")
    );
}

#[test]
fn find_names_exclude_window_indexes_and_use_pane_commands() {
    let fake = fixture();
    let mut state = state(&fake);
    assert_eq!(state.names(Level::Session)[0], ("$0".into(), "work"));
    key(&mut state, &fake, "l");
    assert_eq!(state.names(Level::Window)[1], ("@10".into(), "editor"));
    assert_eq!(state.names(Level::Pane)[0], ("%10".into(), "nvim"));
    find(&mut state, &fake, "1", false, Case::Sensitive);
    assert!(
        state.names(Level::Window).iter().all(|(_, n)| state
            .finder
            .as_ref()
            .unwrap()
            .matched(n)
            .is_none())
    );
    assert_eq!(state.window.as_deref(), Some("@10"));
    key(&mut state, &fake, "<Esc>");
    key(&mut state, &fake, "l");
    find(&mut state, &fake, "sh", false, Case::Sensitive);
    assert_eq!(state.pane.as_deref(), Some("%11"));
}

#[test]
fn find_regex_smart_case_invalid_and_unchanged_queries() {
    let fake = FakeTmux::new(sample_tree(&["api", "Work", "work", "dotfiles", "logs"]));
    let mut state = state(&fake);
    find(&mut state, &fake, "work", false, Case::Smart);
    assert_eq!(state.session.as_deref(), Some("$1"));
    key(&mut state, &fake, "<C-u>");
    type_text(&mut state, &fake, "Work");
    assert_eq!(state.finder.as_ref().unwrap().matched("work"), None);
    key(&mut state, &fake, "<C-u>");
    type_text(&mut state, &fake, "^d.t");
    assert_eq!(state.session.as_deref(), Some("$3"));
    key(&mut state, &fake, "<C-u>");
    type_text(&mut state, &fake, "lo");
    let finder = state.finder.clone();
    type_text(&mut state, &fake, "(");
    assert_eq!(state.finder, finder);
    assert_eq!(state.session.as_deref(), Some("$4"));
    assert!(state.message.is_none());
    let calls = fake.calls();
    key(&mut state, &fake, "<Backspace>");
    assert_eq!(fake.calls(), calls); // Same valid query is not searched again.
    assert_eq!(state.finder, finder);
    key(&mut state, &fake, "<C-w>");
    assert!(state.finder.is_none());
    type_text(&mut state, &fake, "x");
    key(&mut state, &fake, "<Backspace>");
    assert!(state.finder.is_none());
    key(&mut state, &fake, "<Esc>");
    state.session = Some("$2".into());
    find(&mut state, &fake, "Work", false, Case::Smart);
    assert_eq!(state.session.as_deref(), Some("$1"));
    key(&mut state, &fake, "<Esc>");
    find(&mut state, &fake, "Work", false, Case::Insensitive);
    assert!(state.finder.as_ref().unwrap().matched("work").is_some());
}

#[test]
fn find_smart_uppercase_does_not_match_lowercase_only() {
    let fake = FakeTmux::new(sample_tree(&["api", "work"]));
    let mut state = state(&fake);
    find(&mut state, &fake, "Work", false, Case::Smart);
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn find_arrows_next_previous_wrap_no_finder_and_no_other_match() {
    let fake = FakeTmux::new(sample_tree(&["logs", "api", "blog"]));
    let mut state = state(&fake);
    let before = state.clone();
    key(&mut state, &fake, "n");
    assert_eq!(state, before);
    find(&mut state, &fake, "log", false, Case::Smart);
    key(&mut state, &fake, "<Enter>");
    key(&mut state, &fake, "n");
    assert_eq!(state.session.as_deref(), Some("$2"));
    key(&mut state, &fake, "n");
    assert_eq!(state.session.as_deref(), Some("$0"));
    key(&mut state, &fake, "N");
    assert_eq!(state.session.as_deref(), Some("$2"));
    key(&mut state, &fake, "N");
    assert_eq!(state.session.as_deref(), Some("$0"));
    find(&mut state, &fake, "^logs$", false, Case::Smart);
    key(&mut state, &fake, "<Enter>");
    let before = state.clone();
    key(&mut state, &fake, "n");
    assert_eq!(state, before);
}

#[test]
fn finder_follows_focus_after_h_and_l() {
    let mut tree = sample_tree(&["logs", "api", "blog"]);
    tree.sessions[0].windows[0].name = "logs".into();
    tree.sessions[0].windows.push(Window {
        id: "@10".into(),
        index: 1,
        name: "blog".into(),
        active: false,
        panes: vec![],
    });
    let fake = FakeTmux::new(tree);
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    find(&mut state, &fake, "log", false, Case::Smart);
    key(&mut state, &fake, "<Enter>");
    key(&mut state, &fake, "h");
    key(&mut state, &fake, "n");
    assert_eq!(state.session.as_deref(), Some("$2"));
    key(&mut state, &fake, "N");
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "n");
    assert_eq!(state.window.as_deref(), Some("@10"));
}

#[test]
fn escape_order_and_finder_survives_refresh() {
    let fake = FakeTmux::new(sample_tree(&["logs", "api", "blog"]));
    let mut state = state(&fake);
    find(&mut state, &fake, "log", false, Case::Smart);
    state.message = Some("error".into());
    state.which = Which::start(
        "g".parse().unwrap(),
        &Keymap::from_toml(&preset::keymap(), None).unwrap().picker,
    );
    state.apply(&Action::Escape, &fake);
    assert!(state.which.is_none());
    assert!(state.popup.is_some());
    state.apply(&Action::Escape, &fake);
    assert!(state.popup.is_none());
    assert!(state.finder.is_some());
    assert!(!state.refresh(&fake).unwrap());
    let finder = state.finder.clone();
    fake.new_session(Some("external")).unwrap();
    assert!(state.refresh(&fake).unwrap());
    assert_eq!(state.finder, finder);
    key(&mut state, &fake, "n");
    assert_eq!(state.session.as_deref(), Some("$2"));
    state.apply(&Action::Escape, &fake);
    assert!(state.finder.is_none());
    assert_eq!(state.message.as_deref(), Some("error"));
    let session = state.session.clone();
    key(&mut state, &fake, "n");
    assert_eq!(state.session, session);
    state.apply(&Action::Escape, &fake);
    assert!(state.message.is_none());
}

#[test]
fn recent_order_pins_current_and_opens_cursor_on_previous_session() {
    let mut tree = sample_tree(&["notes", "work", "older"]);
    for (session, time) in tree.sessions.iter_mut().zip([300, 100, 200]) {
        session.last_attached = time;
    }
    let current = Current {
        session_id: "$1".into(),
        window_id: "@1".into(),
        pane_id: "%1".into(),
    };
    let fake = FakeTmux::new(tree.clone()).with_current(current);
    let state = State::load(
        PickerOptions {
            order: SessionOrder::Recent,
            ..PickerOptions::default()
        },
        &fake,
        None,
    )
    .unwrap();
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["work", "notes", "older"]
    );
    assert_eq!(state.session.as_deref(), Some("$0"));
    assert_eq!(state.window.as_deref(), Some("@0"));
    assert_eq!(state.pane.as_deref(), Some("%0"));
    for session in state.sessions() {
        assert_eq!(
            session.windows,
            tree.sessions
                .iter()
                .find(|s| s.id == session.id)
                .unwrap()
                .windows
        );
    }
}

#[test]
fn recent_order_without_current_sorts_newest_then_ties_and_never_attached_by_name() {
    let mut tree = sample_tree(&["zeta", "beta", "newest", "alpha", "aardvark", "older"]);
    for (session, time) in tree.sessions.iter_mut().zip([0, 200, 300, 0, 200, 100]) {
        session.last_attached = time;
    }
    let state = State::new(
        PickerOptions {
            order: SessionOrder::Recent,
            ..PickerOptions::default()
        },
        tree,
        None,
        None,
    );
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["newest", "aardvark", "beta", "older", "alpha", "zeta"]
    );
    assert_eq!(state.session.as_deref(), Some("$2"));
}

#[test]
fn recent_order_refresh_reorders_without_losing_cursor() {
    let mut tree = sample_tree(&["kahi", "pmux", "scratch", "shuffle-be"]);
    for (session, time) in tree.sessions.iter_mut().zip([100, 300, 0, 200]) {
        session.last_attached = time;
    }
    let fake = FakeTmux::new(tree.clone()).with_current(Current {
        session_id: "$1".into(),
        window_id: "@1".into(),
        pane_id: "%1".into(),
    });
    let mut state = State::load(
        PickerOptions {
            order: SessionOrder::Recent,
            ..PickerOptions::default()
        },
        &fake,
        None,
    )
    .unwrap();
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["pmux", "shuffle-be", "kahi", "scratch"]
    );
    assert_eq!(state.session.as_deref(), Some("$3"));
    key(&mut state, &fake, "j");
    assert_eq!(state.session.as_deref(), Some("$0"));
    let cursors = (
        state.session.clone(),
        state.window.clone(),
        state.pane.clone(),
    );

    tree.sessions[2].last_attached = 400;
    let refreshed = FakeTmux::new(tree);
    assert!(state.refresh(&refreshed).unwrap());
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["pmux", "scratch", "shuffle-be", "kahi"]
    );
    assert_eq!((state.session, state.window, state.pane), cursors);
}

fn recent_state(tree: Tree, current: Option<Current>) -> State {
    recent_state_with_exclusions(tree, current, &[])
}

fn recent_state_with_exclusions(tree: Tree, current: Option<Current>, excluded: &[&str]) -> State {
    State::new(
        PickerOptions {
            order: SessionOrder::Recent,
            recency_exclude: excluded.iter().map(|name| (*name).into()).collect(),
            ..PickerOptions::default()
        },
        tree,
        current,
        None,
    )
}

fn session_names(state: &State) -> Vec<&str> {
    state.sessions().iter().map(|s| s.name.as_str()).collect()
}

#[test]
fn recent_order_excluded_session_follows_ranked_sessions() {
    let mut tree = sample_tree(&["session1", "session2", "session3", "session4"]);
    for (session, time) in tree.sessions.iter_mut().zip([400, 300, 200, 100]) {
        session.last_attached = time;
    }
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["session2"]);
    assert_eq!(
        session_names(&state),
        ["session1", "session3", "session4", "session2"]
    );
    assert_eq!(state.session.as_deref(), Some("$2"));
}

#[test]
fn recent_order_excluded_sessions_sort_by_name_despite_recency() {
    let mut tree = sample_tree(&["current", "zeta", "alpha", "work"]);
    for (session, time) in tree.sessions.iter_mut().zip([400, 300, 200, 100]) {
        session.last_attached = time;
    }
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["zeta", "alpha"]);
    assert_eq!(session_names(&state), ["current", "work", "alpha", "zeta"]);
    assert_eq!(state.session.as_deref(), Some("$3"));
}

#[test]
fn recent_order_excluded_session_follows_never_attached_session() {
    let mut tree = sample_tree(&["current", "scratch", "fresh"]);
    for (session, time) in tree.sessions.iter_mut().zip([100, 300, 0]) {
        session.last_attached = time;
    }
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["scratch"]);
    assert_eq!(session_names(&state), ["current", "fresh", "scratch"]);
    assert_eq!(state.session.as_deref(), Some("$2"));
}

#[test]
fn recent_order_excluded_current_stays_first() {
    let mut tree = sample_tree(&["_popup", "work"]);
    for (session, time) in tree.sessions.iter_mut().zip([300, 100]) {
        session.last_attached = time;
    }
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["_popup"]);
    assert_eq!(session_names(&state), ["_popup", "work"]);
    assert_eq!(state.session.as_deref(), Some("$1"));
}

#[test]
fn recent_order_only_excluded_sessions_opens_cursor_on_first_by_name() {
    let mut tree = sample_tree(&["work", "b-scratch", "a-popup"]);
    for (session, time) in tree.sessions.iter_mut().zip([100, 300, 200]) {
        session.last_attached = time;
    }
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["b-scratch", "a-popup"]);
    assert_eq!(session_names(&state), ["work", "a-popup", "b-scratch"]);
    assert_eq!(state.session.as_deref(), Some("$2"));
}

#[test]
fn recent_order_unknown_excluded_name_is_ignored() {
    let mut tree = sample_tree(&["current", "older", "newest", "fresh"]);
    for (session, time) in tree.sessions.iter_mut().zip([200, 100, 300, 0]) {
        session.last_attached = time;
    }
    let expected = recent_state(tree.clone(), Some(current_on(0)));
    let state = recent_state_with_exclusions(tree, Some(current_on(0)), &["missing"]);
    assert_eq!(session_names(&state), session_names(&expected));
    assert_eq!(state.session, expected.session);
}

#[test]
fn recent_order_refresh_excluded_session_attachment_preserves_position_and_cursor() {
    let mut tree = sample_tree(&["current", "work", "scratch", "a-popup"]);
    for (session, time) in tree.sessions.iter_mut().zip([300, 200, 0, 100]) {
        session.last_attached = time;
    }
    let mut state =
        recent_state_with_exclusions(tree.clone(), Some(current_on(0)), &["scratch", "a-popup"]);
    assert_eq!(
        session_names(&state),
        ["current", "work", "a-popup", "scratch"]
    );
    assert_eq!(state.session.as_deref(), Some("$1"));
    let cursors = (
        state.session.clone(),
        state.window.clone(),
        state.pane.clone(),
    );

    tree.sessions[2].last_attached = 400;
    let refreshed = FakeTmux::new(tree);
    assert!(state.refresh(&refreshed).unwrap());
    assert_eq!(
        session_names(&state),
        ["current", "work", "a-popup", "scratch"]
    );
    assert_eq!((state.session, state.window, state.pane), cursors);
}

fn current_on(index: usize) -> Current {
    Current {
        session_id: format!("${index}"),
        window_id: format!("@{index}"),
        pane_id: format!("%{index}"),
    }
}

#[test]
fn recent_order_single_session_opens_cursor_on_current() {
    let state = recent_state(sample_tree(&["work"]), Some(current_on(0)));
    assert_eq!(state.session.as_deref(), Some("$0"));
    assert_eq!(state.window.as_deref(), Some("@0"));
    assert_eq!(state.pane.as_deref(), Some("%0"));
}

#[test]
fn recent_order_without_current_opens_cursor_on_first_row() {
    let mut tree = sample_tree(&["work", "notes"]);
    tree.sessions[1].last_attached = 300;
    tree.sessions[0].last_attached = 100;
    let state = recent_state(tree, None);
    assert_eq!(state.sessions()[0].name, "notes");
    assert_eq!(state.session.as_deref(), Some("$1"));
}

#[test]
fn recent_order_with_unknown_current_opens_cursor_on_first_row() {
    let mut tree = sample_tree(&["work", "notes"]);
    tree.sessions[1].last_attached = 300;
    tree.sessions[0].last_attached = 100;
    let state = recent_state(tree, Some(current_on(9)));
    assert_eq!(state.sessions()[0].name, "notes");
    assert_eq!(state.session.as_deref(), Some("$1"));
}

#[test]
fn name_order_opens_cursor_on_first_row_even_when_current() {
    let fake = FakeTmux::new(sample_tree(&["alpha", "beta"])).with_current(current_on(0));
    let state = state(&fake);
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn name_order_preserves_snapshot_despite_recency_and_current() {
    let mut tree = sample_tree(&["alpha", "beta", "zeta"]);
    for (session, time) in tree.sessions.iter_mut().zip([0, 300, 100]) {
        session.last_attached = time;
    }
    let fake = FakeTmux::new(tree.clone()).with_current(Current {
        session_id: "$2".into(),
        window_id: "@2".into(),
        pane_id: "%2".into(),
    });
    let mut state = state(&fake);
    assert_eq!(state.options.order, SessionOrder::Name);
    assert_eq!(state.tree, tree);
    assert_eq!(state.session.as_deref(), Some("$0"));
    state.refresh(&fake).unwrap();
    assert_eq!(state.tree, tree);
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn initial_view_and_current_markers() {
    let fake = fixture();
    let state = state(&fake);
    assert_eq!(state.focus, Level::Session);
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["work", "dotfiles", "extra"]
    );
    assert_eq!(state.windows().len(), 2);
    assert_eq!(state.window.as_deref(), Some("@10"));
    assert_eq!(state.panes().len(), 2);
    assert_eq!(state.pane.as_deref(), Some("%10"));
    for id in ["$0", "@10", "%10"] {
        assert!(state.is_current(id));
    }
    assert!(!state.is_current("$1"));
}

#[test]
fn hover_changes_child_columns_and_active_cursors() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "j");
    assert_eq!(state.session.as_deref(), Some("$1"));
    assert_eq!(state.window.as_deref(), Some("@1"));
    assert_eq!(state.pane.as_deref(), Some("%1"));
    assert_eq!(state.windows()[1].name, "logs");
    assert_eq!(state.panes().len(), 1);
    key(&mut state, &fake, "k");
    assert_eq!(state.window.as_deref(), Some("@10"));
}

#[test]
fn focus_right_selects_active_window_and_stops_at_boundaries() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "h");
    assert_eq!(state.focus, Level::Session);
    key(&mut state, &fake, "l");
    assert_eq!(state.focus, Level::Window);
    assert_eq!(state.window.as_deref(), Some("@10"));
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "l");
    assert_eq!(state.focus, Level::Pane);
}

#[test]
fn navigation_next_previous_top_bottom_pages_and_offsets() {
    let fake = fixture();
    let mut state = state(&fake);
    state.list_height = 1;
    key(&mut state, &fake, "<PageDown>");
    assert_eq!(state.session.as_deref(), Some("$1"));
    key(&mut state, &fake, "G");
    assert_eq!(state.session.as_deref(), Some("$2"));
    key(&mut state, &fake, "j");
    assert_eq!(state.session.as_deref(), Some("$2"));
    key(&mut state, &fake, "<PageUp>");
    assert_eq!(state.session.as_deref(), Some("$1"));
    state.apply(&Action::Arrow(Step::Offset(isize::MIN)), &fake);
    assert_eq!(state.session.as_deref(), Some("$0"));
    state.apply(&Action::Arrow(Step::Offset(isize::MAX)), &fake);
    assert_eq!(state.session.as_deref(), Some("$2"));
    state.apply(&Action::Arrow(Step::Top), &fake);
    key(&mut state, &fake, "k");
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn preview_follows_session_active_descendants() {
    let fake = fixture();
    let mut state = state(&fake);
    assert_eq!(state.preview_target().unwrap().at(Level::Pane), "work:1.0");
    assert!(state.preview.content.starts_with("line 0"));
    key(&mut state, &fake, "j");
    assert_eq!(
        state.preview_target().unwrap().at(Level::Pane),
        "dotfiles:0.0"
    );
    assert_eq!(state.preview.pane_id.as_deref(), Some("%1"));
}

#[test]
fn preview_follows_window_and_pane_hover() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "k");
    assert_eq!(state.preview_target().unwrap().at(Level::Pane), "work:0.0");
    key(&mut state, &fake, "j");
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "j");
    assert_eq!(state.preview_target().unwrap().at(Level::Pane), "work:1.2");
    assert_eq!(state.preview.content, "\u{1b}[31msecond pane\u{1b}[0m\n");
}

#[test]
fn scroll_preview_half_page_clamps_and_resets_on_target_change() {
    let fake = fixture();
    let mut state = state(&fake);
    state.preview_height = 10;
    key(&mut state, &fake, "<C-d>");
    assert_eq!(state.preview.scroll, 5);
    key(&mut state, &fake, "<C-d>");
    key(&mut state, &fake, "<C-u>");
    assert_eq!(state.preview.scroll, 5);
    state.apply(&Action::Seek(i16::MAX), &fake);
    assert_eq!(state.preview.scroll, 40);
    state.apply(&Action::Seek(i16::MIN), &fake);
    assert_eq!(state.preview.scroll, 0);
    key(&mut state, &fake, "<C-d>");
    key(&mut state, &fake, "j");
    assert_eq!(state.preview.scroll, 0);
}

#[test]
fn regular_pane_suppression_skips_capture_but_popup_captures_current() {
    let fake = fixture();
    let suppressed = crate::tmux::suppressed_pane(Some("socket"), Some("%10"));
    let state = State::load(PickerOptions::default(), &fake, suppressed).unwrap();
    assert!(state.preview_suppressed());
    assert!(state.preview.content.is_empty());
    assert!(!fake.calls().iter().any(|call| call == "capture-pane %10"));
    let popup = State::load(
        PickerOptions::default(),
        &fake,
        crate::tmux::suppressed_pane(Some("socket"), None),
    )
    .unwrap();
    assert!(!popup.preview_suppressed());
    assert!(!popup.preview.content.is_empty());
}

#[test]
fn external_session_created_appears_and_cursor_id_survives_refresh() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "j");
    let id = state.session.clone();
    let extra = fake.new_session(Some("external")).unwrap();
    assert!(state.refresh(&fake).unwrap());
    assert_eq!(state.session, id);
    assert!(state.sessions().iter().any(|s| s.id == extra));
    assert!(!state.refresh(&fake).unwrap());
}

#[test]
fn removed_session_clamps_nearest_index_and_resets_children() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "j");
    fake.kill_session("$1").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.session.as_deref(), Some("$2"));
    assert_eq!(state.window.as_deref(), Some("@2"));
    fake.kill_session("$2").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.session.as_deref(), Some("$0"));
    assert_eq!(state.window.as_deref(), Some("@10"));
}

#[test]
fn removed_window_and_pane_clamp_without_losing_parent() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "j");
    fake.kill_pane("%11").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.pane.as_deref(), Some("%10"));
    fake.kill_window("@10").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.window.as_deref(), Some("@0"));
    assert_eq!(state.pane.as_deref(), Some("%0"));
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn session_output_flag_from_pane_column_returns_work() {
    let fake = fixture();
    let mut state = state(&fake);
    state.options.level = Some(Level::Session);
    for k in ["l", "l", "j"] {
        key(&mut state, &fake, k);
    }
    assert_eq!(
        key(&mut state, &fake, "<Enter>"),
        Some(Exit::Pick("work".into()))
    );
}

#[test]
fn default_output_level_follows_focus_and_explicit_levels_override_it() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    assert_eq!(
        key(&mut state, &fake, "<Enter>"),
        Some(Exit::Pick("work:1".into()))
    );
    state.options.level = Some(Level::Pane);
    assert_eq!(
        key(&mut state, &fake, "<Enter>"),
        Some(Exit::Pick("work:1.0".into()))
    );
    state.options.level = Some(Level::Window);
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "j");
    assert_eq!(
        key(&mut state, &fake, "<Enter>"),
        Some(Exit::Pick("work:1".into()))
    );
}

#[test]
fn quit_cancels_and_escape_only_dismisses() {
    let fake = fixture();
    let mut state = state(&fake);
    state.message = Some("error".into());
    assert_eq!(key(&mut state, &fake, "<Esc>"), None);
    assert!(state.message.is_none());
    assert_eq!(key(&mut state, &fake, "<Esc>"), None);
    assert_eq!(key(&mut state, &fake, "q"), Some(Exit::Cancel));
}

#[test]
fn create_session_moves_cursor_and_preserves_focus() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "a");
    type_text(&mut state, &fake, "notes");
    assert_eq!(key(&mut state, &fake, "<Enter>"), None);
    assert_eq!(state.hovered_session().unwrap().name, "notes");
    assert_eq!(state.focus, Level::Session);
    assert!(state.popup.is_none());
}

#[test]
fn create_window_moves_cursor_within_hovered_session() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    key(&mut state, &fake, "a");
    type_text(&mut state, &fake, "logs");
    key(&mut state, &fake, "<Enter>");
    assert_eq!(state.hovered_window().unwrap().name, "logs");
    assert_eq!(state.hovered_session().unwrap().name, "work");
    assert_eq!(state.focus, Level::Window);
}

#[test]
fn empty_create_names_use_tmux_defaults_for_sessions_and_windows() {
    let fake = fixture();
    let mut state = state(&fake);
    for k in ["a", "<Enter>", "l", "a", "<Enter>"] {
        key(&mut state, &fake, k);
    }
    assert!(fake.calls().iter().any(|call| call == "new-session "));
    assert!(
        fake.calls()
            .iter()
            .any(|call| call.starts_with("new-window ") && call.ends_with(' '))
    );
}

#[test]
fn cancel_input_creates_nothing_and_input_layer_inserts_picker_keys() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "a");
    type_text(&mut state, &fake, "qj?");
    assert!(matches!(&state.popup, Some(Popup::Input { value, .. }) if value == "qj?"));
    key(&mut state, &fake, "<Esc>");
    assert!(state.popup.is_none());
    assert!(!fake.calls().iter().any(|call| call.starts_with("new-")));
}

#[test]
fn rename_session_prefills_name_and_keeps_stable_cursor() {
    let fake = fixture();
    let mut state = state(&fake);
    let id = state.session.clone();
    key(&mut state, &fake, "r");
    assert!(matches!(&state.popup, Some(Popup::Input { value, .. }) if value == "work"));
    key(&mut state, &fake, "<C-u>");
    type_text(&mut state, &fake, "job");
    key(&mut state, &fake, "<Enter>");
    assert_eq!(state.session, id);
    assert_eq!(state.hovered_session().unwrap().name, "job");
}

#[test]
fn rename_window_keeps_id_and_pane_create_rename_are_noops() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "l");
    let id = state.window.clone();
    for k in ["r", "<C-u>"] {
        key(&mut state, &fake, k);
    }
    type_text(&mut state, &fake, "new editor");
    key(&mut state, &fake, "<Enter>");
    assert_eq!(state.window, id);
    assert_eq!(state.hovered_window().unwrap().name, "new editor");
    key(&mut state, &fake, "l");
    for k in ["a", "r"] {
        key(&mut state, &fake, k);
        assert!(state.popup.is_none());
    }
}

#[test]
fn delete_confirmed_removes_dotfiles_window_two() {
    let fake = fixture();
    let mut state = state(&fake);
    for k in ["j", "l", "j", "d", "d"] {
        key(&mut state, &fake, k);
    }
    assert!(matches!(&state.popup, Some(Popup::Confirm { id, .. }) if id == "@20"));
    key(&mut state, &fake, "y");
    assert!(!state.windows().iter().any(|w| w.id == "@20"));
    assert!(fake.calls().contains(&"kill-window @20".into()));
}

#[test]
fn delete_declined_does_not_mutate() {
    let fake = fixture();
    let mut state = state(&fake);
    for k in ["j", "d", "d", "n"] {
        key(&mut state, &fake, k);
    }
    assert!(state.popup.is_none());
    assert_eq!(state.sessions().len(), 3);
    assert!(!fake.calls().iter().any(|call| call.starts_with("kill-")));
}

#[test]
fn deleting_current_session_window_or_pane_is_refused_without_prompt() {
    let fake = fixture();
    let mut state = state(&fake);
    for level in [Level::Session, Level::Window, Level::Pane] {
        assert_eq!(state.focus, level);
        key(&mut state, &fake, "d");
        key(&mut state, &fake, "d");
        assert!(state.popup.is_none());
        assert!(state.message.as_ref().unwrap().contains("current"));
        key(&mut state, &fake, "l");
    }
    assert!(!fake.calls().iter().any(|call| call.starts_with("kill-")));
}

#[test]
fn delete_session_and_pane_confirmations_dispatch_correct_operations() {
    let fake = fixture();
    let mut state = state(&fake);
    for k in ["j", "d", "d", "y"] {
        key(&mut state, &fake, k);
    }
    assert!(fake.calls().contains(&"kill-session $1".into()));
    for k in ["k", "l", "l", "j", "d", "d", "y"] {
        key(&mut state, &fake, k);
    }
    assert!(fake.calls().contains(&"kill-pane %11".into()));
}

#[test]
fn duplicate_session_error_is_nonfatal_and_keeps_existing_tree() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "a");
    type_text(&mut state, &fake, "work");
    fake.fail_next("duplicate session: work");
    assert_eq!(key(&mut state, &fake, "<Enter>"), None);
    assert_eq!(state.message.as_deref(), Some("duplicate session: work"));
    assert!(state.popup.is_none());
    assert_eq!(state.sessions().len(), 3);
}

#[test]
fn rename_and_delete_errors_are_nonfatal() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "r");
    fake.fail_next("rename failed");
    assert_eq!(key(&mut state, &fake, "<Enter>"), None);
    assert_eq!(state.message.as_deref(), Some("rename failed"));
    for k in ["j", "d", "d"] {
        key(&mut state, &fake, k);
    }
    fake.fail_next("kill failed");
    assert_eq!(key(&mut state, &fake, "y"), None);
    assert_eq!(state.message.as_deref(), Some("kill failed"));
    assert_eq!(state.sessions().len(), 3);
}

#[test]
fn which_gg_shows_candidates_then_moves_top() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "G");
    key(&mut state, &fake, "g");
    let which = state.which.as_ref().unwrap();
    assert_eq!(which.times, 1);
    assert_eq!(which.candidates.len(), 1);
    assert_eq!(which.candidates[0].on, vec!["g".parse().unwrap(); 2]);
    key(&mut state, &fake, "g");
    assert!(state.which.is_none());
    assert_eq!(state.session.as_deref(), Some("$0"));
}

#[test]
fn which_invalid_second_key_is_swallowed_not_dispatched() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "g");
    assert_eq!(key(&mut state, &fake, "q"), None);
    assert!(state.which.is_none());
    assert!(state.popup.is_none());
    key(&mut state, &fake, "d");
    key(&mut state, &fake, "<Esc>");
    assert!(state.which.is_none());
}

#[test]
fn help_lists_default_picker_bindings_and_uses_help_layer() {
    let fake = fixture();
    let mut state = state(&fake);
    for opener in ["?", "<F1>"] {
        key(&mut state, &fake, opener);
        let Some(Popup::Help { cursor, chords }) = &state.popup else {
            panic!("expected help");
        };
        assert_eq!(*cursor, 0);
        assert!(chords.len() > 10);
        assert!(chords.iter().all(|c| c.desc.is_some()));
        key(&mut state, &fake, "j");
        assert!(matches!(&state.popup, Some(Popup::Help { cursor: 1, .. })));
        assert_eq!(state.session.as_deref(), Some("$0"));
        assert_eq!(key(&mut state, &fake, "q"), None);
        assert!(state.popup.is_none());
    }
}

#[test]
fn unicode_input_backspace_kill_word_and_kill_bol() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "a");
    type_text(&mut state, &fake, "hello 世界  ");
    key(&mut state, &fake, "<C-w>");
    assert!(matches!(&state.popup, Some(Popup::Input { value, .. }) if value == "hello "));
    type_text(&mut state, &fake, "é");
    key(&mut state, &fake, "<Backspace>");
    assert!(matches!(&state.popup, Some(Popup::Input { value, .. }) if value == "hello "));
    key(&mut state, &fake, "<C-u>");
    assert!(matches!(&state.popup, Some(Popup::Input { value, .. }) if value.is_empty()));
}

#[test]
fn empty_tree_navigation_and_pick_are_safe_but_create_is_available() {
    let fake = FakeTmux::new(Tree::default());
    let mut state = state(&fake);
    for k in ["j", "G", "l", "l", "<PageUp>", "<Enter>", "d", "d"] {
        assert_eq!(key(&mut state, &fake, k), None);
    }
    assert!(state.preview_target().is_none());
    assert!(state.popup.is_none());
    key(&mut state, &fake, "h");
    key(&mut state, &fake, "h");
    key(&mut state, &fake, "a");
    key(&mut state, &fake, "<Enter>");
    assert_eq!(state.sessions().len(), 1);
}

#[test]
fn startup_and_refresh_errors_propagate_without_interactive_tmux() {
    let fake = fixture();
    fake.fail_next("no server running");
    assert!(
        State::load(PickerOptions::default(), &fake, None)
            .unwrap_err()
            .to_string()
            .contains("no server")
    );
    let mut state = state(&fake);
    fake.fail_next("server disappeared");
    assert!(state.refresh(&fake).is_err());
}

#[test]
fn refresh_updates_preview_without_resetting_scroll_on_same_pane() {
    let fake = fixture();
    let mut state = state(&fake);
    key(&mut state, &fake, "<C-d>");
    let fake = fake.with_capture("%10", &"updated output\n".repeat(50));
    assert!(state.refresh(&fake).unwrap());
    assert!(state.preview.content.starts_with("updated output"));
    assert_eq!(state.preview.scroll, 5);
    assert!(!state.refresh(&fake).unwrap());
}

#[test]
fn cursor_ids_survive_removal_of_earlier_rows() {
    let fake = fixture();
    let mut state = state(&fake);
    for k in ["l", "l", "j"] {
        key(&mut state, &fake, k);
    }
    fake.kill_window("@0").unwrap();
    fake.kill_pane("%10").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.window.as_deref(), Some("@10"));
    assert_eq!(state.pane.as_deref(), Some("%11"));
}

#[test]
fn help_opened_by_custom_chord_lists_merged_bindings() {
    let fake = fixture();
    let mut state = state(&fake);
    let map = Keymap::from_toml(
        &preset::keymap(),
        Some(
            "[picker]\nprepend_keymap = [{ on = ['z', 'z'], run = 'help', desc = 'Custom help' }]",
        ),
    )
    .unwrap();
    state.handle_key("z".parse().unwrap(), &map, &fake);
    state.handle_key("z".parse().unwrap(), &map, &fake);
    let Some(Popup::Help { chords, .. }) = &state.popup else {
        panic!("expected help");
    };
    assert_eq!(chords, &map.picker);
}
