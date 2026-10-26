//! Window-wide floating-pane stash, serialized per tmux server.

use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use anyhow::{Context, Result, ensure};

use super::Outcome;
use crate::tmux::{
    Tmux,
    model::{FloatGeometry, FloatPane, FloatWindow},
};

impl FloatGeometry {
    /// Convert inner tmux coordinates to border-inclusive command flags.
    pub fn capture(pane: &FloatPane) -> Self {
        Self {
            x: pane.left - 1,
            y: pane.top - 1,
            width: pane.width + 2,
            height: pane.height + 2,
        }
    }

    /// Scale and clamp in border-inclusive cells, rounding to the nearest cell.
    pub fn scaled(self, old: (i32, i32), new: (i32, i32)) -> Self {
        let scale = |v: i32, before: i32, after: i32| {
            (f64::from(v) * f64::from(after) / f64::from(before)).round() as i32
        };
        let width = scale(self.width, old.0, new.0).clamp(3, new.0);
        let height = scale(self.height, old.1, new.1).clamp(3, new.1);
        Self {
            width,
            height,
            x: scale(self.x, old.0, new.0).clamp(0, new.0 - width),
            y: scale(self.y, old.1, new.1).clamp(0, new.1 - height),
        }
    }
}

/// OS-owned lock: a crash releases it, unlike a tmux wait-for channel.
fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("cannot open float lock {}", path.display()))?;
    file.lock().context("cannot lock float commands")?;
    Ok(file)
}

struct Source {
    window: String,
    name: String,
    state: FloatWindow,
}

fn resolve(target: Option<&str>, tmux: &dyn Tmux, inside: bool) -> Result<Source> {
    let pane = match target {
        Some(target) => tmux.resolve_pane(target)?,
        None => {
            ensure!(inside, "float requires a target outside tmux");
            tmux.current()?
                .context("could not determine current pane")?
                .pane_id
        }
    };
    let tree = tmux.tree()?;
    let (session, window) = tree
        .sessions
        .iter()
        .find_map(|s| {
            s.windows
                .iter()
                .find(|w| w.panes.iter().any(|p| p.id == pane))
                .map(|w| (s, w))
        })
        .context("target window not found")?;
    ensure!(
        session.name != "pmux" || tmux.option("-w", &window.id, "@pmux_source")?.is_empty(),
        "float commands cannot target a stash window"
    );
    Ok(Source {
        window: window.id.clone(),
        name: format!("{}:{}", session.name, window.index),
        state: tmux.float_window(&window.id)?,
    })
}

/// Return the owned stash session and matching window, rejecting a name collision.
fn find_stash(tmux: &dyn Tmux, source: &str) -> Result<(Option<String>, Option<String>)> {
    let tree = tmux.tree()?;
    let Some(session) = tree.sessions.iter().find(|s| s.name == "pmux") else {
        return Ok((None, None));
    };
    ensure!(
        tmux.option("session", &session.id, "@pmux_stash")? == "1",
        "session pmux is not a pane-mux stash"
    );
    for w in &session.windows {
        if tmux.option("-w", &w.id, "@pmux_source")? == source {
            return Ok((Some(session.id.clone()), Some(w.id.clone())));
        }
    }
    Ok((Some(session.id.clone()), None))
}

fn create_stash(tmux: &dyn Tmux, source: &Source, session: Option<&str>) -> Result<String> {
    let text = format!("{} — pane-mux float unstash {}", source.name, source.window);
    let quoted = format!("'{}'", text.replace('\'', "'\\''"));
    let command = format!("printf '%s\\n' {quoted}; exec tail -f /dev/null");
    let window = tmux.stash_window(
        session,
        &source.name,
        &command,
        source.state.width,
        source.state.height,
    )?;
    let tree = tmux.tree()?;
    let id = &tree
        .sessions
        .iter()
        .find(|s| s.name == "pmux")
        .context("stash session disappeared")?
        .id;
    tmux.set_option("session", id, "@pmux_stash", "1")?;
    tmux.set_option("-w", &window, "@pmux_source", &source.window)?;
    Ok(window)
}

