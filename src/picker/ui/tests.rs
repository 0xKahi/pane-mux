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
    for area in areas.columns {
        let icon = &buffer[(area.x + 5, area.y + 1)];
        let name = &buffer[(area.x + 7, area.y + 1)];
        let marker = &buffer[(area.x + 3, area.y + 1)];
        if area == areas.columns[0] {
            // Reversed (filled) row: icon and marker share the row's style.
            assert_eq!(
                (icon.bg, icon.fg, icon.modifier),
                (name.bg, name.fg, name.modifier)
            );
            assert_eq!(
                (marker.bg, marker.fg, marker.modifier),
                (name.bg, name.fg, name.modifier)
            );
            assert!(name.modifier.contains(Modifier::REVERSED));
        } else {
            // Underline row: underline everywhere, but icon/marker keep their colours.
            for cell in [icon, name, marker] {
                assert!(cell.modifier.contains(Modifier::UNDERLINED));
            }
            let icon_theme = if area == areas.columns[1] {
                &theme.icons.window
            } else {
                &theme.icons.pane
            };
            assert_eq!(icon.fg, icon_theme.fg.unwrap_or(Color::Reset));
            if marker.symbol() == "*" {
                assert_eq!(marker.fg, Color::Magenta);
            }
        }
    }
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

#[test]
fn hovered_caps_use_highlight_colour_at_inner_edges() {
    let (mut state, _, mut theme, config, _) = fixture();
    for (reversed, fg, bg, expected) in [
        (true, Some(Color::Cyan), Some(Color::Blue), Color::Cyan),
        (true, None, None, Color::Reset),
        (false, None, Some(Color::Blue), Color::Blue),
    ] {
        theme.picker.session.fg = None;
        theme.indicator.current.reversed = Some(reversed);
        theme.indicator.current.fg = fg;
        theme.indicator.current.bg = bg;
        let terminal = draw(&mut state, &theme, &config);
        let area =
            Block::bordered().inner(layout::areas(Rect::new(0, 0, 80, 24), &config).columns[0]);
        let buffer = terminal.backend().buffer();
        for (x, symbol) in [(area.x, "\u{e0b6}"), (area.right() - 1, "\u{e0b4}")] {
            let cap = &buffer[(x, area.y)];
            assert_eq!(cap.symbol(), symbol);
            assert_eq!(cap.fg, expected);
            assert_eq!(cap.bg, Color::Reset);
            assert!(cap.modifier.is_empty());
        }
        assert_eq!(buffer[(area.x, area.y + 1)].symbol(), " ");
        assert_eq!(buffer[(area.right() - 1, area.y + 1)].symbol(), " ");
    }
}

#[test]
fn custom_caps_and_long_names_keep_right_edge() {
    let (mut state, _, _, config, _) = fixture();
    let theme = Theme::from_toml(
        &preset::theme(),
        Some("[indicator]\npadding = { open = '<', close = '>' }"),
    )
    .unwrap();
    state.tree.sessions[0].name = "a very long session name that must be clipped".into();
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    let area = Block::bordered().inner(layout::areas(Rect::new(0, 0, 80, 24), &config).columns[0]);
    assert_eq!(terminal.backend().buffer()[(area.x, area.y)].symbol(), "<");
    assert_eq!(
        terminal.backend().buffer()[(area.right() - 1, area.y)].symbol(),
        ">"
    );
}

#[test]
fn wide_caps_reserve_display_width_and_plain_highlight_uses_spaces() {
    let (mut state, _, mut theme, config, _) = fixture();
    theme.indicator.padding.open = "界".into();
    theme.indicator.padding.close = "界>".into();
    state.tree.sessions[0].name = "界".repeat(40);
    let terminal = draw(&mut state, &theme, &config);
    let area = Block::bordered().inner(layout::areas(Rect::new(0, 0, 80, 24), &config).columns[0]);
    assert_eq!(terminal.backend().buffer()[(area.x, area.y)].symbol(), "界");
    assert_eq!(
        terminal.backend().buffer()[(area.right() - 3, area.y)].symbol(),
        "界"
    );
    assert_eq!(
        terminal.backend().buffer()[(area.right() - 1, area.y)].symbol(),
        ">"
    );
    theme.indicator.current.reversed = Some(false);
    theme.indicator.current.bg = None;
    let terminal = draw(&mut state, &theme, &config);
    assert_eq!(terminal.backend().buffer()[(area.x, area.y)].symbol(), " ");
    assert_eq!(
        terminal.backend().buffer()[(area.right() - 1, area.y)].symbol(),
        " "
    );
}

