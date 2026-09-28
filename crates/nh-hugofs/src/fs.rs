//! Port of `hugofs/fs.go` (and `hugofs/noop_fs.go`).
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::any::Any;
use std::sync::Arc;

use go_value::Time;
use nh_common::Result;
use nh_config::common_config::BaseConfig;
use nh_config::config_provider::Provider;

use crate::afero::{BasePathFs, File, Fs as AferoFs, MemMapFs, OsFs, ReadOnlyFs, flags};
use crate::fileinfo::FileMetaInfo;
use crate::oserror;

/// Go: `hugofs.Fs` — the source, publish and working-dir filesystems of a build.
#[derive(Clone)]
pub struct Fs {
    /// The source (OS) filesystem.
    pub source: Arc<dyn AferoFs>,
    /// The publish dir (`BasePathFs(dest, publishDir)`), wrapped in the HasBytes receiver by deps.
    pub publish_dir: Arc<dyn AferoFs>,
    /// Static files are published here (same as publish_dir unless multihost).
    pub publish_dir_static: Arc<dyn AferoFs>,
    pub publish_dir_server: Arc<dyn AferoFs>,
    pub os: Arc<dyn AferoFs>,
    pub working_dir_read_only: Arc<dyn AferoFs>,
    pub working_dir_writable: Arc<dyn AferoFs>,
}

/// Go: `hugofs.Os` (`&afero.OsFs{}`).
pub fn os() -> Arc<dyn AferoFs> {
    Arc::new(OsFs)
}

/// Go: `hugofs.NewDefault(cfg)`.
// Go: hugofs/fs.go:NewDefault
pub fn new_default(cfg: &dyn Provider) -> Fs {
    let (working_dir, publish_dir) = get_working_publish_dir(cfg);
    let fs = os();
    new_fs(fs.clone(), fs, &working_dir, &publish_dir)
}

/// Go: `hugofs.NewFrom(fs, conf)` — useful for testing.
// Go: hugofs/fs.go:NewFrom
pub fn new_from(fs: Arc<dyn AferoFs>, conf: &BaseConfig) -> Fs {
    new_fs(fs.clone(), fs, &conf.working_dir, &conf.publish_dir)
}

// Go: hugofs/fs.go:NewFromOld
pub fn new_from_old(fs: Arc<dyn AferoFs>, cfg: &dyn Provider) -> Fs {
    let (working_dir, publish_dir) = get_working_publish_dir(cfg);
    new_fs(fs.clone(), fs, &working_dir, &publish_dir)
}

/// Go: `hugofs.NewFromSourceAndDestination(source, destination, cfg)`.
// Go: hugofs/fs.go:NewFromSourceAndDestination
pub fn new_from_source_and_destination(
    source: Arc<dyn AferoFs>,
    destination: Arc<dyn AferoFs>,
    cfg: &dyn Provider,
) -> Fs {
    let (working_dir, publish_dir) = get_working_publish_dir(cfg);
    new_fs(source, destination, &working_dir, &publish_dir)
}

// Go: hugofs/fs.go:getWorkingPublishDir
fn get_working_publish_dir(cfg: &dyn Provider) -> (String, String) {
    let working_dir = cfg.get_string("workingDir");
    let mut publish_dir = cfg.get_string("publishDirDynamic");
    if publish_dir.is_empty() {
        publish_dir = cfg.get_string("publishDir");
    }
    (working_dir, publish_dir)
}

// Go: hugofs/fs.go:newFs
fn new_fs(
    source: Arc<dyn AferoFs>,
    destination: Arc<dyn AferoFs>,
    working_dir: &str,
    publish_dir: &str,
) -> Fs {
    if publish_dir.is_empty() {
        panic!("publishDir is empty");
    }

    let working_dir = if working_dir == "." { "" } else { working_dir };

    // Sanity check
    if is_os_fs(source.as_ref()) && working_dir.len() < 2 {
        panic!("workingDir is too short");
    }

    // If this does not exist, it will be created later.
    let abs_publish_dir = nh_common::paths::path::abs_pathify(working_dir, publish_dir);

    let pub_fs = new_base_path_fs(destination, &abs_publish_dir);

    Fs {
        source: source.clone(),
        publish_dir: pub_fs.clone(),
        publish_dir_server: pub_fs.clone(),
        publish_dir_static: pub_fs,
        os: Arc::new(OsFs),
        working_dir_read_only: get_working_dir_fs_read_only(source.clone(), working_dir),
        working_dir_writable: get_working_dir_fs_writable(source, working_dir),
    }
}

