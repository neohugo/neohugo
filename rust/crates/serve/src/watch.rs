//! Watching the project: what is watched (Hugo's watch set), the watcher itself (notify with a
//! 1 s debounce, or polling), and what a batch of events means for the site.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use neohugo_config::Config;
use neohugo_vfs::{Component, Vfs};
use notify::event::{AccessKind, AccessMode, EventKind, MetadataKind, ModifyKind, RenameMode};
use notify::{PollWatcher, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, NoCache, RecommendedCache, new_debouncer_opt,
};

use crate::ServeError;

/// How long events are held back so that one save (or a burst of saves) is one rebuild.
pub(crate) const DEBOUNCE: Duration = Duration::from_secs(1);

/// How often the debouncer looks for events whose time is up.
const TICK: Duration = Duration::from_millis(100);

/// What the watch loop receives.
pub(crate) enum Message {
    Events(DebounceEventResult),
    Shutdown,
}

/// The directories to watch, each with its mode, and what they are watched for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WatchSet {
    pub(crate) roots: BTreeMap<PathBuf, RecursiveMode>,
    /// The mounted directories and files (for the report).
    pub(crate) sources: Vec<PathBuf>,
    /// The configuration files and directory (for the report).
    pub(crate) config: Vec<PathBuf>,
}

impl WatchSet {
    /// Every mount of the project and its themes that is not `disableWatch` (a directory with
    /// everything below it; a mounted file through its directory), the configuration
    /// directory, and the directories of the configuration files. Missing paths are left out.
    pub(crate) fn new(cfg: &Config, vfs: &Vfs, config_dir: &Path) -> Self {
        let mut recursive: BTreeSet<PathBuf> = BTreeSet::new();
        let mut flat: BTreeSet<PathBuf> = BTreeSet::new();
        let mut sources: BTreeSet<PathBuf> = BTreeSet::new();
        for m in vfs.mounts() {
            if m.disable_watch || m.is_disabled() {
                continue;
            }
            if m.abs.is_dir() {
                recursive.insert(m.abs.clone());
                sources.insert(m.abs.clone());
            } else if m.abs.is_file()
                && let Some(parent) = m.abs.parent()
            {
                flat.insert(parent.to_path_buf());
                sources.insert(m.abs.clone());
            }
        }
        let mut config: Vec<PathBuf> = cfg.config_files.clone();
        if config_dir.is_dir() {
            recursive.insert(config_dir.to_path_buf());
            config.push(config_dir.to_path_buf());
        }
        for f in &cfg.config_files {
            if let Some(parent) = f.parent()
                && parent.is_dir()
            {
                flat.insert(parent.to_path_buf());
            }
        }
        // A root inside a recursive root adds nothing.
        let mut roots = BTreeMap::new();
        let covered = |p: &Path, roots: &BTreeMap<PathBuf, RecursiveMode>| {
            roots
                .iter()
                .any(|(r, m)| *m == RecursiveMode::Recursive && p.starts_with(r))
        };
        for p in recursive {
            if !covered(&p, &roots) {
                roots.insert(p, RecursiveMode::Recursive);
            }
        }
        for p in flat {
            if !covered(&p, &roots) {
                roots.insert(p, RecursiveMode::NonRecursive);
            }
        }
        Self {
            roots,
            sources: sources.into_iter().collect(),
            config,
        }
    }
}

/// The file watcher: the platform's notifications or polling, debounced.
pub(crate) enum Watcher {
    Native(Debouncer<RecommendedWatcher, RecommendedCache>),
    Poll(Debouncer<PollWatcher, NoCache>),
}

impl Watcher {
    /// A watcher that sends its batches to `tx`.
    pub(crate) fn new(poll: Option<Duration>, tx: Sender<Message>) -> Result<Self, ServeError> {
        let handler = move |r: DebounceEventResult| {
            let _ = tx.send(Message::Events(r));
        };
        let watcher = match poll {
            None => Self::Native(new_debouncer_opt(
                DEBOUNCE,
                Some(TICK),
                handler,
                RecommendedCache::new(),
                notify::Config::default(),
            )?),
            // notify's poller keeps modification times in whole seconds, so it also compares
            // contents: a second save within the same second is still seen.
            Some(interval) => Self::Poll(new_debouncer_opt(
                DEBOUNCE,
                Some(TICK),
                handler,
                NoCache,
                notify::Config::default()
                    .with_poll_interval(interval)
                    .with_compare_contents(true),
            )?),
        };
        Ok(watcher)
    }

