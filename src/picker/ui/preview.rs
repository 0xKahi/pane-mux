//! ANSI-aware pane preview and self-pane placeholder.

use ansi_to_tui::IntoText;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph},
};

use crate::{config::Theme, picker::State, tmux::Level};

pub(super) fn render(frame: &mut Frame, area: Rect, state: &State, theme: &Theme) {
    let title = state
        .preview_target()
        .map(|t| t.at(Level::Pane))
        .unwrap_or_else(|| "Preview".into());
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::from(&theme.preview.border))
        .title(Line::styled(title, Style::from(&theme.preview.title)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let placeholder = if state.sessions().is_empty() {
        Some("no tmux sessions")
    } else if state.preview_suppressed() {
        Some("(this is the picker's own pane)")
    } else {
        None
    };
    if let Some(message) = placeholder {
        let row = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1) / 2,
            inner.width,
            inner.height.min(1),
        );
        frame.render_widget(Paragraph::new(message).alignment(Alignment::Center), row);
    } else {
        let text = state
            .preview
            .content
            .into_text()
            .unwrap_or_else(|_| Text::raw("(unable to decode pane preview)"));
        frame.render_widget(
            Paragraph::new(text).scroll((state.preview.scroll, 0)),
            inner,
        );
    }
}
