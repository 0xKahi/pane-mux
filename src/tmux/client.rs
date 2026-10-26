//! All real tmux access goes through this synchronous interface.

use std::process::Command;

use anyhow::{Context, Result, bail, ensure};

use super::{Current, Tree, format};

/// Operations needed by the picker; ids are used for mutations.
pub trait Tmux {
    fn float_window(&self, window: &str) -> Result<super::model::FloatWindow>;
    fn window_layout(&self, window: &str) -> Result<String>;
    /// Sizes accept cells or percentages; positions are border-inclusive.
    fn new_dummy_float(
        &self,
        window: &str,
        width: &str,
        height: &str,
        x: Option<i32>,
        y: Option<i32>,
    ) -> Result<String>;
    fn swap_and_kill(&self, pane: &str, dummy: &str) -> Result<()>;
    /// Scope is "session", "-w" (window), or "-p" (pane).
    fn set_option(&self, scope: &str, target: &str, name: &str, value: &str) -> Result<()>;
    fn option(&self, scope: &str, target: &str, name: &str) -> Result<String>;
    fn resize_window(&self, window: &str, width: i32, height: i32) -> Result<()>;
    fn toggle_zoom(&self, pane: &str) -> Result<()>;
    fn select_pane(&self, pane: &str) -> Result<()>;
    fn display_message(&self, target: &str, message: &str) -> Result<()>;
    fn stash_window(
        &self,
        session: Option<&str>,
        name: &str,
        command: &str,
        width: i32,
        height: i32,
    ) -> Result<String>;
    fn socket_path(&self) -> Result<String>;
    fn tree(&self) -> Result<Tree>;
    fn capture(&self, pane_id: &str) -> Result<String>;
    fn current(&self) -> Result<Option<Current>>;
    fn resolve_pane(&self, target: &str) -> Result<String>;
    fn move_pane_to_new_session(
        &self,
        pane_id: &str,
        session_name: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<(String, String)>;
    fn move_pane_to_new_window(
        &self,
        pane_id: &str,
        session_id: &str,
        name: Option<&str>,
    ) -> Result<String>;
    fn float_pane_in_window(
        &self,
        pane_id: &str,
        window_id: &str,
        floating: &crate::config::settings::FloatingPaneConfig,
    ) -> Result<()>;
    fn new_session(&self, name: Option<&str>) -> Result<String>;
    fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String>;
    fn rename_session(&self, session_id: &str, name: &str) -> Result<()>;
    fn rename_window(&self, window_id: &str, name: &str) -> Result<()>;
    fn kill_session(&self, session_id: &str) -> Result<()>;
    fn kill_window(&self, window_id: &str) -> Result<()>;
    fn kill_pane(&self, pane_id: &str) -> Result<()>;
    fn switch_client(&self, target: &str) -> Result<()>;
}

/// Command-based client. A named socket isolates tests from the user's server.
#[derive(Debug, Clone, Default)]
pub struct TmuxCli {
    socket: Option<String>,
}

impl TmuxCli {
    /// Use the invoking client's server, or an isolated named socket.
    pub fn new(socket: Option<&str>) -> Self {
        Self {
            socket: socket.map(String::from),
        }
    }

    /// Execute arguments directly, never through a shell.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("tmux");
        if let Some(socket) = &self.socket {
            command.args(["-L", socket, "-f", "/dev/null"]);
            command.env_remove("TMUX").env_remove("TMUX_PANE");
        }
        let output = command
            .args(args)
            .output()
            .context("could not run tmux; ensure tmux is installed and on PATH")?;
        if !output.status.success() {
            bail!(
                "tmux {}: {}",
                args.first().unwrap_or(&""),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        String::from_utf8(output.stdout).context("tmux returned invalid UTF-8")
    }

    /// Query current ids with an injected environment value for deterministic tests.
    pub fn current_with_env(&self, tmux: Option<&str>) -> Result<Option<Current>> {
        if tmux.is_none() {
            return Ok(None);
        }
        let output = self.run(&[
            "display-message",
            "-p",
            "#{session_id}\t#{window_id}\t#{pane_id}",
        ])?;
        let fields: Vec<&str> = output.trim_end().split('\t').collect();
        ensure!(
            fields.len() == 3,
            "tmux display-message: expected three current ids"
        );
        Ok(Some(Current {
            session_id: fields[0].into(),
            window_id: fields[1].into(),
            pane_id: fields[2].into(),
        }))
    }
}

impl Tmux for TmuxCli {
    fn float_window(&self, window: &str) -> Result<super::model::FloatWindow> {
        let mut snapshot = format::parse_float_window(&self.run(&[
            "list-panes",
            "-t",
            window,
            "-F",
            &format::float_format(),
        ])?)?;
        if snapshot.zoomed {
            format::apply_layout_floats(&mut snapshot, &self.window_layout(window)?)?;
        }
        Ok(snapshot)
    }

