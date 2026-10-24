use super::*;
use crate::{
    config::{Config, keymap::Keymap, preset},
    tmux::{FakeTmux, sample_tree},
};

fn fixture() -> (State, FakeTmux) {
    let mut tree = sample_tree(&["work", "notes"]);
    tree.sessions[0].windows[0].panes.push(Pane {
        id: "%2".into(),
        index: 1,
        command: "tail".into(),
        ..Pane::default()
    });
    let fake = FakeTmux::new(tree).with_current(Current {
        session_id: "$0".into(),
        window_id: "@0".into(),
        pane_id: "%0".into(),
    });
    let state = State::load(PickerOptions::default(), &fake, None).unwrap();
    (state, fake)
}

fn press(state: &mut State, fake: &FakeTmux, key: &str) -> Option<Exit> {
    let keymap = Keymap::from_toml(&preset::keymap(), None).unwrap();
    state.handle_key(key.parse().unwrap(), &keymap, fake)
}

fn cut(state: &mut State, fake: &FakeTmux) {
    state.focus = Level::Pane;
    press(state, fake, "x");
    assert_eq!(state.cut.as_deref(), Some("%0"));
}

fn destination(state: &mut State, level: Level) {
    state.select(Level::Session, Some("$1".into()));
    state.focus = level;
}

fn submit(state: &mut State, fake: &FakeTmux, text: &str) -> Option<Exit> {
    let Some(Popup::Input { value, .. }) = &mut state.popup else {
        panic!("missing input");
    };
    *value = text.into();
    press(state, fake, "<Enter>")
}

#[test]
fn move_pane_starts_on_source_session_with_sessions_focused() {
    let mut tree = sample_tree(&["alpha", "notes", "work"]);
    tree.sessions[1].windows[0].index = 2;
    tree.sessions[1].windows[0].panes[0].index = 1;
    let fake = FakeTmux::new(tree);
    let options = crate::commands::move_pane::options(Some("notes:2.1"), &fake, false).unwrap();
    assert_eq!(options.order, SessionOrder::Name);
    let mut state = State::load(options, &fake, None).unwrap();
    assert_eq!(
        state
            .sessions()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "notes", "work"]
    );
    assert_eq!(state.hovered_session().unwrap().name, "notes");
    assert_eq!(state.focus, Level::Session);
    assert_eq!(state.cut.as_deref(), Some("%1"));
    // This is only the startup selection; refresh must not pull the cursor back.
    state.select(Level::Session, Some("$2".into()));
    state.refresh(&fake).unwrap();
    assert_eq!(state.hovered_session().unwrap().name, "work");
}

#[test]
fn default_options_start_at_top_without_following_cut_pane() {
    for cut in [None, Some("%1".into())] {
        let state = State::new(
            PickerOptions {
                cut,
                ..PickerOptions::default()
            },
            sample_tree(&["alpha", "notes", "work"]),
            None,
            None,
        );
        assert_eq!(state.hovered_session().unwrap().name, "alpha");
        assert_eq!(state.focus, Level::Session);
    }
}

#[test]
fn missing_initial_pane_falls_back_to_top() {
    let state = State::new(
        PickerOptions {
            initial_pane: Some("%missing".into()),
            ..PickerOptions::default()
        },
        sample_tree(&["alpha", "notes", "work"]),
        None,
        None,
    );
    assert_eq!(state.hovered_session().unwrap().name, "alpha");
    assert_eq!(state.focus, Level::Session);
    let state = State::new(
        PickerOptions {
            initial_pane: Some("%missing".into()),
            ..PickerOptions::default()
        },
        Tree::default(),
        None,
        None,
    );
    assert_eq!(state.session, None);
    assert_eq!(state.focus, Level::Session);
}

