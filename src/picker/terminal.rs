//! Terminal setup: the UI is drawn on **stderr** and keys come from `/dev/tty`
//! (crossterm's `use-dev-tty` feature), so stdout stays free for the picked target.

use std::io::{self, Stderr};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type Term = Terminal<CrosstermBackend<Stderr>>;

/// Enter raw mode + alternate screen and install a panic hook that undoes both.
pub fn setup() -> Result<Term> {
    install_panic_hook();
    enable_raw_mode()?;
    execute!(io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

/// Undo `setup`. Safe to call more than once; errors are ignored because
/// there is nothing useful to do with them while exiting.
pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stderr(), LeaveAlternateScreen, crossterm::cursor::Show);
}

/// Restore the terminal before the default panic message is printed,
/// otherwise the message is lost in the alternate screen.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));
}
