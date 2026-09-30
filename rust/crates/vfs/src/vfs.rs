//! The union file view of each component.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use neohugo_base::{Idx, LangIdx};
use neohugo_config::Config;

use crate::filter::IgnoreRules;
use crate::mount::{self, Mount};
use crate::{Component, VfsError};

/// A file of a component's union view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRef {
    pub component: Component,
    /// The path inside the component: `/`-separated, original case, no leading slash
    /// (`posts/My Post.md`, `vendor/jquery/dist/jquery.js`).
    pub rel: String,
    pub abs: PathBuf,
    /// The language of the mount (`None`: the default language).
    pub mount_lang: Option<LangIdx>,
    /// The index of the mount in [`Vfs::mounts`].
    pub mount_idx: u16,
}

/// Which files of different mounts with the same path a component keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Union {
    /// The first mount's file: layouts, assets, archetypes, static.
    FirstWins,
    /// The first mount's file per mount language: content (and static of multihost sites).
    PerLanguage,
    /// Every file: data and i18n merge their files themselves.
    KeepAll,
}

/// The mounts of a project and the union file view of each component: for every component the
/// project's mounts come first, then the themes', and the first mount holding a path wins.
#[derive(Clone, Debug)]
pub struct Vfs {
    mounts: Vec<Mount>,
    ignore: IgnoreRules,
    multihost: bool,
}

impl Vfs {
    /// The mounts of `cfg`: `[[module.mounts]]` (a missing source is skipped), then the default
    /// mount of each component no mount targets (per language for content, from
    /// `languages.<lang>.contentDir`; per static dir for static), the root JS config files
    /// (`assets/_jsconfig`), and each theme's component directories.
    ///
    /// # Errors
    /// A mount with an unknown language or an invalid glob, an invalid `ignoreFiles` pattern,
    /// or a missing theme.
    pub fn new(cfg: &Config) -> Result<Self, VfsError> {
        let ignore = IgnoreRules::new(&cfg.ignore_files)
            .map_err(|(pattern, error)| VfsError::IgnorePattern { pattern, error })?;
        Ok(Self {
            mounts: mount::mounts(cfg)?,
            ignore,
            multihost: cfg.multihost,
        })
    }

