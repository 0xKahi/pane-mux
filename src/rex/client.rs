use std::{cell::RefCell, io::ErrorKind, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use super::{
    ProcessRunner, Runner, RunnerError, history,
    snapshot::{CAPTURE, SCRIPT, SessionView, Snapshot, UNSCOPED, pane_location},
};
use crate::{
    config::settings::{FloatingPaneConfig, FloatingPosition},
    mux::{Current, Level, Mux, Session, Target, Tree},
};

const MIN_API_REVISION: u32 = 18;
const CAPTURE_BUDGET: u32 = 131_072;

pub struct RexCli {
    runner: Box<dyn Runner>,
    server: Option<String>,
    block: Option<String>,
    app_client: Option<String>,
    history_path: Option<PathBuf>,
    endpoint: RefCell<Option<String>>,
    last_tree: RefCell<Option<Tree>>,
    // Boxed to keep `Backend::Rex` small enough for clippy::large_enum_variant.
    last_current: RefCell<Option<Box<Current>>>,
    self_owner: RefCell<Option<String>>,
}

impl RexCli {
    pub fn new(server: Option<String>, block: Option<String>) -> Self {
        let mut client = Self::with_runner(server, block, ProcessRunner);
        client.app_client = std::env::var("REX_CLIENT")
            .ok()
            .filter(|id| !id.trim().is_empty());
        client.history_path = history::default_path();
        client
    }

    pub fn with_runner(
        server: Option<String>,
        block: Option<String>,
        runner: impl Runner + 'static,
    ) -> Self {
        Self {
            runner: Box::new(runner),
            server,
            block: block.filter(|value| !value.trim().is_empty()),
            app_client: None,
            history_path: None,
            endpoint: RefCell::default(),
            last_tree: RefCell::default(),
            last_current: RefCell::default(),
            self_owner: RefCell::default(),
        }
    }

    pub fn run(&self, args: &[&str], env_remove: &[&str]) -> Result<String> {
        let mut argv = vec!["--autostart=false"];
        if let Some(server) = &self.server {
            argv.extend(["-S", server]);
        }
        argv.extend_from_slice(args);
        match self.runner.run(&argv, env_remove) {
            Ok(output) => Ok(output),
            Err(RunnerError::Spawn(err)) if err.kind() == ErrorKind::NotFound => {
                bail!("`rex` command not found on PATH")
            }
            Err(RunnerError::Spawn(err)) => Err(err).context("could not run `rex`"),
            Err(RunnerError::Exit(stderr)) => {
                let message = stderr.trim();
                if message.starts_with("Error: no Rex server is running")
                    || message.starts_with("Error: connecting to Rex server:")
                {
                    bail!("Rex server unreachable: {message}");
                }
                bail!("{message}")
            }
        }
    }

    pub fn check_api_revision(&self) -> Result<()> {
        #[derive(Deserialize)]
        struct Whoami {
            api_revision: u32,
            endpoint: Option<String>,
        }

        let output = self.run(&["whoami", "--json"], &[])?;
        let Whoami {
            api_revision,
            endpoint,
        } = serde_json::from_str(&output).context("invalid Rex whoami response")?;
        if api_revision < MIN_API_REVISION {
            bail!(
                "Rex too old: pane-mux needs API revision {MIN_API_REVISION} or newer (found {api_revision})"
            );
        }
        self.endpoint.replace(endpoint);
        Ok(())
    }

    pub fn with_history_path(mut self, path: PathBuf) -> Self {
        self.history_path = Some(path);
        self
    }

    pub fn with_app_client(mut self, client: String) -> Self {
        self.app_client = Some(client);
        self
    }

    fn api<T: DeserializeOwned>(
        &self,
        session: Option<&str>,
        method: &str,
        args: Value,
    ) -> Result<T> {
        let arg = args.to_string();
        let mut argv = vec!["api", "call"];
        if let Some(session) = session {
            argv.extend(["-s", session]);
        }
        argv.extend([method, &arg]);
        let output = self.run(&argv, UNSCOPED)?;
        serde_json::from_str(&output).with_context(|| format!("invalid Rex {method} response"))
    }

    fn mutate(&self, session: &str, method: &str, args: Value) -> Result<()> {
        self.api::<Value>(Some(session), method, args).map(|_| ())
    }

    // The picker already fetches a tree before issuing operations. Refresh only
    // on a cache miss so looking up a window or block normally costs no spawn.
    fn lookup<T>(&self, find: impl Fn(&Tree) -> Option<T>) -> Result<Option<T>> {
        if let Some(found) = self.last_tree.borrow().as_ref().and_then(&find) {
            return Ok(Some(found));
        }
        Ok(find(&self.tree()?))
    }

    fn window_session(&self, window: &str) -> Result<String> {
        self.lookup(|tree| {
            tree.sessions
                .iter()
                .find(|s| s.windows.iter().any(|w| w.id == window))
                .map(|s| s.id.clone())
        })?
        .with_context(|| format!("window not found: {window}"))
    }

    fn create_window(&self, session: &str, name: Option<&str>) -> Result<NewWindow> {
        let mut args = shell_window(name);
        args["focus"] = json!(false);
        self.api(Some(session), "session.new_window", args)
    }

    fn close_block(&self, session: &str, pane: &str) -> Result<()> {
        self.mutate(session, "block.close", json!({"block_id":pane}))
    }

    fn close_window(&self, session: &str, window: &str) -> Result<()> {
        self.mutate(session, "session.close_window", json!({"window_id":window}))
    }

    fn move_block(&self, session: &str, pane: &str, anchor: &str) -> Result<()> {
        self.mutate(
            session,
            "session.move_block",
            json!({"block_id":pane, "anchor_block_id":anchor}),
        )
    }

    fn focus_block(&self, session: &str, pane: &str) -> Result<()> {
        self.mutate(session, "session.focus_block", json!({"block_id":pane}))
    }

    fn focus_window(&self, session: &str, window: &str) -> Result<()> {
        self.mutate(session, "session.focus_window", json!({"window_id":window}))
    }

    fn session_view(&self, session: &str) -> Result<Session> {
        let output = self.run(&["session", "inspect", session, "--json"], UNSCOPED)?;
        let view: SessionView =
            serde_json::from_str(&output).context("invalid Rex session inspect response")?;
        // Move cleanup must see every block when testing whether a window is empty.
        Ok(view.into_session(None).0)
    }

    fn finish_move(&self, before: &Session, empty_candidates: &[&str]) -> Result<()> {
        let id = &before.id;
        let after = self.session_view(id)?;
        // Rex usually closes empty windows itself. Never close a window that
        // still has blocks, including a destination of a timed-out move.
        let mut close = Ok(());
        for window in after
            .windows
            .iter()
            .filter(|w| empty_candidates.contains(&w.id.as_str()) && w.panes.is_empty())
        {
            close = combine(close, self.close_window(id, &window.id));
        }
        combine(close, self.restore_focus(before, &after))
    }

    fn restore_focus(&self, before: &Session, after: &Session) -> Result<()> {
        let id = &before.id;
        let Some(previous) = before.windows.iter().find(|w| w.active) else {
            return Ok(());
        };
        let Some(window) = after
            .windows
            .iter()
            .find(|w| w.id == previous.id && !w.panes.is_empty())
        else {
            return Ok(());
        };
        if let Some(pane) = previous.panes.iter().find(|p| p.active)
            && let Some(remaining) = window.panes.iter().find(|p| p.id == pane.id)
            && !remaining.active
        {
            return self.focus_block(id, &pane.id);
        }
        if !window.active {
            self.focus_window(id, &window.id)?;
        }
        Ok(())
    }

    fn app_client_id(&self, left: Option<&str>) -> Result<String> {
        if let Some(id) = &self.app_client {
            return Ok(id.clone());
        }

        let output = self.run(&["client", "ls", "--json"], UNSCOPED)?;
        let clients: Clients =
            serde_json::from_str(&output).context("invalid Rex client ls response")?;
        let mut apps: Vec<_> = clients
            .clients
            .into_iter()
            .filter(|c| {
                c.connection_state == "connected"
                    && c.info.as_ref().is_some_and(|i| i.kind == "app")
            })
            .collect();
        if let Some(owner) = self.self_owner.borrow().as_ref()
            && apps.iter().any(|app| &app.client_id == owner)
        {
            return Ok(owner.clone());
        }

        if let Some(left) = left {
            let attached = |app: &AppCandidate| app.session_ids.iter().any(|id| id == left);
            if apps.iter().any(attached) {
                apps.retain(attached);
            }
        }
        match apps.len() {
            0 => bail!("no connected Rex app client"),
            1 => Ok(apps.swap_remove(0).client_id),
            count => Err(AmbiguousApp(count).into()),
        }
    }

    fn select_app(&self, left: Option<&str>, session: &str, window: Option<&str>) -> Result<()> {
        let id = self.app_client_id(left)?;
        let session_arg = format!("session_id={session}");
        let mut argv = vec!["-C", &id, "do", "session.select", &session_arg];
        let window_arg = window.map(|w| format!("window_id={w}"));
        if let Some(w) = &window_arg {
            argv.push(w);
        }
        argv.push("--json");
        self.run(&argv, UNSCOPED)?;
        Ok(())
    }

    pub(super) fn switch(&self, target: &str) -> Result<Option<String>> {
        let (session, window, pane) = self
            .lookup(|tree| {
                for session in &tree.sessions {
                    if session.id == target {
                        return Some(((session.id.clone(), session.name.clone()), None, None));
                    }
                    for window in &session.windows {
                        if window.id == target {
                            return Some((
                                (session.id.clone(), session.name.clone()),
                                Some(window.id.clone()),
                                None,
                            ));
                        }
                        if window.panes.iter().any(|p| p.id == target) {
                            return Some((
                                (session.id.clone(), session.name.clone()),
                                Some(window.id.clone()),
                                Some(target.to_owned()),
                            ));
                        }
                    }
                }
                None
            })?
            .with_context(|| format!("Rex target not found: {target}"))?;
        let (id, label) = session;
        let left = self.current()?.map(|c| c.session_id);
        if let Some(pane) = pane {
            self.focus_block(&id, &pane)?;
        } else if let Some(window) = &window {
            self.focus_window(&id, window)?;
        }
        let hint = self.select_app(left.as_deref(), &id, window.as_deref()).err().map(|err| {
            if let Some(ambiguous) = err.downcast_ref::<AmbiguousApp>() {
                format!("pane-mux could not tell which Rex app to switch to session {label:?}: {ambiguous}. Set REX_CLIENT to an app's label or client ID to choose one.")
            } else {
                format!("Could not show Rex session {label:?}. Turn on Remote Control in Rex settings to switch sessions from pane-mux.\n{err:#}")
            }
        });

        let endpoint = self.endpoint.borrow();
        if let Some((path, server)) = self
            .history_path
            .as_deref()
            .zip(self.server.as_deref().or(endpoint.as_deref()))
            && let Err(err) = history::record(path, server, left.as_deref(), &id)
        {
            eprintln!("pane-mux: could not save Rex recency: {err:#}");
        }
        Ok(hint)
    }
}

#[derive(Deserialize)]
struct NewWindow {
    window_id: String,
    #[serde(default)]
    block_ids: Vec<String>,
}

#[derive(Debug)]
struct AmbiguousApp(usize);

impl std::fmt::Display for AmbiguousApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} connected apps are candidates", self.0)
    }
}

