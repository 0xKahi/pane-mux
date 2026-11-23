//! Rex backend, backed by the `rex` command-line tool.

mod client;
mod history;
mod runner;
mod snapshot;

#[cfg(test)]
mod tests;

pub use client::RexCli;
pub use runner::{ProcessRunner, Runner, RunnerError};

#[cfg(test)]
pub use runner::{FixtureCall, FixtureRunner};
