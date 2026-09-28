//! Module `afero`.
//!
//! NEW: spf13/afero subset: Fs trait, OsFs, BasePathFs, ReadOnlyFs, MemMapFs (tests)
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! `spf13/afero@v1.14.0` subset: the file-system abstraction every hugofs layer implements.
//! Paths are Go `filepath` strings (OsFs: OS paths).
//!
//! Go composes filesystems by struct embedding: a wrapper inherits every `afero.Fs` method it
//! does not override from the embedded fs. [`Fs::embedded`] is that embedded fs; the default
//! method bodies forward to it. Interface assertions Go makes on filesystems
//! (`FilesystemUnwrapper`, `FilesystemsUnwrapper`, `overlayfs.FilesystemIterator`,
//! `ReverseLookupProvder`) are the optional methods at the end of [`Fs`]; they are never
//! inherited through embedding, as in Go.

use std::any::Any;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use go_value::Time;
use nh_common::Result;
use nh_common::herrors::Error;

use crate::fileinfo::{FileMetaInfo, mode};
use crate::oserror;
use crate::rootmapping_fs::ReverseLookupProvider;

/// Go: `os.O_RDONLY|O_WRONLY|O_RDWR|O_CREATE|O_TRUNC|O_APPEND|O_EXCL` (symbolic; the OsFs maps
/// them to the platform's open flags).
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
    /// Go: `Readdirnames(n)`.
    fn readdirnames(&mut self, count: i32) -> Result<Vec<String>> {
        Ok(self
            .read_dir(count)?
            .iter()
            .map(|fi| fi.name().to_string())
            .collect())
    }
    fn close(&mut self) -> Result<()>;
}

fn unsupported() -> Error {
    Error::new("neohugo-rs: operation not supported by this filesystem")
}

/// Go: `afero.Fs`.
pub trait Fs: Send + Sync {
    fn name(&self) -> &str;
    /// The embedded `afero.Fs` whose methods this fs inherits (Go struct embedding).
    fn embedded(&self) -> Option<&dyn Fs> {
        None
    }
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        self.embedded().ok_or_else(unsupported)?.create(name)
    }
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        self.embedded().ok_or_else(unsupported)?.mkdir(name, perm)
    }
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        self.embedded()
            .ok_or_else(unsupported)?
            .mkdir_all(path, perm)
    }
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.embedded().ok_or_else(unsupported)?.open(name)
    }
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        self.embedded()
            .ok_or_else(unsupported)?
            .open_file(name, flag, perm)
    }
    fn remove(&self, name: &str) -> Result<()> {
        self.embedded().ok_or_else(unsupported)?.remove(name)
    }
    fn remove_all(&self, path: &str) -> Result<()> {
        self.embedded().ok_or_else(unsupported)?.remove_all(path)
    }
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        self.embedded().ok_or_else(unsupported)?.rename(old, new)
    }
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        self.embedded().ok_or_else(unsupported)?.stat(name)
    }
    fn chmod(&self, name: &str, mode: u32) -> Result<()> {
        self.embedded().ok_or_else(unsupported)?.chmod(name, mode)
    }
    fn chown(&self, name: &str, uid: i32, gid: i32) -> Result<()> {
        self.embedded()
            .ok_or_else(unsupported)?
            .chown(name, uid, gid)
    }
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()> {
        self.embedded()
            .ok_or_else(unsupported)?
            .chtimes(name, atime, mtime)
    }
    /// Go: `hugofs.FilesystemUnwrapper`.
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        None
    }
    /// Go: `hugofs.FilesystemsUnwrapper`.
    fn unwrap_filesystems(&self) -> Option<Vec<Arc<dyn Fs>>> {
        None
    }
    /// Go: `overlayfs.FilesystemIterator` (`Filesystem(i)` for `i < NumFilesystems()`).
    fn filesystem_iterator(&self) -> Option<Vec<Arc<dyn Fs>>> {
        None
    }
    /// Go: `hugofs.ReverseLookupProvder`.
    fn reverse_lookup_provider(&self) -> Option<&dyn ReverseLookupProvider> {
        None
    }
    fn as_any(&self) -> &dyn Any;
}

/// Implements `Read`/`Write`/`Seek` for a directory type by failing like Go's
/// `noOpRegularFileOps` (Go panics with `errNoOp`; the port returns the error).
macro_rules! no_regular_file_ops {
    ($t:ty) => {
        impl std::io::Read for $t {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("this operation is not supported"))
            }
        }
        impl std::io::Write for $t {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("this operation is not supported"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl std::io::Seek for $t {
            fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
                Err(std::io::Error::other("this operation is not supported"))
            }
        }
    };
}
pub(crate) use no_regular_file_ops;