#[test]
fn cut_toggle_replace_and_refusals_do_not_mutate_tmux() {
    let (mut state, fake) = fixture();
    let before = fake.tree().unwrap();
    for level in [Level::Session, Level::Window] {
        state.focus = level;
        press(&mut state, &fake, "x");
        assert!(state.message.as_deref().unwrap().contains("Only panes"));
        assert_eq!(state.cut, None);
    }
    cut(&mut state, &fake);
    state.select(Level::Pane, Some("%2".into()));
    press(&mut state, &fake, "x");
    assert_eq!(state.cut.as_deref(), Some("%2"));
    press(&mut state, &fake, "x");
    assert_eq!(state.cut, None);
    state.cut = Some("%0".into());
    state.options.cut_locked = true;
    state.select(Level::Pane, Some("%0".into()));
    press(&mut state, &fake, "x");
    assert!(state.message.as_deref().unwrap().contains("locked"));
    assert_eq!(state.cut.as_deref(), Some("%0"));
    state.select(Level::Pane, Some("%2".into()));
    press(&mut state, &fake, "x");
    assert_eq!(state.cut.as_deref(), Some("%0"));
    assert_eq!(fake.tree().unwrap(), before);
}

fn floating_source() -> (State, FakeTmux) {
    let (_, fake) = fixture();
    let mut tree = fake.tree().unwrap();
    tree.sessions[0].windows[0].panes[1].floating = true;
    let fake = FakeTmux::new(tree);
    let mut state = State::load(PickerOptions::default(), &fake, None).unwrap();
    state.focus = Level::Pane;
    state.select(Level::Pane, Some("%2".into()));
    (state, fake)
}

#[test]
fn cut_floating_pane_can_be_toggled_without_mutating_tmux() {
    let (mut state, fake) = floating_source();
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "x");
    assert_eq!(state.cut.as_deref(), Some("%2"));
    assert_eq!(state.message, None);
    assert!(state.hovered_pane().unwrap().floating);
    assert_eq!(fake.tree().unwrap(), before);
    press(&mut state, &fake, "x");
    assert_eq!(state.cut, None);
    assert_eq!(fake.tree().unwrap(), before);
}

fn paste_floating_source(level: Level) {
    let (mut state, fake) = floating_source();
    let remaining = vec![state.tree.sessions[0].windows[0].panes[0].clone()];
    press(&mut state, &fake, "x");
    assert_eq!(state.cut.as_deref(), Some("%2"));
    destination(&mut state, level);
    assert_eq!(press(&mut state, &fake, "p"), None);
    if level == Level::Session {
        assert_eq!(submit(&mut state, &fake, "logs"), None);
    }
    if level != Level::Pane {
        assert_eq!(submit(&mut state, &fake, "scratch"), None);
    }
    assert_eq!(state.cut, None);
    assert_eq!(state.message, None);
    let target = state.hovered_window().unwrap();
    let moved = target.panes.iter().find(|p| p.id == "%2").unwrap();
    assert_eq!(moved.floating, level == Level::Pane);
    if level == Level::Session {
        assert_eq!(state.hovered_session().unwrap().name, "logs");
        assert_eq!(state.hovered_session().unwrap().windows.len(), 1);
    } else {
        assert_eq!(state.hovered_session().unwrap().name, "notes");
    }
    if level == Level::Pane {
        assert_eq!(state.pane.as_deref(), Some("%2"));
        assert_eq!(target.id, "@1");
        assert_eq!(target.panes.len(), 2);
        assert!(
            fake.calls()
                .iter()
                .any(|c| c == "float-pane-in-window %2 @1 50 50 centre")
        );
    } else {
        assert_eq!(target.name, "scratch");
        assert_eq!(target.panes.len(), 1);
    }
    let tree = fake.tree().unwrap();
    let source = tree
        .sessions
        .iter()
        .find(|s| s.id == "$0")
        .unwrap()
        .windows
        .iter()
        .find(|w| w.id == "@0")
        .unwrap();
    assert_eq!(source.panes, remaining);
}

#[test]
fn floating_source_to_new_session_becomes_tiled() {
    paste_floating_source(Level::Session);
}

