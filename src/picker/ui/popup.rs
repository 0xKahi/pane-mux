//! Modal prompts, help, and bottom-aligned which-key suggestions.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph},
};

use crate::{
    config::{Theme, keymap::Key},
    picker::{InputPurpose, Popup, State},
};

pub(super) fn render(frame: &mut Frame, area: Rect, state: &State, theme: &Theme) {
    if let Some(popup) = &state.popup {
        match popup {
            Popup::Input { value, purpose, .. } => {
                let title = match purpose {
                    InputPurpose::Find {
                        previous: false, ..
                    } => "Find next:",
                    InputPurpose::Find { previous: true, .. } => "Find previous:",
                    InputPurpose::CreateSession => "New session name:",
                    InputPurpose::PasteSession { .. } => "Paste into new session:",
                    InputPurpose::PasteSessionWindow { .. } | InputPurpose::PasteWindow { .. } => {
                        "Paste into new window:"
                    }
                    InputPurpose::CreateWindow { .. } => "New window name:",
                    InputPurpose::RenameSession { .. } => "Rename session:",
                    InputPurpose::RenameWindow { .. } => "Rename window:",
                }
                .to_string();
                input(frame, area, &title, value, theme);
            }
            Popup::Confirm { title, body, .. } => {
                let area = centered(area, 60, 5);
                frame.render_widget(Clear, area);
                let block = Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::from(&theme.confirm.border))
                    .title(Line::styled(
                        title.clone(),
                        Style::from(&theme.confirm.title),
                    ));
                let text = format!("{body}  y/n");
                frame.render_widget(
                    Paragraph::new(text)
                        .style(Style::from(&theme.confirm.body))
                        .block(block),
                    area,
                );
            }
            Popup::Help { cursor, chords } => {
                let area = centered(area, 72, area.height.saturating_sub(2));
                frame.render_widget(Clear, area);
                let items: Vec<ListItem> = chords
                    .iter()
                    .map(|chord| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:<18}", keys(&chord.on)),
                                Style::from(&theme.help.on),
                            ),
                            Span::styled(
                                chord.desc.clone().unwrap_or_default(),
                                Style::from(&theme.help.desc),
                            ),
                        ]))
                    })
                    .collect();
                let block = Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::from(&theme.help.border))
                    .title(Line::styled(
                        "Help — picker bindings",
                        Style::from(&theme.help.title),
                    ));
                let list = List::new(items)
                    .block(block)
                    .highlight_style(Style::from(&theme.help.hovered));
                let mut selection = ListState::default().with_selected(Some(*cursor));
                frame.render_stateful_widget(list, area, &mut selection);
            }
        }
    }
    // Which has highest priority and is drawn last, even above another popup.
    if let Some(which) = &state.which {
        let height = which
            .candidates
            .len()
            .saturating_add(2)
            .min(usize::from(area.height)) as u16;
        let popup_area = Rect::new(
            area.x,
            area.bottom().saturating_sub(height),
            area.width,
            height,
        );
        frame.render_widget(Clear, popup_area);
        let lines: Vec<Line> = which
            .candidates
            .iter()
            .map(|chord| {
                Line::from(vec![
                    Span::styled(
                        keys(chord.on.get(which.times..).unwrap_or_default()),
                        Style::from(&theme.which.key),
                    ),
                    Span::styled(" → ", Style::from(&theme.which.separator)),
                    Span::styled(
                        chord.desc.clone().unwrap_or_default(),
                        Style::from(&theme.which.desc),
                    ),
                ])
            })
            .collect();
        let block = Block::bordered()
            .border_style(Style::from(&theme.which.border))
            .title("Which key?");
        frame.render_widget(Paragraph::new(lines).block(block), popup_area);
    }
}

fn input(frame: &mut Frame, area: Rect, title: &str, value: &str, theme: &Theme) {
    let area = centered(area, 60, 3);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::from(&theme.input.border))
        .title(Line::styled(
            title.to_string(),
            Style::from(&theme.input.title),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let line = Line::from(vec![
        Span::styled(value.to_string(), Style::from(&theme.input.value)),
        Span::styled(" ", Style::from(&theme.input.selected)),
    ]);
    // Text width handles wide Unicode characters without adding a dependency.
    let offset = line
        .width()
        .saturating_sub(usize::from(inner.width))
        .min(usize::from(u16::MAX)) as u16;
    frame.render_widget(Paragraph::new(line).scroll((0, offset)), inner);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn keys(keys: &[Key]) -> String {
    keys.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}
