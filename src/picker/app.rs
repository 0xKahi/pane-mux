//! Synchronous input and refresh loop; state and rendering stay separate.

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};

use super::{Exit, PickerOptions, State, terminal, ui};
use crate::{
    config::{Loaded, keymap::Key},
    tmux::{Tmux, suppressed_pane_from_env},
};

/// Run the picker, restoring the terminal before returning a target or cancellation.
pub fn run(options: PickerOptions, tmux: &dyn Tmux, loaded: &Loaded) -> Result<Option<String>> {
    // Server/startup errors must not enter raw mode or the alternate screen.
    let mut state = State::load(options, tmux, suppressed_pane_from_env())?;
    let mut session = terminal::setup()?;
    let tick = Duration::from_millis(300);
    let mut last_tick = Instant::now();
    let mut dirty = true;

    loop {
        if dirty {
            let heights = ui::heights(session.terminal.size()?.into(), &loaded.config.picker);
            state.list_height = heights.list;
            state.preview_height = heights.preview;
            session.terminal.draw(|frame| {
                ui::render(frame, &state, &loaded.theme, &loaded.config.picker);
            })?;
            dirty = false;
        }
        if event::poll(tick.saturating_sub(last_tick.elapsed()))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(exit) = state.handle_key(Key::from(key), &loaded.keymap, tmux) {
                        return Ok(match exit {
                            Exit::Pick(target) => Some(target),
                            Exit::Cancel => None,
                        });
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick {
            match state.refresh(tmux) {
                Ok(changed) => dirty |= changed,
                Err(error) => {
                    // Keep the picker usable after transient server/capture failures.
                    state.message = Some(format!("{error:#}"));
                    dirty = true;
                }
            }
            last_tick = Instant::now();
        }
    }
}
