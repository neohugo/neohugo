//! Port of `hugofs/walk.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugofs.Walkway`: IgnoreFile filter -> HookPre -> recurse -> HookPost.

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::{Error, ErrorKind};
use nh_common::loggers::Logger;
use nh_common::paths::pathparser::PathParser;

use crate::afero::Fs;
use crate::fileinfo::{FileMetaInfo, dir_entries_to_file_meta_infos};
use crate::fs::NoOpFs;

/// Go: `filepath.SkipDir.Error()`.
pub const SKIP_DIR: &str = "skip this directory";

/// Go: `filepath.SkipDir` — returned by a [`WalkFunc`] or hook to skip a directory.
pub fn skip_dir() -> Error {
    Error::new(SKIP_DIR)
}

/// Go: `err == filepath.SkipDir` (PORTING.md, deviation 3: the sentinel is recognised by its
/// text, as nh-common's error model has no identity).
pub fn is_skip_dir(e: &Error) -> bool {
    e.kind() == ErrorKind::Generic && e.message() == SKIP_DIR && e.pos().is_none()
}

/// Go: `hugofs.WalkFunc`.
pub type WalkFunc<'a> = &'a mut dyn FnMut(&str, &FileMetaInfo) -> Result<()>;
/// Go: `hugofs.WalkHook` (may filter/reorder the dir entries).
pub type WalkHook<'a> =
    &'a mut dyn FnMut(&FileMetaInfo, &str, Vec<FileMetaInfo>) -> Result<Vec<FileMetaInfo>>;

/// Go: `hugofs.WalkwayConfig`.
pub struct WalkwayConfig<'a> {
    /// The filesystem to walk.
    pub fs: Arc<dyn Fs>,
    /// The root to start from in Fs.
    pub root: String,
    /// The logger to use (`loggers.NewDefault()` if `None`).
    pub logger: Option<Logger>,
    pub path_parser: Option<Arc<PathParser>>,
    /// One or both of these may be pre-set: the start info and its dir entries.
    pub info: Option<FileMetaInfo>,
    pub dir_entries: Option<Vec<FileMetaInfo>>,
    pub ignore_file: Option<&'a dyn Fn(&str) -> bool>,
    /// Will be called in order.
    pub hook_pre: Option<WalkHook<'a>>,
    pub walk_fn: Option<WalkFunc<'a>>,
    pub hook_post: Option<WalkHook<'a>>,
    /// If set, return an error if a directory is not found.
    pub fail_on_not_exist: bool,
    /// If set, sort the dir entries by Name before calling the WalkFn, default is ReadDir order.
    pub sort_dir_entries: bool,
}

impl<'a> WalkwayConfig<'a> {
    /// A config with the given fs and walk func and every other field zero (Go's struct literal).
    pub fn new(fs: Arc<dyn Fs>, walk_fn: WalkFunc<'a>) -> Self {
        WalkwayConfig {
            fs,
            root: String::new(),
            logger: None,
            path_parser: None,
            info: None,
            dir_entries: None,
            ignore_file: None,
            hook_pre: None,
            walk_fn: Some(walk_fn),
            hook_post: None,
            fail_on_not_exist: false,
            sort_dir_entries: false,
        }
    }
}

/// Go: `hugofs.Walkway`.
pub struct Walkway<'a> {
    pub(crate) cfg: WalkwayConfig<'a>,
    /// Prevent a walkway to be walked more than once.
    pub(crate) walked: bool,
    logger: Logger,
    path_parser: Arc<PathParser>,
}

impl<'a> Walkway<'a> {
    // Go: hugofs/walk.go:NewWalkway
    pub fn new(cfg: WalkwayConfig<'a>) -> Self {
        let path_parser = cfg
            .path_parser
            .clone()
            .unwrap_or_else(|| Arc::new(nh_media::media::config::default_path_parser()));
        let logger = cfg.logger.clone().unwrap_or_else(Logger::new_default);
        Walkway {
            cfg,
            walked: false,
            logger,
            path_parser,
        }
    }

    // Go: hugofs/walk.go:Walk
    pub fn walk(&mut self) -> Result<()> {
        if self.walked {
            panic!("this walkway is already walked");
        }
        self.walked = true;

        if self.cfg.fs.as_any().is::<NoOpFs>() {
            return Ok(());
        }

        let root = self.cfg.root.clone();
        let info = self.cfg.info.take();
        let dir_entries = self.cfg.dir_entries.take();
        self.walk_path(&root, info, dir_entries)
    }

