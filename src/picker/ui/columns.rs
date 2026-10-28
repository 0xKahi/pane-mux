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
        .map(|s| (s.id.clone(), String::new(), s.name.as_str(), None))
        .collect();
    let windows = state
        .windows()
        .iter()
        .map(|w| {
            let indicator = state
                .stash_counts
                .get(&w.id)
                .filter(|&&count| count > 0)
                .and_then(|count| {
                    row_indicator(
                        &theme.icons.stashed,
                        theme
                            .icons
                            .stashed
                            .text
                            .replace("$count", &count.to_string()),
                    )
                });
            (
                w.id.clone(),
                format!("{}: ", w.index),
                w.name.as_str(),
                indicator,
            )
        })
        .collect();
    let panes = state
        .panes()
        .iter()
        .map(|p| {
            let indicator = p
                .in_copy_mode()
                .then(|| row_indicator(&theme.icons.copy_mode, theme.icons.copy_mode.text.clone()))
                .flatten();
            (
                p.id.clone(),
                format!("{}: ", p.index),
                p.command.as_str(),
                indicator,
            )
        })
        .collect();
    for (area, level, icon, rows) in [
        (areas[0], Level::Session, &theme.icons.session, sessions),
        (areas[1], Level::Window, &theme.icons.window, windows),
        (areas[2], Level::Pane, &theme.icons.pane, panes),
    ] {
        column(frame, area, state, theme, level, icon, rows);
    }
}

fn row_indicator(icon: &Icon, text: String) -> Option<Span<'static>> {
    (!text.is_empty()).then(|| {
        Span::styled(
            format!("  {text}"),
            Style::default().fg(icon.fg.unwrap_or(Color::Reset)),
        )
    })
}

fn column(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    theme: &Theme,
    level: Level,
    icon: &Icon,
    rows: Vec<(String, String, &str, Option<Span<'static>>)>,
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
        .position(|(id, _, _, _)| Some(id.as_str()) == selected)
        .unwrap_or(0);
    let offset = selection
        .saturating_add(1)
        .saturating_sub(usize::from(inner.height));
    let finder = state.finder.as_ref().filter(|_| level == state.focus);
    let position =
        finder.and_then(|finder| selected.and_then(|id| finder.position(&state.names(level), id)));
    for (row, (id, prefix, name, mut row_indicator)) in rows
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
        // A "filled" row (reversed or with a bg) drops the icon/marker colours so the
        // background is uniform; underline-only rows keep them.
        let filled = cap_color.is_some();
        // Underline rows: one underline colour across the row, matching the entry text
        // colour, even under icons/markers that keep their own fg.
        if hovered && !filled && style.add_modifier.contains(Modifier::UNDERLINED) {
            style = style.underline_color(style.fg.unwrap_or(Color::Reset));
        }
        let (open, close) = if filled {
            (
                theme.indicator.padding.open.as_str(),
                theme.indicator.padding.close.as_str(),
            )
        } else {
            (" ", " ")
        };
        let y = inner.y + row as u16;
        if level == Level::Pane && state.cut.as_deref() == Some(id.as_str()) && area.width > 0 {
            frame.render_widget(
                Paragraph::new(theme.marker.symbol.as_str()).style(Style::from(&theme.marker.cut)),
                Rect::new(area.x, y, 1, 1),
            );
        }
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
        let marker_style = if filled {
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
            let icon_style = if filled {
                Style::default()
            } else {
                Style::default().fg(icon.fg.unwrap_or(Color::Reset))
            };
            spans.push(Span::styled(format!("{} ", icon.text), icon_style));
        }
        spans.push(Span::raw(prefix));
        if let Some(matched) = finder.and_then(|finder| finder.matched(name)) {
            spans.push(Span::raw(&name[..matched.start]));
            spans.push(Span::styled(
                &name[matched.clone()],
                style.patch(Style::from(&theme.picker.find_keyword)),
            ));
            spans.push(Span::raw(&name[matched.end..]));
        } else {
            spans.push(Span::raw(name));
        }
        let line = Line::from(spans);
        let badge = position
            .filter(|_| hovered)
            .map(|(index, total)| format!(" [{index}/{total}]"))
            .unwrap_or_default();
        let badge_width = Span::raw(&badge)
            .width()
            .min(usize::from(content_area.width)) as u16;
        if filled && let Some(indicator) = &mut row_indicator {
            indicator.style = Style::default();
        }
        let indicator_width = row_indicator
            .as_ref()
            .map_or(0, Span::width)
            .min(usize::from(content_area.width - badge_width))
            as u16;
        let label_width = content_area.width - badge_width - indicator_width;
        let indicator_x = content_area.x + line.width().min(usize::from(label_width)) as u16;
        let badge_x = indicator_x + indicator_width;
        // Reserve indicators, badge and caps before clipping long names, including wide glyphs.
        frame.render_widget(
            Paragraph::new(line).style(style),
            Rect::new(content_area.x, y, label_width, 1),
        );
        if let Some(indicator) = row_indicator {
            frame.render_widget(
                Paragraph::new(indicator).style(style),
                Rect::new(indicator_x, y, indicator_width, 1),
            );
        }
        frame.render_widget(
            Paragraph::new(badge).style(Style::from(&theme.picker.find_position)),
            Rect::new(badge_x, y, badge_width, 1),
        );
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
