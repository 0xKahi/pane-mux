//! Picker state and synchronous action handling, independent of terminal rendering.

use std::cmp::Reverse;

use anyhow::Result;
use crossterm::event::KeyCode;

use super::{
    action::{Action, Case, Direction, Step},
    finder::Finder,
    options::{PickerOptions, SessionOrder},
    popup::{InputPurpose, Popup},
    which::Which,
};
use crate::{
    config::keymap::{Key, Keymap},
    tmux::{Current, Level, Pane, Session, Target, Tmux, Tree, Window},
};

/// Outcome consumed by the command after restoring the terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    Pick(String),
    Cancel,
}

/// Raw ANSI preview text and its top-line offset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preview {
    pub pane_id: Option<String>,
    pub content: String,
    pub scroll: u16,
}

/// Owned UI state. Rendering only needs an immutable reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub options: PickerOptions,
    pub tree: Tree,
    pub current: Option<Current>,
    pub suppressed_pane: Option<String>,
    pub focus: Level,
    pub session: Option<String>,
    pub window: Option<String>,
    pub pane: Option<String>,
    pub preview: Preview,
    pub popup: Option<Popup>,
    pub finder: Option<Finder>,
    pub which: Option<Which>,
    pub message: Option<String>,
    pub list_height: u16,
    pub preview_height: u16,
}

impl State {
    /// Initialize cursor ids, selecting active children of the first session.
    pub fn new(
        options: PickerOptions,
        tree: Tree,
        current: Option<Current>,
        suppressed_pane: Option<String>,
    ) -> Self {
        let mut state = Self {
            options,
            tree,
            current,
            suppressed_pane,
            focus: Level::Session,
            session: None,
            window: None,
            pane: None,
            preview: Preview::default(),
            popup: None,
            finder: None,
            which: None,
            message: None,
            list_height: 10,
            preview_height: 10,
        };
        state.sort_sessions();
        state.session = state.initial_session();
        state.reset_window();
        state
    }

    /// Under recency order, open on the first non-current session; otherwise
    /// (or when none exists) open on the first session.
    fn initial_session(&self) -> Option<String> {
        let sessions = &self.tree.sessions;
        let previous = match (self.options.order, &self.current) {
            (SessionOrder::Recent, Some(current)) => {
                sessions.iter().find(|s| s.id != current.session_id)
            }
            _ => None,
        };
        previous.or_else(|| sessions.first()).map(|s| s.id.clone())
    }

    /// Load initial server data. Startup errors propagate to the command.
    pub fn load(
        options: PickerOptions,
        tmux: &dyn Tmux,
        suppressed_pane: Option<String>,
    ) -> Result<Self> {
        let mut state = Self::new(options, tmux.tree()?, tmux.current()?, suppressed_pane);
        state.update_preview(tmux)?;
        Ok(state)
    }

    fn sort_sessions(&mut self) {
        if self.options.order == SessionOrder::Recent {
            let current = self.current.as_ref().map(|c| c.session_id.as_str());
            self.tree.sessions.sort_by_key(|s| {
                (
                    Some(s.id.as_str()) != current,
                    Reverse(s.last_attached),
                    s.name.clone(),
                )
            });
        }
    }

    pub fn sessions(&self) -> &[Session] {
        &self.tree.sessions
    }
    pub fn windows(&self) -> &[Window] {
        self.hovered_session().map_or(&[], |s| s.windows.as_slice())
    }
    pub fn panes(&self) -> &[Pane] {
        self.hovered_window().map_or(&[], |w| w.panes.as_slice())
    }
    pub fn hovered_session(&self) -> Option<&Session> {
        self.sessions()
            .iter()
            .find(|s| Some(&s.id) == self.session.as_ref())
    }
    pub fn hovered_window(&self) -> Option<&Window> {
        self.windows()
            .iter()
            .find(|w| Some(&w.id) == self.window.as_ref())
    }
    pub fn hovered_pane(&self) -> Option<&Pane> {
        self.panes()
            .iter()
            .find(|p| Some(&p.id) == self.pane.as_ref())
    }