impl std::error::Error for AmbiguousApp {}

#[derive(Deserialize)]
struct Clients {
    #[serde(deserialize_with = "super::snapshot::list")]
    clients: Vec<AppCandidate>,
}

#[derive(Deserialize)]
struct AppCandidate {
    client_id: String,
    info: Option<ClientInfo>,
    connection_state: String,
    #[serde(default, deserialize_with = "super::snapshot::list")]
    session_ids: Vec<String>,
}

#[derive(Deserialize)]
struct ClientInfo {
    kind: String,
}

fn shell_window(name: Option<&str>) -> Value {
    let mut args = json!({"layout":{"block":{"flavor":"com.superlogical.terminal.shell"}}});
    if let Some(name) = name {
        args["window_label"] = json!(name);
    }
    args
}

pub(super) fn bounds(config: &FloatingPaneConfig) -> [f64; 4] {
    use FloatingPosition::*;
    let (x, y) = match config.position {
        TopLeft => (0., 0.),
        TopCentre => (0.5, 0.),
        TopRight => (1., 0.),
        CentreLeft => (0., 0.5),
        Centre => (0.5, 0.5),
        CentreRight => (1., 0.5),
        BottomLeft => (0., 1.),
        BottomCentre => (0.5, 1.),
        BottomRight => (1., 1.),
        TopLeftCentre => (0.25, 0.),
        TopRightCentre => (0.75, 0.),
        BottomLeftCentre => (0.25, 1.),
        BottomRightCentre => (0.75, 1.),
    };
    let w = f64::from(config.width) / 100.;
    let h = f64::from(config.height) / 100.;
    [x * (1. - w), y * (1. - h), w, h]
}