#[test]
fn unhovered_current_marker_is_magenta_but_name_keeps_base_colour() {
    let (mut state, fake, theme, config, keymap) = fixture();
    press(&mut state, &fake, &keymap, "j");
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(
        "unhovered_current_marker_is_magenta_but_name_is_unstyled",
        terminal.backend()
    );
    let buffer = terminal.backend().buffer();
    let marker = &buffer[(3, 2)];
    assert_eq!(marker.symbol(), "*");
    assert_eq!(marker.fg, Color::Magenta);
    let name = &buffer[(7, 2)];
    assert_eq!(name.symbol(), "w");
    assert_eq!(name.fg, theme.picker.session.fg.unwrap());
    assert_eq!(name.bg, Color::Reset);
    assert!(name.modifier.is_empty());
    assert_eq!(buffer[(5, 2)].fg, theme.icons.session.fg.unwrap());
    assert_eq!(buffer[(3, 3)].symbol(), " ");
    assert_eq!(buffer[(7, 3)].symbol(), "d");
}

#[test]
fn column_roles_follow_focus_and_cover_full_row_width() {
    let (mut state, _, theme, config, _) = fixture();
    for (focus_index, focus) in [Level::Session, Level::Window, Level::Pane]
        .into_iter()
        .enumerate()
    {
        state.focus = focus;
        let terminal = draw(&mut state, &theme, &config);
        let areas = layout::areas(Rect::new(0, 0, 80, 24), &config);
        let buffer = terminal.backend().buffer();
        for (index, area) in areas.columns.into_iter().enumerate() {
            let inner = Block::bordered().inner(area);
            let reversed = index <= focus_index;
            let expected = if reversed {
                Modifier::REVERSED
            } else {
                Modifier::UNDERLINED
            };
            let name = &buffer[(inner.x + 6, inner.y)];
            assert_eq!(name.modifier, expected);
            for x in inner.x..inner.right() {
                let cell = &buffer[(x, inner.y)];
                let edge = x == inner.x || x == inner.right() - 1;
                if reversed && edge {
                    assert_eq!(
                        cell.symbol(),
                        if x == inner.x { "\u{e0b6}" } else { "\u{e0b4}" }
                    );
                    assert_eq!(cell.fg, name.fg);
                    assert_eq!(cell.bg, Color::Reset);
                    assert!(cell.modifier.is_empty());
                } else if reversed {
                    assert_eq!(
                        (cell.fg, cell.bg, cell.modifier),
                        (name.fg, name.bg, name.modifier)
                    );
                } else {
                    // Underline rows: same bg/modifiers across the width; icon and
                    // marker cells may keep their own fg.
                    assert_eq!((cell.bg, cell.modifier), (name.bg, name.modifier));
                    if edge {
                        assert_eq!(cell.symbol(), " ");
                    }
                }
            }
        }
    }
}

#[test]
fn parent_current_and_preview_are_distinct_style_overlays() {
    let (mut state, _, mut theme, config, _) = fixture();
    theme.indicator.parent.bold = Some(true);
    theme.indicator.current.italic = Some(true);
    theme.indicator.preview.dim = Some(true);
    state.focus = Level::Window;
    let terminal = draw(&mut state, &theme, &config);
    insta::assert_snapshot!(terminal.backend());
    let areas = layout::areas(Rect::new(0, 0, 80, 24), &config);
    for (area, expected) in areas.columns.into_iter().zip([
        Modifier::REVERSED | Modifier::BOLD,
        Modifier::REVERSED | Modifier::ITALIC,
        Modifier::UNDERLINED | Modifier::DIM,
    ]) {
        assert_eq!(
            terminal.backend().buffer()[(area.x + 7, area.y + 1)].modifier,
            expected
        );
    }
}

#[test]
fn unhovered_names_use_level_text_colours() {
    let (mut state, _, theme, config, _) = fixture();
    let terminal = draw(&mut state, &theme, &config);
    let areas = layout::areas(Rect::new(0, 0, 80, 24), &config);
    for (area, expected, icon_fg) in [
        (
            areas.columns[0],
            theme.icons.session.fg.unwrap(),
            theme.icons.session.fg.unwrap(),
        ),
        (
            areas.columns[1],
            theme.icons.window.fg.unwrap(),
            theme.icons.window.fg.unwrap(),
        ),
        (areas.columns[2], Color::Reset, theme.icons.pane.fg.unwrap()),
    ] {
        let buffer = terminal.backend().buffer();
        let name = &buffer[(area.x + 7, area.y + 2)];
        assert_eq!(name.fg, expected);
        assert!(name.modifier.is_empty());
        assert_eq!(buffer[(area.x + 5, area.y + 2)].fg, icon_fg);
    }
}
