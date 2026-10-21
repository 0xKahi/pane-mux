//! Synchronous which-key candidate filtering, inspired by yazi.

use crate::{
    config::keymap::{Chord, Key},
    picker::action::Action,
};

/// Pending multi-key bindings, exposed for the suggestions popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Which {
    pub candidates: Vec<Chord>,
    pub times: usize,
}

impl Which {
    /// Collect multi-key bindings beginning with this key.
    pub fn start(key: Key, chords: &[Chord]) -> Option<Self> {
        let candidates: Vec<_> = chords
            .iter()
            .filter(|c| c.on.len() > 1 && c.on.first() == Some(&key))
            .cloned()
            .collect();
        if candidates.is_empty() {
            None
        } else {
            Some(Self {
                candidates,
                times: 1,
            })
        }
    }

    /// Filter the next key. An empty candidate list means dismissal.
    /// Like yazi, a completed binding wins over longer bindings sharing its prefix.
    /// Unlike yazi's early completion of a sole candidate, all keys must be typed.
    pub fn advance(&mut self, key: Key) -> Option<Vec<Action>> {
        self.candidates
            .retain(|c| c.on.get(self.times) == Some(&key));
        self.times += 1;
        let actions = self
            .candidates
            .iter()
            .find(|c| c.on.len() == self.times)
            .map(|c| c.run.clone());
        if actions.is_some() {
            self.candidates.clear();
        }
        actions
    }
}

#[cfg(test)]
mod tests;