/// Implements `Read`/`Write`/`Seek` by forwarding to the embedded file field.
macro_rules! forward_file_io {
    ($t:ty, $f:ident) => {
        impl std::io::Read for $t {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.$f.read(buf)
            }
        }
        impl std::io::Write for $t {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.$f.write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.$f.flush()
            }
        }
        impl std::io::Seek for $t {
            fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
                self.$f.seek(pos)
            }
        }
    };
}
pub(crate) use forward_file_io;

// ---------------------------------------------------------------------------
// OsFs

/// Go: `afero.NewOsFs()`.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsFs;

/// Go: `afero.NewOsFs()`.
// Go: afero/os.go:NewOsFs
pub fn new_os_fs() -> Arc<dyn Fs> {
    Arc::new(OsFs)
}

/// Go: `os` `basename` (the `Name()` of an `os.Stat` result).
fn os_basename(name: &str) -> String {
    let b = name.as_bytes();
    let mut i = b.len();
    // Remove trailing slashes.
    while i > 0 && b[i - 1] == b'/' {
        i -= 1;
    }
    let name = &name[..i];
    // Remove leading directory name.
    match name.rfind('/') {
        Some(j) => name[j + 1..].to_string(),
        None => name.to_string(),
    }
}

/// Go: `fillFileStatFromSys` (the `fs.FileMode` of a `stat_t`).
#[cfg(unix)]
fn go_file_mode(md: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    let m = md.mode();
    let mut mode = m & 0o777;
    const S_IFMT: u32 = 0o170000;
    match m & S_IFMT {
        0o060000 => mode |= mode::MODE_DEVICE,
        0o020000 => mode |= mode::MODE_DEVICE | mode::MODE_CHAR_DEVICE,
        0o040000 => mode |= mode::MODE_DIR,
        0o010000 => mode |= mode::MODE_NAMED_PIPE,
        0o120000 => mode |= mode::MODE_SYMLINK,
        0o140000 => mode |= mode::MODE_SOCKET,
        _ => {}
    }
    if m & 0o2000 != 0 {
        mode |= mode::MODE_SETGID;
    }
    if m & 0o4000 != 0 {
        mode |= mode::MODE_SETUID;
    }
    if m & 0o1000 != 0 {
        mode |= mode::MODE_STICKY;
    }
    mode
}

#[cfg(unix)]
fn go_mod_time(md: &std::fs::Metadata) -> Time {
    use std::os::unix::fs::MetadataExt;
    Time::from_unix(md.mtime(), md.mtime_nsec(), None)
}

/// A Go `os.FileInfo` from `os.Stat`/`os.Lstat`.
fn os_file_info(name: &str, md: &std::fs::Metadata) -> FileMetaInfo {
    FileMetaInfo::new_plain(
        &os_basename(name),
        md.is_dir(),
        md.len() as i64,
        go_file_mode(md),
        go_mod_time(md),
    )
}

/// Go: `os.Stat(name)`.
pub fn os_stat(name: &str) -> Result<FileMetaInfo> {
    match std::fs::metadata(name) {
        Ok(md) => Ok(os_file_info(name, &md)),
        Err(e) => Err(oserror::from_io("stat", name, &e)),
    }
}

/// Go: `os.Lstat(name)`.
pub fn os_lstat(name: &str) -> Result<FileMetaInfo> {
    match std::fs::symlink_metadata(name) {
        Ok(md) => Ok(os_file_info(name, &md)),
        Err(e) => Err(oserror::from_io("lstat", name, &e)),
    }
}

/// Go: `*os.File`.
pub struct OsFile {
    name: String,
    file: Option<std::fs::File>,
    /// Remaining directory entries (read on the first `ReadDir`).
    dir_entries: Option<std::collections::VecDeque<FileMetaInfo>>,
}

impl OsFile {
    fn file(&mut self) -> io::Result<&mut std::fs::File> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other(oserror::ERR_CLOSED))
    }

    // Go: os/dir_unix.go:(*File).readdir (mode readdirDirEntry)
    fn read_all_entries(&self) -> Result<std::collections::VecDeque<FileMetaInfo>> {
        let rd = std::fs::read_dir(&self.name)
            .map_err(|e| oserror::from_io("readdirent", &self.name, &e))?;
        let mut out = std::collections::VecDeque::new();
        for de in rd {
            let de = de.map_err(|e| oserror::from_io("readdirent", &self.name, &e))?;
            let name = de.file_name().to_string_lossy().into_owned();
            // Go: IsDir from d_type (lstat when the type is unknown); Info() is an lstat.
            let is_dir = match de.file_type() {
                Ok(t) => t.is_dir(),
                Err(_) => false,
            };
            let full = format!("{}/{}", self.name.trim_end_matches('/'), name);
            let fi = match de.metadata() {
                Ok(md) => {
                    let mut fi = os_file_info(&name, &md);
                    fi.is_dir = is_dir;
                    fi
                }
                Err(e) => {
                    let mut fi = FileMetaInfo::new_plain(&name, is_dir, 0, 0, Time::zero());
                    fi.info_err = Some(oserror::from_io("lstat", &full, &e));
                    fi
                }
            };
            out.push_back(fi);
        }
        Ok(out)
    }
}

