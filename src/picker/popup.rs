//! Owned prompt data; all text editing happens at the end of the input.

use crate::{config::keymap::Chord, tmux::Level};

/// Operation to run when a name prompt is submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputPurpose {
    CreateSession,
    CreateWindow { session_id: String },
    RenameSession { id: String },
    RenameWindow { id: String },
}

/// One modal popup. Which-key suggestions are tracked separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    Input {
        title: String,
        value: String,
        purpose: InputPurpose,
    },
    Confirm {
        title: String,
        body: String,
        level: Level,
        id: String,
    },
    Help {
        cursor: usize,
        chords: Vec<Chord>,
    },
}
