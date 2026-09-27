//! Port of `hugofs/walk.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugofs.Walkway`: IgnoreFile filter -> HookPre -> recurse -> HookPost.

use std::sync::Arc;

use nh_common::paths::pathparser::PathParser;
use nh_common::Result;

use crate::afero::Fs;
use crate::fileinfo::FileMetaInfo;

/// Go: `hugofs.WalkFunc`.
pub type WalkFunc<'a> = &'a mut dyn FnMut(&str, &FileMetaInfo) -> Result<()>;
/// Go: `hugofs.WalkHook` (may filter/reorder the dir entries).
pub type WalkHook<'a> = &'a mut dyn FnMut(&FileMetaInfo, &str, Vec<FileMetaInfo>) -> Result<Vec<FileMetaInfo>>;

/// Go: `hugofs.WalkwayConfig`.
pub struct WalkwayConfig<'a> {
    pub fs: Arc<dyn Fs>,
    pub root: String,
    pub path_parser: Option<Arc<PathParser>>,
    pub info: Option<FileMetaInfo>,
    pub dir_entries: Option<Vec<FileMetaInfo>>,
    pub ignore_file: Option<&'a dyn Fn(&str) -> bool>,
    pub hook_pre: Option<WalkHook<'a>>,
    pub walk_fn: Option<WalkFunc<'a>>,
    pub hook_post: Option<WalkHook<'a>>,
    pub fail_on_not_exist: bool,
    pub sort_dir_entries: bool,
}

/// Go: `hugofs.Walkway`.
pub struct Walkway<'a> {
    pub(crate) cfg: WalkwayConfig<'a>,
    pub(crate) walked: bool,
}

impl<'a> Walkway<'a> {
    // Go: hugofs/walk.go:NewWalkway
    pub fn new(cfg: WalkwayConfig<'a>) -> Self {
        Walkway { cfg, walked: false }
    }

    // Go: hugofs/walk.go:Walk
    pub fn walk(&mut self) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/walk.go (225 lines; 3/4 funcs executed)
//   types: (group), Walkway, WalkwayConfig
// EX L72-90: NewWalkway(cfg WalkwayConfig) *Walkway
// EX L92-103: (w *Walkway) Walk() error
//    L106-116: (w *Walkway) checkErr(filename string, err error) bool
// EX L119-225: (w *Walkway) walk(path string, info FileMetaInfo, dirEntries []FileMetaInfo) error
// ---------------------------------------------------------------------------