    fn window_layout(&self, window: &str) -> Result<String> {
        self.run(&["display-message", "-p", "-t", window, "#{window_layout}"])
    }

    fn new_dummy_float(
        &self,
        window: &str,
        width: &str,
        height: &str,
        x: Option<i32>,
        y: Option<i32>,
    ) -> Result<String> {
        let mut args = vec![
            "new-pane",
            "-d",
            "-A",
            "-E",
            "-T",
            "pane-mux-dummy",
            "-t",
            window,
            "-x",
            width,
            "-y",
            height,
        ];
        let x = x.map(|v| v.to_string());
        let y = y.map(|v| v.to_string());
        if let Some(x) = &x {
            args.extend(["-X", x]);
        }
        if let Some(y) = &y {
            args.extend(["-Y", y]);
        }
        args.extend(["-P", "-F", "#{pane_id}"]);
        Ok(self.run(&args)?.trim().into())
    }

    fn swap_and_kill(&self, pane: &str, dummy: &str) -> Result<()> {
        self.run(&[
            "swap-pane",
            "-d",
            "-Z",
            "-s",
            pane,
            "-t",
            dummy,
            ";",
            "kill-pane",
            "-t",
            dummy,
        ])
        .map(|_| ())
    }

    fn set_option(&self, scope: &str, target: &str, name: &str, value: &str) -> Result<()> {
        let mut args = vec!["set-option"];
        if scope != "session" {
            args.push(scope);
        }
        args.extend(["-t", target, name, value]);
        self.run(&args).map(|_| ())
    }

    fn option(&self, scope: &str, target: &str, name: &str) -> Result<String> {
        let mut args = vec!["show-options"];
        if scope != "session" {
            args.push(scope);
        }
        args.extend(["-qv", "-t", target, name]);
        Ok(self.run(&args)?.trim_end().into())
    }

    fn resize_window(&self, window: &str, width: i32, height: i32) -> Result<()> {
        self.run(&[
            "resize-window",
            "-t",
            window,
            "-x",
            &width.to_string(),
            "-y",
            &height.to_string(),
        ])
        .map(|_| ())
    }

    fn toggle_zoom(&self, pane: &str) -> Result<()> {
        self.run(&["resize-pane", "-Z", "-t", pane]).map(|_| ())
    }

    fn select_pane(&self, pane: &str) -> Result<()> {
        self.run(&["select-pane", "-t", pane]).map(|_| ())
    }

    fn display_message(&self, target: &str, message: &str) -> Result<()> {
        self.run(&["display-message", "-t", target, message])
            .map(|_| ())
    }

    fn stash_window(
        &self,
        session: Option<&str>,
        name: &str,
        command: &str,
        width: i32,
        height: i32,
    ) -> Result<String> {
        let output = if let Some(session) = session {
            self.run(&[
                "new-window",
                "-d",
                "-P",
                "-F",
                "#{window_id}",
                "-t",
                &format!("{session}:"),
                "-n",
                name,
                command,
            ])?
        } else {
            self.run(&[
                "new-session",
                "-d",
                "-P",
                "-F",
                "#{window_id}",
                "-s",
                "pmux",
                "-x",
                &width.to_string(),
                "-y",
                &height.to_string(),
                "-n",
                name,
                command,
            ])?
        };
        let window = output.trim().to_owned();
        self.resize_window(&window, width, height)?;
        Ok(window)
    }

    fn socket_path(&self) -> Result<String> {
        Ok(self
            .run(&["display-message", "-p", "#{socket_path}"])?
            .trim()
            .into())
    }
    fn tree(&self) -> Result<Tree> {
        format::parse_tree(&self.run(&["list-panes", "-a", "-F", format::PANE_FORMAT])?)
    }

    fn capture(&self, pane_id: &str) -> Result<String> {
        self.run(&["capture-pane", "-e", "-p", "-J", "-t", pane_id])
    }

    fn current(&self) -> Result<Option<Current>> {
        self.current_with_env(std::env::var("TMUX").ok().as_deref())
    }

    fn resolve_pane(&self, target: &str) -> Result<String> {
        let id = self.run(&["display-message", "-p", "-t", target, "#{pane_id}"])?;
        let id = id.trim();
        ensure!(id.starts_with('%'), "pane not found: {target}");
        Ok(id.into())
    }

