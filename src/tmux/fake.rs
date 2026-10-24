//! In-memory tmux implementation for picker and rendering tests.

use std::{cell::RefCell, collections::HashMap};

use anyhow::{Result, anyhow, bail, ensure};

use super::{Current, Pane, Session, Tmux, Tree, Window};

/// Mutable fake server with a call log and one-shot error injection.
#[derive(Debug)]
pub struct FakeTmux {
    // RefCell provides interior mutability because Tmux methods take &self.
    tree: RefCell<Tree>,
    current: Option<Current>,
    captures: HashMap<String, String>,
    calls: RefCell<Vec<String>>,
    failure: RefCell<Option<String>>,
    next_id: RefCell<u64>,
}

impl FakeTmux {
    /// Start a fake server from an owned snapshot.
    pub fn new(tree: Tree) -> Self {
        let next_id = tree
            .sessions
            .iter()
            .flat_map(|s| {
                std::iter::once(s.id.as_str()).chain(s.windows.iter().flat_map(|w| {
                    std::iter::once(w.id.as_str()).chain(w.panes.iter().map(|p| p.id.as_str()))
                }))
            })
            .filter_map(|id| id.get(1..)?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        Self {
            tree: RefCell::new(tree),
            current: None,
            captures: HashMap::new(),
            calls: RefCell::new(Vec::new()),
            failure: RefCell::new(None),
            next_id: RefCell::new(next_id),
        }
    }

    /// Set the invoking client's current items.
    pub fn with_current(mut self, current: Current) -> Self {
        self.current = Some(current);
        self
    }

    /// Add captured preview text for a pane.
    pub fn with_capture(mut self, pane_id: &str, text: &str) -> Self {
        self.captures.insert(pane_id.into(), text.into());
        self
    }

    /// Fail the next trait call, recording the attempted call as usual.
    pub fn fail_next(&self, message: &str) {
        *self.failure.borrow_mut() = Some(message.into());
    }

    /// Return a copy of the recorded calls.
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    fn record(&self, call: String) -> Result<()> {
        self.calls.borrow_mut().push(call);
        if let Some(message) = self.failure.borrow_mut().take() {
            bail!(message);
        }
        Ok(())
    }

    fn fresh_id(&self, prefix: &str) -> String {
        let mut next = self.next_id.borrow_mut();
        let id = format!("{prefix}{next}");
        *next += 1;
        id
    }

    // tmux moves the whole window when break-pane takes its only pane.
    fn single_pane_window(tree: &Tree, pane_id: &str) -> Option<(String, String)> {
        tree.sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.panes.len() == 1 && w.panes[0].id == pane_id)
            .map(|w| (w.id.clone(), w.name.clone()))
    }

    fn take_pane(tree: &mut Tree, pane_id: &str) -> Result<Pane> {
        for session in &mut tree.sessions {
            for window in &mut session.windows {
                if let Some(index) = window.panes.iter().position(|p| p.id == pane_id) {
                    return Ok(window.panes.remove(index));
                }
            }
        }
        bail!("pane not found: {pane_id}")
    }

    fn prune(tree: &mut Tree) {
        for session in &mut tree.sessions {
            for window in &mut session.windows {
                if !window.panes.iter().any(|p| p.active)
                    && let Some(pane) = window.panes.first_mut()
                {
                    pane.active = true;
                }
            }
            session.windows.retain(|w| !w.panes.is_empty());
            if !session.windows.iter().any(|w| w.active)
                && let Some(window) = session.windows.first_mut()
            {
                window.active = true;
            }
        }
        tree.sessions.retain(|s| !s.windows.is_empty());
    }

    fn window(&self, index: u32, name: Option<&str>, active: bool) -> Window {
        Window {
            id: self.fresh_id("@"),
            index,
            name: name.unwrap_or("shell").into(),
            active,
            zoomed: false,
            panes: vec![Pane {
                id: self.fresh_id("%"),
                index: 0,
                active: true,
                command: "sh".into(),
                floating: false,
            }],
        }
    }
}

impl Tmux for FakeTmux {
    fn tree(&self) -> Result<Tree> {
        self.record("tree".into())?;
        Ok(self.tree.borrow().clone())
    }

    fn capture(&self, pane_id: &str) -> Result<String> {
        self.record(format!("capture-pane {pane_id}"))?;
        Ok(self.captures.get(pane_id).cloned().unwrap_or_default())
    }

    fn current(&self) -> Result<Option<Current>> {
        self.record("current".into())?;
        Ok(self.current.clone())
    }

