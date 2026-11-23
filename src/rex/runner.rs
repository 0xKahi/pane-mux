use std::{io, process::Command};

/// The two ways a Rex invocation can fail before parsing its output.
#[derive(Debug)]
pub enum RunnerError {
    Spawn(io::Error),
    Exit(String),
}

/// Execute Rex with the supplied argv and environment removals.
pub trait Runner {
    fn run(&self, args: &[&str], env_remove: &[&str]) -> Result<String, RunnerError>;
}

pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run(&self, args: &[&str], env_remove: &[&str]) -> Result<String, RunnerError> {
        let mut command = Command::new("rex");
        command.args(args);
        for key in env_remove {
            command.env_remove(key);
        }
        let output = command.output().map_err(RunnerError::Spawn)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(RunnerError::Exit(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    }
}

#[cfg(test)]
pub use fixtures::{FixtureCall, FixtureRunner};

#[cfg(test)]
mod fixtures {
    use super::{Runner, RunnerError};
    use std::{
        cell::RefCell,
        collections::{HashMap, VecDeque},
        rc::Rc,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FixtureCall {
        pub args: Vec<String>,
        pub env_remove: Vec<String>,
    }

    #[derive(Default)]
    struct Fixtures {
        replies: HashMap<Vec<String>, VecDeque<Result<String, RunnerError>>>,
        calls: Vec<FixtureCall>,
    }

    /// Script replies by full argv; clones share the fixture and call log.
    #[derive(Clone, Default)]
    pub struct FixtureRunner(Rc<RefCell<Fixtures>>);

    impl FixtureRunner {
        /// Queue one reply for these arguments. Repeated calls consume replies in order.
        pub fn reply(&self, args: &[&str], result: Result<String, RunnerError>) {
            self.0
                .borrow_mut()
                .replies
                .entry(args.iter().map(|arg| (*arg).into()).collect())
                .or_default()
                .push_back(result);
        }

        pub fn calls(&self) -> Vec<FixtureCall> {
            self.0.borrow().calls.clone()
        }
    }

    impl Runner for FixtureRunner {
        fn run(&self, args: &[&str], env_remove: &[&str]) -> Result<String, RunnerError> {
            let mut fixtures = self.0.borrow_mut();
            let args: Vec<String> = args.iter().map(|arg| (*arg).into()).collect();
            fixtures.calls.push(FixtureCall {
                args: args.clone(),
                env_remove: env_remove.iter().map(|key| (*key).into()).collect(),
            });
            fixtures
                .replies
                .get_mut(&args)
                .and_then(VecDeque::pop_front)
                .unwrap_or_else(|| panic!("no Rex fixture reply for {args:?}"))
        }
    }
}