// Go: hugofs/fs.go:getWorkingDirFsReadOnly
fn get_working_dir_fs_read_only(base: Arc<dyn AferoFs>, working_dir: &str) -> Arc<dyn AferoFs> {
    if working_dir.is_empty() {
        return new_read_only_fs(base);
    }
    new_base_path_fs(new_read_only_fs(base), working_dir)
}

// Go: hugofs/fs.go:getWorkingDirFsWritable
fn get_working_dir_fs_writable(base: Arc<dyn AferoFs>, working_dir: &str) -> Arc<dyn AferoFs> {
    if working_dir.is_empty() {
        return base;
    }
    new_base_path_fs(base, working_dir)
}

// Go: hugofs/fs.go:isWrite
pub(crate) fn is_write(flag: i32) -> bool {
    flag & flags::O_RDWR != 0 || flag & flags::O_WRONLY != 0
}

/// Go: `MakeReadableAndRemoveAllModulePkgDir` (Go module cache cleanup; not supported).
// Go: hugofs/fs.go:MakeReadableAndRemoveAllModulePkgDir
pub fn make_readable_and_remove_all_module_pkg_dir(_fs: &dyn AferoFs, _dir: &str) -> Result<i64> {
    Err(nh_common::herrors::Error::new(
        "neohugo-rs: hugo mod clean is not supported",
    ))
}

/// Go: `hugofs.IsOsFs(fs)` — whether fs is an OsFs or wraps one.
// Go: hugofs/fs.go:IsOsFs
pub fn is_os_fs(fs: &dyn AferoFs) -> bool {
    let mut is_os_fs = false;
    walk_filesystems_ref(fs, &mut |fs| {
        if fs.as_any().is::<MemMapFs>() {
            is_os_fs = false;
        } else if fs.as_any().is::<OsFs>() {
            is_os_fs = true;
        }
        is_os_fs
    });
    is_os_fs
}

/// Go: `hugofs.WalkFn`.
pub type WalkFn<'a> = &'a mut dyn FnMut(&dyn AferoFs) -> bool;

/// Go: `hugofs.WalkFilesystems(fs, fn)` — walks fs recursively and calls fn; if fn returns true,
/// walking is stopped.
// Go: hugofs/fs.go:WalkFilesystems
pub fn walk_filesystems(fs: &Arc<dyn AferoFs>, f: WalkFn<'_>) -> bool {
    walk_filesystems_ref(fs.as_ref(), f)
}

fn walk_filesystems_ref(fs: &dyn AferoFs, f: WalkFn<'_>) -> bool {
    if f(fs) {
        return true;
    }

    if let Some(afs) = fs.unwrap_filesystem() {
        if walk_filesystems_ref(afs.as_ref(), f) {
            return true;
        }
    } else if let Some(bfs) = fs.unwrap_filesystems() {
        for sf in bfs {
            if walk_filesystems_ref(sf.as_ref(), f) {
                return true;
            }
        }
    } else if let Some(cfs) = fs.filesystem_iterator() {
        for sf in cfs {
            if walk_filesystems_ref(sf.as_ref(), f) {
                return true;
            }
        }
    }

    false
}

/// Go: `hugofs.NewBasePathFs`.
// Go: hugofs/fs.go:NewBasePathFs
pub fn new_base_path_fs(source: Arc<dyn AferoFs>, path: &str) -> Arc<dyn AferoFs> {
    wrap_filesystem(Arc::new(BasePathFs::new(source.clone(), path)), source)
}

/// Go: `hugofs.NewReadOnlyFs`.
// Go: hugofs/fs.go:NewReadOnlyFs
pub fn new_read_only_fs(source: Arc<dyn AferoFs>) -> Arc<dyn AferoFs> {
    wrap_filesystem(Arc::new(ReadOnlyFs::new(source.clone())), source)
}

/// Go: `hugofs.WrapFilesystem(container, content)` — typically used to wrap an
/// `afero.BasePathFs` to allow access to the underlying filesystem if needed.
// Go: hugofs/fs.go:WrapFilesystem
pub fn wrap_filesystem(container: Arc<dyn AferoFs>, content: Arc<dyn AferoFs>) -> Arc<dyn AferoFs> {
    Arc::new(FilesystemsWrapper { container, content })
}

/// Go: `hugofs.filesystemsWrapper`.
pub struct FilesystemsWrapper {
    container: Arc<dyn AferoFs>,
    content: Arc<dyn AferoFs>,
}