    // Go: hugofs/walk.go:checkErr
    /// Returns true if the error is handled.
    fn check_err(&self, filename: &str, err: &Error) -> bool {
        if err.is_not_exist() && !self.cfg.fail_on_not_exist {
            // The file may be removed in process.
            // This may be a ERROR situation, but it is not possible
            // to determine as a general case.
            self.logger.warnf(format!(
                "File {} not found, skipping.",
                go_strconv::quote(filename)
            ));
            return true;
        }

        false
    }

    // Go: hugofs/walk.go:walk
    /// Recursively descends path, calling walkFn.
    fn walk_path(
        &mut self,
        path: &str,
        info: Option<FileMetaInfo>,
        dir_entries: Option<Vec<FileMetaInfo>>,
    ) -> Result<()> {
        let path_rel = path
            .strip_prefix(self.cfg.root.as_str())
            .unwrap_or(path)
            .to_string();

        let info = match info {
            Some(info) => info,
            None => match self.cfg.fs.stat(path) {
                Ok(fi) => fi,
                Err(err) => {
                    if path == self.cfg.root && err.is_not_exist() {
                        return Ok(());
                    }
                    if self.check_err(path, &err) {
                        return Ok(());
                    }
                    // Go: fmt.Errorf("walk: stat: %s", err) (not wrapped).
                    return Err(Error::new(format!("walk: stat: {}", err.message())));
                }
            },
        };

        let walk_fn = self
            .cfg
            .walk_fn
            .as_mut()
            .expect("WalkwayConfig.WalkFn must be set");
        if let Err(err) = walk_fn(path, &info) {
            if info.is_dir() && is_skip_dir(&err) {
                return Ok(());
            }
            return Err(err);
        }

        if !info.is_dir() {
            return Ok(());
        }

        let mut dir_entries = match dir_entries {
            Some(d) => d,
            None => {
                let mut f = match self.cfg.fs.open(path) {
                    Ok(f) => f,
                    Err(err) => {
                        if self.check_err(path, &err) {
                            return Ok(());
                        }
                        return Err(Error::new(format!(
                            "walk: open: path: {} filename: {}: {}",
                            go_strconv::quote(path),
                            go_strconv::quote(&info.meta().filename),
                            err.message()
                        )));
                    }
                };
                let fis = f.read_dir(-1);

                let _ = f.close();
                let fis = match fis {
                    Ok(fis) => fis,
                    Err(err) => {
                        if self.check_err(path, &err) {
                            return Ok(());
                        }
                        return Err(err.wrap("walk: Readdir"));
                    }
                };

                let mut dir_entries = dir_entries_to_file_meta_infos(fis);
                for fi in dir_entries.iter_mut() {
                    if fi.meta().path_info.is_none() {
                        let p = self.path_parser.parse(
                            "",
                            &go_path::filepath::join(&[path_rel.as_str(), fi.name()]),
                        );
                        fi.meta_mut().path_info = Some(Arc::new(p));
                    }
                }

                if self.cfg.sort_dir_entries {
                    go_sort::sort_by(&mut dir_entries, |a, b| a.name() < b.name());
                }

                dir_entries
            }
        };

        if let Some(ignore_file) = self.cfg.ignore_file {
            dir_entries.retain(|fi| !ignore_file(&fi.meta().filename));
        }

        if let Some(hook_pre) = self.cfg.hook_pre.as_mut() {
            match hook_pre(&info, path, dir_entries) {
                Ok(d) => dir_entries = d,
                Err(err) => {
                    if is_skip_dir(&err) {
                        return Ok(());
                    }
                    return Err(err);
                }
            }
        }

        for fim in &dir_entries {
            let next_path = go_path::filepath::join(&[path, fim.name()]);
            if let Err(err) = self.walk_path(&next_path, Some(fim.clone()), None)
                && (!fim.is_dir() || !is_skip_dir(&err))
            {
                return Err(err);
            }
        }

        if let Some(hook_post) = self.cfg.hook_post.as_mut()
            && let Err(err) = hook_post(&info, path, dir_entries)
        {
            if is_skip_dir(&err) {
                return Ok(());
            }
            return Err(err);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/walk.go (225 lines; 3/4 funcs executed)
//   types: (group), Walkway, WalkwayConfig
// OK L72-90: NewWalkway(cfg WalkwayConfig) *Walkway
// OK L92-103: (w *Walkway) Walk() error
// OK L106-116: (w *Walkway) checkErr(filename string, err error) bool
// OK L119-225: (w *Walkway) walk(path string, info FileMetaInfo, dirEntries []FileMetaInfo) error
// ---------------------------------------------------------------------------
