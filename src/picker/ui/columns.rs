//! Three independently scrolling lists with stable-id selections.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
};

use crate::{
    config::{Theme, theme::Icon},
    picker::State,
    tmux::Level,
};

pub(super) fn render(frame: &mut Frame, areas: [Rect; 3], state: &State, theme: &Theme) {
    let sessions = state
        .sessions()
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    let windows = state
        .windows()
        .iter()
        .map(|w| (w.id.clone(), format!("{}: {}", w.index, w.name)))
        .collect();
    let panes = state
        .panes()
        .iter()
        .map(|p| (p.id.clone(), format!("{}: {}", p.index, p.command)))
        .collect();
    for (area, level, icon, rows) in [
        (areas[0], Level::Session, &theme.icons.session, sessions),
        (areas[1], Level::Window, &theme.icons.window, windows),
        (areas[2], Level::Pane, &theme.icons.pane, panes),
    ] {
        column(frame, area, state, theme, level, icon, rows);
    }
}

fn column(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    theme: &Theme,
    level: Level,
    icon: &Icon,
    rows: Vec<(String, String)>,
) {
    let (title, selected) = match level {
        Level::Session => ("Sessions", state.session.as_deref()),
        Level::Window => ("Windows", state.window.as_deref()),
        Level::Pane => ("Panes", state.pane.as_deref()),
    };
    let selection = rows
        .iter()
        .position(|(id, _)| Some(id.as_str()) == selected);
    let items: Vec<ListItem> = rows
        .into_iter()
        .map(|(id, label)| {
            let mut style = Style::default();
            if state.is_current(&id) {
                style = style.patch(Style::from(&theme.picker.current));
            }
            if Some(id.as_str()) == selected {
                let hover = if state.focus == level {
                    &theme.picker.hovered
                } else {
                    &theme.picker.hovered_unfocused
                };
                style = style.patch(Style::from(hover));
            }
            let mut spans = Vec::new();
            if !icon.text.is_empty() {
                let mut icon_style = Style::default();
                if let Some(fg) = icon.fg {
                    icon_style = icon_style.fg(fg);
                }
                spans.push(Span::styled(format!("{} ", icon.text), icon_style));
            }
            spans.push(Span::raw(label));
            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();
    let block = Block::bordered()
        .title(title)
        .border_style(Style::from(&theme.picker.border));
    let list = List::new(items).block(block);
    // Widget-local state scrolls to the selected row without mutating picker state.
    let mut list_state = ListState::default().with_selected(selection);
    frame.render_stateful_widget(list, area, &mut list_state);
}
