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