impl FilesystemsWrapper {
    /// The wrapped container (Go: the embedded `afero.Fs`).
    pub fn container(&self) -> &Arc<dyn AferoFs> {
        &self.container
    }
}

impl AferoFs for FilesystemsWrapper {
    fn name(&self) -> &str {
        self.container.name()
    }
    fn embedded(&self) -> Option<&dyn AferoFs> {
        Some(self.container.as_ref())
    }
    // Go: hugofs/fs.go:(w filesystemsWrapper) UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn AferoFs>> {
        Some(self.content.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `hugofs.NoOpFs` — a no-op filesystem (`Open`/`Stat`: not found, writes: nothing).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoOpFs;

/// Go: `hugofs.NoOpFs`.
pub fn no_op_fs() -> Arc<dyn AferoFs> {
    Arc::new(NoOpFs)
}

fn err_no_op() -> nh_common::herrors::Error {
    nh_common::herrors::Error::new("this operation is not supported")
}

impl AferoFs for NoOpFs {
    fn name(&self) -> &str {
        "noOpFs"
    }
    // Go: hugofs/noop_fs.go:Create (panics in Go)
    fn create(&self, _name: &str) -> Result<Box<dyn File>> {
        Err(err_no_op())
    }
    fn mkdir(&self, _name: &str, _perm: u32) -> Result<()> {
        Ok(())
    }
    fn mkdir_all(&self, _path: &str, _perm: u32) -> Result<()> {
        Ok(())
    }
    fn open(&self, _name: &str) -> Result<Box<dyn File>> {
        Err(oserror::err_not_exist())
    }
    fn open_file(&self, _name: &str, _flag: i32, _perm: u32) -> Result<Box<dyn File>> {
        Err(oserror::err_not_exist())
    }
    fn remove(&self, _name: &str) -> Result<()> {
        Ok(())
    }
    fn remove_all(&self, _path: &str) -> Result<()> {
        Ok(())
    }
    fn rename(&self, _old: &str, _new: &str) -> Result<()> {
        Err(err_no_op())
    }
    fn stat(&self, _name: &str) -> Result<FileMetaInfo> {
        Err(oserror::err_not_exist())
    }
    fn chmod(&self, _name: &str, _mode: u32) -> Result<()> {
        Err(err_no_op())
    }
    fn chown(&self, _name: &str, _uid: i32, _gid: i32) -> Result<()> {
        Err(err_no_op())
    }
    fn chtimes(&self, _name: &str, _atime: &Time, _mtime: &Time) -> Result<()> {
        Err(err_no_op())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/fs.go (253 lines; 12/16 funcs executed)
//   types: Fs, FilesystemsUnwrapper, FilesystemUnwrapper, WalkFn, filesystemsWrapper
// OK L63-67: NewDefault(cfg config.Provider) *Fs
// OK L72-74: NewFrom(fs afero.Fs, conf config.BaseConfig) *Fs
// OK L76-79: NewFromOld(fs afero.Fs, cfg config.Provider) *Fs
// OK L83-86: NewFromSourceAndDestination(source, destination afero.Fs, cfg config.Provider) *Fs
// OK L88-95: getWorkingPublishDir(cfg config.Provider) (string, string)
// OK L97-125: newFs(source, destination afero.Fs, workingDir, publishDir string) *Fs
// OK L127-132: getWorkingDirFsReadOnly(base afero.Fs, workingDir string) afero.Fs
// OK L134-139: getWorkingDirFsWritable(base afero.Fs, workingDir string) afero.Fs
// OK L141-143: isWrite(flag int) bool
// OK L148-169: MakeReadableAndRemoveAllModulePkgDir(fs afero.Fs, dir string) (int, error) (STUB: unsupported)
// OK L173-185: IsOsFs(fs afero.Fs) bool
// OK L202-226: WalkFilesystems(fs afero.Fs, fn WalkFn) bool
// OK L231-233: NewBasePathFs(source afero.Fs, path string) afero.Fs
// OK L236-238: NewReadOnlyFs(source afero.Fs) afero.Fs
// OK L242-244: WrapFilesystem(container, content afero.Fs) afero.Fs
// OK L251-253: (w filesystemsWrapper) UnwrapFilesystem() afero.Fs
// Source: hugofs/noop_fs.go (not in the generated list): noOpFs (all methods; Go panics with
//    errNoOp where the port returns it), noOpRegularFileOps (afero::no_regular_file_ops!)
// ---------------------------------------------------------------------------