    fn resolve_pane(&self, target: &str) -> Result<String> {
        self.record(format!("resolve-pane {target}"))?;
        for session in &self.tree.borrow().sessions {
            for window in &session.windows {
                for pane in &window.panes {
                    if pane.id == target
                        || format!("{}:{}.{}", session.name, window.index, pane.index) == target
                    {
                        return Ok(pane.id.clone());
                    }
                }
            }
        }
        bail!("pane not found: {target}")
    }

    fn move_pane_to_new_session(
        &self,
        pane_id: &str,
        session_name: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<(String, String)> {
        self.record(format!(
            "move-pane-to-new-session {pane_id} {} {}",
            session_name.unwrap_or_default(),
            window_name.unwrap_or_default()
        ))?;
        let mut tree = self.tree.borrow_mut();
        if let Some(name) = session_name {
            ensure!(
                !tree.sessions.iter().any(|s| s.name == name),
                "duplicate session: {name}"
            );
        }
        let whole_window = Self::single_pane_window(&tree, pane_id);
        let mut pane = Self::take_pane(&mut tree, pane_id)?;
        pane.index = 0;
        pane.active = true;
        pane.floating = false;
        let session_id = self.fresh_id("$");
        let (window_id, default_name) =
            whole_window.unwrap_or_else(|| (self.fresh_id("@"), pane.command.clone()));
        // Remove the emptied source before inserting a window with its reused id.
        Self::prune(&mut tree);
        tree.sessions.push(Session {
            id: session_id.clone(),
            name: session_name
                .map(String::from)
                .unwrap_or_else(|| session_id[1..].into()),
            last_attached: 0,
            windows: vec![Window {
                id: window_id.clone(),
                index: 0,
                name: window_name.unwrap_or(&default_name).into(),
                active: true,
                zoomed: false,
                panes: vec![pane],
            }],
        });
        Self::prune(&mut tree);
        Ok((session_id, window_id))
    }

    fn move_pane_to_new_window(
        &self,
        pane_id: &str,
        session_id: &str,
        name: Option<&str>,
    ) -> Result<String> {
        self.record(format!(
            "move-pane-to-new-window {pane_id} {session_id} {}",
            name.unwrap_or_default()
        ))?;
        let mut tree = self.tree.borrow_mut();
        ensure!(
            tree.sessions.iter().any(|s| s.id == session_id),
            "session not found: {session_id}"
        );
        let whole_window = Self::single_pane_window(&tree, pane_id);
        let mut pane = Self::take_pane(&mut tree, pane_id)?;
        pane.index = 0;
        pane.active = true;
        pane.floating = false;
        let session = tree
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .unwrap();
        let index = session
            .windows
            .iter()
            .map(|w| w.index)
            .max()
            .map_or(0, |i| i + 1);
        let (id, default_name) =
            whole_window.unwrap_or_else(|| (self.fresh_id("@"), pane.command.clone()));
        session.windows.retain(|w| !w.panes.is_empty());
        session.windows.push(Window {
            id: id.clone(),
            index,
            name: name.unwrap_or(&default_name).into(),
            active: false,
            zoomed: false,
            panes: vec![pane],
        });
        Self::prune(&mut tree);
        Ok(id)
    }

    fn float_pane_in_window(
        &self,
        pane_id: &str,
        window_id: &str,
        floating: &crate::config::settings::FloatingPaneConfig,
    ) -> Result<()> {
        self.record(format!(
            "float-pane-in-window {pane_id} {window_id} {} {} {}",
            floating.width, floating.height, floating.position
        ))?;
        let mut tree = self.tree.borrow_mut();
        let target = tree
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.id == window_id)
            .ok_or_else(|| anyhow!("window not found: {window_id}"))?;
        ensure!(!target.zoomed, "window is zoomed");
        let mut pane = Self::take_pane(&mut tree, pane_id)?;
        // join-pane accepts floating sources and makes them tiled first.
        pane.floating = false;
        let target = tree
            .sessions
            .iter_mut()
            .flat_map(|s| &mut s.windows)
            .find(|w| w.id == window_id)
            .unwrap();
        pane.index = target
            .panes
            .iter()
            .map(|p| p.index)
            .max()
            .map_or(0, |i| i + 1);
        pane.active = false;
        pane.floating = true;
        target.panes.push(pane);
        Self::prune(&mut tree);
        Ok(())
    }

    fn new_session(&self, name: Option<&str>) -> Result<String> {
        self.record(format!("new-session {}", name.unwrap_or_default()))?;
        let mut tree = self.tree.borrow_mut();
        if let Some(name) = name {
            ensure!(
                !tree.sessions.iter().any(|s| s.name == name),
                "duplicate session: {name}"
            );
        }
        let id = self.fresh_id("$");
        let name = name.map(String::from).unwrap_or_else(|| id[1..].into());
        tree.sessions.push(Session {
            id: id.clone(),
            name,
            last_attached: 0,
            windows: vec![self.window(0, None, true)],
        });
        Ok(id)
    }

    fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String> {
        self.record(format!(
            "new-window {session_id} {}",
            name.unwrap_or_default()
        ))?;
        let mut tree = self.tree.borrow_mut();
        let session = tree
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .ok_or_else(|| anyhow!("session not found: {session_id}"))?;
        let index = session
            .windows
            .iter()
            .map(|w| w.index)
            .max()
            .map_or(0, |i| i + 1);
        let window = self.window(index, name, session.windows.is_empty());
        let id = window.id.clone();
        session.windows.push(window);
        Ok(id)
    }

    fn rename_session(&self, session_id: &str, name: &str) -> Result<()> {
        self.record(format!("rename-session {session_id} {name}"))?;
        let mut tree = self.tree.borrow_mut();
        ensure!(
            !tree
                .sessions
                .iter()
                .any(|s| s.name == name && s.id != session_id),
            "duplicate session: {name}"
        );
        let session = tree
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .ok_or_else(|| anyhow!("session not found: {session_id}"))?;
        session.name = name.into();
        Ok(())
    }

    fn rename_window(&self, window_id: &str, name: &str) -> Result<()> {
        self.record(format!("rename-window {window_id} {name}"))?;
        let mut tree = self.tree.borrow_mut();
        let window = tree
            .sessions
            .iter_mut()
            .flat_map(|s| &mut s.windows)
            .find(|w| w.id == window_id)
            .ok_or_else(|| anyhow!("window not found: {window_id}"))?;
        window.name = name.into();
        Ok(())
    }

    fn kill_session(&self, session_id: &str) -> Result<()> {
        self.record(format!("kill-session {session_id}"))?;
        let mut tree = self.tree.borrow_mut();
        ensure!(
            tree.sessions.iter().any(|s| s.id == session_id),
            "session not found: {session_id}"
        );
        tree.sessions.retain(|s| s.id != session_id);
        Ok(())
    }

    fn kill_window(&self, window_id: &str) -> Result<()> {
        self.record(format!("kill-window {window_id}"))?;
        let mut tree = self.tree.borrow_mut();
        ensure!(
            tree.sessions
                .iter()
                .any(|s| s.windows.iter().any(|w| w.id == window_id)),
            "window not found: {window_id}"
        );
        for session in &mut tree.sessions {
            session.windows.retain(|w| w.id != window_id);
            if !session.windows.iter().any(|w| w.active)
                && let Some(window) = session.windows.first_mut()
            {
                window.active = true;
            }
        }
        tree.sessions.retain(|s| !s.windows.is_empty());
        Ok(())
    }

    fn kill_pane(&self, pane_id: &str) -> Result<()> {
        self.record(format!("kill-pane {pane_id}"))?;
        let mut tree = self.tree.borrow_mut();
        ensure!(
            tree.sessions
                .iter()
                .flat_map(|s| &s.windows)
                .any(|w| w.panes.iter().any(|p| p.id == pane_id)),
            "pane not found: {pane_id}"
        );
        for session in &mut tree.sessions {
            for window in &mut session.windows {
                window.panes.retain(|p| p.id != pane_id);
                if !window.panes.iter().any(|p| p.active)
                    && let Some(pane) = window.panes.first_mut()
                {
                    pane.active = true;
                }
            }
            session.windows.retain(|w| !w.panes.is_empty());
            if !session.windows.iter().any(|w| w.active)
                && let Some(window) = session.windows.first_mut()
            {
                window.active = true;
            }
        }
        tree.sessions.retain(|s| !s.windows.is_empty());
        Ok(())
    }

    fn switch_client(&self, target: &str) -> Result<()> {
        self.record(format!("switch-client {target}"))
    }
}

