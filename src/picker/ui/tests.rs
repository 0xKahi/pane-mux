use super::*;
use crate::{
    config::{Config, keymap::Keymap, preset, settings::PreviewPosition},
    picker::{PickerOptions, action::Action},
    tmux::{Current, FakeTmux, Pane, Tree, Window, sample_tree},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};

fn fixture() -> (State, FakeTmux, Theme, PickerConfig, Keymap) {
    let mut tree = sample_tree(&["work", "dotfiles"]);
    tree.sessions[0].windows = vec![
        Window {
            id: "@5".into(),
            index: 1,
            name: "editor".into(),
            active: true,
            panes: vec![
                Pane {
                    id: "%12".into(),
                    index: 0,
                    active: true,
                    command: "nvim".into(),
                },
                Pane {
                    id: "%13".into(),
                    index: 2,
                    active: false,
                    command: "zsh".into(),
                },
            ],
        },
        Window {
            id: "@6".into(),
            index: 2,
            name: "server logs".into(),
            active: false,
            panes: Vec::new(),
        },
    ];
    let fake = FakeTmux::new(tree)
        .with_current(Current {
            session_id: "$0".into(),
            window_id: "@5".into(),
            pane_id: "%12".into(),
        })
        .with_capture(
            "%12",
            "\x1b[31mred output\x1b[0m\n\x1b[1;32mready\x1b[0m\n$ cargo test\nall tests passed\n",
        );
    let state = State::load(
        PickerOptions {
            level: None,
            header: "[Pick session]".into(),
        },
        &fake,
        None,
    )
    .unwrap();
    let theme = Theme::from_toml(&preset::theme(), None).unwrap();
    let config = Config::from_toml(&preset::config(), None).unwrap().picker;
    let keymap = Keymap::from_toml(&preset::keymap(), None).unwrap();
    (state, fake, theme, config, keymap)
}

fn draw(state: &mut State, theme: &Theme, config: &PickerConfig) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| {
            let hints = heights(frame.area(), config);
            state.list_height = hints.list;
            state.preview_height = hints.preview;
            render(frame, state, theme, config);
        })
        .unwrap();
    terminal
}

fn press(state: &mut State, fake: &FakeTmux, keymap: &Keymap, key: &str) {
    state.handle_key(key.parse().unwrap(), keymap, fake);
}

#[test]
fn bottom_50_layout_and_current_columns() {
    let (mut state, _, theme, config, _) = fixture();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    let areas = layout::areas(Rect::new(0, 0, 80, 24), &config);
    let buffer = terminal.backend().buffer();
    let current = &buffer[(areas.columns[0].x + 3, areas.columns[0].y + 1)];
    assert_eq!(current.fg, theme.picker.current.fg.unwrap());
    assert!(
        current
            .modifier
            .contains(Modifier::BOLD | Modifier::REVERSED)
    );
    let child = &buffer[(areas.columns[1].x + 3, areas.columns[1].y + 1)];
    assert!(
        child
            .modifier
            .contains(Modifier::UNDERLINED | Modifier::BOLD)
    );
    assert_eq!(buffer[(1, 2)].fg, theme.icons.session.fg.unwrap());
}

