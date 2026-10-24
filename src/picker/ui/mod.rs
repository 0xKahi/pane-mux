//! Rendering. Pure functions of `&State`; no tmux calls, no mutation.

mod columns;
mod layout;
mod popup;
mod preview;

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::{popup::Popup, state::State};
use crate::{
    config::{PickerConfig, Theme},
    tmux::Level,
};

/// Heights the state needs for paging/scrolling, computed from the same layout as `render`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heights {
    /// Rows available for items in one column (inside borders).
    pub list: u16,
    /// Rows available for preview text (inside borders).
    pub preview: u16,
}

/// Compute `Heights` for a terminal area. The event loop copies these into `State` before drawing.
pub fn heights(area: Rect, config: &PickerConfig) -> Heights {
    let areas = layout::areas(area, config);
    let block = Block::bordered();
    Heights {
        list: block.inner(areas.columns[0]).height,
        preview: block.inner(areas.preview).height,
    }
}

/// Draw the whole picker.
pub fn render(frame: &mut Frame, state: &State, theme: &Theme, config: &PickerConfig) {
    let areas = layout::areas(frame.area(), config);
    frame.render_widget(
        Paragraph::new(state.options.header.clone()).style(Style::from(&theme.picker.header)),
        areas.header,
    );
    if let Some(target) = state.cut_target() {
        let badge = format!(" cut {} ", target.at(Level::Pane));
        let width = Span::raw(&badge)
            .width()
            .min(usize::from(areas.header.width)) as u16;
        frame.render_widget(
            Paragraph::new(badge).style(Style::from(&theme.marker.badge_cut)),
            Rect::new(
                areas.header.right() - width,
                areas.header.y,
                width,
                areas.header.height,
            ),
        );
    }
    columns::render(frame, areas.columns, state, theme);
    preview::render(frame, areas.preview, state, theme);
    status(frame, areas.status, state, theme);
    popup::render(frame, areas.main, state, theme);
}

fn status(frame: &mut Frame, area: Rect, state: &State, theme: &Theme) {
    if let Some(message) = &state.message {
        frame.render_widget(
            Paragraph::new(format!("Error: {message}"))
                .style(Style::from(&theme.status.overall).patch(Style::from(&theme.notify.error))),
            area,
        );
        return;
    }
    let (mode, style) = if state.which.is_some() {
        ("which", &theme.mode.normal)
    } else {
        match state.popup {
            Some(Popup::Input { .. }) => ("input", &theme.mode.input),
            Some(Popup::Confirm { .. }) => ("confirm", &theme.mode.confirm),
            Some(Popup::Help { .. }) => ("help", &theme.mode.normal),
            None => ("picker", &theme.mode.normal),
        }
    };
    let level = match state.focus {
        Level::Session => "session",
        Level::Window => "window",
        Level::Pane => "pane",
    };
    let target = state
        .hovered()
        .map(|t| t.at(state.focus))
        .unwrap_or_default();
    let line = Line::from(vec![
        Span::styled(format!(" {mode} "), Style::from(style)),
        Span::styled(format!(" {level} "), Style::from(&theme.status.label)),
        Span::styled(format!("{target}  "), Style::from(&theme.status.message)),
        Span::raw(match mode {
            "input" => "Enter submit · Esc cancel",
            "confirm" => "y confirm · n cancel",
            "help" => "↑/↓ scroll · q close",
            "which" => "next key · Esc cancel",
            _ => "Enter select · q quit · ? help",
        }),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::from(&theme.status.overall)),
        area,
    );
}

#[cfg(test)]
mod tests;
