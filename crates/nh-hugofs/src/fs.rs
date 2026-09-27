//! Port of `hugofs/fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


use std::sync::Arc;

use nh_config::config_provider::Provider;

use crate::afero::Fs as AferoFs;

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

/// Go: `hugofs.NewFromSourceAndDestination(source, destination, cfg)`.
// Go: hugofs/fs.go:NewFromSourceAndDestination
pub fn new_from_source_and_destination(source: Arc<dyn AferoFs>, destination: Arc<dyn AferoFs>, cfg: &dyn Provider) -> Fs {
    todo!()
}

/// Go: `hugofs.NewBasePathFs`.
pub fn new_base_path_fs(source: Arc<dyn AferoFs>, path: &str) -> Arc<dyn AferoFs> {
    todo!()
}

/// Go: `hugofs.NewReadOnlyFs`.
pub fn new_read_only_fs(source: Arc<dyn AferoFs>) -> Arc<dyn AferoFs> {
    todo!()
}

/// Go: `hugofs.IsOsFs(fs)`.
pub fn is_os_fs(fs: &dyn AferoFs) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/fs.go (253 lines; 12/16 funcs executed)
//   types: Fs, FilesystemsUnwrapper, FilesystemUnwrapper, WalkFn, filesystemsWrapper
//    L63-67: NewDefault(cfg config.Provider) *Fs
//    L72-74: NewFrom(fs afero.Fs, conf config.BaseConfig) *Fs
//    L76-79: NewFromOld(fs afero.Fs, cfg config.Provider) *Fs
// EX L83-86: NewFromSourceAndDestination(source, destination afero.Fs, cfg config.Provider) *Fs
// EX L88-95: getWorkingPublishDir(cfg config.Provider) (string, string)
// EX L97-125: newFs(source, destination afero.Fs, workingDir, publishDir string) *Fs
// EX L127-132: getWorkingDirFsReadOnly(base afero.Fs, workingDir string) afero.Fs
// EX L134-139: getWorkingDirFsWritable(base afero.Fs, workingDir string) afero.Fs
// EX L141-143: isWrite(flag int) bool
//    L148-169: MakeReadableAndRemoveAllModulePkgDir(fs afero.Fs, dir string) (int, error)
// EX L173-185: IsOsFs(fs afero.Fs) bool
// EX L202-226: WalkFilesystems(fs afero.Fs, fn WalkFn) bool
// EX L231-233: NewBasePathFs(source afero.Fs, path string) afero.Fs
// EX L236-238: NewReadOnlyFs(source afero.Fs) afero.Fs
// EX L242-244: WrapFilesystem(container, content afero.Fs) afero.Fs
// EX L251-253: (w filesystemsWrapper) UnwrapFilesystem() afero.Fs
// ---------------------------------------------------------------------------
