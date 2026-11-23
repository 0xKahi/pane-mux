use std::{
    collections::BTreeMap,
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::mux::Tree;
use anyhow::{Context, Result};

type History = BTreeMap<String, Vec<String>>;
const LIMIT: usize = 50;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn default_path() -> Option<PathBuf> {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .filter(|s| !s.is_empty())
                .map(|home| PathBuf::from(home).join(".local/state"))
        })
        .map(|p| p.join("pane-mux/rex-recent.json"))
}

fn load(path: &Path) -> Result<History> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("invalid Rex recency history"),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(History::new()),
        Err(err) => Err(err.into()),
    }
}

pub(super) fn rank(tree: &mut Tree, path: Option<&Path>, server: Option<&str>) {
    let ids = path
        .zip(server)
        .and_then(|(path, server)| load(path).ok()?.remove(server))
        .unwrap_or_default();
    let mut order: Vec<_> = (0..tree.sessions.len()).collect();
    order.sort_by_key(|&i| {
        ids.iter()
            .position(|id| *id == tree.sessions[i].id)
            .unwrap_or(usize::MAX)
    });
    let count = order.len() as u64;
    for (rank, i) in order.into_iter().enumerate() {
        tree.sessions[i].last_attached = count - rank as u64;
    }
}

pub(super) fn record(path: &Path, server: &str, left: Option<&str>, target: &str) -> Result<()> {
    let mut history = load(path)?;
    let ids = history.entry(server.into()).or_default();
    for id in left.into_iter().chain(std::iter::once(target)) {
        ids.retain(|item| item != id);
        ids.insert(0, id.into());
    }
    ids.truncate(LIMIT);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut temp;
    let mut file = loop {
        temp = path.with_extension(format!(
            "tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break file,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    };
    let result = (|| -> Result<()> {
        serde_json::to_writer(&mut file, &history)?;
        file.flush()?;
        drop(file);
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
