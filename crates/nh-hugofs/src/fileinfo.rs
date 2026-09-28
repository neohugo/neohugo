//! Port of `hugofs/fileinfo.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugofs.FileMeta` / `FileMetaInfo`: every file seen through the component filesystems
//! carries its mount metadata (real filename, module ordinal, weight, language, parsed path).
//!
//! Go passes `*FileMeta` around and mutates it in place (`Merge`, `applyMeta`, the walker's
//! `PathInfo`). Here a [`FileMetaInfo`] owns an `Arc<FileMeta>` and mutation goes through
//! [`FileMetaInfo::meta_mut`] (`Arc::make_mut`). Go only mutates metas that were created for the
//! entry being processed (the decorators, `collectDirEntries` and `newDirNameOnlyFileInfo` make a
//! fresh one or a copy each time), so the copy-on-write never hides a mutation Go would show
//! through another holder (PORTING.md, deviation 1).

use std::fmt;
use std::sync::Arc;

use go_value::Time;
use nh_common::Result;
use nh_common::glob::filename_filter::FilenameFilter;
use nh_common::herrors::Error;
use nh_common::paths::pathparser::Path;

use crate::afero::File;

/// Go `io/fs.FileMode` bits.
pub mod mode {
    /// `fs.ModeDir`.
    pub const MODE_DIR: u32 = 1 << 31;
    /// `fs.ModeAppend`.
    pub const MODE_APPEND: u32 = 1 << 30;
    /// `fs.ModeExclusive`.
    pub const MODE_EXCLUSIVE: u32 = 1 << 29;
    /// `fs.ModeTemporary`.
    pub const MODE_TEMPORARY: u32 = 1 << 28;
    /// `fs.ModeSymlink`.
    pub const MODE_SYMLINK: u32 = 1 << 27;
    /// `fs.ModeDevice`.
    pub const MODE_DEVICE: u32 = 1 << 26;
    /// `fs.ModeNamedPipe`.
    pub const MODE_NAMED_PIPE: u32 = 1 << 25;
    /// `fs.ModeSocket`.
    pub const MODE_SOCKET: u32 = 1 << 24;
    /// `fs.ModeSetuid`.
    pub const MODE_SETUID: u32 = 1 << 23;
    /// `fs.ModeSetgid`.
    pub const MODE_SETGID: u32 = 1 << 22;
    /// `fs.ModeCharDevice`.
    pub const MODE_CHAR_DEVICE: u32 = 1 << 21;
    /// `fs.ModeSticky`.
    pub const MODE_STICKY: u32 = 1 << 20;
    /// `fs.ModeIrregular`.
    pub const MODE_IRREGULAR: u32 = 1 << 19;
    /// `fs.ModePerm`.
    pub const MODE_PERM: u32 = 0o777;
    /// `fs.ModeType`.
    pub const MODE_TYPE: u32 = MODE_DIR
        | MODE_SYMLINK
        | MODE_NAMED_PIPE
        | MODE_SOCKET
        | MODE_DEVICE
        | MODE_CHAR_DEVICE
        | MODE_IRREGULAR;
}

/// Go: `FileMeta.OpenFunc`.
pub type OpenFunc = Arc<dyn Fn() -> Result<Box<dyn File>> + Send + Sync>;
/// Go: `FileMeta.JoinStatFunc`.
pub type JoinStatFunc = Arc<dyn Fn(&str) -> Result<FileMetaInfo> + Send + Sync>;
/// Go: `FileMeta.Rename` — `(name, toFrom) -> (newName, include)`.
pub type RenameFunc = Arc<dyn Fn(&str, bool) -> (String, bool) + Send + Sync>;

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
    pub open_func: Option<OpenFunc>,
    pub join_stat_func: Option<JoinStatFunc>,
    pub inclusion_filter: Option<FilenameFilter>,
    /// Rename the name part of the file (not the directory). Returns the new name and whether
    /// the file should be included (single file mounts).
    pub rename: Option<RenameFunc>,
}