const CROSS_SESSION: &str = "rex does not allow moving panes to different session";

impl Mux for RexCli {
    fn target_string(&self, _tree: &Tree, target: &Target, level: Level) -> String {
        match level {
            Level::Session => &target.session_id,
            Level::Window => target.window_id.as_deref().unwrap_or_default(),
            Level::Pane => target.pane_id.as_deref().unwrap_or_default(),
        }
        .to_owned()
    }

    fn display_target(&self, _tree: &Tree, target: &Target, level: Level) -> String {
        match level {
            Level::Session => target.session.clone(),
            Level::Window => format!("{}:{}", target.session, target.window_index),
            Level::Pane => format!(
                "{}:{}.{}",
                target.session, target.window_index, target.pane_index
            ),
        }
    }

    fn kind(&self) -> crate::mux::BackendKind {
        crate::mux::BackendKind::Rex
    }

    fn in_client(&self) -> bool {
        self.block.is_some()
    }

    fn suppressed_pane(&self) -> Option<String> {
        self.block.clone()
    }

    fn tree(&self) -> Result<Tree> {
        let output = self.run(&["do", "-e", SCRIPT, "--json"], UNSCOPED)?;
        let snapshot: Snapshot = serde_json::from_str(&output).context("invalid Rex snapshot")?;
        let (mut tree, self_block) = snapshot.into_tree(self.block.as_deref());
        let endpoint = self.endpoint.borrow();
        history::rank(
            &mut tree,
            self.history_path.as_deref(),
            self.server.as_deref().or(endpoint.as_deref()),
        );
        self.last_tree.replace(Some(tree.clone()));
        let (current, owner) = self_block
            .map(|block| (Some(Box::new(block.current)), block.owner))
            .unwrap_or_default();
        self.last_current.replace(current);
        self.self_owner.replace(owner);
        Ok(tree)
    }

