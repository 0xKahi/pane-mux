//! Backend-neutral multiplexer types and operations.

pub mod backend;
pub mod model;

pub use backend::Mux;
pub use model::{Current, Level, Pane, Session, Target, Tree, Window};

use std::{env, ffi::OsString};

use anyhow::{Result, bail};

use crate::{
    rex::RexCli,
    tmux::{FloatMux, TmuxCli},
};

/// Environment inputs used for backend detection and Rex startup.
#[derive(Debug, Default)]
pub struct EnvSnapshot {
    pub backend_override: Option<OsString>,
    pub tmux_set: bool,
    pub rex_block: Option<String>,
    pub rex_server: Option<String>,
}

impl EnvSnapshot {
    pub fn from_env() -> Self {
        Self {
            backend_override: env::var_os("PANE_MUX_BACKEND"),
            tmux_set: env::var_os("TMUX").is_some(),
            rex_block: env::var("REX_BLOCK").ok(),
            rex_server: env::var("REX_SERVER").ok(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Tmux,
    Rex,
}

/// Select a backend without starting any process.
pub fn detect(env: &EnvSnapshot) -> Result<BackendKind> {
    if let Some(value) = &env.backend_override
        && !value.is_empty()
    {
        return match value.to_str() {
            Some("tmux") => Ok(BackendKind::Tmux),
            Some("rex") => Ok(BackendKind::Rex),
            _ => bail!("PANE_MUX_BACKEND must be `tmux` or `rex` (got {value:?})"),
        };
    }

    if !env.tmux_set
        && env
            .rex_block
            .as_deref()
            .is_some_and(|block| !block.trim().is_empty())
    {
        Ok(BackendKind::Rex)
    } else {
        Ok(BackendKind::Tmux)
    }
}

pub enum Backend {
    Tmux(TmuxCli),
    Rex(RexCli),
}

impl Backend {
    pub fn as_mux(&self) -> &dyn Mux {
        match self {
            Self::Tmux(client) => client,
            Self::Rex(client) => client,
        }
    }

    pub fn as_float(&self) -> Option<&dyn FloatMux> {
        match self {
            Self::Tmux(client) => Some(client),
            Self::Rex(_) => None,
        }
    }
}

/// Detect and open the active backend, checking availability on Rex.
pub fn open() -> Result<Backend> {
    open_with_env(&EnvSnapshot::from_env())
}

/// Open from a captured environment, for callers that check capabilities before startup.
pub fn open_with_env(env: &EnvSnapshot) -> Result<Backend> {
    match detect(env)? {
        BackendKind::Tmux => Ok(Backend::Tmux(TmuxCli::default())),
        BackendKind::Rex => {
            let client = RexCli::new(env.rex_server.clone(), env.rex_block.clone());
            client.check_api_revision()?;
            Ok(Backend::Rex(client))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_selection() {
        use BackendKind::{Rex, Tmux};

        for (name, override_value, tmux_set, rex_block, expected) in [
            ("inside tmux", None, true, None, Tmux),
            ("inside Rex", None, false, Some("block:abc"), Rex),
            ("nested tmux", None, true, Some("block:abc"), Tmux),
            ("empty Rex block", None, false, Some(""), Tmux),
            ("whitespace Rex block", None, false, Some(" \t"), Tmux),
            ("neither", None, false, None, Tmux),
            ("force Rex", Some("rex"), true, None, Rex),
            ("force tmux", Some("tmux"), false, Some("block:abc"), Tmux),
            ("empty override", Some(""), false, Some("block:abc"), Rex),
        ] {
            let env = EnvSnapshot {
                backend_override: override_value.map(Into::into),
                tmux_set,
                rex_block: rex_block.map(Into::into),
                ..EnvSnapshot::default()
            };
            assert_eq!(detect(&env).unwrap(), expected, "{name}");
        }
    }

    #[test]
    fn invalid_override_does_not_fall_back_to_auto() {
        for value in ["zellij", " ", "REX"] {
            let env = EnvSnapshot {
                backend_override: Some(value.into()),
                tmux_set: true,
                ..EnvSnapshot::default()
            };
            let error = detect(&env).unwrap_err().to_string();
            assert!(error.contains("PANE_MUX_BACKEND must be `tmux` or `rex`"));
        }
    }
}