    fn watch(&mut self, path: &Path, mode: RecursiveMode) -> notify::Result<()> {
        match self {
            Self::Native(d) => d.watch(path, mode),
            Self::Poll(d) => d.watch(path, mode),
        }
    }

    fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
        match self {
            Self::Native(d) => d.unwatch(path),
            Self::Poll(d) => d.unwatch(path),
        }
    }

    /// Moves from watching `old` to watching `new`. A path that cannot be watched (removed
    /// meanwhile) is skipped and returned.
    pub(crate) fn update(
        &mut self,
        old: &WatchSet,
        new: &WatchSet,
    ) -> Vec<(PathBuf, notify::Error)> {
        let mut failed = Vec::new();
        for (p, mode) in &old.roots {
            if new.roots.get(p) != Some(mode) {
                let _ = self.unwatch(p);
            }
        }
        for (p, mode) in &new.roots {
            if old.roots.get(p) != Some(mode)
                && let Err(e) = self.watch(p, *mode)
            {
                failed.push((p.clone(), e));
            }
        }
        failed
    }
}

/// What a batch of file events changed.
#[derive(Debug, Default)]
pub(crate) struct Changes {
    /// Configuration files (a configuration change rebuilds everything).
    pub(crate) config: Vec<PathBuf>,
    /// Static files and directories.
    pub(crate) statics: Vec<PathBuf>,
    /// Content files, with whether the event wrote or created them.
    pub(crate) content: Vec<(PathBuf, bool)>,
    /// Everything else of the site: layouts, assets, data, i18n, archetypes.
    pub(crate) other: Vec<PathBuf>,
    /// The watcher lost events: rebuild everything.
    pub(crate) rescan: bool,
}

impl Changes {
    pub(crate) fn is_empty(&self) -> bool {
        self.config.is_empty()
            && self.statics.is_empty()
            && self.content.is_empty()
            && self.other.is_empty()
            && !self.rescan
    }

    /// Whether only static files changed.
    pub(crate) fn static_only(&self) -> bool {
        !self.statics.is_empty()
            && self.config.is_empty()
            && self.content.is_empty()
            && self.other.is_empty()
            && !self.rescan
    }

