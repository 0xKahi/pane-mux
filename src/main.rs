use std::process::ExitCode;

use clap::Parser;
use pane_mux::{
    cli::{Cli, Command, FloatAction},
    commands::{self, Outcome},
    config, mux,
};

fn execute(cli: Cli) -> anyhow::Result<Outcome> {
    let loaded = config::load()?;
    const FLOAT_ONLY: &str =
        "`float` is tmux-only; use Rex's built-in minimize to manage floating panes";
    let backend = if matches!(&cli.command, Command::Float { .. }) {
        // Reject unsupported commands without connecting to Rex.
        let env = mux::EnvSnapshot::from_env();
        if mux::detect(&env)? == mux::BackendKind::Rex {
            anyhow::bail!(FLOAT_ONLY);
        }
        mux::open_with_env(&env)?
    } else {
        mux::open()?
    };

    match cli.command {
        Command::Picker(args) => commands::picker::run(args.level(), backend.as_mux(), &loaded),
        Command::SwitchSession => commands::switch_session::run(backend.as_mux(), &loaded),
        Command::MovePane { target } => {
            commands::move_pane::run(target.as_deref(), backend.as_mux(), &loaded)
        }
        Command::Float { action } => {
            let float = backend
                .as_float()
                .ok_or_else(|| anyhow::anyhow!(FLOAT_ONLY))?;
            match action {
                FloatAction::Stash { target } => {
                    commands::float::run_stash(target.as_deref(), float)
                }
                FloatAction::Unstash { target } => {
                    commands::float::run_unstash(target.as_deref(), float)
                }
                FloatAction::Toggle { target } => {
                    commands::float::run_toggle(target.as_deref(), float)
                }
            }
        }
    }
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(Outcome::Done) => ExitCode::SUCCESS,
        Ok(Outcome::Cancelled) => ExitCode::from(130),
        Err(err) => {
            eprintln!("pane-mux: {err:#}");
            ExitCode::from(2)
        }
    }
}