    /// Return the id at the focused column.
    pub fn hovered_id(&self) -> Option<&str> {
        match self.focus {
            Level::Session => self.session.as_deref(),
            Level::Window => self.window.as_deref(),
            Level::Pane => self.pane.as_deref(),
        }
    }

    fn selected_window(&self) -> Option<&Window> {
        if self.focus == Level::Session {
            self.hovered_session()?.active_window()
        } else {
            self.hovered_window()
        }
    }

    fn selected_pane(&self) -> Option<&Pane> {
        if self.focus == Level::Pane {
            self.hovered_pane()
        } else {
            self.selected_window()?.active_pane()
        }
    }

    /// Resolve a printable target; absent children are allowed for session/window picks.
    pub fn hovered(&self) -> Option<Target> {
        let session = self.hovered_session()?;
        if self.focus == Level::Window {
            self.hovered_window()?;
        }
        if self.focus == Level::Pane {
            self.hovered_pane()?;
        }
        Some(Target {
            session: session.name.clone(),
            window_index: self.selected_window().map_or(0, |w| w.index),
            pane_index: self.selected_pane().map_or(0, |p| p.index),
        })
    }

    /// Session/window focus previews active descendants; pane focus previews the cursor.
    pub fn preview_target(&self) -> Option<Target> {
        self.selected_pane()?;
        self.hovered()
    }