impl Read for OsFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file()?.read(buf)
    }
}

impl Write for OsFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file()?.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file()?.flush()
    }
}

impl Seek for OsFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file()?.seek(pos)
    }
}

impl File for OsFile {
    fn name(&self) -> String {
        self.name.clone()
    }

    // Go: os/stat_unix.go:(*File).Stat
    fn stat(&self) -> Result<FileMetaInfo> {
        let f = self.file.as_ref().ok_or_else(|| {
            oserror::path_error("stat", &self.name, &Error::new(oserror::ERR_CLOSED))
        })?;
        match f.metadata() {
            Ok(md) => Ok(os_file_info(&self.name, &md)),
            Err(e) => Err(oserror::from_io("stat", &self.name, &e)),
        }
    }

    // Go: os/dir.go:(*File).ReadDir
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        if self.file.is_none() {
            return Err(oserror::path_error(
                "readdirent",
                &self.name,
                &Error::new(oserror::ERR_CLOSED),
            ));
        }
        if self.dir_entries.is_none() {
            self.dir_entries = Some(self.read_all_entries()?);
        }
        let entries = self.dir_entries.as_mut().unwrap();
        if count <= 0 {
            return Ok(entries.drain(..).collect());
        }
        if entries.is_empty() {
            return Err(crate::oserror::eof());
        }
        let n = (count as usize).min(entries.len());
        Ok(entries.drain(..n).collect())
    }

    fn close(&mut self) -> Result<()> {
        if self.file.take().is_none() {
            return Err(oserror::path_error(
                "close",
                &self.name,
                &Error::new(oserror::ERR_CLOSED),
            ));
        }
        Ok(())
    }
}

/// Go: `os.OpenFile(name, flag, perm)`.
pub fn os_open_file(name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut o = std::fs::OpenOptions::new();
    let acc = flag & 0x3;
    o.read(acc == flags::O_RDONLY || acc == flags::O_RDWR);
    o.write(acc == flags::O_WRONLY || acc == flags::O_RDWR);
    if flag & flags::O_APPEND != 0 {
        o.append(true);
    }
    if flag & flags::O_CREATE != 0 {
        if flag & flags::O_EXCL != 0 {
            o.create_new(true);
        } else {
            o.create(true);
        }
    }
    if flag & flags::O_TRUNC != 0 {
        o.truncate(true);
    }
    // Go's syscallMode: the permission bits plus setuid/setgid/sticky.
    let mut m = perm & 0o777;
    if perm & mode::MODE_SETUID != 0 {
        m |= 0o4000;
    }
    if perm & mode::MODE_SETGID != 0 {
        m |= 0o2000;
    }
    if perm & mode::MODE_STICKY != 0 {
        m |= 0o1000;
    }
    o.mode(m);
    match o.open(name) {
        Ok(f) => Ok(Box::new(OsFile {
            name: name.to_string(),
            file: Some(f),
            dir_entries: None,
        })),
        Err(e) => Err(oserror::from_io("open", name, &e)),
    }
}

