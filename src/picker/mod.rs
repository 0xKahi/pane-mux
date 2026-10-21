//! Reusable TUI picker.

pub mod action;
pub mod terminal;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::{Block, BorderType, Paragraph};

/// TEMPORARY vertical slice (HANDOVER step 2): proves UI-on-stderr / result-on-stdout.
/// Replaced by the real loop in task 5.6.
/// Returns `Some(target)` on Enter, `None` on q / Esc.
pub fn run_slice() -> Result<Option<String>> {
    let mut terminal = terminal::setup()?;
    let result = loop {
        terminal.draw(|frame| {
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" pane-mux slice ");
            let text =
                Paragraph::new("Enter = print 'work:1.0', q = cancel, p = panic").block(block);
            frame.render_widget(text, frame.area());
        })?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Enter => break Some("work:1.0".to_string()),
                KeyCode::Char('q') | KeyCode::Esc => break None,
                KeyCode::Char('p') => panic!("forced panic to test terminal restore"),
                _ => {}
            }
        }
    };
    terminal::restore();
    Ok(result)
}