fn command(
    target: Option<&str>,
    tmux: &dyn Tmux,
    operation: impl FnOnce(&Source, &dyn Tmux) -> Result<()>,
) -> Result<Outcome> {
    let inside = std::env::var_os("TMUX").is_some();
    ensure!(
        target.is_some() || inside,
        "float requires a target outside tmux"
    );
    let path = format!("{}.pane-mux-float.lock", tmux.socket_path()?);
    let _guard = lock(Path::new(&path))?;
    let source = resolve(target, tmux, inside)?;
    operation(&source, tmux)?;
    Ok(Outcome::Done)
}

/// Stash all floating panes of the target window.
pub fn run_stash(target: Option<&str>, tmux: &dyn Tmux) -> Result<Outcome> {
    command(target, tmux, stash)
}

fn recorded(p: &FloatPane) -> bool {
    p.options.contains_key("@pmux_w")
}

fn number(p: &FloatPane, key: &str) -> Result<i32> {
    p.options
        .get(key)
        .with_context(|| format!("{} missing {key}", p.id))?
        .parse()
        .with_context(|| format!("{} invalid {key}", p.id))
}

fn stored(p: &FloatPane, size: Option<(i32, i32)>) -> Result<FloatGeometry> {
    let g = FloatGeometry {
        x: number(p, "@pmux_x")?,
        y: number(p, "@pmux_y")?,
        width: number(p, "@pmux_w")?,
        height: number(p, "@pmux_h")?,
    };
    let old = (number(p, "@pmux_win_w")?, number(p, "@pmux_win_h")?);
    ensure!(
        old.0 > 0 && old.1 > 0,
        "invalid stored window size for {}",
        p.id
    );
    Ok(match size {
        Some(size) if size != old => g.scaled(old, size),
        _ => g,
    })
}

fn save(tmux: &dyn Tmux, p: &FloatPane, source: &Source) -> Result<()> {
    let g = FloatGeometry::capture(p);
    for (key, value) in [
        ("@pmux_x", g.x),
        ("@pmux_y", g.y),
        ("@pmux_w", g.width),
        ("@pmux_h", g.height),
        ("@pmux_z", p.z),
        ("@pmux_win_w", source.state.width),
        ("@pmux_win_h", source.state.height),
        ("@pmux_active", i32::from(p.active)),
    ] {
        tmux.set_option("-p", &p.id, key, &value.to_string())?;
    }
    Ok(())
}

fn stash_windows(tmux: &dyn Tmux, source: &str) -> Result<(Option<String>, Vec<String>)> {
    let (session, _) = find_stash(tmux, source)?;
    let mut windows = Vec::new();
    if let Some(id) = &session {
        let tree = tmux.tree()?;
        if let Some(s) = tree.sessions.iter().find(|s| &s.id == id) {
            for w in &s.windows {
                if tmux.option("-w", &w.id, "@pmux_source")? == source {
                    windows.push(w.id.clone());
                }
            }
        }
    }
    Ok((session, windows))
}

fn failures(errors: Vec<String>) -> Result<()> {
    ensure!(
        errors.is_empty(),
        "failed floating panes: {}",
        errors.join("; ")
    );
    Ok(())
}

fn attempt(errors: &mut Vec<String>, pane: &str, f: impl FnOnce() -> Result<()>) {
    if let Err(e) = f() {
        errors.push(format!("{pane}: {e:#}"));
    }
}

fn window_exists(tmux: &dyn Tmux, window: &str) -> Result<bool> {
    Ok(tmux
        .tree()?
        .sessions
        .iter()
        .flat_map(|s| &s.windows)
        .any(|w| w.id == window))
}