#[test]
fn top_75_layout_with_switch_header() {
    let (mut state, _, theme, mut config, _) = fixture();
    config.preview_position = PreviewPosition::Top;
    config.preview_ratio = 75;
    state.options.header = "[Switch]".into();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn coloured_preview_preserves_ansi_styles() {
    let (mut state, _, theme, config, _) = fixture();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    let area = layout::areas(Rect::new(0, 0, 80, 24), &config).preview;
    let red = &terminal.backend().buffer()[(area.x + 1, area.y + 1)];
    assert_eq!(red.symbol(), "r");
    assert_eq!(red.fg, Color::Red);
    let green = &terminal.backend().buffer()[(area.x + 1, area.y + 2)];
    assert_eq!(green.fg, Color::Green);
    assert!(green.modifier.contains(Modifier::BOLD));
}

#[test]
fn own_pane_placeholder() {
    let (mut state, fake, theme, config, _) = fixture();
    state.suppressed_pane = Some("%12".into());
    state.refresh(&fake).unwrap();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn empty_tree_placeholder() {
    let (mut state, _, theme, config, _) = fixture();
    let fake = FakeTmux::new(Tree::default());
    state.refresh(&fake).unwrap();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn which_popup_shows_remaining_keys() {
    let (mut state, fake, theme, config, keymap) = fixture();
    press(&mut state, &fake, &keymap, "g");
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn input_popup_with_cursor() {
    let (mut state, fake, theme, config, keymap) = fixture();
    for key in ["a", "n", "o", "t", "e", "s"] {
        press(&mut state, &fake, &keymap, key);
    }
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    let buffer = terminal.backend().buffer();
    let cursor = &buffer[(16, 11)];
    assert_eq!(cursor.symbol(), " ");
    assert!(cursor.modifier.contains(Modifier::REVERSED));
    assert_eq!(cursor.fg, theme.input.value.fg.unwrap());
}

#[test]
fn confirm_popup() {
    let (mut state, fake, theme, config, keymap) = fixture();
    for key in ["j", "d", "d"] {
        press(&mut state, &fake, &keymap, key);
    }
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn help_popup_scrolls_to_cursor() {
    let (mut state, fake, theme, config, keymap) = fixture();
    press(&mut state, &fake, &keymap, "?");
    for _ in 0..22 {
        press(&mut state, &fake, &keymap, "j");
    }
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn error_message_uses_notify_style() {
    let (mut state, _, theme, config, _) = fixture();
    state.message = Some("duplicate session: work".into());
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    assert_eq!(
        terminal.backend().buffer()[(0, 23)].fg,
        theme.notify.error.fg.unwrap()
    );
}

#[test]
fn heights_match_render_regions_for_both_positions() {
    let (mut state, _, theme, mut config, _) = fixture();
    for (position, ratio) in [(PreviewPosition::Bottom, 50), (PreviewPosition::Top, 75)] {
        config.preview_position = position;
        config.preview_ratio = ratio;
        let terminal = draw(&mut state, &theme, &config);
        let area = Rect::new(0, 0, 80, 24);
        let regions = layout::areas(area, &config);
        let hints = heights(area, &config);
        assert_eq!(
            hints.list,
            Block::bordered().inner(regions.columns[0]).height
        );
        assert_eq!(
            hints.preview,
            Block::bordered().inner(regions.preview).height
        );
        assert_eq!(state.list_height, hints.list);
        assert_eq!(state.preview_height, hints.preview);
        // Inspect actual rendered borders, not just two copies of layout arithmetic.
        assert_eq!(
            terminal.backend().buffer()[(regions.columns[0].x, regions.columns[0].y)].symbol(),
            "┌"
        );
        assert_eq!(
            terminal.backend().buffer()[(regions.preview.x, regions.preview.y)].symbol(),
            "╭"
        );
    }
}

#[test]
fn long_columns_scroll_selected_row_into_view() {
    let (mut state, _, theme, config, _) = fixture();
    let names: Vec<String> = (0..30).map(|i| format!("session-{i:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let fake = FakeTmux::new(sample_tree(&refs));
    state.refresh(&fake).unwrap();
    state.apply(&Action::Arrow(crate::picker::action::Step::Bot), &fake);
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
}

#[test]
fn preview_scroll_uses_line_offset() {
    let (mut state, _, theme, config, _) = fixture();
    state.preview.scroll = 2;
    let terminal = draw(&mut state, &theme, &config);
    let area = layout::areas(Rect::new(0, 0, 80, 24), &config).preview;
    assert_eq!(
        terminal.backend().buffer()[(area.x + 1, area.y + 1)].symbol(),
        "$"
    );
}

#[test]
fn tiny_terminals_do_not_panic() {
    let (mut state, fake, theme, config, keymap) = fixture();
    press(&mut state, &fake, &keymap, "a");
    for (width, height) in [(1, 1), (2, 2), (8, 4)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, &state, &theme, &config))
            .unwrap();
    }
    assert_eq!(
        heights(Rect::new(0, 0, 1, 1), &config),
        Heights {
            list: 0,
            preview: 0
        }
    );
}