impl Fs for OsFs {
    fn name(&self) -> &str {
        "OsFs"
    }
    // Go: afero/os.go:OsFs.Create (os.Create: O_RDWR|O_CREATE|O_TRUNC, 0666)
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        os_open_file(
            name,
            flags::O_RDWR | flags::O_CREATE | flags::O_TRUNC,
            0o666,
        )
    }
    // Go: afero/os.go:OsFs.Mkdir
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(perm & 0o7777)
            .create(name)
            .map_err(|e| oserror::from_io("mkdir", name, &e))
    }
    // Go: afero/os.go:OsFs.MkdirAll (os.MkdirAll)
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        // Fast path: if we can tell whether path is a directory or file, stop with success or error.
        if let Ok(md) = std::fs::metadata(path) {
            if md.is_dir() {
                return Ok(());
            }
            return Err(oserror::path_error("mkdir", path, &oserror::enotdir()));
        }
        // Slow path: make sure parent exists and then call Mkdir for path.
        let b = path.as_bytes();
        let mut i = b.len();
        while i > 0 && b[i - 1] == b'/' {
            i -= 1;
        }
        let mut j = i;
        while j > 0 && b[j - 1] != b'/' {
            j -= 1;
        }
        if j > 1 {
            self.mkdir_all(&path[..j - 1], perm)?;
        }
        match self.mkdir(path, perm) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Handle arguments like "foo/." by double-checking that directory doesn't exist.
                if let Ok(md) = std::fs::symlink_metadata(path)
                    && md.is_dir()
                {
                    return Ok(());
                }
                Err(e)
            }
        }
    }
    // Go: afero/os.go:OsFs.Open (os.Open)
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        os_open_file(name, flags::O_RDONLY, 0)
    }
    // Go: afero/os.go:OsFs.OpenFile
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        os_open_file(name, flag, perm)
    }
    // Go: afero/os.go:OsFs.Remove
    fn remove(&self, name: &str) -> Result<()> {
        match std::fs::remove_file(name) {
            Ok(()) => Ok(()),
            Err(e) => match std::fs::remove_dir(name) {
                Ok(()) => Ok(()),
                Err(e1) => {
                    // Go: prefer the unlink error unless it is EISDIR.
                    if e.raw_os_error() == Some(21) {
                        Err(oserror::from_io("remove", name, &e1))
                    } else {
                        Err(oserror::from_io("remove", name, &e))
                    }
                }
            },
        }
    }
    // Go: afero/os.go:OsFs.RemoveAll
    fn remove_all(&self, path: &str) -> Result<()> {
        match std::fs::symlink_metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(oserror::from_io("unlinkat", path, &e)),
            Ok(md) => {
                let r = if md.is_dir() {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                };
                r.map_err(|e| oserror::from_io("unlinkat", path, &e))
            }
        }
    }
    // Go: afero/os.go:OsFs.Rename
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        std::fs::rename(old, new).map_err(|e| {
            Error::with_kind(
                oserror::io_errno(&e).kind(),
                format!("rename {old} {new}: {}", oserror::io_errno(&e).message()),
            )
        })
    }
    // Go: afero/os.go:OsFs.Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        os_stat(name)
    }
    // Go: afero/os.go:OsFs.Chmod
    fn chmod(&self, name: &str, m: u32) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(name, std::fs::Permissions::from_mode(m & 0o7777))
            .map_err(|e| oserror::from_io("chmod", name, &e))
    }
    fn chown(&self, name: &str, _uid: i32, _gid: i32) -> Result<()> {
        Err(oserror::path_error(
            "chown",
            name,
            &Error::new("neohugo-rs: chown is not supported"),
        ))
    }
    // Go: afero/os.go:OsFs.Chtimes
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()> {
        let to_st = |t: &Time| {
            std::time::UNIX_EPOCH + std::time::Duration::new(t.unix_sec.max(0) as u64, t.nsec)
        };
        let f = std::fs::File::options()
            .write(true)
            .open(name)
            .or_else(|_| std::fs::File::open(name))
            .map_err(|e| oserror::from_io("chtimes", name, &e))?;
        f.set_times(
            std::fs::FileTimes::new()
                .set_accessed(to_st(atime))
                .set_modified(to_st(mtime)),
        )
        .map_err(|e| oserror::from_io("chtimes", name, &e))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// BasePathFs

/// Go: `afero.NewBasePathFs(source, path)`.
pub struct BasePathFs {
    pub source: Arc<dyn Fs>,
    pub path: String,
}

impl BasePathFs {
    // Go: afero/basepath.go:NewBasePathFs
    pub fn new(source: Arc<dyn Fs>, path: &str) -> BasePathFs {
        BasePathFs {
            source,
            path: path.to_string(),
        }
    }

    /// Go: `RealPath(name)` — on a file outside the base path it returns the given file name and
    /// an error, else the given file with the base path prepended.
    // Go: afero/basepath.go:RealPath
    pub fn real_path(&self, name: &str) -> std::result::Result<String, (String, Error)> {
        let bpath = go_path::filepath::clean(&self.path);
        let path = go_path::filepath::clean(&go_path::filepath::join(&[bpath.as_str(), name]));
        if !path.starts_with(&bpath) {
            return Err((name.to_string(), oserror::err_not_exist()));
        }
        Ok(path)
    }

    fn real(&self, op: &str, name: &str) -> Result<String> {
        self.real_path(name)
            .map_err(|(n, e)| oserror::path_error(op, &n, &e))
    }

    fn wrap(&self, f: Box<dyn File>) -> Box<dyn File> {
        Box::new(BasePathFile {
            file: f,
            path: self.path.clone(),
        })
    }
}

/// Go: `afero.BasePathFile`.
pub struct BasePathFile {
    file: Box<dyn File>,
    path: String,
}

forward_file_io!(BasePathFile, file);

impl File for BasePathFile {
    // Go: afero/basepath.go:BasePathFile.Name
    fn name(&self) -> String {
        let sourcename = self.file.name();
        let base = go_path::filepath::clean(&self.path);
        sourcename
            .strip_prefix(base.as_str())
            .unwrap_or(&sourcename)
            .to_string()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        self.file.stat()
    }
    // Go: afero/basepath.go:BasePathFile.ReadDir
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        self.file.read_dir(count)
    }
    fn readdirnames(&mut self, count: i32) -> Result<Vec<String>> {
        self.file.readdirnames(count)
    }
    fn close(&mut self) -> Result<()> {
        self.file.close()
    }
}

