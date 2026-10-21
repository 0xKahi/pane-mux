//! Draw on stderr when it is a terminal, otherwise on /dev/tty; never stdout.

use std::{
    fs::OpenOptions,
    io::{self, IsTerminal, Write},
    sync::Once,
};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type Term = Terminal<CrosstermBackend<Box<dyn Write>>>;

/// Own the terminal and restore its mode even when drawing or input returns an error.
pub struct Session {
    pub terminal: Term,
    _restore: RestoreGuard,
}

struct RestoreGuard;

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn output() -> Result<Box<dyn Write>> {
    if io::stderr().is_terminal() {
        Ok(Box::new(io::stderr()))
    } else {
        Ok(Box::new(OpenOptions::new().write(true).open("/dev/tty")?))
    }
}

/// Enter raw mode and the alternate screen, with cleanup on every return path.
pub fn setup() -> Result<Session> {
    install_panic_hook();
    let mut writer = output()?;
    let guard = RestoreGuard;
    enable_raw_mode()?;
    execute!(writer, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(writer))?;
    terminal.hide_cursor()?;
    Ok(Session {
        terminal,
        _restore: guard,
    })
}

/// Best-effort restoration, also used before printing a panic message.
pub fn restore() {
    let _ = disable_raw_mode();
    if let Ok(mut writer) = output() {
        let _ = execute!(writer, LeaveAlternateScreen, crossterm::cursor::Show);
    }
}

fn install_panic_hook() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            default_hook(info);
        }));
    });
}