    /// Every mount, in precedence order.
    #[must_use]
    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    /// The mounts of component `c` with their indices, in precedence order.
    pub fn mounts_of(&self, c: Component) -> impl Iterator<Item = (u16, &Mount)> + '_ {
        self.mounts
            .iter()
            .enumerate()
            .filter(move |(_, m)| m.component == c && !m.is_disabled())
            .map(|(i, m)| (u16::try_from(i).unwrap_or(u16::MAX), m))
    }

    fn union(&self, c: Component) -> Union {
        match c {
            Component::Content => Union::PerLanguage,
            Component::Static if self.multihost => Union::PerLanguage,
            Component::Data | Component::I18n => Union::KeepAll,
            _ => Union::FirstWins,
        }
    }

    /// Every file of component `c`, sorted by path (byte order) and, for one path, by mount
    /// precedence. The ignore rules and the mounts' file filters are applied, and symbolic
    /// links below a mount root are skipped. Files hidden by an earlier mount are left out,
    /// except for data and i18n, which keep every file.
    ///
    /// # Errors
    /// A directory that cannot be read, or a file name that is not UTF-8.
    pub fn walk(&self, c: Component) -> Result<Vec<FileRef>, VfsError> {
        let mut files = Vec::new();
        for (idx, m) in self.mounts_of(c) {
            let meta = match fs::metadata(&m.abs) {
                Ok(meta) => meta,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(VfsError::io(&m.abs, e)),
            };
            if meta.is_file() {
                let rel = file_mount_rel(m);
                let name = rel.rsplit('/').next().unwrap_or_default();
                if !self.ignore.skips(c, name, &m.abs, false) {
                    files.push(file_ref(m, idx, rel, m.abs.clone()));
                }
            } else {
                self.walk_dir(m, idx, &m.abs, "", &mut files)?;
            }
        }
        files.sort_by(|a, b| a.rel.cmp(&b.rel));

        let union = self.union(c);
        if union != Union::KeepAll {
            let default = LangIdx::from_index(0);
            let mut kept: Vec<FileRef> = Vec::with_capacity(files.len());
            let mut group_start = 0;
            for f in files {
                if kept.get(group_start).is_none_or(|g| g.rel != f.rel) {
                    group_start = kept.len();
                }
                let hidden = kept[group_start..].iter().any(|k| {
                    union == Union::FirstWins
                        || k.mount_lang.unwrap_or(default) == f.mount_lang.unwrap_or(default)
                });
                if !hidden {
                    kept.push(f);
                }
            }
            files = kept;
        }
        Ok(files)
    }

    /// Walks `dir`, the directory at `below` (`""` or `/a/b`) under mount `m`'s source.
    fn walk_dir(
        &self,
        m: &Mount,
        idx: u16,
        dir: &Path,
        below: &str,
        out: &mut Vec<FileRef>,
    ) -> Result<(), VfsError> {
        let mut entries = Vec::new();
        for e in fs::read_dir(dir).map_err(|e| VfsError::io(dir, e))? {
            let e = e.map_err(|e| VfsError::io(dir, e))?;
            let ft = e.file_type().map_err(|err| VfsError::io(e.path(), err))?;
            let name = e
                .file_name()
                .into_string()
                .map_err(|_| VfsError::NonUtf8 { path: e.path() })?;
            entries.push((name, ft));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, ft) in entries {
            if ft.is_symlink() {
                continue;
            }
            let abs = dir.join(&name);
            let below = format!("{below}/{name}");
            if !self.admits(m, &name, &below, &abs, ft.is_dir()) {
                continue;
            }
            if ft.is_dir() {
                self.walk_dir(m, idx, &abs, &below, out)?;
            } else if ft.is_file() {
                let rel = join_rel(&m.sub, &below[1..]);
                out.push(file_ref(m, idx, rel, abs));
            }
        }
        Ok(())
    }

    fn admits(&self, m: &Mount, name: &str, below: &str, abs: &Path, is_dir: bool) -> bool {
        !self.ignore.skips(m.component, name, abs, is_dir)
            && m.filter.as_ref().is_none_or(|f| f.admits(below, is_dir))
    }

    /// The file at `rel` (a path inside component `c`) in the first mount that has it, with the
    /// same rules as [`Vfs::walk`].
    #[must_use]
    pub fn open(&self, c: Component, rel: &str) -> Option<FileRef> {
        let rel = rel.trim_start_matches('/');
        self.mounts_of(c).find_map(|(idx, m)| {
            let meta = fs::metadata(&m.abs).ok()?;
            if meta.is_file() {
                let own = file_mount_rel(m);
                return (own == rel).then(|| file_ref(m, idx, own, m.abs.clone()));
            }
            let rest = if m.sub.is_empty() {
                rel
            } else {
                rel.strip_prefix(m.sub.as_str())?.strip_prefix('/')?
            };
            let mut abs = m.abs.clone();
            let mut below = String::new();
            let segments: Vec<&str> = rest.split('/').collect();
            for (i, seg) in segments.iter().enumerate() {
                if seg.is_empty() || *seg == "." || *seg == ".." {
                    return None;
                }
                let is_dir = i + 1 < segments.len();
                abs.push(seg);
                below.push('/');
                below.push_str(seg);
                let ft = fs::symlink_metadata(&abs).ok()?.file_type();
                if ft.is_symlink()
                    || ft.is_dir() != is_dir
                    || !self.admits(m, seg, &below, &abs, is_dir)
                {
                    return None;
                }
            }
            Some(file_ref(m, idx, rel.to_owned(), abs))
        })
    }
}

fn file_ref(m: &Mount, idx: u16, rel: String, abs: PathBuf) -> FileRef {
    FileRef {
        component: m.component,
        rel,
        abs,
        mount_lang: m.lang_idx(),
        mount_idx: idx,
    }
}

/// The path of a mounted single file inside its component: the target below the component, or
/// the file name when the target is the component itself.
fn file_mount_rel(m: &Mount) -> String {
    if m.sub.is_empty() {
        m.abs
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        m.sub.clone()
    }
}

fn join_rel(sub: &str, rest: &str) -> String {
    if sub.is_empty() {
        rest.to_owned()
    } else {
        format!("{sub}/{rest}")
    }
}
