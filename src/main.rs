use std::process::ExitCode;

use clap::Parser;
use pane_mux::{
    cli::{Cli, Command, FloatAction},
    commands::{self, Outcome},
    config,
    tmux::TmuxCli,
};

fn execute(cli: Cli) -> anyhow::Result<Outcome> {
    let loaded = config::load()?;
    let tmux = TmuxCli::default();
    match cli.command {
        Command::Picker(args) => commands::picker::run(args.level(), &tmux, &loaded),
        Command::SwitchSession => commands::switch_session::run(&tmux, &loaded),
        Command::MovePane { target } => commands::move_pane::run(target.as_deref(), &tmux, &loaded),
        Command::Float { action } => match action {
            FloatAction::Stash { target } => commands::float::run_stash(target.as_deref(), &tmux),
            FloatAction::Unstash { target } => {
                commands::float::run_unstash(target.as_deref(), &tmux)
            }
            FloatAction::Toggle { target } => commands::float::run_toggle(target.as_deref(), &tmux),
        },
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