impl fmt::Debug for FileMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileMeta")
            .field(
                "path_info",
                &self.path_info.as_ref().map(|p| p.path().to_string()),
            )
            .field("name", &self.name)
            .field("filename", &self.filename)
            .field("base_dir", &self.base_dir)
            .field("source_root", &self.source_root)
            .field("module", &self.module)
            .field("module_ordinal", &self.module_ordinal)
            .field("component", &self.component)
            .field("weight", &self.weight)
            .field("is_project", &self.is_project)
            .field("watch", &self.watch)
            .field("lang", &self.lang)
            .field("lang_index", &self.lang_index)
            .field("open_func", &self.open_func.is_some())
            .field("join_stat_func", &self.join_stat_func.is_some())
            .field("inclusion_filter", &self.inclusion_filter.is_some())
            .field("rename", &self.rename.is_some())
            .finish()
    }
}

impl FileMeta {
    // Go: hugofs/fileinfo.go:NewFileMeta
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `(*FileMeta).Copy()` (a nil receiver gives a new meta: use `Option::cloned`).
    // Go: hugofs/fileinfo.go:Copy
    pub fn copy(m: Option<&FileMeta>) -> FileMeta {
        match m {
            None => FileMeta::new(),
            Some(m) => m.clone(),
        }
    }

    /// Go: `FileMeta.Merge(from)` — every field that is not truthy (`hreflect.IsTruthfulValue`:
    /// empty string, 0, false, nil pointer or func) is set from `from`.
    // Go: hugofs/fileinfo.go:Merge
    pub fn merge(&mut self, from: &FileMeta) {
        if self.path_info.is_none() {
            self.path_info = from.path_info.clone();
        }
        if self.name.is_empty() {
            self.name = from.name.clone();
        }
        if self.filename.is_empty() {
            self.filename = from.filename.clone();
        }
        if self.base_dir.is_empty() {
            self.base_dir = from.base_dir.clone();
        }
        if self.source_root.is_empty() {
            self.source_root = from.source_root.clone();
        }
        if self.module.is_empty() {
            self.module = from.module.clone();
        }
        if self.module_ordinal == 0 {
            self.module_ordinal = from.module_ordinal;
        }
        if self.component.is_empty() {
            self.component = from.component.clone();
        }
        if self.weight == 0 {
            self.weight = from.weight;
        }
        if !self.is_project {
            self.is_project = from.is_project;
        }
        if !self.watch {
            self.watch = from.watch;
        }
        if self.lang.is_empty() {
            self.lang = from.lang.clone();
        }
        if self.lang_index == 0 {
            self.lang_index = from.lang_index;
        }
        if self.open_func.is_none() {
            self.open_func = from.open_func.clone();
        }
        if self.join_stat_func.is_none() {
            self.join_stat_func = from.join_stat_func.clone();
        }
        if self.inclusion_filter.is_none() {
            self.inclusion_filter = from.inclusion_filter.clone();
        }
        if self.rename.is_none() {
            self.rename = from.rename.clone();
        }

        // Go: `if m.InclusionFilter == nil { m.InclusionFilter = from.InclusionFilter }` (again).
        if self.inclusion_filter.is_none() {
            self.inclusion_filter = from.inclusion_filter.clone();
        }
    }

    // Go: hugofs/fileinfo.go:Open
    pub fn open(&self) -> Result<Box<dyn File>> {
        match &self.open_func {
            None => Err(Error::new("OpenFunc not set")),
            Some(f) => f(),
        }
    }

    // Go: hugofs/fileinfo.go:ReadAll
    pub fn read_all(&self) -> Result<Vec<u8>> {
        let mut file = self.open()?;
        let mut b = Vec::new();
        let r = std::io::Read::read_to_end(&mut file, &mut b);
        let _ = file.close();
        r.map_err(crate::oserror::from_io_plain)?;
        Ok(b)
    }