impl Fs for BasePathFs {
    fn name(&self) -> &str {
        "BasePathFs"
    }
    // Go: afero/basepath.go:Create
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        let name = self.real("create", name)?;
        let f = self.source.create(&name)?;
        Ok(self.wrap(f))
    }
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        let name = self.real("mkdir", name)?;
        self.source.mkdir(&name, perm)
    }
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        let name = self.real("mkdir", path)?;
        self.source.mkdir_all(&name, perm)
    }
    // Go: afero/basepath.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        let name = self.real("open", name)?;
        let f = self.source.open(&name)?;
        Ok(self.wrap(f))
    }
    // Go: afero/basepath.go:OpenFile
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        let name = self.real("openfile", name)?;
        let f = self.source.open_file(&name, flag, perm)?;
        Ok(self.wrap(f))
    }
    fn remove(&self, name: &str) -> Result<()> {
        let name = self.real("remove", name)?;
        self.source.remove(&name)
    }
    fn remove_all(&self, path: &str) -> Result<()> {
        let name = self.real("remove_all", path)?;
        self.source.remove_all(&name)
    }
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        let old = self.real("rename", old)?;
        let new = self.real("rename", new)?;
        self.source.rename(&old, &new)
    }
    // Go: afero/basepath.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let name = self.real("stat", name)?;
        self.source.stat(&name)
    }
    fn chmod(&self, name: &str, mode: u32) -> Result<()> {
        let name = self.real("chmod", name)?;
        self.source.chmod(&name, mode)
    }
    fn chown(&self, name: &str, uid: i32, gid: i32) -> Result<()> {
        let name = self.real("chown", name)?;
        self.source.chown(&name, uid, gid)
    }
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()> {
        let name = self.real("chtimes", name)?;
        self.source.chtimes(&name, atime, mtime)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ReadOnlyFs

/// Go: `afero.NewReadOnlyFs(source)`.
pub struct ReadOnlyFs {
    pub source: Arc<dyn Fs>,
}

impl ReadOnlyFs {
    // Go: afero/readonlyfs.go:NewReadOnlyFs
    pub fn new(source: Arc<dyn Fs>) -> ReadOnlyFs {
        ReadOnlyFs { source }
    }
}

impl Fs for ReadOnlyFs {
    fn name(&self) -> &str {
        "ReadOnlyFilter"
    }
    fn create(&self, _name: &str) -> Result<Box<dyn File>> {
        Err(oserror::eperm())
    }
    fn mkdir(&self, _name: &str, _perm: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    fn mkdir_all(&self, _path: &str, _perm: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.source.open(name)
    }
    // Go: afero/readonlyfs.go:OpenFile
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        if flag
            & (flags::O_WRONLY | flags::O_RDWR | flags::O_APPEND | flags::O_CREATE | flags::O_TRUNC)
            != 0
        {
            return Err(oserror::eperm());
        }
        self.source.open_file(name, flag, perm)
    }
    fn remove(&self, _name: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    fn remove_all(&self, _path: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    fn rename(&self, _old: &str, _new: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        self.source.stat(name)
    }
    fn chmod(&self, _name: &str, _mode: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    fn chown(&self, _name: &str, _uid: i32, _gid: i32) -> Result<()> {
        Err(oserror::eperm())
    }
    fn chtimes(&self, _name: &str, _atime: &Time, _mtime: &Time) -> Result<()> {
        Err(oserror::eperm())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// MemMapFs (tests)

pub(crate) struct MemData {
    name: String,
    data: Vec<u8>,
    dir: bool,
    mode: u32,
    mod_time: Time,
}

type MemNode = Arc<Mutex<MemData>>;

/// Go: `afero.NewMemMapFs()` (tests and `renderToMemory`). Directories list their entries by
/// name (`mem.DirMap.Files`).
#[derive(Default)]
pub struct MemMapFs {
    pub(crate) files: Mutex<std::collections::BTreeMap<String, MemNode>>,
}

// Go: afero/memmap.go:normalizePath
fn mem_normalize_path(path: &str) -> String {
    let path = go_path::filepath::clean(path);
    match path.as_str() {
        "." | ".." => "/".to_string(),
        _ => path,
    }
}

/// Go: `afero.NewMemMapFs()`.
// Go: afero/memmap.go:NewMemMapFs
pub fn new_mem_map_fs() -> Arc<dyn Fs> {
    Arc::new(MemMapFs::default())
}

impl MemMapFs {
    fn data(&self) -> std::sync::MutexGuard<'_, std::collections::BTreeMap<String, MemNode>> {
        let mut m = self.files.lock().unwrap_or_else(|e| e.into_inner());
        if m.is_empty() {
            m.insert(
                "/".to_string(),
                Arc::new(Mutex::new(MemData {
                    name: "/".to_string(),
                    data: Vec::new(),
                    dir: true,
                    mode: mode::MODE_DIR | 0o755,
                    mod_time: nh_common::htime::now(),
                })),
            );
        }
        m
    }

    // Go: afero/memmap.go:registerWithParent (parents are created on demand)
    fn register_with_parent(
        m: &mut std::collections::BTreeMap<String, MemNode>,
        name: &str,
        perm: u32,
    ) {
        let (pdir, _) = go_path::filepath::split(name);
        let pdir = mem_normalize_path(pdir);
        if pdir == name || m.contains_key(&pdir) {
            return;
        }
        Self::lockfree_mkdir(m, &pdir, perm);
    }

    // Go: afero/memmap.go:lockfreeMkdir
    fn lockfree_mkdir(m: &mut std::collections::BTreeMap<String, MemNode>, name: &str, perm: u32) {
        let name = mem_normalize_path(name);
        if m.contains_key(&name) {
            return;
        }
        m.insert(
            name.clone(),
            Arc::new(Mutex::new(MemData {
                name: name.clone(),
                data: Vec::new(),
                dir: true,
                mode: mode::MODE_DIR | perm,
                mod_time: nh_common::htime::now(),
            })),
        );
        Self::register_with_parent(m, &name, perm);
    }

    fn info(node: &MemNode) -> FileMetaInfo {
        let d = node.lock().unwrap_or_else(|e| e.into_inner());
        let (_, base) = go_path::filepath::split(&d.name);
        FileMetaInfo::new_plain(
            base,
            d.dir,
            if d.dir { 42 } else { d.data.len() as i64 },
            d.mode,
            d.mod_time.clone(),
        )
    }

    fn handle(&self, node: MemNode, read_only: bool) -> Box<dyn File> {
        let name = node.lock().unwrap_or_else(|e| e.into_inner()).name.clone();
        Box::new(MemFile {
            fs_entries: self.children_snapshot(&name),
            node,
            at: 0,
            read_only,
            closed: false,
            read_dir_count: 0,
        })
    }

    fn children_snapshot(&self, dir: &str) -> Vec<FileMetaInfo> {
        let m = self.data();
        let mut out = Vec::new();
        for (k, v) in m.iter() {
            if k == dir {
                continue;
            }
            let (p, _) = go_path::filepath::split(k);
            if mem_normalize_path(p) == dir {
                out.push(Self::info(v));
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

/// Go: `mem.File`.
pub struct MemFile {
    node: MemNode,
    at: usize,
    read_only: bool,
    closed: bool,
    read_dir_count: usize,
    fs_entries: Vec<FileMetaInfo>,
}

impl Read for MemFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let d = self.node.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed {
            return Err(io::Error::other(oserror::ERR_CLOSED));
        }
        if self.at >= d.data.len() {
            return Ok(0);
        }
        let n = buf.len().min(d.data.len() - self.at);
        buf[..n].copy_from_slice(&d.data[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

impl Write for MemFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.closed {
            return Err(io::Error::other(oserror::ERR_CLOSED));
        }
        if self.read_only {
            return Err(io::Error::other("file handle is read only"));
        }
        let mut d = self.node.lock().unwrap_or_else(|e| e.into_inner());
        let end = self.at + buf.len();
        if d.data.len() < end {
            d.data.resize(end, 0);
        }
        d.data[self.at..end].copy_from_slice(buf);
        self.at = end;
        d.mod_time = nh_common::htime::now();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for MemFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let len = self
            .node
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .data
            .len() as i64;
        let at = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.at as i64 + n,
            SeekFrom::End(n) => len + n,
        };
        if at < 0 {
            return Err(io::Error::other("negative position"));
        }
        self.at = at as usize;
        Ok(at as u64)
    }
}

impl File for MemFile {
    fn name(&self) -> String {
        self.node
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .name
            .clone()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        Ok(MemMapFs::info(&self.node))
    }
    // Go: afero/mem/file.go:File.ReadDir (Readdir: sorted by name)
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        let (dir, name) = {
            let d = self.node.lock().unwrap_or_else(|e| e.into_inner());
            (d.dir, d.name.clone())
        };
        if !dir {
            return Err(oserror::path_error(
                "readdir",
                &name,
                &Error::new("not a dir"),
            ));
        }
        let files = &self.fs_entries[self.read_dir_count.min(self.fs_entries.len())..];
        let out_len = if count > 0 {
            if files.is_empty() {
                return Err(oserror::eof());
            }
            files.len().min(count as usize)
        } else {
            files.len()
        };
        self.read_dir_count += out_len;
        Ok(files[..out_len].to_vec())
    }
    fn close(&mut self) -> Result<()> {
        self.closed = true;
        Ok(())
    }
}

impl Fs for MemMapFs {
    fn name(&self) -> &str {
        "MemMapFS"
    }
    // Go: afero/memmap.go:Create
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        let name = mem_normalize_path(name);
        let node = {
            let mut m = self.data();
            let node = Arc::new(Mutex::new(MemData {
                name: name.clone(),
                data: Vec::new(),
                dir: false,
                mode: mode::MODE_TEMPORARY,
                mod_time: nh_common::htime::now(),
            }));
            m.insert(name.clone(), node.clone());
            Self::register_with_parent(&mut m, &name, 0);
            node
        };
        Ok(self.handle(node, false))
    }
    // Go: afero/memmap.go:Mkdir
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        let perm =
            perm & (mode::MODE_PERM | mode::MODE_SETUID | mode::MODE_SETGID | mode::MODE_STICKY);
        let name = mem_normalize_path(name);
        let mut m = self.data();
        if m.contains_key(&name) {
            return Err(oserror::path_error(
                "mkdir",
                &name,
                &Error::with_kind(nh_common::herrors::ErrorKind::Exist, "file already exists"),
            ));
        }
        Self::lockfree_mkdir(&mut m, &name, perm);
        Ok(())
    }
    // Go: afero/memmap.go:MkdirAll
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        match self.mkdir(path, perm) {
            Err(e) if e.kind() == nh_common::herrors::ErrorKind::Exist => Ok(()),
            r => r,
        }
    }
    // Go: afero/memmap.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        let n = mem_normalize_path(name);
        let node = self.data().get(&n).cloned();
        match node {
            None => Err(oserror::path_error("open", &n, &oserror::err_not_exist())),
            Some(node) => Ok(self.handle(node, true)),
        }
    }
    // Go: afero/memmap.go:OpenFile
    fn open_file(&self, name: &str, flag: i32, _perm: u32) -> Result<Box<dyn File>> {
        let n = mem_normalize_path(name);
        let node = self.data().get(&n).cloned();
        let mut file = match node {
            Some(_) if flag & flags::O_EXCL != 0 => {
                return Err(oserror::path_error(
                    "open",
                    name,
                    &Error::with_kind(nh_common::herrors::ErrorKind::Exist, "file already exists"),
                ));
            }
            Some(node) => self.handle(node, flag == flags::O_RDONLY),
            None if flag & flags::O_CREATE != 0 => self.create(name)?,
            None => return Err(oserror::path_error("open", &n, &oserror::err_not_exist())),
        };
        if flag & flags::O_APPEND != 0 {
            file.seek(SeekFrom::End(0))
                .map_err(oserror::from_io_plain)?;
        }
        if flag & flags::O_TRUNC != 0 && flag & (flags::O_RDWR | flags::O_WRONLY) != 0 {
            let n = mem_normalize_path(name);
            if let Some(node) = self.data().get(&n) {
                node.lock().unwrap_or_else(|e| e.into_inner()).data.clear();
            }
        }
        Ok(file)
    }
    // Go: afero/memmap.go:Remove
    fn remove(&self, name: &str) -> Result<()> {
        let name = mem_normalize_path(name);
        let mut m = self.data();
        if m.remove(&name).is_none() {
            return Err(oserror::path_error(
                "remove",
                &name,
                &oserror::err_not_exist(),
            ));
        }
        Ok(())
    }
    // Go: afero/memmap.go:RemoveAll
    fn remove_all(&self, path: &str) -> Result<()> {
        let path = mem_normalize_path(path);
        let prefix = format!("{path}/");
        self.data()
            .retain(|k, _| !(k == &path || k.starts_with(&prefix)));
        Ok(())
    }
    // Go: afero/memmap.go:Rename
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        let old = mem_normalize_path(old);
        let new = mem_normalize_path(new);
        if old == new {
            return Ok(());
        }
        let mut m = self.data();
        if !m.contains_key(&old) {
            return Err(oserror::path_error(
                "rename",
                &old,
                &oserror::err_not_exist(),
            ));
        }
        let prefix = format!("{old}/");
        let keys: Vec<String> = m
            .keys()
            .filter(|k| **k == old || k.starts_with(&prefix))
            .cloned()
            .collect();
        for k in keys {
            let node = m.remove(&k).unwrap();
            let nk = k.replacen(&old, &new, 1);
            node.lock().unwrap_or_else(|e| e.into_inner()).name = nk.clone();
            m.insert(nk, node);
        }
        Self::register_with_parent(&mut m, &new, 0);
        Ok(())
    }
    // Go: afero/memmap.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let n = mem_normalize_path(name);
        let node = self.data().get(&n).cloned();
        match node {
            None => Err(oserror::path_error("open", &n, &oserror::err_not_exist())),
            Some(node) => Ok(Self::info(&node)),
        }
    }
    // Go: afero/memmap.go:Chmod
    fn chmod(&self, name: &str, m: u32) -> Result<()> {
        let chmod_bits =
            mode::MODE_PERM | mode::MODE_SETUID | mode::MODE_SETGID | mode::MODE_STICKY;
        let node = self.data().get(name).cloned();
        match node {
            None => Err(oserror::path_error(
                "chmod",
                name,
                &oserror::err_not_exist(),
            )),
            Some(node) => {
                let mut d = node.lock().unwrap_or_else(|e| e.into_inner());
                d.mode = (d.mode & !chmod_bits) | (m & chmod_bits);
                Ok(())
            }
        }
    }
    fn chown(&self, name: &str, _uid: i32, _gid: i32) -> Result<()> {
        let n = mem_normalize_path(name);
        if !self.data().contains_key(&n) {
            return Err(oserror::path_error("chown", &n, &oserror::err_not_exist()));
        }
        Ok(())
    }
    // Go: afero/memmap.go:Chtimes
    fn chtimes(&self, name: &str, _atime: &Time, mtime: &Time) -> Result<()> {
        let n = mem_normalize_path(name);
        let node = self.data().get(&n).cloned();
        match node {
            None => Err(oserror::path_error(
                "chtimes",
                &n,
                &oserror::err_not_exist(),
            )),
            Some(node) => {
                node.lock().unwrap_or_else(|e| e.into_inner()).mod_time = mtime.clone();
                Ok(())
            }
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Helpers (afero/util.go, ioutil.go)

/// Go: `afero.ReadFile(fs, name)`.
pub fn read_file(fs: &dyn Fs, name: &str) -> Result<Vec<u8>> {
    let mut f = fs.open(name)?;
    let mut v = Vec::new();
    let r = f.read_to_end(&mut v);
    let _ = f.close();
    r.map_err(oserror::from_io_plain)?;
    Ok(v)
}

/// Go: `afero.WriteFile(fs, name, data, perm)`.
pub fn write_file(fs: &dyn Fs, name: &str, data: &[u8], perm: u32) -> Result<()> {
    let mut f = fs.open_file(
        name,
        flags::O_WRONLY | flags::O_CREATE | flags::O_TRUNC,
        perm,
    )?;
    let mut err = f.write_all(data).map_err(oserror::from_io_plain).err();
    let err1 = f.close().err();
    if err.is_none() {
        err = err1;
    }
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Go: `afero.Exists(fs, path)`.
pub fn exists(fs: &dyn Fs, path: &str) -> Result<bool> {
    match fs.stat(path) {
        Ok(_) => Ok(true),
        Err(e) if e.is_not_exist() => Ok(false),
        Err(e) => Err(e),
    }
}

/// Go: `afero.IsDir(fs, path)`.
pub fn is_dir(fs: &dyn Fs, path: &str) -> Result<bool> {
    let fi = fs.stat(path)?;
    Ok(fi.is_dir())
}

/// Go: `afero.ReadDir(fs, dirname)` — the entries sorted by name.
pub fn read_dir(fs: &dyn Fs, dirname: &str) -> Result<Vec<FileMetaInfo>> {
    let mut f = fs.open(dirname)?;
    let list = f.read_dir(-1);
    let _ = f.close();
    let mut list = list?;
    go_sort::sort_by(&mut list, |a, b| a.name() < b.name());
    Ok(list)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (spf13/afero@v1.14.0 subset; not generated)
// OK afero.go: Fs, File
// OK os.go: OsFs (Create, Mkdir, MkdirAll, Open, OpenFile, Remove, RemoveAll, Rename, Stat,
//    Chmod, Chtimes; Chown is an explicit unsupported error)
// OK basepath.go: BasePathFs (RealPath, all Fs methods), BasePathFile (Name, ReadDir)
// OK readonlyfs.go: ReadOnlyFs
// OK memmap.go + mem/*: MemMapFs (tests; parents created on demand, sorted ReadDir)
// OK util.go/ioutil.go: ReadFile, WriteFile, Exists, IsDir, ReadDir
// ---------------------------------------------------------------------------