#[test]
fn floating_source_to_new_window_becomes_tiled() {
    paste_floating_source(Level::Window);
}

#[test]
fn floating_source_to_another_window_floats_again() {
    paste_floating_source(Level::Pane);
}

#[test]
fn cut_refused_when_no_pane_is_hovered() {
    let fake = FakeTmux::new(Tree::default());
    let mut state = State::load(PickerOptions::default(), &fake, None).unwrap();
    state.focus = Level::Pane;
    press(&mut state, &fake, "x");
    assert!(state.message.as_deref().unwrap().contains("Only panes"));
}

#[test]
fn cut_rebinding_and_default_paste_with_empty_buffer() {
    let (mut state, fake) = fixture();
    state.focus = Level::Pane;
    let keymap = Keymap::from_toml(
        &preset::keymap(),
        Some(
            r#"[picker]
prepend_keymap = [{ on = "<C-x>", run = "cut" }]"#,
        ),
    )
    .unwrap();
    state.handle_key("<C-x>".parse().unwrap(), &keymap, &fake);
    assert_eq!(state.cut.as_deref(), Some("%0"));
    state.cut = None;
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "p");
    assert!(state.message.as_deref().unwrap().contains("Nothing is cut"));
    assert_eq!(fake.tree().unwrap(), before);
}

#[test]
fn paste_session_prompts_then_moves_and_selects_without_exiting() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    state.focus = Level::Session;
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "p");
    assert!(matches!(
        state.popup,
        Some(Popup::Input {
            purpose: InputPurpose::PasteSession { .. },
            ..
        })
    ));
    assert_eq!(submit(&mut state, &fake, "logs"), None);
    assert!(matches!(
        state.popup,
        Some(Popup::Input {
            purpose: InputPurpose::PasteSessionWindow { .. },
            ..
        })
    ));
    assert_eq!(fake.tree().unwrap(), before);
    assert_eq!(submit(&mut state, &fake, "tail"), None);
    assert_eq!(state.cut, None);
    let session = state.hovered_session().unwrap();
    assert_eq!(session.name, "logs");
    assert_eq!(session.windows.len(), 1);
    assert_eq!(session.windows[0].name, "tail");
    assert_eq!(session.windows[0].panes[0].id, "%0");
    assert_eq!(state.focus, Level::Session);
    assert_eq!(fake.current().unwrap(), state.current);
}

#[test]
fn cancelling_either_session_prompt_changes_nothing() {
    for second_prompt in [false, true] {
        let (mut state, fake) = fixture();
        cut(&mut state, &fake);
        state.focus = Level::Session;
        let before = fake.tree().unwrap();
        press(&mut state, &fake, "p");
        if second_prompt {
            submit(&mut state, &fake, "logs");
        }
        press(&mut state, &fake, "<Esc>");
        assert!(state.popup.is_none());
        assert_eq!(state.cut.as_deref(), Some("%0"));
        assert_eq!(fake.tree().unwrap(), before);
    }
}

#[test]
fn duplicate_session_shows_error_and_keeps_cut() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    state.focus = Level::Session;
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "p");
    submit(&mut state, &fake, "notes");
    assert_eq!(submit(&mut state, &fake, "tail"), None);
    assert!(
        state
            .message
            .as_deref()
            .unwrap()
            .contains("duplicate session")
    );
    assert_eq!(state.cut.as_deref(), Some("%0"));
    assert_eq!(fake.tree().unwrap(), before);
}

#[test]
fn empty_names_use_tmux_defaults_for_both_prompts() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    state.focus = Level::Session;
    press(&mut state, &fake, "p");
    submit(&mut state, &fake, "");
    submit(&mut state, &fake, "");
    assert!(
        fake.calls()
            .iter()
            .any(|c| c == "move-pane-to-new-session %0  ")
    );
    assert_eq!(state.cut, None);
}