    // Go: hugofs/fileinfo.go:JoinStat
    pub fn join_stat(&self, name: &str) -> Result<FileMetaInfo> {
        match &self.join_stat_func {
            None => Err(crate::oserror::err_not_exist()),
            Some(f) => f(name),
        }
    }
}

/// Go: `hugofs.FileMetaInfo` (a `fs.DirEntry` + `os.FileInfo` + `Meta()`).
///
/// Every file info in the Rust port carries a meta (Go's plain `os.FileInfo` values get an empty
/// one, which `Merge` treats exactly like Go treats a missing one).
#[derive(Clone)]
pub struct FileMetaInfo {
    /// The name of the underlying entry (Go `DirEntry.Name()`); [`FileMetaInfo::name`] prefers
    /// `meta.name` like Go's `dirEntryMeta.Name`.
    pub name: String,
    pub is_dir: bool,
    pub size: i64,
    pub mode: u32,
    pub mod_time: Time,
    pub meta: Arc<FileMeta>,
    /// Go: the error of `DirEntry.Info()` (an entry that vanished after `ReadDir`).
    pub(crate) info_err: Option<Error>,
}

impl fmt::Debug for FileMetaInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileMetaInfo")
            .field("name", &self.name())
            .field("is_dir", &self.is_dir)
            .field("mode", &format_args!("{:#o}", self.mode))
            .field("meta", &self.meta)
            .finish()
    }
}

impl FileMetaInfo {
    /// A file info with a fresh (empty) meta: Go's plain `os.FileInfo`.
    pub fn new_plain(name: &str, is_dir: bool, size: i64, mode: u32, mod_time: Time) -> Self {
        FileMetaInfo {
            name: name.to_string(),
            is_dir,
            size,
            mode,
            mod_time,
            meta: Arc::new(FileMeta::new()),
            info_err: None,
        }
    }

    pub fn meta(&self) -> &FileMeta {
        &self.meta
    }

    /// The meta for in-place updates (Go mutates `fi.Meta()` directly).
    pub fn meta_mut(&mut self) -> &mut FileMeta {
        Arc::make_mut(&mut self.meta)
    }

