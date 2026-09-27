//! Port of `hugofs/fileinfo.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugofs.FileMeta` / `FileMetaInfo`: every file seen through the component filesystems
//! carries its mount metadata (real filename, module ordinal, weight, language, parsed path).

use std::sync::Arc;

use go_value::Time;
use nh_common::glob::filename_filter::FilenameFilter;
use nh_common::paths::pathparser::Path;
use nh_common::Result;

use crate::afero::File;

/// Go: `hugofs.FileMeta`.
#[derive(Clone, Default)]
pub struct FileMeta {
    /// Parsed path relative to the component root (set by ComponentFs.applyMeta).
    pub path_info: Option<Arc<Path>>,
    pub name: String,
    /// Real OS filename.
    pub filename: String,
    pub base_dir: String,
    pub source_root: String,
    pub module: String,
    pub module_ordinal: i64,
    pub component: String,
    pub weight: i64,
    pub is_project: bool,
    pub watch: bool,
    /// The language key if set on the mount or derived from the file name (`index.th.md`).
    pub lang: String,
    pub lang_index: i64,
    pub open_func: Option<Arc<dyn Fn() -> Result<Box<dyn File>> + Send + Sync>>,
    pub join_stat_func: Option<Arc<dyn Fn(&str) -> Result<FileMetaInfo> + Send + Sync>>,
    pub inclusion_filter: Option<FilenameFilter>,
}

impl FileMeta {
    // Go: hugofs/fileinfo.go:NewFileMeta
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `FileMeta.Merge(from)` — fills zero fields from `from`.
    // Go: hugofs/fileinfo.go:Merge
    pub fn merge(&mut self, from: &FileMeta) {
        todo!()
    }

    // Go: hugofs/fileinfo.go:Open
    pub fn open(&self) -> Result<Box<dyn File>> {
        todo!()
    }

    // Go: hugofs/fileinfo.go:JoinStat
    pub fn join_stat(&self, name: &str) -> Result<FileMetaInfo> {
        todo!()
    }
}

/// Go: `hugofs.FileMetaInfo` (a `fs.DirEntry` + `os.FileInfo` + `Meta()`).
#[derive(Clone)]
pub struct FileMetaInfo {
    /// Base name (NFC-normalized on darwin by ComponentFs).
    pub name: String,
    pub is_dir: bool,
    pub size: i64,
    pub mode: u32,
    pub mod_time: Time,
    pub meta: Arc<FileMeta>,
}

impl FileMetaInfo {
    pub fn meta(&self) -> &FileMeta {
        &self.meta
    }
}

/// Go: `hugofs.NewFileMetaInfo(fi, m)`.
// Go: hugofs/fileinfo.go:NewFileMetaInfo
pub fn new_file_meta_info(name: &str, is_dir: bool, meta: FileMeta) -> FileMetaInfo {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/fileinfo.go (390 lines; 18/29 funcs executed)
//   types: FileMeta, FileMetaInfo, MetaProvider, FileInfoOptionals, FileNameIsDir, FileInfoProvider, DirOnlyOps,
//          dirEntryMeta, dirEntry, dirNameOnlyFileInfo
// EX L42-44: NewFileMeta() *FileMeta
// EX L81-87: (m *FileMeta) Copy() *FileMeta
// EX L89-109: (m *FileMeta) Merge(from *FileMeta)
// EX L111-116: (f *FileMeta) Open() (afero.File, error)
//    L118-125: (f *FileMeta) ReadAll() ([]byte, error)
// EX L127-132: (f *FileMeta) JoinStat(name string) (FileMetaInfo, error)
// EX L180-182: (fi *dirEntryMeta) Meta() *FileMeta
//    L185-187: (fi *dirEntryMeta) Filename() string
// EX L189-198: (fi *dirEntryMeta) fileInfo() fs.FileInfo
//    L200-202: (fi *dirEntryMeta) Size() int64
// EX L204-206: (fi *dirEntryMeta) Mode() fs.FileMode
// EX L208-210: (fi *dirEntryMeta) ModTime() time.Time
//    L212-214: (fi *dirEntryMeta) Sys() any
// EX L217-222: (fi *dirEntryMeta) Name() string
//    L231-231: (d dirEntry) Type() fs.FileMode
// EX L233-233: (d dirEntry) Info() (fs.FileInfo, error)
// EX L235-252: NewFileMetaInfo(fi FileNameIsDir, m *FileMeta) FileMetaInfo
//    L259-261: (fi *dirNameOnlyFileInfo) Name() string
//    L263-265: (fi *dirNameOnlyFileInfo) Size() int64
//    L267-269: (fi *dirNameOnlyFileInfo) Mode() os.FileMode
// EX L271-273: (fi *dirNameOnlyFileInfo) ModTime() time.Time
// EX L275-277: (fi *dirNameOnlyFileInfo) IsDir() bool
//    L279-281: (fi *dirNameOnlyFileInfo) Sys() any
// EX L283-297: newDirNameOnlyFileInfo(name string, meta *FileMeta, fileOpener func() (afero.File, error)) FileMetaInfo
// EX L299-323: decorateFileInfo(fi FileNameIsDir, opener func() (afero.File, error), filename string, inMeta *FileMeta) FileMetaInfo
// EX L325-332: DirEntriesToFileMetaInfos(fis []fs.DirEntry) []FileMetaInfo
// EX L334-343: normalizeFilename(filename string) string
//    L345-350: sortDirEntries(fis []fs.DirEntry)
//    L353-390: AddFileInfoToError(err error, fi FileMetaInfo, fs afero.Fs) error
// ---------------------------------------------------------------------------
