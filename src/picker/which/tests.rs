use super::*;
use crate::config::keymap::Keymap;
use crate::picker::{Exit, PickerOptions, State};
use crate::tmux::{FakeTmux, sample_tree};

#[test]
fn filters_candidates_across_three_keys_without_early_completion() {
    let map = Keymap::from_toml("[picker]\nkeymap = [{on = ['g', 'a', 'b'], run = 'quit'}, {on = ['g', 'c'], run = 'help'}]", None).unwrap();
    let mut which = Which::start("g".parse().unwrap(), &map.picker).unwrap();
    assert_eq!(which.candidates.len(), 2);
    assert_eq!(which.advance("a".parse().unwrap()), None);
    assert_eq!(which.candidates.len(), 1);
    assert_eq!(
        which.advance("b".parse().unwrap()),
        Some(vec![Action::Quit])
    );
    assert!(which.candidates.is_empty());
}

#[test]
fn completed_binding_wins_over_longer_shared_prefix() {
    let map = Keymap::from_toml("[picker]\nkeymap = [{on = ['g', 'g'], run = 'quit'}, {on = ['g', 'g', 'g'], run = 'help'}]", None).unwrap();
    let mut which = Which::start("g".parse().unwrap(), &map.picker).unwrap();
    assert_eq!(
        which.advance("g".parse().unwrap()),
        Some(vec![Action::Quit])
    );
}

#[test]
fn single_key_binding_wins_over_multi_key_prefix() {
    let map = Keymap::from_toml(
        "[picker]\nkeymap = [{on = 'g', run = 'quit'}, {on = ['g', 'g'], run = 'help'}]",
        None,
    )
    .unwrap();
    let fake = FakeTmux::new(sample_tree(&["work"]));
    let mut state = State::load(PickerOptions::default(), &fake, None).unwrap();
    assert_eq!(
        state.handle_key("g".parse().unwrap(), &map, &fake),
        Some(Exit::Cancel)
    );
    assert!(state.which.is_none());
}
