//! Keys use the same notation in TOML and in help popups.

use std::{fmt, str::FromStr};

use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A key and its modifiers, normalized for comparison with terminal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Key {
    fn normalized(mut self) -> Self {
        if let KeyCode::Char(c) = self.code {
            // Printable characters already encode shift (for example `G` or `?`).
            self.code = KeyCode::Char(if self.shift {
                c.to_ascii_uppercase()
            } else {
                c
            });
            self.shift = false;
        } else if self.code == KeyCode::BackTab {
            self.code = KeyCode::Tab;
            self.shift = true;
        }
        self
    }
}

impl From<KeyEvent> for Key {
    fn from(event: KeyEvent) -> Self {
        Self {
            code: event.code,
            ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
            shift: event.modifiers.contains(KeyModifiers::SHIFT),
            alt: event.modifiers.contains(KeyModifiers::ALT),
        }
        .normalized()
    }
}

impl FromStr for Key {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let invalid = || anyhow::anyhow!("invalid key {text:?}");
        let mut key = Self {
            code: KeyCode::Null,
            ctrl: false,
            shift: false,
            alt: false,
        };
        let mut chars = text.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            key.code = KeyCode::Char(c);
            return Ok(key.normalized());
        }
        let mut name = text
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .ok_or_else(invalid)?;
        loop {
            if name.len() < 2 || name.as_bytes()[1] != b'-' {
                break;
            }
            match name.as_bytes()[0].to_ascii_lowercase() {
                b'c' => key.ctrl = true,
                b's' => key.shift = true,
                b'a' => key.alt = true,
                _ => return Err(invalid()),
            }
            name = &name[2..];
        }
        key.code = match name.to_ascii_lowercase().as_str() {
            "enter" => KeyCode::Enter,
            "esc" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "tab" => KeyCode::Tab,
            "space" => KeyCode::Char(' '),
            "lt" => KeyCode::Char('<'),
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            _ => {
                if let Some(number) = name.strip_prefix('F').or_else(|| name.strip_prefix('f'))
                    && let Ok(n @ 1..=12) = number.parse::<u8>()
                {
                    key.code = KeyCode::F(n);
                    return Ok(key.normalized());
                }
                let mut chars = name.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => KeyCode::Char(c),
                    _ => bail!("invalid key {text:?}"),
                }
            }
        };
        Ok(key.normalized())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.code {
            KeyCode::Char(' ') => "Space".to_owned(),
            KeyCode::Char('<') => "lt".to_owned(),
            KeyCode::Char(c) if !self.ctrl && !self.alt && !self.shift => return write!(f, "{c}"),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_owned(),
            KeyCode::Esc => "Esc".to_owned(),
            KeyCode::Backspace => "Backspace".to_owned(),
            KeyCode::Tab => "Tab".to_owned(),
            KeyCode::BackTab => "S-Tab".to_owned(),
            KeyCode::PageUp => "PageUp".to_owned(),
            KeyCode::PageDown => "PageDown".to_owned(),
            KeyCode::Up => "Up".to_owned(),
            KeyCode::Down => "Down".to_owned(),
            KeyCode::Left => "Left".to_owned(),
            KeyCode::Right => "Right".to_owned(),
            KeyCode::Home => "Home".to_owned(),
            KeyCode::End => "End".to_owned(),
            KeyCode::Delete => "Delete".to_owned(),
            KeyCode::Insert => "Insert".to_owned(),
            KeyCode::F(n) => format!("F{n}"),
            _ => "Unknown".to_owned(),
        };
        write!(
            f,
            "<{}{}{}{name}>",
            if self.ctrl { "C-" } else { "" },
            if self.alt { "A-" } else { "" },
            if self.shift { "S-" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_control_and_page_up() {
        let control: Key = "<C-u>".parse().unwrap();
        assert!(control.ctrl);
        assert_eq!(control.code, KeyCode::Char('u'));
        assert_eq!("<PageUp>".parse::<Key>().unwrap().code, KeyCode::PageUp);
    }

    #[test]
    fn unknown_key_names_identify_the_key() {
        assert_eq!(
            "<Foo>".parse::<Key>().unwrap_err().to_string(),
            "invalid key \"<Foo>\""
        );
    }

    #[test]
    fn terminal_events_match_config_keys() {
        for (text, code, modifiers) in [
            ("G", KeyCode::Char('G'), KeyModifiers::SHIFT),
            ("G", KeyCode::Char('g'), KeyModifiers::SHIFT),
            ("?", KeyCode::Char('?'), KeyModifiers::SHIFT),
            ("<C-u>", KeyCode::Char('u'), KeyModifiers::CONTROL),
            ("<S-Tab>", KeyCode::BackTab, KeyModifiers::SHIFT),
        ] {
            assert_eq!(
                text.parse::<Key>().unwrap(),
                Key::from(KeyEvent::new(code, modifiers))
            );
        }
    }

    #[test]
    fn display_round_trips_supported_keys() {
        for text in [
            "j",
            "G",
            "?",
            "<C-u>",
            "<S-Tab>",
            "<A-x>",
            "<Enter>",
            "<Esc>",
            "<Backspace>",
            "<Tab>",
            "<Space>",
            "<PageUp>",
            "<PageDown>",
            "<Up>",
            "<Down>",
            "<Left>",
            "<Right>",
            "<Home>",
            "<End>",
            "<F1>",
            "<F12>",
            "<Delete>",
            "<lt>",
        ] {
            let key: Key = text.parse().unwrap();
            assert_eq!(key.to_string(), text);
            assert_eq!(key.to_string().parse::<Key>().unwrap(), key);
        }
    }
}
