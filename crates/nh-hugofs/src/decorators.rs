//! Port of `hugofs/decorators.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


use std::sync::Arc;

use crate::afero::Fs;
use crate::fileinfo::{FileMeta, FileMetaInfo};

/// Go: `hugofs.NewBaseFileDecorator(fs, callbacks...)` — attaches `FileMeta` (filename, open func,
/// join-stat func) to every file info from `fs`.
// Go: hugofs/decorators.go:NewBaseFileDecorator
pub fn new_base_file_decorator(fs: Arc<dyn Fs>, callbacks: Vec<Arc<dyn Fn(&FileMetaInfo) + Send + Sync>>) -> Arc<dyn Fs> {
    todo!()
}

// Go: hugofs/decorators.go:decorateDirs
pub(crate) fn decorate_dirs(fs: Arc<dyn Fs>, meta: Arc<FileMeta>) -> Arc<dyn Fs> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/decorators.go (154 lines; 7/8 funcs executed)
//   types: baseFileDecoratorFs, baseFileDecoratorFile
// EX L27-42: decorateDirs(fs afero.Fs, meta *FileMeta) afero.Fs
// EX L46-85: NewBaseFileDecorator(fs afero.Fs, callbacks ...func(fi FileMetaInfo)) afero.Fs
// EX L92-94: (fs *baseFileDecoratorFs) UnwrapFilesystem() afero.Fs
// EX L96-107: (fs *baseFileDecoratorFs) Stat(name string) (os.FileInfo, error)
// EX L109-111: (fs *baseFileDecoratorFs) Open(name string) (afero.File, error)
// EX L113-119: (fs *baseFileDecoratorFs) open(name string) (afero.File, error)
// EX L126-150: (l *baseFileDecoratorFile) ReadDir(n int) ([]fs.DirEntry, error)
//    L152-154: (l *baseFileDecoratorFile) Readdir(c int) (ofi []os.FileInfo, err error)
// ---------------------------------------------------------------------------
