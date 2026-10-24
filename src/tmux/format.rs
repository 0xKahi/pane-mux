//! Tab-separated tmux snapshot format and parser.

use anyhow::{Context, Result, ensure};

use super::model::{Pane, Session, Tree, Window};

/// One row per pane; tabs preserve spaces in names and commands.
pub const PANE_FORMAT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_current_command}\t#{session_last_attached}\t#{window_zoomed_flag}\t#{pane_floating_flag}";

/// Parse a snapshot, preserving the first-seen session/window/pane order.
pub fn parse_tree(output: &str) -> Result<Tree> {
    let mut tree = Tree::default();
    for (number, line) in output.lines().enumerate() {
        let row =
            parse_row(line).with_context(|| format!("invalid tmux pane row {}", number + 1))?;
        let session_index = tree
            .sessions
            .iter()
            .position(|s| s.id == row.0)
            .unwrap_or_else(|| {
                tree.sessions.push(Session {
                    id: row.0.into(),
                    name: row.1.into(),
                    last_attached: row.4,
                    windows: Vec::new(),
                });
                tree.sessions.len() - 1
            });
        let session = &mut tree.sessions[session_index];
        let window_index = session
            .windows
            .iter()
            .position(|w| w.id == row.2.id)
            .unwrap_or_else(|| {
                session.windows.push(row.2.clone());
                session.windows.len() - 1
            });
        session.windows[window_index].panes.push(row.3);
    }
    Ok(tree)
}

fn parse_row(line: &str) -> Result<(&str, &str, Window, Pane, u64)> {
    let fields: Vec<&str> = line.split('\t').collect();
    ensure!(
        fields.len() == 13,
        "expected 13 tab-separated fields, got {}",
        fields.len()
    );
    ensure!(
        fields[0].starts_with('$') && fields[2].starts_with('@') && fields[6].starts_with('%'),
        "invalid tmux ids"
    );
    let window = Window {
        id: fields[2].into(),
        index: fields[3].parse().context("invalid window index")?,
        name: fields[4].into(),
        active: parse_active(fields[5])?,
        zoomed: parse_active(fields[11])?,
        panes: Vec::new(),
    };
    let pane = Pane {
        id: fields[6].into(),
        index: fields[7].parse().context("invalid pane index")?,
        active: parse_active(fields[8])?,
        command: fields[9].into(),
        // Older tmux versions expand the unsupported floating flag to empty.
        floating: if fields[12].is_empty() {
            false
        } else {
            parse_active(fields[12])?
        },
    };
    let last_attached = if fields[10].is_empty() {
        0
    } else {
        fields[10]
            .parse()
            .context("invalid session last attached")?
    };
    Ok((fields[0], fields[1], window, pane, last_attached))
}

fn parse_active(value: &str) -> Result<bool> {
    ensure!(value == "0" || value == "1", "invalid active flag: {value}");
    Ok(value == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_multiple_sessions_and_preserves_order_and_spaces() {
        let tree = parse_tree("$3\twork space\t@5\t1\tmy editor\t1\t%12\t0\t1\tnvim\t100\t1\t0\n$3\twork space\t@5\t1\tmy editor\t1\t%13\t1\t0\tzsh\t200\t1\t1\n$8\tnotes\t@9\t0\tlogs\t1\t%20\t0\t1\ttail\t\t0\t\n$3\twork space\t@6\t2\tshell\t0\t%14\t0\t1\tzsh\t300\t0\t0\n").unwrap();
        assert_eq!(tree.sessions.len(), 2);
        assert!(tree.sessions[0].windows[0].zoomed);
        assert!(tree.sessions[0].windows[0].panes[1].floating);
        assert!(!tree.sessions[1].windows[0].panes[0].floating);
        assert_eq!(tree.sessions[0].last_attached, 100);
        assert_eq!(tree.sessions[1].last_attached, 0);
        assert_eq!(tree.sessions[0].name, "work space");
        assert_eq!(tree.sessions[0].windows[0].name, "my editor");
        assert_eq!(tree.sessions[0].windows[0].panes.len(), 2);
        assert_eq!(tree.sessions[0].windows[1].id, "@6");
        assert_eq!(tree.sessions[1].id, "$8");
    }

    #[test]
    fn malformed_line_reports_row() {
        let error = parse_tree("not a pane").unwrap_err();
        assert!(format!("{error:#}").contains("row 1"));
        assert!(parse_tree("$0\tx\t@0\tbad\tx\t1\t%0\t0\t1\tsh\t0").is_err());
    }

    #[test]
    fn invalid_last_attached_reports_row() {
        let error = parse_tree("$0\tx\t@0\t0\tx\t1\t%0\t0\t1\tsh\tinvalid\t0\t0").unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("row 1"));
        assert!(message.contains("invalid session last attached"));
    }

    #[test]
    fn invalid_zoomed_or_floating_flags_report_row() {
        for flags in ["bad\t0", "0\tbad"] {
            let error =
                parse_tree(&format!("$0\tx\t@0\t0\tx\t1\t%0\t0\t1\tsh\t0\t{flags}")).unwrap_err();
            let message = format!("{error:#}");
            assert!(message.contains("row 1"));
            assert!(message.contains("invalid active flag"));
        }
    }

    #[test]
    fn empty_snapshot_is_empty_tree() {
        assert_eq!(parse_tree("").unwrap(), Tree::default());
    }
}
