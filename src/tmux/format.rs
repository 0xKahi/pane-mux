//! Tab-separated tmux snapshot format and parser.

use anyhow::{Context, Result, ensure};

use super::model::{Pane, Session, Tree, Window};

/// One row per pane; tabs preserve spaces in names and commands.
pub const PANE_FORMAT: &str = "#{session_id}\t#{session_name}\t#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_current_command}\t#{session_last_attached}\t#{window_zoomed_flag}\t#{pane_floating_flag}\t#{pane_mode}\t#{@pmux_stash}\t#{@pmux_source}";

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
                    stash: row.5,
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

fn parse_row(line: &str) -> Result<(&str, &str, Window, Pane, u64, bool)> {
    let fields: Vec<&str> = line.split('\t').collect();
    ensure!(
        fields.len() == 16,
        "expected 16 tab-separated fields, got {}",
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
        stash_source: (!fields[15].is_empty()).then(|| fields[15].into()),
        panes: Vec::new(),
    };
    let pane = Pane {
        id: fields[6].into(),
        index: fields[7].parse().context("invalid pane index")?,
        active: parse_active(fields[8])?,
        command: fields[9].into(),
        mode: fields[13].into(),
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
    Ok((
        fields[0],
        fields[1],
        window,
        pane,
        last_attached,
        fields[14] == "1",
    ))
}

/// Pane-local metadata used by the stash, including recovery after a failed move.
pub const FLOAT_OPTIONS: &[&str] = &[
    "@pmux_x",
    "@pmux_y",
    "@pmux_w",
    "@pmux_h",
    "@pmux_z",
    "@pmux_win_w",
    "@pmux_win_h",
    "@pmux_active",
];

/// Build the detailed query; each option is read from the pane, not its window.
pub fn float_format() -> String {
    let mut format = "#{window_width}\t#{window_height}\t#{window_zoomed_flag}\t#{pane_id}\t#{pane_floating_flag}\t#{pane_active}\t#{pane_z}\t#{pane_width}\t#{pane_height}\t#{pane_left}\t#{pane_top}".to_owned();
    for option in FLOAT_OPTIONS {
        format.push_str(&format!("\t#{{{option}}}"));
    }
    // Title must be last: splitn below preserves embedded tabs.
    format.push_str("\t#{pane_flags}\t#{pane_title}");
    format
}

/// Parse a window's detailed pane snapshot (empty flags support older tmux).
pub fn parse_float_window(output: &str) -> Result<super::model::FloatWindow> {
    use super::model::{FloatPane, FloatWindow};
    let mut window = FloatWindow::default();
    for line in output.lines() {
        let f: Vec<_> = line.splitn(13 + FLOAT_OPTIONS.len(), '\t').collect();
        ensure!(
            f.len() == 13 + FLOAT_OPTIONS.len(),
            "invalid float pane row"
        );
        let number = |i: usize| -> Result<i32> {
            if f[i].is_empty() {
                Ok(0)
            } else {
                f[i].parse().context("invalid float geometry")
            }
        };
        window.width = number(0)?;
        window.height = number(1)?;
        window.zoomed = parse_active(f[2])?;
        ensure!(f[3].starts_with('%'), "invalid pane id");
        window.panes.push(FloatPane {
            id: f[3].into(),
            flags: f[11 + FLOAT_OPTIONS.len()].into(),
            title: f[12 + FLOAT_OPTIONS.len()].into(),
            floating: !f[4].is_empty() && parse_active(f[4])?,
            active: parse_active(f[5])?,
            z: number(6)?,
            width: number(7)?,
            height: number(8)?,
            left: number(9)?,
            top: number(10)?,
            options: FLOAT_OPTIONS
                .iter()
                .zip(&f[11..11 + FLOAT_OPTIONS.len()])
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        });
    }
    Ok(window)
}

/// Float leaves from window_layout, using inner (pane-format) coordinates.
/// Scan every leaf, independent of the surrounding split/container nesting.
pub fn parse_layout_floats(layout: &str) -> Result<Vec<super::model::FloatPane>> {
    use super::model::FloatPane;
    let mut floats = Vec::new();
    for tail in layout.split("{\"t\":\"p\"").skip(1) {
        let (leaf, _) = tail.split_once('}').context("unterminated layout leaf")?;
        ensure!(!leaf.contains('{'), "invalid layout leaf");
        let fields: std::collections::HashMap<_, _> = leaf
            .split(',')
            .filter_map(|field| field.split_once(':'))
            .map(|(key, value)| (key.trim().trim_matches('"'), value.trim()))
            .collect();
        if !fields.contains_key("z") {
            continue;
        }
        let number = |key: &str| -> Result<i32> {
            fields
                .get(key)
                .context("missing layout geometry")?
                .parse()
                .context("invalid layout geometry")
        };
        let id = fields
            .get("I")
            .context("missing layout pane id")?
            .trim_matches('"');
        ensure!(id.starts_with('%'), "invalid layout pane id");
        floats.push(FloatPane {
            id: id.into(),
            floating: true,
            width: number("w")?,
            height: number("h")?,
            left: number("x")?,
            top: number("y")?,
            z: number("z")?,
            ..Default::default()
        });
    }
    Ok(floats)
}

/// Hidden floats misreport both membership and z while a window is zoomed.
pub fn apply_layout_floats(window: &mut super::model::FloatWindow, layout: &str) -> Result<()> {
    let floats = parse_layout_floats(layout)?;
    for pane in &mut window.panes {
        let entry = floats.iter().find(|entry| entry.id == pane.id);
        pane.floating = entry.is_some();
        if let Some(entry) = entry {
            pane.z = entry.z;
            pane.width = entry.width;
            pane.height = entry.height;
            pane.left = entry.left;
            pane.top = entry.top;
        }
    }
    Ok(())
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
        let tree = parse_tree("$3\twork space\t@5\t1\tmy editor\t1\t%12\t0\t1\tnvim\t100\t1\t0\t\t\t\n$3\twork space\t@5\t1\tmy editor\t1\t%13\t1\t0\tzsh\t200\t1\t1\tcopy-mode\t1\t@3\n$8\tnotes\t@9\t0\tlogs\t1\t%20\t0\t1\ttail\t\t0\t\t\t\t\n$3\twork space\t@6\t2\tshell\t0\t%14\t0\t1\tzsh\t300\t0\t0\t\t\t\n").unwrap();
        assert_eq!(tree.sessions.len(), 2);
        assert!(tree.sessions[0].windows[0].zoomed);
        assert!(tree.sessions[0].windows[0].panes[1].floating);
        assert!(tree.sessions[0].windows[0].panes[1].in_copy_mode());
        assert!(tree.sessions[0].windows[0].panes[0].mode.is_empty());
        assert!(!tree.sessions[0].stash);
        assert!(tree.sessions[0].windows[0].stash_source.is_none());
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
    fn parses_stash_metadata_and_raw_mode() {
        let tree =
            parse_tree("$1\tpmux\t@2\t0\tstash\t1\t%3\t0\t1\tsh\t0\t0\t1\tview-mode\t1\t@9\n")
                .unwrap();
        let session = &tree.sessions[0];
        assert!(session.stash);
        assert_eq!(session.windows[0].stash_source.as_deref(), Some("@9"));
        assert_eq!(session.windows[0].panes[0].mode, "view-mode");
    }

    #[test]
    fn malformed_line_reports_row() {
        let error = parse_tree("not a pane").unwrap_err();
        assert!(format!("{error:#}").contains("row 1"));
        assert!(parse_tree("$0\tx\t@0\tbad\tx\t1\t%0\t0\t1\tsh\t0").is_err());
    }

    #[test]
    fn invalid_last_attached_reports_row() {
        let error =
            parse_tree("$0\tx\t@0\t0\tx\t1\t%0\t0\t1\tsh\tinvalid\t0\t0\t\t\t").unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("row 1"));
        assert!(message.contains("invalid session last attached"));
    }

    #[test]
    fn invalid_zoomed_or_floating_flags_report_row() {
        for flags in ["bad\t0", "0\tbad"] {
            let error = parse_tree(&format!(
                "$0\tx\t@0\t0\tx\t1\t%0\t0\t1\tsh\t0\t{flags}\t\t\t"
            ))
            .unwrap_err();
            let message = format!("{error:#}");
            assert!(message.contains("row 1"));
            assert!(message.contains("invalid active flag"));
        }
    }

    #[test]
    fn detailed_float_rows_and_missing_options() {
        let empty = "\t".repeat(FLOAT_OPTIONS.len());
        let w = parse_float_window(&format!(
            "200\t50\t0\t%1\t\t1\t\t60\t15\t120\t5{empty}\t\t\n"
        ))
        .unwrap();
        assert!(!w.panes[0].floating);
        assert!(w.panes[0].options.is_empty());
        let options = ["119", "4", "62", "17", "2", "200", "50", "1"].join("\t");
        let w = parse_float_window(&format!(
            "200\t50\t1\t%2\t1\t0\t2\t60\t15\t120\t5\t{options}\tFA\ttitle\twith tabs\n"
        ))
        .unwrap();
        assert!(w.zoomed && w.panes[0].floating);
        assert_eq!(w.panes[0].z, 2);
        assert_eq!(w.panes[0].title, "title\twith tabs");
        assert_eq!(w.panes[0].flags, "FA");
        assert_eq!(w.panes[0].options["@pmux_w"], "62");
        assert!(parse_float_window("bad").is_err());
    }

    #[test]
    fn nested_layout_leaves_use_inner_geometry_and_only_z_marks_floats() {
        let layout = r#"{"V":2,"L":{"t":"v","w":200,"h":50,"x":0,"y":0,"c":[{"t":"h","w":200,"h":50,"x":0,"y":0,"c":[{"t":"p","w":100,"h":50,"x":0,"y":0,"i":0,"a":true,"I":"%0"},{"t":"p","w":99,"h":50,"x":101,"y":0,"i":1,"I":"%2"}]},{"t":"p","w":60,"h":15,"x":120,"y":5,"i":3,"z":0,"I":"%1"}]}}"#;
        let floats = parse_layout_floats(layout).unwrap();
        assert_eq!(floats.len(), 1);
        assert_eq!(floats[0].id, "%1");
        assert_eq!(
            (
                floats[0].width,
                floats[0].height,
                floats[0].left,
                floats[0].top,
                floats[0].z
            ),
            (60, 15, 120, 5, 0)
        );
    }

    #[test]
    fn zoomed_layout_overrides_membership_geometry_and_bogus_z() {
        let layout = r#"{"V":2,"L":{"t":"v","w":200,"h":50,"x":0,"y":0,"c":[{"t":"p","w":200,"h":50,"x":0,"y":0,"i":0,"a":true,"I":"%0"},{"t":"p","w":60,"h":15,"x":120,"y":5,"i":1,"z":1,"I":"%1"},{"t":"p","w":38,"h":10,"x":10,"y":8,"i":2,"z":0,"I":"%2"}]}}"#;
        let mut window = super::super::model::FloatWindow {
            zoomed: true,
            panes: vec![
                super::super::model::FloatPane {
                    id: "%1".into(),
                    z: 2,
                    title: "keep".into(),
                    ..Default::default()
                },
                super::super::model::FloatPane {
                    id: "%0".into(),
                    floating: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        apply_layout_floats(&mut window, layout).unwrap();
        let pane = &window.panes[0];
        assert!(pane.floating);
        assert_eq!(
            (pane.z, pane.width, pane.height, pane.left, pane.top),
            (1, 60, 15, 120, 5)
        );
        assert_eq!(pane.title, "keep");
        assert!(!window.panes[1].floating);
        assert!(parse_layout_floats(r#"{"t":"p","z":0}"#).is_err());
    }

    #[test]
    fn empty_snapshot_is_empty_tree() {
        assert_eq!(parse_tree("").unwrap(), Tree::default());
    }
}
