//! Everything a key binding can do. Data only: `picker::state` decides what each action means.
//!
//! Config parses `run = "arrow next"` strings into these values (see `config::keymap`).

/// One command from a key binding's `run` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // --- picker layer ---
    /// Close the topmost popup / clear the message; does not quit.
    Escape,
    /// Exit without picking (exit code 130).
    Quit,
    /// Move the cursor in the focused column (or the help list).
    Arrow(Step),
    /// Move focus to the previous/next column.
    Focus(Direction),
    /// Scroll the preview by a percentage of its height (negative = up).
    Seek(i16),
    /// Exit and output the hovered item's target.
    Pick,
    /// Open the name prompt to create a session/window.
    Create,
    /// Open the name prompt to rename the hovered session/window.
    Rename,
    /// Ask to delete the hovered item.
    Remove,
    /// Open the help popup.
    Help,

    // --- input / confirm / help layers ---
    /// Close the popup; `submit: true` accepts it (`close --submit`).
    Close { submit: bool },
    /// Delete the character before the cursor.
    Backspace,
    /// Delete the word before the cursor.
    KillWord,
    /// Delete everything before the cursor.
    KillBol,
}

/// Argument of `arrow`: `next`, `prev`, `top`, `bot`, `N` or `N%` (N may be negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Next,
    Prev,
    Top,
    Bot,
    /// Move by N rows.
    Offset(isize),
    /// Move by N percent of the visible rows (`-100%` = one page up).
    Percent(i16),
}

/// Argument of `focus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}
