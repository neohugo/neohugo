//! Module `afero`.
//!
//! NEW: spf13/afero subset: Fs trait, OsFs, BasePathFs, ReadOnlyFs, MemMapFs (tests)
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! `spf13/afero` subset: the file-system abstraction every hugofs layer implements. Paths are
//! slash-separated strings relative to the fs root (OsFs: absolute OS paths).
//! Only the operations Hugo uses are modelled.

use std::any::Any;
use std::io::{Read, Seek, Write};
use std::sync::Arc;

use go_value::Time;
use nh_common::Result;

use crate::fileinfo::FileMetaInfo;

/// Go: `os.O_RDONLY|O_WRONLY|O_RDWR|O_CREATE|O_TRUNC|O_APPEND|O_EXCL`.
pub mod flags {
    pub const O_RDONLY: i32 = 0x0;
    pub const O_WRONLY: i32 = 0x1;
    pub const O_RDWR: i32 = 0x2;
    pub const O_APPEND: i32 = 0x8;
    pub const O_CREATE: i32 = 0x200;
    pub const O_TRUNC: i32 = 0x400;
    pub const O_EXCL: i32 = 0x800;
}

/// Go: `afero.File` (+ `fs.ReadDirFile`).
pub trait File: Read + Write + Seek + Send {
    fn name(&self) -> String;
    fn stat(&self) -> Result<FileMetaInfo>;
    /// Go: `ReadDir(count)`; `count <= 0` reads all. Order is the fs's order (hugofs layers sort).
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>>;
    fn close(&mut self) -> Result<()>;
}

/// Go: `afero.Fs`.
pub trait Fs: Send + Sync {
    fn name(&self) -> &str;
    fn create(&self, name: &str) -> Result<Box<dyn File>>;
    fn mkdir(&self, name: &str, perm: u32) -> Result<()>;
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()>;
    fn open(&self, name: &str) -> Result<Box<dyn File>>;
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>>;
    fn remove(&self, name: &str) -> Result<()>;
    fn remove_all(&self, path: &str) -> Result<()>;
    fn rename(&self, old: &str, new: &str) -> Result<()>;
    fn stat(&self, name: &str) -> Result<FileMetaInfo>;
    fn chmod(&self, name: &str, mode: u32) -> Result<()>;
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()>;
    /// Go: `hugofs.FilesystemUnwrapper` / type switches (`IsOsFs`, `WalkFilesystems`).
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        None
    }
    fn as_any(&self) -> &dyn Any;
}

/// Go: `afero.NewOsFs()`.
pub struct OsFs;

/// Go: `afero.NewBasePathFs(source, path)`.
pub struct BasePathFs {
    pub source: Arc<dyn Fs>,
    pub path: String,
}

/// Go: `afero.NewReadOnlyFs(source)`.
pub struct ReadOnlyFs {
    pub source: Arc<dyn Fs>,
}

/// Go: `afero.NewMemMapFs()` (tests and `renderToMemory`).
#[derive(Default)]
pub struct MemMapFs {
    pub(crate) files: std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>,
}

/// Go: `afero.ReadFile(fs, name)`.
pub fn read_file(fs: &dyn Fs, name: &str) -> Result<Vec<u8>> {
    let mut f = fs.open(name)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

/// Go: `afero.WriteFile(fs, name, data, perm)`.
pub fn write_file(fs: &dyn Fs, name: &str, data: &[u8], perm: u32) -> Result<()> {
    todo!()
}

/// Go: `afero.Exists(fs, path)`.
pub fn exists(fs: &dyn Fs, path: &str) -> Result<bool> {
    todo!()
}

/// Go: `afero.IsDir(fs, path)`.
pub fn is_dir(fs: &dyn Fs, path: &str) -> Result<bool> {
    todo!()
}
