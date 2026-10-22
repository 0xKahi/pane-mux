//! Three independently scrolling lists with stable-id selections.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
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
    let block = Block::bordered()
        .title(title)
        .border_style(Style::from(&theme.picker.border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let selection = rows
        .iter()
        .position(|(id, _)| Some(id.as_str()) == selected)
        .unwrap_or(0);
    let offset = selection
        .saturating_add(1)
        .saturating_sub(usize::from(inner.height));
    for (row, (id, label)) in rows
        .into_iter()
        .skip(offset)
        .take(usize::from(inner.height))
        .enumerate()
    {
        let hovered = Some(id.as_str()) == selected;
        let base = match level {
            Level::Session => &theme.picker.session,
            Level::Window => &theme.picker.window,
            Level::Pane => &theme.picker.pane,
        };
        let indicator = if column_index(level) < column_index(state.focus) {
            &theme.indicator.parent
        } else if level == state.focus {
            &theme.indicator.current
        } else {
            &theme.indicator.preview
        };
        let mut style = Style::from(base);
        if hovered {
            style = style.patch(Style::from(indicator));
        }
        let cap_color = if hovered {
            highlight_color(style)
        } else {
            None
        };
        let (open, close) = if cap_color.is_some() {
            (
                theme.indicator.padding.open.as_str(),
                theme.indicator.padding.close.as_str(),
            )
        } else {
            (" ", " ")
        };
        let y = inner.y + row as u16;
        // Style the entire row first, including the edge spaces on underline rows.
        frame
            .buffer_mut()
            .set_style(Rect::new(inner.x, y, inner.width, 1), style);
        let open_width = Span::raw(open).width().min(usize::from(inner.width)) as u16;
        let close_width = Span::raw(close)
            .width()
            .min(usize::from(inner.width - open_width)) as u16;
        let content_area = Rect::new(
            inner.x + open_width,
            y,
            inner.width - open_width - close_width,
            1,
        );
        let marker = &theme.indicator.active;
        let marker_text = if state.is_current(&id) {
            marker.text.clone()
        } else {
            " ".repeat(Span::raw(&marker.text).width())
        };
        let marker_style = if hovered {
            Style::default()
        } else {
            Style::from(&marker.style)
        };
        let mut spans = vec![
            Span::raw(" "),
            Span::styled(marker_text, marker_style),
            Span::raw(" "),
        ];
        if !icon.text.is_empty() {
            let icon_style = if hovered {
                Style::default()
            } else {
                Style::default().fg(icon.fg.unwrap_or(Color::Reset))
            };
            spans.push(Span::styled(format!("{} ", icon.text), icon_style));
        }
        spans.push(Span::raw(label));
        // Separate regions clip long labels before the right cap, including wide glyphs.
        frame.render_widget(Paragraph::new(Line::from(spans)).style(style), content_area);
        let cap_style = match cap_color {
            // Reset removes the row's modifiers as well as its background on actual caps.
            Some(color) => Style::reset().fg(color).bg(Color::Reset),
            None => style,
        };
        frame.render_widget(
            Paragraph::new(open).style(cap_style),
            Rect::new(inner.x, y, open_width, 1),
        );
        frame.render_widget(
            Paragraph::new(close).style(cap_style),
            Rect::new(inner.right() - close_width, y, close_width, 1),
        );
    }
}

// Reversal uses the foreground as the visible background; caps must not be reversed.
fn highlight_color(style: Style) -> Option<Color> {
    if style.add_modifier.contains(Modifier::REVERSED) {
        Some(style.fg.unwrap_or(Color::Reset))
    } else {
        style.bg
    }
}

fn column_index(level: Level) -> u8 {
    match level {
        Level::Session => 0,
        Level::Window => 1,
        Level::Pane => 2,
    }
}
