//! One layout calculation shared by rendering and the event loop's height hints.

use ratatui::layout::{Constraint, Layout, Rect};

use crate::config::{PickerConfig, settings::PreviewPosition};

/// Screen regions including widget borders.
#[derive(Debug)]
pub(super) struct Areas {
    pub header: Rect,
    pub main: Rect,
    pub columns: [Rect; 3],
    pub preview: Rect,
    pub status: Rect,
}

pub(super) fn areas(area: Rect, config: &PickerConfig) -> Areas {
    let [header, main, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let ratio = config.preview_ratio.min(100);
    let (columns, preview) = match config.preview_position {
        PreviewPosition::Bottom => {
            let [columns, preview] = Layout::vertical([
                Constraint::Percentage(100 - ratio),
                Constraint::Percentage(ratio),
            ])
            .areas(main);
            (columns, preview)
        }
        PreviewPosition::Top => {
            let [preview, columns] = Layout::vertical([
                Constraint::Percentage(ratio),
                Constraint::Percentage(100 - ratio),
            ])
            .areas(main);
            (columns, preview)
        }
    };
    let columns = Layout::horizontal([Constraint::Ratio(1, 3); 3]).areas(columns);
    Areas {
        header,
        main,
        columns,
        preview,
        status,
    }
}