#[test]
fn paste_new_window_other_or_own_session_and_cancel() {
    for own in [false, true] {
        let (mut state, fake) = fixture();
        cut(&mut state, &fake);
        if !own {
            destination(&mut state, Level::Window);
        }
        state.focus = Level::Window;
        let before = fake.tree().unwrap();
        press(&mut state, &fake, "p");
        press(&mut state, &fake, "<Esc>");
        assert_eq!(fake.tree().unwrap(), before);
        assert_eq!(state.cut.as_deref(), Some("%0"));
        press(&mut state, &fake, "p");
        assert_eq!(submit(&mut state, &fake, "scratch"), None);
        assert_eq!(
            state.hovered_session().unwrap().name,
            if own { "work" } else { "notes" }
        );
        assert_eq!(state.hovered_window().unwrap().name, "scratch");
        assert_eq!(state.hovered_window().unwrap().panes.len(), 1);
        assert_eq!(state.hovered_window().unwrap().panes[0].id, "%0");
        assert_eq!(state.cut, None);
    }
}

#[test]
fn floating_paste_uses_configuration_and_selects_moved_pane() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    destination(&mut state, Level::Pane);
    state.floating_pane = Config::from_toml(
        &preset::config(),
        Some("[floating-pane]\nwidth = 80\nheight = 30\nposition = 'bottom-centre'"),
    )
    .unwrap()
    .floating_pane;
    assert_eq!(press(&mut state, &fake, "p"), None);
    assert_eq!(state.cut, None);
    assert!(state.popup.is_none());
    assert_eq!(state.session.as_deref(), Some("$1"));
    assert_eq!(state.window.as_deref(), Some("@1"));
    assert_eq!(state.pane.as_deref(), Some("%0"));
    assert!(state.hovered_pane().unwrap().floating);
    assert!(
        fake.calls()
            .iter()
            .any(|c| c == "float-pane-in-window %0 @1 80 30 bottom-centre")
    );
}

#[test]
fn same_window_is_silent_noop_even_when_zoomed() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "p");
    assert_eq!(state.cut.as_deref(), Some("%0"));
    assert_eq!(state.message, None);
    assert_eq!(fake.tree().unwrap(), before);

    let mut tree = before;
    tree.sessions[0].windows[0].zoomed = true;
    let fake = FakeTmux::new(tree);
    state.refresh(&fake).unwrap();
    press(&mut state, &fake, "p");
    assert_eq!(state.message, None);
    assert_eq!(state.cut.as_deref(), Some("%0"));
}

#[test]
fn zoomed_destination_is_refused_without_mutation() {
    let (mut state, _) = fixture();
    let mut tree = state.tree.clone();
    tree.sessions[1].windows[0].zoomed = true;
    let fake = FakeTmux::new(tree);
    cut(&mut state, &fake);
    destination(&mut state, Level::Pane);
    let before = fake.tree().unwrap();
    press(&mut state, &fake, "p");
    assert!(state.message.as_deref().unwrap().contains("zoomed"));
    assert_eq!(state.cut.as_deref(), Some("%0"));
    assert_eq!(fake.tree().unwrap(), before);
}

#[test]
fn tmux_failures_keep_buffer_and_picker_open() {
    for level in [Level::Session, Level::Window, Level::Pane] {
        let (mut state, fake) = fixture();
        cut(&mut state, &fake);
        destination(&mut state, level);
        if level == Level::Pane {
            fake.fail_next("floating unsupported");
            assert_eq!(press(&mut state, &fake, "p"), None);
        } else {
            press(&mut state, &fake, "p");
            if level == Level::Session {
                submit(&mut state, &fake, "logs");
            }
            fake.fail_next("move failed");
            assert_eq!(submit(&mut state, &fake, "scratch"), None);
        }
        assert!(
            state
                .message
                .as_deref()
                .unwrap()
                .contains(if level == Level::Pane {
                    "floating unsupported"
                } else {
                    "move failed"
                })
        );
        assert_eq!(state.cut.as_deref(), Some("%0"));
    }
}