/// Refresh after cleanup: dummies must not influence toggle or geometry capture.
fn clean(source: &Source, tmux: &dyn Tmux) -> Result<Source> {
    let (_, mut windows) = stash_windows(tmux, &source.window)?;
    windows.push(source.window.clone());
    for window in windows {
        for pane in tmux.float_window(&window)?.panes {
            if pane.title == "pane-mux-dummy" {
                tmux.kill_pane(&pane.id)?;
            }
        }
    }
    Ok(Source {
        window: source.window.clone(),
        name: source.name.clone(),
        state: tmux.float_window(&source.window)?,
    })
}

fn move_float(tmux: &dyn Tmux, pane: &str, window: &str, g: FloatGeometry) -> Result<()> {
    let dummy = tmux.new_dummy_float(
        window,
        &g.width.to_string(),
        &g.height.to_string(),
        Some(g.x),
        Some(g.y),
    )?;
    let result = tmux.swap_and_kill(pane, &dummy);
    if result.is_err() {
        let _ = tmux.kill_pane(&dummy);
    }
    result
}

fn stash(source: &Source, tmux: &dyn Tmux) -> Result<()> {
    let source = clean(source, tmux)?;
    let (session, windows) = stash_windows(tmux, &source.window)?;
    let mut floats: Vec<_> = source.state.panes.iter().filter(|p| p.floating).collect();
    floats.sort_by_key(|p| std::cmp::Reverse(p.z));
    if floats.is_empty() {
        return tmux.display_message(&source.window, "no floating panes");
    }
    let window = match windows.first() {
        Some(w) => w.clone(),
        None => create_stash(tmux, &source, session.as_deref())?,
    };
    let mut width = source.state.width;
    let mut height = source.state.height;
    for old in &windows {
        for p in tmux.float_window(old)?.panes.iter().filter(|p| recorded(p)) {
            let g = stored(p, None)?;
            width = width.max(g.x.saturating_add(g.width));
            height = height.max(g.y.saturating_add(g.height));
        }
    }
    tmux.resize_window(&window, width, height)?;
    let mut errors = Vec::new();
    for p in floats {
        attempt(&mut errors, &p.id, || {
            save(tmux, p, &source)?;
            move_float(tmux, &p.id, &window, FloatGeometry::capture(p))
        });
    }
    if source.state.zoomed && !tmux.float_window(&source.window)?.zoomed {
        let pane = source
            .state
            .panes
            .iter()
            .find(|p| p.active && !p.floating)
            .context("no zoomed tiled pane")?;
        tmux.toggle_zoom(&pane.id)?;
    }
    failures(errors)
}

/// Restore all stashed panes for the target window.
pub fn run_unstash(target: Option<&str>, tmux: &dyn Tmux) -> Result<Outcome> {
    command(target, tmux, unstash)
}

fn unstash(source: &Source, tmux: &dyn Tmux) -> Result<()> {
    let source = clean(source, tmux)?;
    let (_, windows) = stash_windows(tmux, &source.window)?;
    let mut panes = Vec::new();
    for window in &windows {
        panes.extend(
            tmux.float_window(window)?
                .panes
                .into_iter()
                .filter(|p| p.floating),
        );
    }
    if source.state.zoomed {
        let target = source
            .state
            .panes
            .iter()
            .find(|p| p.active && !p.floating)
            .context("no zoomed tiled pane")?;
        tmux.toggle_zoom(&target.id)?;
    }
    let state = tmux.float_window(&source.window)?;
    ensure!(
        state.width >= 3 && state.height >= 3,
        "source window is too small for floats"
    );
    panes.sort_by_key(|p| std::cmp::Reverse(number(p, "@pmux_z").unwrap_or(p.z)));
    let empty = panes.is_empty();
    let mut errors = Vec::new();
    for p in panes {
        attempt(&mut errors, &p.id, || {
            let g = stored(&p, Some((state.width, state.height)))?;
            move_float(tmux, &p.id, &source.window, g)?;
            if p.options.get("@pmux_active").is_some_and(|v| v == "1") {
                tmux.select_pane(&p.id)?;
            }
            Ok(())
        });
    }
    for window in windows {
        if !window_exists(tmux, &window)? {
            continue;
        }
        let state = tmux.float_window(&window)?;
        if !state.panes.iter().any(|p| p.floating) && state.panes.iter().any(|p| !p.floating) {
            tmux.kill_window(&window)?;
        }
    }
    if empty {
        tmux.display_message(&source.window, "nothing stashed")?;
    }
    failures(errors)
}