    fn capture(&self, pane_id: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Capture {
            rows: u32,
            content: String,
            partial: bool,
        }

        let location = self
            .lookup(|tree| pane_location(tree, pane_id))?
            .with_context(|| format!("pane not found: {pane_id}"))?;
        let args = json!({
            "session_id": location.session_id,
            "block_id": pane_id,
            "budget": CAPTURE_BUDGET,
        })
        .to_string();
        let output = self.run(&["do", "-e", CAPTURE, "--json", "--args", &args], UNSCOPED)?;
        let capture: Capture =
            serde_json::from_str(&output).context("invalid Rex capture response")?;
        Ok(visible_rows(
            &capture.content,
            capture.rows as usize,
            capture.partial,
        ))
    }

    fn preview_interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn cross_session_moves(&self) -> bool {
        false
    }

    fn current(&self) -> Result<Option<Current>> {
        if self.block.is_none() {
            return Ok(None);
        }
        if self.last_tree.borrow().is_none() {
            self.tree()?;
        }
        Ok(self.last_current.borrow().as_deref().cloned())
    }

    fn resolve_pane(&self, target: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Inspect {
            block: Block,
        }
        #[derive(Deserialize)]
        struct Block {
            block_id: String,
        }
        let output = self
            .run(&["block", "inspect", "--json", "--", target], UNSCOPED)
            .map_err(|err| {
                let message = err.to_string();
                if message.starts_with("Error: block target ") && message.contains(" not found") {
                    anyhow::anyhow!("pane not found: {target}")
                } else {
                    err
                }
            })?;
        let inspected: Inspect =
            serde_json::from_str(&output).context("invalid Rex block inspect response")?;
        Ok(inspected.block.block_id)
    }

    fn move_pane_to_new_session(
        &self,
        _pane_id: &str,
        _session_name: Option<&str>,
        _window_name: Option<&str>,
    ) -> Result<(String, String)> {
        bail!(CROSS_SESSION)
    }

