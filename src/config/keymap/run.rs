//! Parse the small, whitespace-separated command language used by key bindings.

use anyhow::{Result, bail};

use crate::picker::action::{Action, Direction, Step};

/// Parse one command, rejecting unknown commands and unexpected arguments.
pub fn parse_run(text: &str) -> Result<Action> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let Some(name) = words.first().copied() else {
        bail!("empty run command");
    };
    let args = &words[1..];
    let bad_args = || anyhow::anyhow!("invalid arguments for command {name:?}: {text:?}");
    let action = match (name, args) {
        ("escape", []) => Action::Escape,
        ("quit", []) => Action::Quit,
        ("pick", []) => Action::Pick,
        ("create", []) => Action::Create,
        ("rename", []) => Action::Rename,
        ("remove", []) => Action::Remove,
        ("help", []) => Action::Help,
        ("close", []) => Action::Close { submit: false },
        ("close", ["--submit"]) => Action::Close { submit: true },
        ("backspace", []) => Action::Backspace,
        ("kill-word", []) => Action::KillWord,
        ("kill-bol", []) => Action::KillBol,
        ("focus", ["prev"]) => Action::Focus(Direction::Prev),
        ("focus", ["next"]) => Action::Focus(Direction::Next),
        ("arrow", [arg]) => Action::Arrow(match *arg {
            "next" => Step::Next,
            "prev" => Step::Prev,
            "top" => Step::Top,
            "bot" => Step::Bot,
            _ => {
                if let Some(percent) = arg.strip_suffix('%') {
                    Step::Percent(percent.parse().map_err(|_| bad_args())?)
                } else {
                    Step::Offset(arg.parse().map_err(|_| bad_args())?)
                }
            }
        }),
        ("seek", [arg]) => Action::Seek(
            arg.strip_suffix('%')
                .ok_or_else(bad_args)?
                .parse()
                .map_err(|_| bad_args())?,
        ),
        (
            "escape" | "quit" | "pick" | "create" | "rename" | "remove" | "help" | "close"
            | "backspace" | "kill-word" | "kill-bol" | "focus" | "arrow" | "seek",
            _,
        ) => return Err(bad_args()),
        _ => bail!("unknown command {name:?}"),
    };
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_navigation_and_popup_commands() {
        for (text, action) in [
            ("arrow next", Action::Arrow(Step::Next)),
            ("arrow prev", Action::Arrow(Step::Prev)),
            ("arrow top", Action::Arrow(Step::Top)),
            ("arrow bot", Action::Arrow(Step::Bot)),
            ("arrow -3", Action::Arrow(Step::Offset(-3))),
            ("arrow -100%", Action::Arrow(Step::Percent(-100))),
            ("focus next", Action::Focus(Direction::Next)),
            ("focus prev", Action::Focus(Direction::Prev)),
            ("seek -50%", Action::Seek(-50)),
            ("close --submit", Action::Close { submit: true }),
            ("close", Action::Close { submit: false }),
        ] {
            assert_eq!(parse_run(text).unwrap(), action);
        }
    }

    #[test]
    fn unknown_command_names_are_reported() {
        assert!(
            parse_run("explode now")
                .unwrap_err()
                .to_string()
                .contains("unknown command \"explode\"")
        );
    }

    #[test]
    fn bad_arguments_name_the_command() {
        for text in [
            "arrow",
            "arrow sideways",
            "seek 50",
            "seek 999999%",
            "focus top",
            "quit now",
            "close --other",
            "arrow 1 extra",
        ] {
            let name = text.split_whitespace().next().unwrap();
            assert!(parse_run(text).unwrap_err().to_string().contains(name));
        }
    }
}