#[test]
fn cut_tracks_external_moves_renames_and_disappearance() {
    let (mut state, fake) = fixture();
    cut(&mut state, &fake);
    let window = fake
        .move_pane_to_new_window("%0", "$1", Some("external"))
        .unwrap();
    fake.rename_session("$1", "renamed").unwrap();
    state.refresh(&fake).unwrap();
    let target = state.cut_target().unwrap();
    assert_eq!(target.session, "renamed");
    assert_eq!(
        target.window_index,
        state.tree.sessions[1]
            .windows
            .iter()
            .find(|w| w.id == window)
            .unwrap()
            .index
    );
    assert_eq!(target.pane_index, 0);
    fake.kill_pane("%0").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.cut, None);
    assert!(
        state
            .message
            .as_deref()
            .unwrap()
            .contains("no longer exists")
    );
}

#[test]
fn vanished_source_is_rechecked_at_both_session_prompts_and_window_submit() {
    for stage in 0..3 {
        let (mut state, fake) = fixture();
        cut(&mut state, &fake);
        state.focus = if stage == 2 {
            Level::Window
        } else {
            Level::Session
        };
        press(&mut state, &fake, "p");
        if stage == 1 {
            submit(&mut state, &fake, "logs");
        }
        fake.kill_pane("%0").unwrap();
        let before = fake.tree().unwrap();
        submit(&mut state, &fake, "name");
        assert!(
            state
                .message
                .as_deref()
                .unwrap()
                .contains("no longer exists")
        );
        assert_eq!(state.cut, None);
        assert_eq!(fake.tree().unwrap(), before);
    }
}

#[test]
fn locked_moves_exit_only_after_successful_paste() {
    for level in [Level::Session, Level::Window, Level::Pane] {
        let (mut state, fake) = fixture();
        state.cut = Some("%0".into());
        state.options.cut_locked = true;
        destination(&mut state, level);
        assert_eq!(press(&mut state, &fake, "<Enter>"), None);
        let exit = press(&mut state, &fake, "p");
        let exit = if level == Level::Pane {
            exit
        } else {
            if level == Level::Session {
                submit(&mut state, &fake, "logs");
            }
            submit(&mut state, &fake, "scratch")
        };
        assert_eq!(exit, Some(Exit::Moved));
        assert_eq!(state.cut, None);
    }
}

#[test]
fn locked_cancel_noop_failure_and_vanished_source_do_not_exit() {
    let (mut state, fake) = fixture();
    state.cut = Some("%0".into());
    state.options.cut_locked = true;
    state.focus = Level::Pane;
    assert_eq!(press(&mut state, &fake, "p"), None);
    assert_eq!(state.cut.as_deref(), Some("%0"));
    state.focus = Level::Session;
    press(&mut state, &fake, "p");
    assert_eq!(press(&mut state, &fake, "<Esc>"), None);
    destination(&mut state, Level::Pane);
    fake.fail_next("move failed");
    assert_eq!(press(&mut state, &fake, "p"), None);
    assert_eq!(state.cut.as_deref(), Some("%0"));
    fake.kill_pane("%0").unwrap();
    state.refresh(&fake).unwrap();
    assert_eq!(state.cut, None);
    assert!(
        state
            .message
            .as_deref()
            .unwrap()
            .contains("no longer exists")
    );
    assert_eq!(press(&mut state, &fake, "q"), Some(Exit::Cancel));
}

#[test]
fn moving_last_pane_closes_source_window_and_session() {
    let fake = FakeTmux::new(sample_tree(&["work", "notes"]));
    let mut state = State::load(
        PickerOptions {
            cut: Some("%0".into()),
            ..PickerOptions::default()
        },
        &fake,
        None,
    )
    .unwrap();
    destination(&mut state, Level::Pane);
    press(&mut state, &fake, "p");
    assert_eq!(state.tree.sessions.len(), 1);
    assert_eq!(state.tree.sessions[0].name, "notes");
    assert_eq!(state.hovered_pane().unwrap().id, "%0");
}