    // Go: hugofs/fileinfo.go:dirEntryMeta.Name
    /// Go: `Name()` — `meta.Name` if set, else the entry's name.
    pub fn name(&self) -> &str {
        if !self.meta.name.is_empty() {
            return &self.meta.name;
        }
        &self.name
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    // Go: hugofs/fileinfo.go:dirEntryMeta.Mode
    pub fn mode(&self) -> u32 {
        self.mode
    }

    // Go: hugofs/fileinfo.go:dirEntryMeta.ModTime
    pub fn mod_time(&self) -> &Time {
        &self.mod_time
    }

    pub fn size(&self) -> i64 {
        self.size
    }

    /// Go: `Filename()`.
    // Go: hugofs/fileinfo.go:dirEntryMeta.Filename
    pub fn filename(&self) -> &str {
        &self.meta.filename
    }

    /// Go: `DirEntry.Info()` — the error is Go's lazy `lstat` failing.
    // Go: hugofs/fileinfo.go:dirEntry.Info
    pub fn info(&self) -> Result<&FileMetaInfo> {
        match &self.info_err {
            Some(e) => Err(e.clone()),
            None => Ok(self),
        }
    }

    /// Go: `DirEntry.Type()` (`Mode().Type()`).
    // Go: hugofs/fileinfo.go:dirEntry.Type
    pub fn type_bits(&self) -> u32 {
        self.mode & mode::MODE_TYPE
    }
}

/// Go: `hugofs.NewFileMetaInfo(fi, m)` — `m` gets the zero fields from `fi`'s meta.
// Go: hugofs/fileinfo.go:NewFileMetaInfo
pub fn new_file_meta_info_from(fi: &FileMetaInfo, mut m: FileMeta) -> FileMetaInfo {
    m.merge(&fi.meta);
    FileMetaInfo {
        name: fi.name().to_string(),
        is_dir: fi.is_dir,
        size: fi.size,
        mode: fi.mode,
        mod_time: fi.mod_time.clone(),
        meta: Arc::new(m),
        info_err: fi.info_err.clone(),
    }
}

/// Go: `hugofs.NewFileMetaInfo(fi, m)` for a name-only `FileNameIsDir`.
// Go: hugofs/fileinfo.go:NewFileMetaInfo
pub fn new_file_meta_info(name: &str, is_dir: bool, meta: FileMeta) -> FileMetaInfo {
    FileMetaInfo {
        name: name.to_string(),
        is_dir,
        size: 0,
        mode: if is_dir { mode::MODE_DIR } else { 0 },
        mod_time: Time::zero(),
        meta: Arc::new(meta),
        info_err: None,
    }
}

// Go: hugofs/fileinfo.go:newDirNameOnlyFileInfo
/// Go: `newDirNameOnlyFileInfo(name, meta, fileOpener)` — a virtual directory (`ModeDir`,
/// `ModTime` = `htime.Now()`); the meta is a copy of `meta` (a new one for `None`).
pub(crate) fn new_dir_name_only_file_info(
    name: &str,
    meta: Option<&FileMeta>,
    file_opener: OpenFunc,
) -> FileMetaInfo {
    let name = normalize_filename(name);
    let (_, base) = go_path::filepath::split(&name);

    let mut m = FileMeta::copy(meta);
    if m.filename.is_empty() {
        m.filename = name.clone();
    }
    m.open_func = Some(file_opener);

    // Go: NewFileMetaInfo(&dirNameOnlyFileInfo{...}, m) — not a MetaProvider, so no merge.
    FileMetaInfo {
        name: base.to_string(),
        is_dir: true,
        size: 0,
        mode: mode::MODE_DIR,
        mod_time: nh_common::htime::now(),
        meta: Arc::new(m),
        info_err: None,
    }
}

// Go: hugofs/fileinfo.go:decorateFileInfo
/// Go: `decorateFileInfo(fi, opener, filename, inMeta)` — updates `fi`'s meta in place: the
/// opener (if any), the (normalized) filename (if any), then `Merge(inMeta)`.
pub(crate) fn decorate_file_info(
    mut fi: FileMetaInfo,
    opener: Option<OpenFunc>,
    filename: &str,
    in_meta: Option<&FileMeta>,
) -> FileMetaInfo {
    let meta = fi.meta_mut();
    if let Some(opener) = opener {
        meta.open_func = Some(opener);
    }

    let nfilename = normalize_filename(filename);
    if !nfilename.is_empty() {
        meta.filename = nfilename;
    }

    if let Some(in_meta) = in_meta {
        meta.merge(in_meta);
    }

    fi
}

/// Go: `DirEntriesToFileMetaInfos(fis)` (every entry already is a `FileMetaInfo`).
// Go: hugofs/fileinfo.go:DirEntriesToFileMetaInfos
pub fn dir_entries_to_file_meta_infos(fis: Vec<FileMetaInfo>) -> Vec<FileMetaInfo> {
    fis
}

// Go: hugofs/fileinfo.go:normalizeFilename
/// Go: `normalizeFilename` — NFC on darwin (HFS+ stores NFD), unchanged elsewhere.
pub(crate) fn normalize_filename(filename: &str) -> String {
    if filename.is_empty() {
        return String::new();
    }
    #[cfg(target_os = "macos")]
    {
        // When a file system is HFS+, its filepath is in NFD form.
        crate::nfc::nfc_string(filename)
    }
    #[cfg(not(target_os = "macos"))]
    {
        filename.to_string()
    }
}

// Go: hugofs/fileinfo.go:sortDirEntries
/// Go: `sortDirEntries` — by `Meta().Filename` (`sort.Slice`).
pub fn sort_dir_entries(fis: &mut [FileMetaInfo]) {
    go_sort::sort_by(fis, |a, b| a.meta.filename < b.meta.filename);
}

// Go: hugofs/fileinfo.go:AddFileInfoToError
/// Go: `AddFileInfoToError(err, fi, fs)` — attaches the file name as the error position (the
/// source excerpt of Go's file errors is not ported; nh-common deviation 9).
pub fn add_file_info_to_error(err: Error, fi: &FileMetaInfo) -> Error {
    if let Some(pos) = err.pos()
        && !pos.filename.is_empty()
    {
        return err;
    }
    let filename = fi.meta.filename.clone();
    let pos = match err.pos() {
        Some(p) => nh_common::herrors::FilePos {
            filename,
            line: p.line,
            column: p.column,
        },
        None => nh_common::herrors::FilePos {
            filename,
            line: 0,
            column: 0,
        },
    };
    err.at(pos)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/fileinfo.go (390 lines; 18/29 funcs executed)
//   types: FileMeta, FileMetaInfo, MetaProvider, FileInfoOptionals, FileNameIsDir, FileInfoProvider, DirOnlyOps,
//          dirEntryMeta, dirEntry, dirNameOnlyFileInfo
// OK L42-44: NewFileMeta() *FileMeta
// OK L81-87: (m *FileMeta) Copy() *FileMeta
// OK L89-109: (m *FileMeta) Merge(from *FileMeta)
// OK L111-116: (f *FileMeta) Open() (afero.File, error)
// OK L118-125: (f *FileMeta) ReadAll() ([]byte, error)
// OK L127-132: (f *FileMeta) JoinStat(name string) (FileMetaInfo, error)
// OK L180-182: (fi *dirEntryMeta) Meta() *FileMeta
// OK L185-187: (fi *dirEntryMeta) Filename() string
// OK L189-198: (fi *dirEntryMeta) fileInfo() fs.FileInfo (the lstat is eager; `info()`)
// OK L200-202: (fi *dirEntryMeta) Size() int64
// OK L204-206: (fi *dirEntryMeta) Mode() fs.FileMode
// OK L208-210: (fi *dirEntryMeta) ModTime() time.Time
// OK L212-214: (fi *dirEntryMeta) Sys() any (not modelled)
// OK L217-222: (fi *dirEntryMeta) Name() string
// OK L231-231: (d dirEntry) Type() fs.FileMode
// OK L233-233: (d dirEntry) Info() (fs.FileInfo, error)
// OK L235-252: NewFileMetaInfo(fi FileNameIsDir, m *FileMeta) FileMetaInfo
// OK L259-261: (fi *dirNameOnlyFileInfo) Name() string
// OK L263-265: (fi *dirNameOnlyFileInfo) Size() int64 (Go panics; 0 here)
// OK L267-269: (fi *dirNameOnlyFileInfo) Mode() os.FileMode
// OK L271-273: (fi *dirNameOnlyFileInfo) ModTime() time.Time
// OK L275-277: (fi *dirNameOnlyFileInfo) IsDir() bool
// OK L279-281: (fi *dirNameOnlyFileInfo) Sys() any (not modelled)
// OK L283-297: newDirNameOnlyFileInfo(name string, meta *FileMeta, fileOpener func() (afero.File, error)) FileMetaInfo
// OK L299-323: decorateFileInfo(fi FileNameIsDir, opener func() (afero.File, error), filename string, inMeta *FileMeta) FileMetaInfo
// OK L325-332: DirEntriesToFileMetaInfos(fis []fs.DirEntry) []FileMetaInfo
// OK L334-343: normalizeFilename(filename string) string
// OK L345-350: sortDirEntries(fis []fs.DirEntry)
// OK L353-390: AddFileInfoToError(err error, fi FileMetaInfo, fs afero.Fs) error (position only)
// ---------------------------------------------------------------------------
