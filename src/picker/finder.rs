//! Compiled name matching, independent of the focused column and its cursors.

use std::{fmt, ops::Range};

use regex::{Regex, RegexBuilder};

use super::action::Case;

#[derive(Clone)]
pub struct Finder {
    pub query: String,
    pub case: Case,
    regex: Regex,
}

impl Finder {
    pub fn new(query: String, case: Case) -> Result<Self, regex::Error> {
        let insensitive = match case {
            Case::Sensitive => false,
            Case::Insensitive => true,
            Case::Smart => !query.chars().any(char::is_uppercase),
        };
        let regex = RegexBuilder::new(&query)
            .case_insensitive(insensitive)
            .build()?;
        Ok(Self { query, case, regex })
    }

    /// Byte range of the first match, suitable for splitting a UTF-8 name.
    pub fn matched(&self, name: &str) -> Option<Range<usize>> {
        self.regex.find(name).map(|m| m.range())
    }

    pub fn next(&self, names: &[(String, &str)], cursor: usize, include: bool) -> Option<String> {
        self.jump(names, cursor, include, false)
    }

    pub fn prev(&self, names: &[(String, &str)], cursor: usize, include: bool) -> Option<String> {
        self.jump(names, cursor, include, true)
    }

    fn jump(
        &self,
        names: &[(String, &str)],
        cursor: usize,
        include: bool,
        previous: bool,
    ) -> Option<String> {
        let len = names.len();
        if len == 0 {
            return None;
        }
        let cursor = cursor % len;
        (usize::from(!include)..len).find_map(|offset| {
            let index = if previous {
                (cursor + len - offset) % len
            } else {
                (cursor + offset) % len
            };
            let (id, name) = &names[index];
            self.matched(name).map(|_| id.clone())
        })
    }

    /// The id's 1-based position and total count among matching names.
    pub fn position(&self, names: &[(String, &str)], id: &str) -> Option<(usize, usize)> {
        let mut position = None;
        let mut total = 0;
        for (candidate, name) in names {
            if self.matched(name).is_some() {
                total += 1;
                if candidate == id {
                    position = Some(total);
                }
            }
        }
        position.map(|position| (position, total))
    }
}

impl PartialEq for Finder {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query && self.case == other.case
    }
}

impl Eq for Finder {}

impl fmt::Debug for Finder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Finder")
            .field("query", &self.query)
            .field("case", &self.case)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_and_regex() {
        for (query, case, name, matches) in [
            ("work", Case::Smart, "Work", true),
            ("Work", Case::Smart, "work", false),
            ("Work", Case::Insensitive, "work", true),
            ("work", Case::Sensitive, "Work", false),
            ("^d.t", Case::Smart, "dotfiles", true),
            (r"\Swork", Case::Smart, "xWORK", false),
            (r"\Swork", Case::Insensitive, "xWORK", true),
        ] {
            let finder = Finder::new(query.into(), case).unwrap();
            assert_eq!(finder.matched(name).is_some(), matches);
        }
        assert!(Finder::new("(".into(), Case::Smart).is_err());
        let finder = Finder::new("é".into(), Case::Smart).unwrap();
        assert_eq!(finder.matched("café"), Some(3..5));
        assert_eq!(finder, finder.clone());
        assert_ne!(finder, Finder::new("é".into(), Case::Sensitive).unwrap());
    }

    #[test]
    fn navigation_and_position() {
        let names = vec![
            ("a".into(), "logs"),
            ("b".into(), "api"),
            ("c".into(), "blog"),
        ];
        let finder = Finder::new("log".into(), Case::Sensitive).unwrap();
        assert_eq!(finder.next(&names, 0, true).as_deref(), Some("a"));
        assert_eq!(finder.prev(&names, 0, true).as_deref(), Some("a"));
        assert_eq!(finder.next(&names, 0, false).as_deref(), Some("c"));
        assert_eq!(finder.prev(&names, 0, false).as_deref(), Some("c"));
        assert_eq!(finder.next(&names, 2, false).as_deref(), Some("a"));
        assert_eq!(finder.prev(&names, 2, false).as_deref(), Some("a"));
        assert_eq!(finder.position(&names, "c"), Some((2, 2)));
        assert_eq!(finder.position(&names, "b"), None);
        assert_eq!(finder.next(&names[..1], 0, false), None);
        assert_eq!(finder.prev(&[], 0, true), None);
        let absent = Finder::new("zzz".into(), Case::Sensitive).unwrap();
        assert_eq!(absent.next(&names, 0, true), None);
        assert_eq!(absent.prev(&names, 0, false), None);
    }
}