    /// The changed paths, for the report.
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        let mut all: BTreeSet<PathBuf> = BTreeSet::new();
        all.extend(self.config.iter().cloned());
        all.extend(self.statics.iter().cloned());
        all.extend(self.content.iter().map(|(p, _)| p.clone()));
        all.extend(self.other.iter().cloned());
        all.into_iter().collect()
    }

    fn push(&mut self, kind: Kind, path: PathBuf, written: bool) {
        let list = match kind {
            Kind::Config => &mut self.config,
            Kind::Static => &mut self.statics,
            Kind::Content => {
                match self.content.iter_mut().find(|(p, _)| *p == path) {
                    Some(seen) => seen.1 |= written,
                    None => self.content.push((path, written)),
                }
                return;
            }
            Kind::Other => &mut self.other,
        };
        if !list.contains(&path) {
            list.push(path);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Config,
    Static,
    Content,
    Other,
}

/// A watched mount, as far as events are concerned.
#[derive(Clone, Debug)]
struct WatchedMount {
    abs: PathBuf,
    component: Component,
    watched: bool,
}

/// Sorts file events into [`Changes`] (Hugo's `handleEvents` filters).
#[derive(Clone, Debug)]
pub(crate) struct Classifier {
    mounts: Vec<WatchedMount>,
    config_files: BTreeSet<PathBuf>,
    config_dir: PathBuf,
    /// `hugo_stats.json`, which the build itself writes.
    stats_file: PathBuf,
}

impl Classifier {
    pub(crate) fn new(cfg: &Config, vfs: &Vfs, config_dir: &Path) -> Self {
        Self {
            mounts: vfs
                .mounts()
                .iter()
                .filter(|m| !m.is_disabled())
                .map(|m| WatchedMount {
                    abs: m.abs.clone(),
                    component: m.component,
                    watched: !m.disable_watch,
                })
                .collect(),
            config_files: cfg.config_files.iter().cloned().collect(),
            config_dir: config_dir.to_path_buf(),
            stats_file: cfg.project_dir.join("hugo_stats.json"),
        }
    }

    /// The changes of a batch of debounced events.
    pub(crate) fn classify(&self, events: &[DebouncedEvent]) -> Changes {
        let mut changes = Changes::default();
        for e in events {
            if e.need_rescan() {
                changes.rescan = true;
                continue;
            }
            let kind = &e.kind;
            let wrote = match kind {
                // A write (inotify's close after writing; the poll watcher's newer time).
                EventKind::Access(AccessKind::Close(AccessMode::Write))
                | EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => true,
                // Reads, and permission or time changes (Hugo skips chmod events).
                EventKind::Access(_) | EventKind::Modify(ModifyKind::Metadata(_)) => continue,
                _ => false,
            };
            for (i, path) in e.paths.iter().enumerate() {
                let (written, must_exist) = match kind {
                    _ if wrote => (true, true),
                    EventKind::Create(_) | EventKind::Modify(ModifyKind::Data(_)) => (true, true),
                    EventKind::Modify(ModifyKind::Name(RenameMode::To)) => (true, false),
                    // The second path of a rename is where the file went.
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => (i == 1, false),
                    EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_)) => (false, false),
                    _ => (true, false),
                };
                // A file created or written and gone again (an editor's temporary file).
                if must_exist && !path.exists() {
                    continue;
                }
                if let Some(k) = self.kind(path) {
                    changes.push(k, path.clone(), written);
                }
            }
        }
        changes
    }

    fn kind(&self, path: &Path) -> Option<Kind> {
        if is_ignored(path) || *path == self.stats_file {
            return None;
        }
        if self.config_files.contains(path) || path.starts_with(&self.config_dir) {
            return Some(Kind::Config);
        }
        // The watched mounts holding the path: content wins, then anything but static.
        let mut kind: Option<Kind> = None;
        for m in self
            .mounts
            .iter()
            .filter(|m| m.watched && path.starts_with(&m.abs))
        {
            // Hugo skips these directories when it walks the mounts.
            let below = path.strip_prefix(&m.abs).unwrap_or(path);
            if below.components().any(|c| {
                matches!(c, PathComponent::Normal(n)
                    if n == ".git" || n == "node_modules" || n == "bower_components")
            }) {
                return None;
            }
            let k = match m.component {
                Component::Static => Kind::Static,
                Component::Content => Kind::Content,
                _ => Kind::Other,
            };
            kind = Some(match (kind, k) {
                (None, k) => k,
                (Some(Kind::Content), _) | (_, Kind::Content) => Kind::Content,
                (Some(Kind::Static), Kind::Static) => Kind::Static,
                _ => Kind::Other,
            });
        }
        kind
    }
}

/// Editors' temporary and backup files, and names Hugo ignores (a leading `.` or `#`, a
/// trailing `~`).
fn is_ignored(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let ext = name.rfind('.').map_or("", |i| &name[i..]);
    name.starts_with('.')
        || name.starts_with('#')
        || name.ends_with('~')
        || name == "4913"
        || matches!(ext, ".swp" | ".swx" | ".bck" | ".tmp")
        || ext.starts_with(".goutputstream")
        || ext.starts_with(".sb-")
        || ["jb_old___", "jb_tmp___", "jb_bak___"]
            .iter()
            .any(|s| ext.ends_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_names() {
        for name in [
            ".hugo.toml.swp",
            "one.md~",
            "4913",
            "#one.md#",
            ".#one.md",
            "one.md.tmp",
            "one.md.swx",
            "a.css___jb_tmp___",
            "x.goutputstream-ABC",
            ".DS_Store",
            "one.md.sb-123",
        ] {
            assert!(is_ignored(Path::new(name)), "{name}");
        }
        for name in ["one.md", "site.css", "index.html", "4914", "a~b.md"] {
            assert!(!is_ignored(Path::new(name)), "{name}");
        }
    }
}