    fn move_pane_to_new_window(
        &self,
        pane: &str,
        session: &str,
        name: Option<&str>,
    ) -> Result<String> {
        let source = self
            .lookup(|tree| pane_location(tree, pane))?
            .with_context(|| format!("pane not found: {pane}"))?;
        ensure!(source.session_id == session, CROSS_SESSION);
        let before = self.session_view(session)?;
        let new = self.create_window(session, name)?;
        let Some(placeholder) = new.block_ids.first() else {
            self.close_window(session, &new.window_id)
                .context("could not clean up Rex move window")?;
            bail!("Rex did not create a placeholder block");
        };
        if let Err(err) = self.move_block(session, pane, placeholder) {
            // A failed/timed-out request might still have moved the pane.
            // Close only the placeholder, never a window that may contain it.
            let cleanup = self.close_block(session, placeholder);
            let restore = self.finish_move(&before, &[&source.window_id, &new.window_id]);
            return Err(with_cleanup(with_cleanup(err, cleanup), restore));
        }
        let remove = self.close_block(session, placeholder).or_else(|err| {
            self.close_block(session, placeholder)
                .map_err(|retry| with_cleanup(err, Err(retry)))
        });
        let restore = self.finish_move(&before, &[&source.window_id]);
        combine(remove, restore)?;
        Ok(new.window_id)
    }

    fn float_pane_in_window(
        &self,
        pane: &str,
        window_id: &str,
        floating: &FloatingPaneConfig,
    ) -> Result<()> {
        let source = self
            .lookup(|tree| pane_location(tree, pane))?
            .with_context(|| format!("pane not found: {pane}"))?;
        let session = self.window_session(window_id)?;
        ensure!(source.session_id == session, CROSS_SESSION);
        if source.window_id == window_id {
            return Ok(());
        }
        let before = self.session_view(&session)?;
        let window = before
            .windows
            .iter()
            .find(|w| w.id == window_id)
            .with_context(|| format!("window not found: {window_id}"))?;
        let anchor = window
            .panes
            .iter()
            .find(|p| p.active && !p.floating)
            .or_else(|| window.panes.iter().find(|p| !p.floating))
            .context("Rex target window has no tiled block")?;
        let operation = self.move_block(&session, pane, &anchor.id).and_then(|()| {
            self.mutate(
                &session,
                "session.float_block",
                json!({"block_id":pane, "bounds":bounds(floating)}),
            )
        });
        let restore = self.finish_move(&before, &[&source.window_id]);
        combine(operation, restore)
    }

    fn new_session(&self, name: Option<&str>) -> Result<String> {
        #[derive(Deserialize)]
        struct Created {
            session_id: String,
        }
        let mut args = json!({"initial_windows":[shell_window(None)]});
        if let Some(name) = name {
            args["label"] = json!(name);
        }
        let created: Created = self.api(None, "session.create", args)?;
        Ok(created.session_id)
    }

    fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String> {
        Ok(self.create_window(session_id, name)?.window_id)
    }

    fn rename_session(&self, session_id: &str, name: &str) -> Result<()> {
        self.mutate(session_id, "session.set_label", json!({"label":name}))
    }

    fn rename_window(&self, window_id: &str, name: &str) -> Result<()> {
        self.mutate(
            &self.window_session(window_id)?,
            "session.set_window_label",
            json!({"window_id":window_id,"label":name}),
        )
    }

    fn kill_session(&self, session_id: &str) -> Result<()> {
        self.mutate(session_id, "session.destroy", json!({}))
    }

    fn kill_window(&self, window_id: &str) -> Result<()> {
        self.close_window(&self.window_session(window_id)?, window_id)
    }

    fn kill_pane(&self, pane_id: &str) -> Result<()> {
        let location = self
            .lookup(|tree| pane_location(tree, pane_id))?
            .with_context(|| format!("pane not found: {pane_id}"))?;
        self.close_block(&location.session_id, pane_id)
    }

    fn switch_client(&self, target: &str) -> Result<()> {
        if let Some(hint) = self.switch(target)? {
            eprintln!("{hint}");
        }
        Ok(())
    }
}

fn with_cleanup(err: anyhow::Error, cleanup: Result<()>) -> anyhow::Error {
    match cleanup {
        Ok(()) => err,
        Err(cleanup) => anyhow::anyhow!("{err:#}; Rex move cleanup failed: {cleanup:#}"),
    }
}