/// Build a concise fixture: each name gets one active shell window and pane.
pub fn sample_tree(names: &[&str]) -> Tree {
    Tree {
        sessions: names
            .iter()
            .enumerate()
            .map(|(index, name)| Session {
                id: format!("${index}"),
                name: (*name).into(),
                last_attached: 0,
                windows: vec![Window {
                    id: format!("@{index}"),
                    index: 0,
                    name: "shell".into(),
                    active: true,
                    zoomed: false,
                    panes: vec![Pane {
                        id: format!("%{index}"),
                        index: 0,
                        active: true,
                        command: "sh".into(),
                        floating: false,
                    }],
                }],
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floating_config() -> crate::config::settings::FloatingPaneConfig {
        crate::config::settings::FloatingPaneConfig {
            width: 50,
            height: 50,
            position: crate::config::settings::FloatingPosition::Centre,
        }
    }

    #[test]
    fn pane_moves_preserve_ids_and_close_empty_containers() {
        let fake = FakeTmux::new(sample_tree(&["work", "notes"]));
        assert_eq!(fake.resolve_pane("work:0.0").unwrap(), "%0");
        assert!(fake.resolve_pane("missing:0.0").is_err());
        let (session, window) = fake
            .move_pane_to_new_session("%0", Some("logs"), Some("tail"))
            .unwrap();
        let tree = fake.tree().unwrap();
        assert!(!tree.sessions.iter().any(|s| s.id == "$0"));
        let created = tree.sessions.iter().find(|s| s.id == session).unwrap();
        assert_eq!(created.windows.len(), 1);
        assert_eq!(created.windows[0].id, window);
        assert_eq!(created.windows[0].name, "tail");
        assert_eq!(created.windows[0].panes[0].id, "%0");
        let window = fake
            .move_pane_to_new_window("%0", "$1", Some("scratch"))
            .unwrap();
        assert_eq!(fake.tree().unwrap().sessions.len(), 1);
        fake.float_pane_in_window("%0", "@1", &floating_config())
            .unwrap();
        let tree = fake.tree().unwrap();
        assert!(!tree.sessions[0].windows.iter().any(|w| w.id == window));
        let pane = tree.sessions[0].windows[0]
            .panes
            .iter()
            .find(|p| p.id == "%0")
            .unwrap();
        assert!(pane.floating);
    }

    #[test]
    fn moving_within_own_session_keeps_it_alive() {
        let fake = FakeTmux::new(sample_tree(&["work"]));
        let window = fake.move_pane_to_new_window("%0", "$0", None).unwrap();
        let tree = fake.tree().unwrap();
        assert_eq!(tree.sessions[0].id, "$0");
        assert_eq!(tree.sessions[0].windows.len(), 1);
        assert_eq!(tree.sessions[0].windows[0].id, window);
        assert!(tree.sessions[0].windows[0].active);
    }

    #[test]
    fn failed_moves_leave_tree_unchanged() {
        let original = sample_tree(&["work", "notes"]);
        let fake = FakeTmux::new(original.clone());
        assert!(
            fake.move_pane_to_new_session("%0", Some("notes"), None)
                .is_err()
        );
        assert!(
            fake.move_pane_to_new_session("%missing", Some("new"), None)
                .is_err()
        );
        assert!(
            fake.move_pane_to_new_window("%0", "$missing", None)
                .is_err()
        );
        fake.fail_next("floating unsupported");
        assert_eq!(
            fake.float_pane_in_window("%0", "@1", &floating_config())
                .unwrap_err()
                .to_string(),
            "floating unsupported"
        );
        assert_eq!(fake.tree().unwrap(), original);
        fake.fail_next("move failed");
        assert!(fake.move_pane_to_new_window("%0", "$1", None).is_err());
        assert_eq!(fake.tree().unwrap(), original);
        fake.fail_next("move failed");
        assert!(
            fake.move_pane_to_new_session("%0", Some("logs"), None)
                .is_err()
        );
        assert_eq!(fake.tree().unwrap(), original);
    }

    #[test]
    fn mutations_change_tree_and_record_calls() {
        let fake = FakeTmux::new(sample_tree(&["work"]));
        let session = fake.new_session(Some("notes")).unwrap();
        let window = fake.new_window(&session, Some("my logs")).unwrap();
        fake.rename_session(&session, "writing").unwrap();
        fake.rename_window(&window, "logs").unwrap();
        let tree = fake.tree().unwrap();
        assert_eq!(tree.sessions[1].name, "writing");
        assert_eq!(tree.sessions[1].windows[1].name, "logs");
        fake.kill_window(&window).unwrap();
        let pane = tree.sessions[1].windows[0].panes[0].id.clone();
        fake.kill_pane(&pane).unwrap();
        assert_eq!(fake.tree().unwrap().sessions.len(), 1);
        fake.kill_session("$0").unwrap();
        assert!(fake.tree().unwrap().sessions.is_empty());
        assert!(fake.calls().contains(&format!("kill-window {window}")));
        assert_eq!(fake.calls()[0], "new-session notes");
    }

    #[test]
    fn injected_failure_is_one_shot_and_does_not_mutate() {
        let fake = FakeTmux::new(sample_tree(&["work"]));
        fake.fail_next("duplicate session: work");
        assert_eq!(
            fake.new_session(Some("work")).unwrap_err().to_string(),
            "duplicate session: work"
        );
        assert_eq!(fake.tree().unwrap().sessions.len(), 1);
        assert_eq!(fake.calls().len(), 2);
    }
}