    /// Check stable ids for current-item markers and deletion protection.
    pub fn is_current(&self, id: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|c| c.session_id == id || c.window_id == id || c.pane_id == id)
    }

    pub fn preview_suppressed(&self) -> bool {
        self.selected_pane()
            .is_some_and(|p| self.suppressed_pane.as_deref() == Some(&p.id))
    }

    fn reset_window(&mut self) {
        self.window = self
            .hovered_session()
            .and_then(Session::active_window)
            .map(|w| w.id.clone());
        self.reset_pane();
    }

    fn reset_pane(&mut self) {
        self.pane = self
            .hovered_window()
            .and_then(Window::active_pane)
            .map(|p| p.id.clone());
    }

    fn ids(&self, level: Level) -> Vec<String> {
        match level {
            Level::Session => self.sessions().iter().map(|s| s.id.clone()).collect(),
            Level::Window => self.windows().iter().map(|w| w.id.clone()).collect(),
            Level::Pane => self.panes().iter().map(|p| p.id.clone()).collect(),
        }
    }

    /// Visible ids and names only (no indexes, icons or markers), in row order.
    pub fn names(&self, level: Level) -> Vec<(String, &str)> {
        match level {
            Level::Session => self
                .sessions()
                .iter()
                .map(|s| (s.id.clone(), s.name.as_str()))
                .collect(),
            Level::Window => self
                .windows()
                .iter()
                .map(|w| (w.id.clone(), w.name.as_str()))
                .collect(),
            Level::Pane => self
                .panes()
                .iter()
                .map(|p| (p.id.clone(), p.command.as_str()))
                .collect(),
        }
    }

    fn cursor(&self, level: Level) -> &Option<String> {
        match level {
            Level::Session => &self.session,
            Level::Window => &self.window,
            Level::Pane => &self.pane,
        }
    }

    fn select(&mut self, level: Level, id: Option<String>) {
        if self.cursor(level) == &id {
            return;
        }
        match level {
            Level::Session => {
                self.session = id;
                self.reset_window();
            }
            Level::Window => {
                self.window = id;
                self.reset_pane();
            }
            Level::Pane => self.pane = id,
        }
    }

    /// Refresh snapshots and preview, retaining ids or clamping vanished cursors.
    pub fn refresh(&mut self, tmux: &dyn Tmux) -> Result<bool> {
        let before = self.clone();
        let levels = [Level::Session, Level::Window, Level::Pane];
        let positions = levels.map(|level| {
            self.ids(level)
                .iter()
                .position(|id| Some(id) == self.cursor(level).as_ref())
                .unwrap_or(0)
        });
        self.tree = tmux.tree()?;
        self.sort_sessions();
        for (level, position) in levels.into_iter().zip(positions) {
            let ids = self.ids(level);
            if !ids.iter().any(|id| Some(id) == self.cursor(level).as_ref()) {
                let id = ids.get(position.min(ids.len().saturating_sub(1))).cloned();
                self.select(level, id);
            }
        }
        self.update_preview(tmux)?;
        Ok(*self != before)
    }

    fn update_preview(&mut self, tmux: &dyn Tmux) -> Result<()> {
        let pane_id = self.selected_pane().map(|p| p.id.clone());
        if self.preview.pane_id != pane_id {
            self.preview.scroll = 0;
            self.preview.content.clear();
            self.preview.pane_id = pane_id.clone();
        }
        self.preview.content = match pane_id {
            Some(_) if self.preview_suppressed() => String::new(),
            Some(id) => tmux.capture(&id)?,
            None => String::new(),
        };
        self.clamp_scroll();
        Ok(())
    }

    fn clamp_scroll(&mut self) {
        let max = self
            .preview
            .content
            .lines()
            .count()
            .saturating_sub(usize::from(self.preview_height))
            .min(usize::from(u16::MAX)) as u16;
        self.preview.scroll = self.preview.scroll.min(max);
    }

    /// Resolve the active modal layer, swallowing invalid which-key continuations.
    pub fn handle_key(&mut self, key: Key, keymap: &Keymap, tmux: &dyn Tmux) -> Option<Exit> {
        if let Some(which) = &mut self.which {
            let actions = which.advance(key);
            if which.candidates.is_empty() {
                self.which = None;
            }
            return self.dispatch(actions.unwrap_or_default(), keymap, tmux);
        }
        let chords = match &self.popup {
            Some(Popup::Help { .. }) => &keymap.help,
            Some(Popup::Confirm { .. }) => &keymap.confirm,
            Some(Popup::Input { .. }) => &keymap.input,
            None => &keymap.picker,
        };
        // A full single-key binding wins over a multi-key prefix, as in yazi.
        if let Some(chord) = chords.iter().find(|c| c.on.as_slice() == [key]) {
            return self.dispatch(chord.run.clone(), keymap, tmux);
        }
        self.which = Which::start(key, chords);
        if self.which.is_none()
            && !key.ctrl
            && !key.alt
            && let KeyCode::Char(c) = key.code
            && !c.is_control()
            && let Some(Popup::Input { value, .. }) = &mut self.popup
        {
            value.push(c);
            self.input_changed(tmux);
        }
        None
    }

    fn dispatch(&mut self, actions: Vec<Action>, keymap: &Keymap, tmux: &dyn Tmux) -> Option<Exit> {
        for action in actions {
            if let Some(exit) = self.apply(&action, tmux) {
                return Some(exit);
            }
        }
        if let Some(Popup::Help { chords, .. }) = &mut self.popup {
            *chords = keymap.picker.clone();
        }
        None
    }

    /// Apply one action. Operation errors become messages rather than exiting.
    pub fn apply(&mut self, action: &Action, tmux: &dyn Tmux) -> Option<Exit> {
        match action {
            Action::Quit => return Some(Exit::Cancel),
            Action::Pick => {
                return self
                    .hovered()
                    .map(|target| Exit::Pick(target.at(self.options.level.unwrap_or(self.focus))));
            }
            Action::Escape => {
                if self.which.take().is_none()
                    && self.popup.take().is_none()
                    && self.finder.take().is_none()
                {
                    self.message = None;
                }
            }
            Action::Arrow(step) => {
                if let Some(Popup::Help { cursor, chords }) = &mut self.popup {
                    *cursor = moved(*cursor, chords.len(), *step, self.list_height);
                } else if self.popup.is_none() {
                    let ids = self.ids(self.focus);
                    let index = ids
                        .iter()
                        .position(|id| Some(id) == self.cursor(self.focus).as_ref())
                        .unwrap_or(0);
                    let next = moved(index, ids.len(), *step, self.list_height);
                    self.select(self.focus, ids.get(next).cloned());
                    self.preview_or_message(tmux);
                }
            }
            Action::Focus(direction) if self.popup.is_none() => {
                if *direction == Direction::Next {
                    match self.focus {
                        Level::Session => self.reset_window(),
                        Level::Window => self.reset_pane(),
                        Level::Pane => {}
                    }
                }
                self.focus = match (self.focus, direction) {
                    (Level::Session, Direction::Next) | (Level::Pane, Direction::Prev) => {
                        Level::Window
                    }
                    (Level::Window, Direction::Next) => Level::Pane,
                    (Level::Window, Direction::Prev) => Level::Session,
                    (level, _) => level,
                };
                self.preview_or_message(tmux);
            }
            Action::Seek(percent) => {
                let delta = i32::from(self.preview_height) * i32::from(*percent) / 100;
                self.preview.scroll =
                    (i32::from(self.preview.scroll) + delta).clamp(0, i32::from(u16::MAX)) as u16;
                self.clamp_scroll();
            }
            Action::Find { previous, case } => {
                self.popup = Some(Popup::Input {
                    title: if *previous {
                        "Find previous:"
                    } else {
                        "Find next:"
                    }
                    .into(),
                    value: String::new(),
                    purpose: InputPurpose::Find {
                        previous: *previous,
                        case: *case,
                    },
                });
            }
            Action::FindArrow { previous } => {
                if let Some(finder) = &self.finder {
                    self.find_jump(finder.clone(), *previous, false, tmux);
                }
            }
            Action::Create => self.open_input(false),
            Action::Rename => self.open_input(true),
            Action::Remove => self.open_confirm(),
            Action::Help => {
                self.popup = Some(Popup::Help {
                    cursor: 0,
                    chords: Vec::new(),
                })
            }
            Action::Close { submit } => {
                if let Some(popup) = self.popup.take()
                    && *submit
                    && let Err(error) = self.submit(popup, tmux)
                {
                    self.message = Some(error.to_string());
                }
            }
            Action::Backspace | Action::KillWord | Action::KillBol => {
                if let Some(Popup::Input { value, .. }) = &mut self.popup {
                    match action {
                        Action::Backspace => {
                            value.pop();
                        }
                        Action::KillBol => value.clear(),
                        Action::KillWord => {
                            while value.ends_with(char::is_whitespace) {
                                value.pop();
                            }
                            while !value.is_empty() && !value.ends_with(char::is_whitespace) {
                                value.pop();
                            }
                        }
                        _ => {}
                    }
                    self.input_changed(tmux);
                }
            }
            Action::Focus(_) => {}
        }
        None
    }

    fn input_changed(&mut self, tmux: &dyn Tmux) {
        if let Some(Popup::Input {
            value,
            purpose: InputPurpose::Find { previous, case },
            ..
        }) = &self.popup
        {
            self.find_do(value.clone(), *previous, *case, tmux);
        }
    }

    fn find_do(&mut self, query: String, previous: bool, case: Case, tmux: &dyn Tmux) {
        if query.is_empty() {
            self.finder = None;
            return;
        }
        if self
            .finder
            .as_ref()
            .is_some_and(|f| f.query == query && f.case == case)
        {
            return;
        }
        let Ok(finder) = Finder::new(query, case) else {
            return;
        };
        self.find_jump(finder.clone(), previous, true, tmux);
        self.finder = Some(finder);
    }

    fn find_jump(&mut self, finder: Finder, previous: bool, include: bool, tmux: &dyn Tmux) {
        let names = self.names(self.focus);
        let cursor = names
            .iter()
            .position(|(id, _)| Some(id.as_str()) == self.hovered_id())
            .unwrap_or(0);
        let id = if previous {
            finder.prev(&names, cursor, include)
        } else {
            finder.next(&names, cursor, include)
        };
        if let Some(id) = id {
            self.select(self.focus, Some(id));
            self.preview_or_message(tmux);
        }
    }

    fn preview_or_message(&mut self, tmux: &dyn Tmux) {
        if let Err(error) = self.update_preview(tmux) {
            self.message = Some(error.to_string());
        }
    }

    fn open_input(&mut self, rename: bool) {
        let (purpose, value) = match (self.focus, rename) {
            (Level::Session, false) => (InputPurpose::CreateSession, String::new()),
            (Level::Window, false) => {
                let Some(session) = self.hovered_session() else {
                    return;
                };
                (
                    InputPurpose::CreateWindow {
                        session_id: session.id.clone(),
                    },
                    String::new(),
                )
            }
            (Level::Session, true) => {
                let Some(session) = self.hovered_session() else {
                    return;
                };
                (
                    InputPurpose::RenameSession {
                        id: session.id.clone(),
                    },
                    session.name.clone(),
                )
            }
            (Level::Window, true) => {
                let Some(window) = self.hovered_window() else {
                    return;
                };
                (
                    InputPurpose::RenameWindow {
                        id: window.id.clone(),
                    },
                    window.name.clone(),
                )
            }
            (Level::Pane, _) => return,
        };
        self.popup = Some(Popup::Input {
            title: if rename { "Rename" } else { "Create" }.into(),
            value,
            purpose,
        });
    }

    fn open_confirm(&mut self) {
        let Some(id) = self.hovered_id().map(String::from) else {
            return;
        };
        if self.is_current(&id) {
            self.message = Some("Cannot delete the current item".into());
            return;
        }
        self.popup = Some(Popup::Confirm {
            title: "Delete?".into(),
            body: format!(
                "Delete {}?",
                self.hovered()
                    .map(|t| t.at(self.focus))
                    .unwrap_or_else(|| id.clone())
            ),
            level: self.focus,
            id,
        });
    }

    fn submit(&mut self, popup: Popup, tmux: &dyn Tmux) -> Result<()> {
        let mut created = None;
        match popup {
            Popup::Input { value, purpose, .. } => {
                let name = if value.is_empty() {
                    None
                } else {
                    Some(value.as_str())
                };
                match purpose {
                    InputPurpose::Find { .. } => return Ok(()),
                    InputPurpose::CreateSession => {
                        created = Some((Level::Session, tmux.new_session(name)?))
                    }
                    InputPurpose::CreateWindow { session_id } => {
                        created = Some((Level::Window, tmux.new_window(&session_id, name)?))
                    }
                    InputPurpose::RenameSession { id } => tmux.rename_session(&id, &value)?,
                    InputPurpose::RenameWindow { id } => tmux.rename_window(&id, &value)?,
                }
            }
            Popup::Confirm { level, id, .. } => {
                // Re-check on submission in case current ids were updated while prompting.
                anyhow::ensure!(!self.is_current(&id), "Cannot delete the current item");
                match level {
                    Level::Session => tmux.kill_session(&id)?,
                    Level::Window => tmux.kill_window(&id)?,
                    Level::Pane => tmux.kill_pane(&id)?,
                }
            }
            Popup::Help { .. } => return Ok(()),
        }
        self.refresh(tmux)?;
        if let Some((level, id)) = created {
            self.select(level, Some(id));
            self.update_preview(tmux)?;
        }
        Ok(())
    }
}

// Saturating arithmetic keeps navigation safe at empty lists and numeric extremes.
fn moved(index: usize, len: usize, step: Step, height: u16) -> usize {
    let delta = match step {
        Step::Top => return 0,
        Step::Bot => return len.saturating_sub(1),
        Step::Next => 1,
        Step::Prev => -1,
        Step::Offset(n) => n,
        Step::Percent(n) => (i32::from(n) * i32::from(height) / 100) as isize,
    };
    index
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

#[cfg(test)]
mod tests;