fn visible_rows(content: &str, rows: usize, partial: bool) -> String {
    let normalized = content.replace("\r\n", "\n");
    let content = if partial {
        normalized.split_once('\n').map_or("", |(_, tail)| tail)
    } else {
        &normalized
    };
    if rows == 0 {
        return content.to_owned();
    }

    // Don't count a final newline as a row, but retain it in the returned slice.
    let start = content
        .strip_suffix('\n')
        .unwrap_or(content)
        .rmatch_indices('\n')
        .nth(rows - 1)
        .map_or(0, |(i, _)| i + 1);
    content[start..].to_owned()
}

fn combine(operation: Result<()>, cleanup: Result<()>) -> Result<()> {
    match operation {
        Ok(()) => cleanup,
        Err(err) => Err(with_cleanup(err, cleanup)),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::rex::{FixtureCall, FixtureRunner};

    const WHOAMI: &[&str] = &["--autostart=false", "whoami", "--json"];

    #[test]
    fn check_api_revision_accepts_supported_server() {
        let runner = FixtureRunner::default();
        let server = "unix:///tmp/pane-mux-fixture.sock";
        let args = ["--autostart=false", "-S", server, "whoami", "--json"];
        runner.reply(&args, Ok(include_str!("fixtures/core_whoami.json").into()));
        runner.reply(&args, Ok(r#"{"api_revision": 19}"#.into()));
        let client = RexCli::with_runner(Some(server.into()), None, runner.clone());
        client.check_api_revision().unwrap();
        client.check_api_revision().unwrap();
        assert_eq!(
            runner.calls(),
            vec![
                FixtureCall {
                    args: args.map(String::from).into(),
                    env_remove: vec![],
                };
                2
            ]
        );
    }

    #[test]
    fn check_api_revision_rejects_old_server() {
        let runner = FixtureRunner::default();
        runner.reply(
            WHOAMI,
            Ok(include_str!("fixtures/core_whoami_old.json").into()),
        );
        let client = RexCli::with_runner(None, None, runner);
        assert_eq!(
            client.check_api_revision().unwrap_err().to_string(),
            "Rex too old: pane-mux needs API revision 18 or newer (found 17)"
        );
    }

    #[test]
    fn maps_runner_errors() {
        let unreachable = include_str!("fixtures/core_unreachable.txt");
        for (input, expected) in [
            (
                RunnerError::Spawn(io::Error::from(ErrorKind::NotFound)),
                "`rex` command not found on PATH".into(),
            ),
            (
                RunnerError::Spawn(io::Error::from(ErrorKind::PermissionDenied)),
                "could not run `rex`".into(),
            ),
            (
                RunnerError::Exit(unreachable.into()),
                format!("Rex server unreachable: {}", unreachable.trim()),
            ),
            (
                RunnerError::Exit("  Error: session not found\n".into()),
                "Error: session not found".into(),
            ),
        ] {
            let runner = FixtureRunner::default();
            runner.reply(WHOAMI, Err(input));
            let client = RexCli::with_runner(None, None, runner);
            assert_eq!(
                client.check_api_revision().unwrap_err().to_string(),
                expected
            );
        }
    }

    #[test]
    fn forwards_environment_removals() {
        let runner = FixtureRunner::default();
        let args = ["--autostart=false", "do", "-e", "return 1"];
        runner.reply(&args, Ok("1\n".into()));
        let client = RexCli::with_runner(None, None, runner.clone());
        assert_eq!(
            client
                .run(&args[1..], &["REX_SESSION", "REX_BLOCK"])
                .unwrap(),
            "1\n"
        );
        assert_eq!(
            runner.calls(),
            [FixtureCall {
                args: args.map(String::from).into(),
                env_remove: vec!["REX_SESSION".into(), "REX_BLOCK".into()],
            }]
        );
    }

    #[test]
    fn client_context_comes_from_nonblank_block() {
        for (block, expected) in [
            (None, None),
            (Some(""), None),
            (Some(" \t"), None),
            (Some("block:abc"), Some("block:abc")),
        ] {
            let client = RexCli::new(None, block.map(String::from));
            assert_eq!(client.in_client(), expected.is_some());
            assert_eq!(client.suppressed_pane().as_deref(), expected);
        }
    }
}
