//! Module `fsync`.
//!
//! NEW: spf13/fsync@v0.10.1 Syncer (copy-if-different, chmod, mtimes)
//!
//! Owner: Wave B task T25 (commands-cli).

//!
//! Port of `github.com/spf13/fsync` v0.10.1 `Syncer` as used by `copyStaticTo`:
//! recursive copy-if-different (size + mtime, then content), directory creation, optional
//! chmod/mtime sync, optional delete of extraneous destination files (`cleanDestinationDir`),
//! `DeleteFilter` / `ChmodFilter` hooks. Files are compared/copied through the afero ports.

use std::sync::Arc;

use nh_common::Result;
use nh_hugofs::afero::Fs;

/// Go: `fsync.Syncer`.
pub struct Syncer {
    pub delete: bool,
    pub no_times: bool,
    pub no_chmod: bool,
    pub src_fs: Arc<dyn Fs>,
    pub dest_fs: Arc<dyn Fs>,
    /// Go: `ChmodFilter func(dst, src os.FileInfo) bool` (Hugo: skip dirs / preserve dst mode).
    pub chmod_filter: Option<fn(dst_mode: u32, src_mode: u32, is_dir: bool) -> bool>,
    /// Go: `DeleteFilter func(f fsync.FileInfo) bool`.
    pub delete_filter: Option<fn(path: &str, is_dir: bool) -> bool>,
}

impl Syncer {
    /// Go: `(*Syncer).Sync(dst, src string) error`.
    pub fn sync(&self, dst: &str, src: &str) -> Result<()> {
        todo!()
    }
}
