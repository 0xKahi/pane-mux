//! In-memory tmux implementation for picker and rendering tests.

use std::{cell::RefCell, collections::HashMap};

use anyhow::{Context, Result, anyhow, bail, ensure};

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
    failure_match: RefCell<Option<String>>,
    next_id: RefCell<u64>,
    details: RefCell<HashMap<String, super::model::FloatPane>>,
    sizes: RefCell<HashMap<String, (i32, i32)>>,
    options: RefCell<HashMap<(String, String, String), String>>,
    last_tiled: RefCell<HashMap<String, String>>,
    above_zoom: RefCell<HashMap<String, bool>>,
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
            failure_match: RefCell::new(None),
            next_id: RefCell::new(next_id),
            details: RefCell::new(HashMap::new()),
            sizes: RefCell::new(HashMap::new()),
            options: RefCell::new(HashMap::new()),
            last_tiled: RefCell::new(HashMap::new()),
            above_zoom: RefCell::new(HashMap::new()),
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

    /// Delay the next injected failure until a matching call (for multi-step commands).
    pub fn fail_next_matching(&self, pattern: &str, message: &str) {
        self.fail_next(message);
        *self.failure_match.borrow_mut() = Some(pattern.into());
    }

    /// Return a copy of the recorded calls.
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    fn record(&self, call: String) -> Result<()> {
        let matches = self
            .failure_match
            .borrow()
            .as_ref()
            .is_none_or(|pattern| call.contains(pattern));
        self.calls.borrow_mut().push(call);
        if matches && let Some(message) = self.failure.borrow_mut().take() {
            self.failure_match.borrow_mut().take();
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
    fn float_window(&self, window: &str) -> Result<super::model::FloatWindow> {
        self.record(format!("float-window {window}"))?;
        let tree = self.tree.borrow();
        let w = tree
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.id == window)
            .ok_or_else(|| anyhow!("window not found: {window}"))?;
        let (width, height) = self
            .sizes
            .borrow()
            .get(window)
            .copied()
            .unwrap_or((200, 50));
        let mut snapshot = super::model::FloatWindow {
            width,
            height,
            zoomed: w.zoomed,
            panes: w
                .panes
                .iter()
                .map(|p| {
                    let mut d = self
                        .details
                        .borrow()
                        .get(&p.id)
                        .cloned()
                        .unwrap_or_else(|| super::model::FloatPane {
                            width: 60,
                            height: 15,
                            left: 1,
                            top: 1,
                            ..Default::default()
                        });
                    d.id = p.id.clone();
                    d.active = p.active;
                    let above = self
                        .above_zoom
                        .borrow()
                        .get(&p.id)
                        .copied()
                        .unwrap_or(false);
                    d.floating = p.floating && (!w.zoomed || above);
                    d.flags = format!(
                        "{}{}{}",
                        if d.floating { "F" } else { "" },
                        if above { "A" } else { "" },
                        if p.active { "*" } else { "" }
                    );
                    if p.floating && w.zoomed && !above {
                        d.z = 999;
                    }
                    d
                })
                .collect(),
        };
        if snapshot.zoomed {
            super::format::apply_layout_floats(&mut snapshot, &self.window_layout(window)?)?;
        }
        Ok(snapshot)
    }

    fn window_layout(&self, window: &str) -> Result<String> {
        self.record(format!("window-layout {window}"))?;
        let tree = self.tree.borrow();
        let w = tree
            .sessions
            .iter()
            .flat_map(|s| &s.windows)
            .find(|w| w.id == window)
            .context("window not found")?;
        let details = self.details.borrow();
        let leaves: Vec<_> = w
            .panes
            .iter()
            .map(|p| {
                let d = details
                    .get(&p.id)
                    .cloned()
                    .unwrap_or_else(|| super::model::FloatPane {
                        width: 60,
                        height: 15,
                        left: 1,
                        top: 1,
                        ..Default::default()
                    });
                let z = if p.floating {
                    format!(",\"z\":{}", d.z)
                } else {
                    String::new()
                };
                format!(
                    "{{\"t\":\"p\",\"w\":{},\"h\":{},\"x\":{},\"y\":{}{},\"I\":\"{}\"}}",
                    d.width, d.height, d.left, d.top, z, p.id
                )
            })
            .collect();
        Ok(format!(
            "{{\"V\":2,\"L\":{{\"t\":\"v\",\"c\":[{}]}}}}",
            leaves.join(",")
        ))
    }

    fn new_dummy_float(
        &self,
        window: &str,
        width: &str,
        height: &str,
        x: Option<i32>,
        y: Option<i32>,
    ) -> Result<String> {
        self.record(format!("new-pane -d -A -E -T pane-mux-dummy -t {window} -x {width} -y {height}{}{} -P -F #{{pane_id}}",
            x.map(|v| format!(" -X {v}")).unwrap_or_default(), y.map(|v| format!(" -Y {v}")).unwrap_or_default()))?;
        let mut tree = self.tree.borrow_mut();
        let w = tree
            .sessions
            .iter_mut()
            .flat_map(|s| &mut s.windows)
            .find(|w| w.id == window)
            .context("window not found")?;
        let (ww, wh) = self
            .sizes
            .borrow()
            .get(window)
            .copied()
            .unwrap_or((200, 50));
        let size = |value: &str, total: i32| -> Result<i32> {
            Ok(if let Some(percent) = value.strip_suffix('%') {
                percent.parse::<i32>()? * total / 100
            } else {
                value.parse()?
            })
        };
        let width = size(width, ww)?;
        let height = size(height, wh)?;
        let x = x.unwrap_or((ww - width) / 2);
        let y = y.unwrap_or((wh - height) / 2);
        ensure!(x < ww && y < wh, "position too large");
        let id = self.fresh_id("%");
        let mut details = self.details.borrow_mut();
        for p in &w.panes {
            if p.floating {
                details.entry(p.id.clone()).or_default().z += 1;
            }
        }
        details.insert(
            id.clone(),
            super::model::FloatPane {
                id: id.clone(),
                title: "pane-mux-dummy".into(),
                width: width - 2,
                height: height - 2,
                left: x + 1,
                top: y + 1,
                ..Default::default()
            },
        );
        self.above_zoom.borrow_mut().insert(id.clone(), true);
        w.panes.push(Pane {
            id: id.clone(),
            index: w.panes.iter().map(|p| p.index).max().unwrap_or(0) + 1,
            floating: true,
            ..Default::default()
        });
        Ok(id)
    }

    fn swap_and_kill(&self, pane: &str, dummy: &str) -> Result<()> {
        self.record(format!(
            "swap-pane -d -Z -s {pane} -t {dummy} ; kill-pane -t {dummy}"
        ))?;
        {
            let mut tree = self.tree.borrow_mut();
            let locate = |id: &str| -> Result<(usize, usize, usize)> {
                tree.sessions
                    .iter()
                    .enumerate()
                    .find_map(|(si, s)| {
                        s.windows.iter().enumerate().find_map(|(wi, w)| {
                            w.panes
                                .iter()
                                .position(|p| p.id == id)
                                .map(|pi| (si, wi, pi))
                        })
                    })
                    .context("pane not found")
            };
            let (si, wi, pi) = locate(pane)?;
            let (ds, dw, dp) = locate(dummy)?;
            let a = tree.sessions[si].windows[wi].panes[pi].clone();
            let b = tree.sessions[ds].windows[dw].panes[dp].clone();
            let mut moved = a.clone();
            moved.index = b.index;
            moved.active = b.active;
            moved.floating = b.floating;
            let mut replaced = b;
            replaced.index = a.index;
            replaced.active = a.active;
            replaced.floating = a.floating;
            tree.sessions[si].windows[wi].panes[pi] = replaced;
            tree.sessions[ds].windows[dw].panes[dp] = moved;
            let mut details = self.details.borrow_mut();
            let a = details
                .get(pane)
                .cloned()
                .unwrap_or_else(|| super::model::FloatPane {
                    width: 60,
                    height: 15,
                    left: 1,
                    top: 1,
                    ..Default::default()
                });
            let b = details.get(dummy).cloned().unwrap_or_default();
            for (id, own, slot) in [(pane, &a, &b), (dummy, &b, &a)] {
                let mut d = own.clone();
                d.width = slot.width;
                d.height = slot.height;
                d.left = slot.left;
                d.top = slot.top;
                d.z = slot.z;
                details.insert(id.into(), d);
            }
        }
        self.kill_pane(dummy)
    }

    fn set_option(&self, scope: &str, target: &str, name: &str, value: &str) -> Result<()> {
        self.record(format!("set-option {scope} {target} {name} {value}"))?;
        if scope == "-p" {
            self.details
                .borrow_mut()
                .entry(target.into())
                .or_default()
                .options
                .insert(name.into(), value.into());
        } else {
            self.options
                .borrow_mut()
                .insert((scope.into(), target.into(), name.into()), value.into());
        }
        Ok(())
    }

    fn option(&self, scope: &str, target: &str, name: &str) -> Result<String> {
        self.record(format!("show-options {scope} {target} {name}"))?;
        if scope == "-p" {
            return Ok(self
                .details
                .borrow()
                .get(target)
                .and_then(|p| p.options.get(name))
                .cloned()
                .unwrap_or_default());
        }
        Ok(self
            .options
            .borrow()
            .get(&(scope.into(), target.into(), name.into()))
            .cloned()
            .unwrap_or_default())
    }

    fn resize_window(&self, window: &str, width: i32, height: i32) -> Result<()> {
        self.record(format!("resize-window -t {window} -x {width} -y {height}"))?;
        self.sizes
            .borrow_mut()
            .insert(window.into(), (width, height));
        Ok(())
    }

    fn toggle_zoom(&self, pane: &str) -> Result<()> {
        self.record(format!("resize-pane -Z -t {pane}"))?;
        let mut tree = self.tree.borrow_mut();
        let w = tree
            .sessions
            .iter_mut()
            .flat_map(|s| &mut s.windows)
            .find(|w| w.panes.iter().any(|p| p.id == pane))
            .context("pane not found")?;
        w.zoomed = !w.zoomed;
        Ok(())
    }

    fn select_pane(&self, pane: &str) -> Result<()> {
        self.record(format!("select-pane -t {pane}"))?;
        let mut tree = self.tree.borrow_mut();
        let w = tree
            .sessions
            .iter_mut()
            .flat_map(|s| &mut s.windows)
            .find(|w| w.panes.iter().any(|p| p.id == pane))
            .context("pane not found")?;
        if let Some(p) = w.panes.iter().find(|p| p.active && !p.floating) {
            self.last_tiled
                .borrow_mut()
                .insert(w.id.clone(), p.id.clone());
        }
        for p in &mut w.panes {
            p.active = p.id == pane;
        }
        Ok(())
    }

    fn display_message(&self, target: &str, message: &str) -> Result<()> {
        self.record(format!("display-message -t {target} {message}"))
    }

    fn stash_window(
        &self,
        session: Option<&str>,
        name: &str,
        command: &str,
        width: i32,
        height: i32,
    ) -> Result<String> {
        self.record(format!(
            "stash-window {} {name} {command} {width} {height}",
            session.unwrap_or("pmux")
        ))?;
        let creating_session = session.is_none();
        let session = match session {
            Some(s) => s.to_owned(),
            None => self.new_session(Some("pmux"))?,
        };
        let window = if creating_session {
            let id = self
                .tree
                .borrow()
                .sessions
                .iter()
                .find(|s| s.id == session)
                .unwrap()
                .windows[0]
                .id
                .clone();
            self.rename_window(&id, name)?;
            id
        } else {
            self.new_window(&session, Some(name))?
        };
        self.resize_window(&window, width, height)?;
        Ok(window)
    }

    fn socket_path(&self) -> Result<String> {
        self.record("socket-path".into())?;
        Ok(format!(
            "{}/pane-mux-fake-{}",
            std::env::temp_dir().display(),
            std::process::id()
        ))
    }
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
                        || (pane.active
                            && (target == window.id
                                || target == format!("{}:{}", session.name, window.index)
                                || target == format!("{}:{}", session.name, window.name)))
                        || (pane.active
                            && window.active
                            && (target == session.id || target == session.name))
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
        ensure!(!self.float_window(window_id)?.zoomed, "window is zoomed");
        let dummy = self.new_dummy_float(
            window_id,
            &format!("{}%", floating.width),
            &format!("{}%", floating.height),
            None,
            None,
        )?;
        self.record(format!("move-pane -P {} -t {dummy}", floating.position))?;
        let (ww, wh) = self
            .sizes
            .borrow()
            .get(window_id)
            .copied()
            .unwrap_or((200, 50));
        {
            let mut details = self.details.borrow_mut();
            let d = details.get_mut(&dummy).unwrap();
            let position = floating.position.to_string();
            d.left = if position.contains("left") {
                1
            } else if position.contains("right") {
                ww - d.width - 1
            } else {
                (ww - d.width) / 2
            };
            d.top = if position.contains("top") {
                1
            } else if position.contains("bottom") {
                wh - d.height - 1
            } else {
                (wh - d.height) / 2
            };
        }
        self.swap_and_kill(pane_id, &dummy)
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
                let removed_z = window
                    .panes
                    .iter()
                    .find(|p| p.id == pane_id && p.floating)
                    .map(|p| self.details.borrow().get(&p.id).map_or(0, |d| d.z));
                window.panes.retain(|p| p.id != pane_id);
                if let Some(z) = removed_z {
                    let mut details = self.details.borrow_mut();
                    for p in &window.panes {
                        if p.floating {
                            let d = details.entry(p.id.clone()).or_default();
                            if d.z > z {
                                d.z -= 1;
                            }
                        }
                    }
                }
                if !window.panes.iter().any(|p| p.active) {
                    let last = self.last_tiled.borrow().get(&window.id).cloned();
                    let next = window
                        .panes
                        .iter()
                        .position(|p| Some(&p.id) == last.as_ref() && !p.floating)
                        .or_else(|| window.panes.iter().position(|p| !p.floating))
                        .or_else(|| (!window.panes.is_empty()).then_some(0));
                    if let Some(i) = next {
                        window.panes[i].active = true;
                    }
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
    fn cross_window_swap_keeps_identity_options_and_destination_slot() {
        let fake = FakeTmux::new(sample_tree(&["work", "stash"]));
        fake.set_option("-p", "%0", "@pmux_test", "kept").unwrap();
        let back = fake
            .new_dummy_float("@1", "42", "12", Some(3), Some(4))
            .unwrap();
        let dummy = fake
            .new_dummy_float("@1", "62", "17", Some(119), Some(4))
            .unwrap();
        fake.select_pane(&dummy).unwrap();
        fake.swap_and_kill("%0", &dummy).unwrap();
        let tree = fake.tree().unwrap();
        assert_eq!(tree.sessions.len(), 1); // Empty source window and session closed.
        let window = fake.float_window("@1").unwrap();
        let moved = window.panes.iter().find(|p| p.id == "%0").unwrap();
        assert!(moved.floating && moved.active);
        assert_eq!(
            (moved.width, moved.height, moved.left, moved.top, moved.z),
            (60, 15, 120, 5, 0)
        );
        assert_eq!(moved.options["@pmux_test"], "kept");
        assert!(!moved.flags.contains('A')); // Attribute belongs to identity, not slot.
        assert_eq!(window.panes.iter().find(|p| p.id == back).unwrap().z, 1);
        assert!(!window.panes.iter().any(|p| p.id == dummy));
        assert!(fake.calls().contains(&format!(
            "swap-pane -d -Z -s %0 -t {dummy} ; kill-pane -t {dummy}"
        )));
    }

    #[test]
    fn swap_keeps_zoom_and_source_last_tiled_focus() {
        let fake = FakeTmux::new(sample_tree(&["work", "stash"]));
        let source = fake
            .new_dummy_float("@0", "62", "17", Some(10), Some(5))
            .unwrap();
        fake.above_zoom.borrow_mut().insert(source.clone(), false);
        fake.select_pane(&source).unwrap();
        fake.toggle_zoom("%0").unwrap();
        fake.toggle_zoom("%1").unwrap();
        let dummy = fake
            .new_dummy_float("@1", "62", "17", Some(10), Some(5))
            .unwrap();
        fake.swap_and_kill(&source, &dummy).unwrap();
        let src = fake.float_window("@0").unwrap();
        assert!(src.zoomed && src.panes[0].active);
        let dst = fake.float_window("@1").unwrap();
        assert!(dst.zoomed);
        let moved = dst.panes.iter().find(|p| p.id == source).unwrap();
        assert!(moved.floating && !moved.active);
        assert_eq!(
            (moved.width, moved.height, moved.left, moved.top, moved.z),
            (60, 15, 11, 6, 0)
        );
    }

    #[test]
    fn zoomed_query_recovers_hidden_float_geometry_and_z_from_layout() {
        let fake = FakeTmux::new(sample_tree(&["work"]));
        let hidden = fake
            .new_dummy_float("@0", "62", "17", Some(119), Some(4))
            .unwrap();
        fake.above_zoom.borrow_mut().insert(hidden.clone(), false);
        let visible = fake
            .new_dummy_float("@0", "42", "12", Some(9), Some(7))
            .unwrap();
        fake.toggle_zoom("%0").unwrap();
        let window = fake.float_window("@0").unwrap();
        let p = window.panes.iter().find(|p| p.id == hidden).unwrap();
        assert!(p.floating);
        assert!(!p.flags.contains('F')); // Underlying pane query hid it.
        assert_eq!((p.z, p.width, p.height, p.left, p.top), (1, 60, 15, 120, 5));
        let p = window.panes.iter().find(|p| p.id == visible).unwrap();
        assert!(p.floating && p.flags.contains('A'));
        assert_eq!(p.z, 0);
        assert!(fake.calls().contains(&"window-layout @0".into()));
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
