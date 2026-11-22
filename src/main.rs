use std::process::ExitCode;

use clap::Parser;
use pane_mux::{
    cli::{Cli, Command, FloatAction},
    commands::{self, Outcome},
    config, mux,
};

fn execute(cli: Cli) -> anyhow::Result<Outcome> {
    let loaded = config::load()?;
    let backend = mux::open()?;
    match cli.command {
        Command::Picker(args) => commands::picker::run(args.level(), &*backend, &loaded),
        Command::SwitchSession => commands::switch_session::run(&*backend, &loaded),
        Command::MovePane { target } => {
            commands::move_pane::run(target.as_deref(), &*backend, &loaded)
        }
        Command::Float { action } => match action {
            FloatAction::Stash { target } => {
                commands::float::run_stash(target.as_deref(), &*backend)
            }
            FloatAction::Unstash { target } => {
                commands::float::run_unstash(target.as_deref(), &*backend)
            }
            FloatAction::Toggle { target } => {
                commands::float::run_toggle(target.as_deref(), &*backend)
            }
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