    fn move_pane_to_new_session(
        &self,
        pane_id: &str,
        session_name: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<(String, String)> {
        let mut args = vec![
            "new-session",
            "-d",
            "-P",
            "-F",
            "#{session_id} #{window_id}",
        ];
        if let Some(name) = session_name {
            args.extend(["-s", name]);
        }
        let output = self.run(&args)?;
        let ids: Vec<_> = output.split_whitespace().collect();
        ensure!(
            ids.len() == 2,
            "tmux new-session: expected session and window ids"
        );
        let (session, placeholder) = (ids[0], ids[1]);
        let window = match self.move_pane_to_new_window(pane_id, session, window_name) {
            Ok(window) => window,
            Err(error) => {
                let _ = self.kill_session(session);
                return Err(error);
            }
        };
        self.kill_window(placeholder)?;
        self.run(&["move-window", "-r", "-t", session])?;
        Ok((session.into(), window))
    }

    fn move_pane_to_new_window(
        &self,
        pane_id: &str,
        session_id: &str,
        name: Option<&str>,
    ) -> Result<String> {
        let target = format!("{session_id}:");
        let mut args = vec!["break-pane", "-d", "-s", pane_id, "-t", &target];
        if let Some(name) = name {
            args.extend(["-n", name]);
        }
        self.run(&args)?;
        // Ask for the window afterwards: tmux before 3.8 prints nothing for
        // `break-pane -P` when the pane was alone in its window.
        let window = self.run(&["display-message", "-p", "-t", pane_id, "#{window_id}"])?;
        Ok(window.trim().into())
    }

    fn float_pane_in_window(
        &self,
        pane_id: &str,
        window_id: &str,
        floating: &crate::config::settings::FloatingPaneConfig,
    ) -> Result<()> {
        let dummy = self.new_dummy_float(
            window_id,
            &format!("{}%", floating.width),
            &format!("{}%", floating.height),
            None,
            None,
        )?;
        let result = (|| {
            self.run(&[
                "move-pane",
                "-P",
                &floating.position.to_string(),
                "-t",
                &dummy,
            ])?;
            self.swap_and_kill(pane_id, &dummy)
        })();
        if result.is_err() {
            let _ = self.kill_pane(&dummy);
        }
        result
    }

    fn new_session(&self, name: Option<&str>) -> Result<String> {
        let mut args = vec!["new-session", "-d", "-P", "-F", "#{session_id}"];
        if let Some(name) = name {
            args.extend(["-s", name]);
        }
        Ok(self.run(&args)?.trim().into())
    }

    fn new_window(&self, session_id: &str, name: Option<&str>) -> Result<String> {
        // A trailing colon makes the target a session, not a window name.
        let target = format!("{session_id}:");
        let mut args = vec![
            "new-window",
            "-d",
            "-P",
            "-F",
            "#{window_id}",
            "-t",
            &target,
        ];
        if let Some(name) = name {
            args.extend(["-n", name]);
        }
        Ok(self.run(&args)?.trim().into())
    }

    fn rename_session(&self, session_id: &str, name: &str) -> Result<()> {
        self.run(&["rename-session", "-t", session_id, name])
            .map(|_| ())
    }

    fn rename_window(&self, window_id: &str, name: &str) -> Result<()> {
        self.run(&["rename-window", "-t", window_id, name])
            .map(|_| ())
    }

    fn kill_session(&self, session_id: &str) -> Result<()> {
        self.run(&["kill-session", "-t", session_id]).map(|_| ())
    }

    fn kill_window(&self, window_id: &str) -> Result<()> {
        self.run(&["kill-window", "-t", window_id]).map(|_| ())
    }

    fn kill_pane(&self, pane_id: &str) -> Result<()> {
        self.run(&["kill-pane", "-t", pane_id]).map(|_| ())
    }

    fn switch_client(&self, target: &str) -> Result<()> {
        self.run(&["switch-client", "-t", target]).map(|_| ())
    }
}

/// Suppress only the invoking regular pane, never a popup or an outside process.
pub fn suppressed_pane(tmux: Option<&str>, tmux_pane: Option<&str>) -> Option<String> {
    tmux.and(tmux_pane).map(String::from)
}

/// Read the real environment for preview suppression.
pub fn suppressed_pane_from_env() -> Option<String> {
    suppressed_pane(
        std::env::var("TMUX").ok().as_deref(),
        std::env::var("TMUX_PANE").ok().as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppression_regular_pane_popup_and_outside() {
        assert_eq!(
            suppressed_pane(Some("socket"), Some("%12")),
            Some("%12".into())
        );
        assert_eq!(suppressed_pane(Some("socket"), None), None);
        assert_eq!(suppressed_pane(None, Some("%12")), None);
        assert_eq!(suppressed_pane(None, None), None);
    }

    #[test]
    fn current_outside_tmux_needs_no_command() {
        assert_eq!(
            TmuxCli::new(Some("unused")).current_with_env(None).unwrap(),
            None
        );
    }
}