/// Stash when any floats exist (including hidden by zoom), otherwise unstash.
pub fn run_toggle(target: Option<&str>, tmux: &dyn Tmux) -> Result<Outcome> {
    command(target, tmux, toggle)
}

fn toggle(source: &Source, tmux: &dyn Tmux) -> Result<()> {
    let source = clean(source, tmux)?;
    if source.state.panes.iter().any(|p| p.floating) {
        stash(&source, tmux)
    } else {
        unstash(&source, tmux)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{Current, FakeTmux, sample_tree};

    fn source(tmux: &dyn Tmux) -> Source {
        resolve(Some("%0"), tmux, false).unwrap()
    }

    fn with_floats() -> FakeTmux {
        let fake = FakeTmux::new(sample_tree(&["work", "a", "b"]));
        for (id, g) in [
            (
                "%1",
                FloatGeometry {
                    x: 119,
                    y: 4,
                    width: 62,
                    height: 17,
                },
            ),
            (
                "%2",
                FloatGeometry {
                    x: 20,
                    y: 10,
                    width: 42,
                    height: 12,
                },
            ),
        ] {
            move_float(&fake, id, "@0", g).unwrap();
        }
        fake
    }

    #[test]
    fn stash_two_merge_empty_focus_zoom() {
        let fake = with_floats();
        let before = source(&fake);
        fake.select_pane("%1").unwrap();
        stash(&source(&fake), &fake).unwrap();
        let src = source(&fake);
        assert_eq!(src.state.panes.len(), 1);
        assert!(src.state.panes[0].active);
        let w = find_stash(&fake, "@0").unwrap().1.unwrap();
        let state = fake.float_window(&w).unwrap();
        assert_eq!(state.panes.len(), 3);
        for p in state.panes.iter().filter(|p| p.floating) {
            let old = before.state.panes.iter().find(|o| o.id == p.id).unwrap();
            assert_eq!(FloatGeometry::capture(p), FloatGeometry::capture(old));
            assert_eq!(
                p.options["@pmux_active"],
                if p.id == "%1" { "1" } else { "0" }
            );
        }
        let new = fake.new_window("$0", None).unwrap();
        let p = fake.float_window(&new).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &p,
            "@0",
            FloatGeometry {
                x: 0,
                y: 0,
                width: 12,
                height: 8,
            },
        )
        .unwrap();
        fake.toggle_zoom("%0").unwrap();
        stash(&source(&fake), &fake).unwrap();
        assert!(source(&fake).state.zoomed);
        assert_eq!(fake.float_window(&w).unwrap().panes.len(), 4);
        assert_eq!(fake.option("-p", "%1", "@pmux_x").unwrap(), "119");
        let empty = FakeTmux::new(sample_tree(&["work"]));
        stash(&source(&empty), &empty).unwrap();
        assert!(find_stash(&empty, "@0").unwrap().1.is_none());
        assert!(
            empty
                .calls()
                .iter()
                .any(|c| c.ends_with("no floating panes"))
        );
    }

    #[test]
    fn stash_without_placeholder_reuses_window_and_unstashes_everything() {
        let fake = with_floats();
        stash(&source(&fake), &fake).unwrap();
        let old = find_stash(&fake, "@0").unwrap().1.unwrap();
        let p = fake
            .float_window(&old)
            .unwrap()
            .panes
            .into_iter()
            .find(|p| !p.floating)
            .unwrap()
            .id;
        fake.kill_pane(&p).unwrap();
        fake.resize_window("@0", 100, 25).unwrap();
        let new = fake.new_window("$0", None).unwrap();
        let id = fake.float_window(&new).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &id,
            "@0",
            FloatGeometry {
                x: 1,
                y: 1,
                width: 20,
                height: 10,
            },
        )
        .unwrap();
        stash(&source(&fake), &fake).unwrap();
        let w = find_stash(&fake, "@0").unwrap().1.unwrap();
        assert_eq!(w, old);
        let state = fake.float_window(&w).unwrap();
        assert!(state.panes.iter().all(|p| p.floating));
        assert_eq!(state.panes.iter().filter(|p| p.floating).count(), 3);
        assert!(state.width >= 181 && state.height >= 22);
        unstash(&source(&fake), &fake).unwrap();
        assert_eq!(
            source(&fake)
                .state
                .panes
                .iter()
                .filter(|p| p.floating)
                .count(),
            3
        );
        assert!(!window_exists(&fake, &old).unwrap());
        assert!(find_stash(&fake, "@0").unwrap().0.is_none());
    }

    #[test]
    fn stash_best_effort_and_retry_after_create_or_swap_failure() {
        for step in ["new-pane -d -A -E -T", "swap-pane -d -Z -s %1"] {
            let fake = with_floats();
            fake.fail_next_matching(step, "injected");
            let e = stash(&source(&fake), &fake).unwrap_err().to_string();
            assert!(e.contains("%1") && e.contains("injected"));
            let w = find_stash(&fake, "@0").unwrap().1.unwrap();
            assert!(
                fake.float_window(&w)
                    .unwrap()
                    .panes
                    .iter()
                    .any(|p| p.id == "%2" && p.floating)
            );
            assert!(
                fake.float_window(&w)
                    .unwrap()
                    .panes
                    .iter()
                    .all(|p| p.title != "pane-mux-dummy")
            );
            assert!(
                source(&fake)
                    .state
                    .panes
                    .iter()
                    .all(|p| p.title != "pane-mux-dummy")
            );
            stash(&source(&fake), &fake).unwrap();
            let state = fake.float_window(&w).unwrap();
            assert_eq!(state.panes.iter().filter(|p| p.floating).count(), 2);
            assert_eq!(source(&fake).state.panes.len(), 1);
        }
    }

    #[test]
    fn unstash_exact_geometry_z_focus_cleanup_and_unchanged_focus() {
        for focus in ["%0", "%1"] {
            let fake = with_floats();
            fake.select_pane(focus).unwrap();
            let before = source(&fake).state;
            stash(&source(&fake), &fake).unwrap();
            unstash(&source(&fake), &fake).unwrap();
            let after = source(&fake).state;
            assert_eq!(after.panes.len(), before.panes.len());
            for p in &before.panes {
                let restored = after.panes.iter().find(|r| r.id == p.id).unwrap();
                assert_eq!(p.active, restored.active);
                if p.floating {
                    assert_eq!(FloatGeometry::capture(restored), FloatGeometry::capture(p));
                    assert_eq!(restored.z, p.z);
                }
            }
            assert!(find_stash(&fake, "@0").unwrap().0.is_none());
            unstash(&source(&fake), &fake).unwrap();
            assert!(fake.calls().iter().any(|c| c.ends_with("nothing stashed")));
        }
    }

    #[test]
    fn unstash_into_window_with_only_floats() {
        let fake = with_floats();
        stash(&source(&fake), &fake).unwrap();
        let temp = fake.new_window("$0", None).unwrap();
        let visible = fake.float_window(&temp).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &visible,
            "@0",
            FloatGeometry {
                width: 20,
                height: 10,
                x: 0,
                y: 0,
            },
        )
        .unwrap();
        fake.kill_pane("%0").unwrap();
        let src = resolve(Some(&visible), &fake, false).unwrap();
        assert!(src.state.panes.iter().all(|p| p.floating));
        unstash(&src, &fake).unwrap();
        let state = fake.float_window("@0").unwrap();
        assert_eq!(state.panes.len(), 3);
        assert!(state.panes.iter().all(|p| p.floating));
        assert_eq!(state.panes.iter().find(|p| p.id == visible).unwrap().z, 2);
        assert!(find_stash(&fake, "@0").unwrap().0.is_none());
    }

    #[test]
    fn unstash_scales_ignores_stash_resize_and_unzooms() {
        let fake = with_floats();
        let before = source(&fake).state;
        stash(&source(&fake), &fake).unwrap();
        let w = find_stash(&fake, "@0").unwrap().1.unwrap();
        fake.resize_window(&w, 80, 20).unwrap();
        fake.resize_window("@0", 100, 25).unwrap();
        fake.toggle_zoom("%0").unwrap();
        unstash(&source(&fake), &fake).unwrap();
        let after = source(&fake).state;
        assert!(!after.zoomed);
        for old in before.panes.iter().filter(|p| p.floating) {
            let p = after.panes.iter().find(|p| p.id == old.id).unwrap();
            assert_eq!(
                FloatGeometry::capture(p),
                FloatGeometry::capture(old).scaled((200, 50), (100, 25))
            );
        }
    }

    #[test]
    fn unstash_keeps_visible_floats_and_other_stashes() {
        let fake = with_floats();
        stash(&source(&fake), &fake).unwrap();
        let other = fake.new_window("$0", Some("other")).unwrap();
        let tiled = fake.float_window(&other).unwrap().panes[0].id.clone();
        let temp = fake.new_window("$0", None).unwrap();
        let float = fake.float_window(&temp).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &float,
            &other,
            FloatGeometry {
                x: 0,
                y: 0,
                width: 20,
                height: 10,
            },
        )
        .unwrap();
        let other_source = resolve(Some(&tiled), &fake, false).unwrap();
        stash(&other_source, &fake).unwrap();
        let temp = fake.new_window("$0", None).unwrap();
        let visible = fake.float_window(&temp).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &visible,
            "@0",
            FloatGeometry {
                x: 0,
                y: 0,
                width: 20,
                height: 10,
            },
        )
        .unwrap();
        unstash(&source(&fake), &fake).unwrap();
        let state = source(&fake).state;
        assert_eq!(state.panes.iter().filter(|p| p.floating).count(), 3);
        assert_eq!(state.panes.iter().find(|p| p.id == visible).unwrap().z, 2);
        assert!(find_stash(&fake, "@0").unwrap().1.is_none());
        assert!(find_stash(&fake, &other).unwrap().1.is_some());
    }

    #[test]
    fn unstash_partial_failure_keeps_stash_and_retry_converges() {
        for step in ["new-pane -d -A -E -T", "swap-pane -d -Z -s %1"] {
            let fake = with_floats();
            fake.select_pane("%1").unwrap();
            stash(&source(&fake), &fake).unwrap();
            fake.fail_next_matching(step, "injected unstash");
            let error = unstash(&source(&fake), &fake).unwrap_err().to_string();
            assert!(error.contains("%1"));
            assert!(find_stash(&fake, "@0").unwrap().1.is_some());
            for window in fake
                .tree()
                .unwrap()
                .sessions
                .iter()
                .flat_map(|s| &s.windows)
            {
                assert!(
                    fake.float_window(&window.id)
                        .unwrap()
                        .panes
                        .iter()
                        .all(|p| p.title != "pane-mux-dummy")
                );
            }
            assert!(
                source(&fake)
                    .state
                    .panes
                    .iter()
                    .any(|p| p.id == "%2" && p.floating)
            );
            unstash(&source(&fake), &fake).unwrap();
            let state = source(&fake).state;
            assert_eq!(state.panes.iter().filter(|p| p.floating).count(), 2);
            assert!(state.panes.iter().find(|p| p.id == "%1").unwrap().active);
            assert!(find_stash(&fake, "@0").unwrap().0.is_none());
        }
    }

    #[test]
    fn toggle_visible_hidden_and_mixed_double_toggle() {
        let fake = with_floats();
        toggle(&source(&fake), &fake).unwrap();
        assert_eq!(source(&fake).state.panes.len(), 1);
        toggle(&source(&fake), &fake).unwrap();
        assert_eq!(source(&fake).state.panes.len(), 3);
        stash(&source(&fake), &fake).unwrap();
        let temp = fake.new_window("$0", None).unwrap();
        let id = fake.float_window(&temp).unwrap().panes[0].id.clone();
        move_float(
            &fake,
            &id,
            "@0",
            FloatGeometry {
                x: 1,
                y: 1,
                width: 20,
                height: 10,
            },
        )
        .unwrap();
        toggle(&source(&fake), &fake).unwrap();
        assert_eq!(source(&fake).state.panes.len(), 1);
        toggle(&source(&fake), &fake).unwrap();
        assert_eq!(
            source(&fake)
                .state
                .panes
                .iter()
                .filter(|p| p.floating)
                .count(),
            3
        );
        assert!(find_stash(&fake, "@0").unwrap().0.is_none());
        let empty = FakeTmux::new(sample_tree(&["work"]));
        toggle(&source(&empty), &empty).unwrap();
        assert!(empty.calls().iter().any(|c| c.ends_with("nothing stashed")));
    }

    #[test]
    fn zoom_hidden_floats_are_stashed_by_stash_and_toggle() {
        for operation in [stash, toggle] {
            let fake = with_floats();
            let before = source(&fake).state;
            fake.toggle_zoom("%0").unwrap();
            let hidden = source(&fake).state;
            assert!(
                hidden
                    .panes
                    .iter()
                    .filter(|p| p.floating)
                    .all(|p| !p.flags.contains('F'))
            );
            operation(&source(&fake), &fake).unwrap();
            let src = source(&fake).state;
            assert!(src.zoomed && src.panes[0].active);
            assert_eq!(src.panes.len(), 1);
            let window = find_stash(&fake, "@0").unwrap().1.unwrap();
            let state = fake.float_window(&window).unwrap();
            for p in before.panes.iter().filter(|p| p.floating) {
                let moved = state.panes.iter().find(|r| r.id == p.id).unwrap();
                assert_eq!(FloatGeometry::capture(p), FloatGeometry::capture(moved));
                assert_eq!(moved.options["@pmux_z"], p.z.to_string());
            }
            toggle(&source(&fake), &fake).unwrap();
            assert!(!source(&fake).state.zoomed);
            assert_eq!(source(&fake).state.panes.len(), 3);
        }
    }

    #[test]
    fn every_action_removes_leftover_dummies_in_source_and_all_stashes() {
        for operation in [stash, unstash, toggle] {
            let fake = with_floats();
            stash(&source(&fake), &fake).unwrap();
            let window = find_stash(&fake, "@0").unwrap().1.unwrap();
            let extra = create_stash(
                &fake,
                &source(&fake),
                find_stash(&fake, "@0").unwrap().0.as_deref(),
            )
            .unwrap();
            let mut dummies = Vec::new();
            for w in ["@0", &window, &extra] {
                dummies.push(fake.new_dummy_float(w, "20", "10", None, None).unwrap());
            }
            operation(&source(&fake), &fake).unwrap();
            let tree = fake.tree().unwrap();
            assert!(
                !tree
                    .sessions
                    .iter()
                    .flat_map(|s| &s.windows)
                    .flat_map(|w| &w.panes)
                    .any(|p| dummies.contains(&p.id))
            );
            assert!(
                source(&fake)
                    .state
                    .panes
                    .iter()
                    .all(|p| p.title != "pane-mux-dummy")
            );
        }
    }

    #[test]
    fn capture_scale_and_clamp() {
        let g = FloatGeometry::capture(&FloatPane {
            width: 60,
            height: 15,
            left: 120,
            top: 5,
            ..Default::default()
        });
        assert_eq!(
            g,
            FloatGeometry {
                x: 119,
                y: 4,
                width: 62,
                height: 17
            }
        );
        assert_eq!(g.scaled((200, 50), (200, 50)), g);
        assert_eq!(
            g.scaled((200, 50), (100, 25)),
            FloatGeometry {
                x: 60,
                y: 2,
                width: 31,
                height: 9
            }
        );
        assert_eq!(
            g.scaled((200, 50), (400, 100)),
            FloatGeometry {
                x: 238,
                y: 8,
                width: 124,
                height: 34
            }
        );
        let huge = FloatGeometry {
            x: 999,
            y: -5,
            width: 900,
            height: 1,
        }
        .scaled((200, 50), (10, 5));
        assert_eq!(
            huge,
            FloatGeometry {
                x: 0,
                y: 0,
                width: 10,
                height: 3
            }
        );
    }

    #[test]
    fn stash_creation_reuse_foreign_and_rename() {
        let fake = FakeTmux::new(sample_tree(&["work"]));
        let src = source(&fake);
        assert_eq!(find_stash(&fake, &src.window).unwrap(), (None, None));
        let w = create_stash(&fake, &src, None).unwrap();
        let (session, found) = find_stash(&fake, &src.window).unwrap();
        assert_eq!(found.as_deref(), Some(w.as_str()));
        fake.rename_window(&src.window, "renamed").unwrap();
        assert_eq!(find_stash(&fake, &src.window).unwrap().1, found);
        let other = fake.new_window("$0", Some("other")).unwrap();
        let p = fake.tree().unwrap().sessions[0]
            .windows
            .iter()
            .find(|w| w.id == other)
            .unwrap()
            .panes[0]
            .id
            .clone();
        let second = resolve(Some(&p), &fake, false).unwrap();
        let next = create_stash(&fake, &second, session.as_deref()).unwrap();
        assert_ne!(next, w);
        let foreign = FakeTmux::new(sample_tree(&["work", "pmux"]));
        let before = foreign.tree().unwrap();
        assert!(find_stash(&foreign, "@0").is_err());
        assert_eq!(foreign.tree().unwrap(), before);
    }

    #[test]
    fn second_lock_blocks_until_release() {
        use std::{sync::mpsc, time::Duration};
        let path = std::env::temp_dir().join(format!("pane-mux-lock-test-{}", std::process::id()));
        let first = lock(&path).unwrap();
        let (tx, rx) = mpsc::channel();
        let path2 = path.clone();
        let thread = std::thread::spawn(move || {
            let _guard = lock(&path2).unwrap();
            tx.send(()).unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
        drop(first);
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        thread.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn targets_explicit_current_unknown_outside_and_stash() {
        let fake = FakeTmux::new(sample_tree(&["work"])).with_current(Current {
            session_id: "$0".into(),
            window_id: "@0".into(),
            pane_id: "%0".into(),
        });
        assert_eq!(resolve(None, &fake, true).unwrap().window, "@0");
        assert_eq!(
            resolve(Some("work:0.0"), &fake, false).unwrap().window,
            "@0"
        );
        assert!(resolve(None, &fake, false).is_err());
        assert!(resolve(Some("nosuch:9"), &fake, true).is_err());
        let w = create_stash(&fake, &source(&fake), None).unwrap();
        let pane = fake.float_window(&w).unwrap().panes[0].id.clone();
        assert!(resolve(Some(&pane), &fake, false).is_err());
    }
}
