//! Port of `hugofs/component_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugofs.componentFs`: the per-component view used by the content/layout/data/i18n walkers.
//! ReadDir order (component_fs.go:64-146) and `applyMeta` (NFC names on darwin, PathInfo parsing,
//! filename language, disabled languages dropped) define the order files are processed in.

use std::sync::Arc;

use nh_common::paths::pathparser::PathParser;
use nh_common::Result;

use crate::afero::Fs;
use crate::fileinfo::FileMetaInfo;

/// Go: `hugofs.ComponentFsOptions`.
#[derive(Clone)]
pub struct ComponentFsOptions {
    /// The filesystem where one or more components are mounted.
    pub fs: Arc<dyn Fs>,
    /// The component name, e.g. "content", "layouts" etc.
    pub component: String,
    pub default_content_language: String,
    /// The parser used to parse paths provided by this filesystem.
    pub path_parser: Arc<PathParser>,
}

/// Go: `hugofs.componentFs`.
pub struct ComponentFs {
    pub opts: ComponentFsOptions,
}

impl ComponentFs {
    // Go: hugofs/component_fs.go:NewComponentFs
    pub fn new(opts: ComponentFsOptions) -> Arc<ComponentFs> {
        todo!()
    }

    /// Go: `componentFsDir.ReadDir` — merged, filtered, decorated and sorted entries.
    // Go: hugofs/component_fs.go:ReadDir
    pub fn read_dir(&self, name: &str) -> Result<Vec<FileMetaInfo>> {
        todo!()
    }

    // Go: hugofs/component_fs.go:Stat
    pub fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        todo!()
    }

    // Go: hugofs/component_fs.go:applyMeta
    pub(crate) fn apply_meta(&self, fi: FileMetaInfo, name: &str) -> Option<FileMetaInfo> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/component_fs.go (272 lines; 6/10 funcs executed)
//   types: componentFs, componentFsDir, ComponentFsOptions
// EX L31-40: NewComponentFs(opts ComponentFsOptions) *componentFs
// EX L51-53: (fs *componentFs) UnwrapFilesystem() afero.Fs
// EX L64-153: (f *componentFsDir) ReadDir(count int) ([]iofs.DirEntry, error)
//    L155-162: (f *componentFsDir) Stat() (iofs.FileInfo, error)
// EX L164-171: (fs *componentFs) Stat(name string) (os.FileInfo, error)
// EX L173-215: (fs *componentFs) applyMeta(fi FileNameIsDir, name string) (FileMetaInfo, bool)
//    L217-219: (f *componentFsDir) Readdir(count int) ([]os.FileInfo, error)
//    L221-232: (f *componentFsDir) Readdirnames(count int) ([]string, error)
// EX L247-268: (fs *componentFs) Open(name string) (afero.File, error)
//    L270-272: (fs *componentFs) ReadDir(name string) ([]os.FileInfo, error)
// ---------------------------------------------------------------------------
