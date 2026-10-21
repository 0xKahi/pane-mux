//! Non-interactive CLI checks: all failures must happen before terminal setup.

use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pane-mux-cli-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pane-mux"));
        command
            .env("PANE_MUX_CONFIG_HOME", &self.0)
            .env("TMUX_TMPDIR", &self.0)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(command: &mut Command) -> Output {
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("CLI hung instead of failing before terminal setup: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn assert_error(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(!stderr.is_empty());
    // No alternate-screen escape: failures must not enter the UI.
    assert!(!stderr.contains("\u{1b}[?1049h"));
    stderr
}

#[test]
fn conflicting_level_flags_are_usage_error() {
    let dir = TempDir::new();
    let output = run(dir.command().args(["picker", "--session", "--pane"]));
    assert!(assert_error(&output).contains("Usage:"));
}

#[test]
fn missing_server_errors_without_opening_terminal() {
    let dir = TempDir::new();
    let output = run(dir.command().arg("picker"));
    assert!(assert_error(&output).contains("tmux"));
}

#[test]
fn switch_session_requires_tmux() {
    let dir = TempDir::new();
    let output = run(dir.command().arg("switch-session"));
    assert!(assert_error(&output).contains("switch-session must run inside tmux"));
}

#[test]
fn invalid_keymap_from_config_home_names_file() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("keymap.toml"), "[picker\n").unwrap();
    let output = run(dir.command().arg("picker"));
    let stderr = assert_error(&output);
    assert!(stderr.contains(&dir.0.join("keymap.toml").display().to_string()));
    assert!(stderr.contains("TOML parse error"));
}

#[test]
fn help_lists_both_commands() {
    let dir = TempDir::new();
    let output = run(dir.command().arg("--help"));
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("picker"));
    assert!(stdout.contains("switch-session"));
}
