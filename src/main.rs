use std::process::ExitCode;

use clap::Parser;
use pane_mux::{
    cli::{Cli, Command},
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
