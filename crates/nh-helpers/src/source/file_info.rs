//! Port of `source/fileInfo.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use go_value::{GoString, HostCtx, Object, Value};
use nh_hugofs::fileinfo::FileMetaInfo;

/// Go: `source.File` (template: `.File`; `with .File` is false when the page has no file:
/// Go passes a *nil* `*source.File` whose `IsZero()` reports true).
pub struct File {
    pub fim: FileMetaInfo,
    pub(crate) unique_id: OnceLock<String>,
}

impl File {
    /// Go: `source.NewFileInfo(fi)`.
    // Go: source/fileInfo.go:NewFileInfo
    pub fn new(fim: FileMetaInfo) -> Arc<File> {
        Arc::new(File { fim, unique_id: OnceLock::new() })
    }

    // Go: source/fileInfo.go:Filename
    pub fn filename(&self) -> &str { &self.fim.meta.filename }
    /// Go: `Path()` — content-relative path with OS separators.
    // Go: source/fileInfo.go:Path
    pub fn path(&self) -> String { todo!() }
    // Go: source/fileInfo.go:Dir
    pub fn dir(&self) -> String { todo!() }
    // Go: source/fileInfo.go:Ext
    pub fn ext(&self) -> String { todo!() }
    // Go: source/fileInfo.go:Lang
    pub fn lang(&self) -> String { todo!() }
    // Go: source/fileInfo.go:LogicalName
    pub fn logical_name(&self) -> String { todo!() }
    // Go: source/fileInfo.go:BaseFileName
    pub fn base_file_name(&self) -> String { todo!() }
    // Go: source/fileInfo.go:TranslationBaseName
    pub fn translation_base_name(&self) -> String { todo!() }
    // Go: source/fileInfo.go:ContentBaseName
    pub fn content_base_name(&self) -> String { todo!() }
    // Go: source/fileInfo.go:Section
    pub fn section(&self) -> String { todo!() }
    /// Go: `UniqueID()` = md5 hex of the content-relative path (`biscuit/x/index.en.md`).
    // Go: source/fileInfo.go:UniqueID
    pub fn unique_id(&self) -> &str { todo!() }
}

/// `*source.File` as a template value; `FileObject(None)` is the typed nil (IsZero true).
pub struct FileObject(pub Option<Arc<File>>);

nh_common::go_methods!(FileObject {
    "Filename" => |f, _c, _a| todo!(),
    "Path" => |f, _c, _a| todo!(),
    "Dir" => |f, _c, _a| todo!(),
    "Ext" => |f, _c, _a| todo!(),
    "Lang" => |f, _c, _a| todo!(),
    "LogicalName" => |f, _c, _a| todo!(),
    "BaseFileName" => |f, _c, _a| todo!(),
    "TranslationBaseName" => |f, _c, _a| todo!(),
    "ContentBaseName" => |f, _c, _a| todo!(),
    "Section" => |f, _c, _a| todo!(),
    "UniqueID" => |f, _c, _a| todo!(),
    "IsZero" => |f, _c, _a| Ok(Value::Bool(f.0.is_none())),
});

impl Object for FileObject {
    nh_common::object_basics!("*source.File");

    fn is_zero(&self) -> Option<bool> {
        Some(self.0.is_none())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: source/fileInfo.go (157 lines; 13/21 funcs executed)
//   types: File, GitInfo
// EX L41-43: (fi *File) IsContentAdapter() bool
// EX L46-46: (fi *File) Filename() string
// EX L50-50: (fi *File) Path() string
//    L54-56: (fi *File) Dir() string
// EX L59-59: (fi *File) Ext() string
//    L63-66: (fi *File) Lang() string
// EX L69-71: (fi *File) LogicalName() string
// EX L74-76: (fi *File) BaseFileName() string
//    L80-80: (fi *File) TranslationBaseName() string
// EX L84-86: (fi *File) ContentBaseName() string
//    L89-91: (fi *File) Section() string
// EX L94-97: (fi *File) UniqueID() string
// EX L100-100: (fi *File) FileInfo() hugofs.FileMetaInfo
//    L102-102: (fi *File) String() string
//    L105-109: (fi *File) Open() (hugio.ReadSeekCloser, error)
// EX L111-113: (fi *File) IsZero() bool
// EX L117-121: (fi *File) init()
//    L123-128: (fi *File) pathToDir(s string) string
// EX L130-132: (fi *File) p() *paths.Path
//    L141-148: NewContentFileInfoFrom(path, filename string) *File
// EX L150-154: NewFileInfo(fi hugofs.FileMetaInfo) *File
// ---------------------------------------------------------------------------
