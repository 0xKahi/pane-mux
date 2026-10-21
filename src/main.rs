use std::process::ExitCode;

use clap::Parser;
use pane_mux::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Picker => match pane_mux::picker::run_slice() {
            Ok(Some(target)) => {
                println!("{target}");
                ExitCode::SUCCESS
            }
            Ok(None) => ExitCode::from(130),
            Err(err) => {
                eprintln!("pane-mux: {err:#}");
                ExitCode::from(2)
            }
        },
    }
}
