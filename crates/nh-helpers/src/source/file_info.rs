//! Port of `source/fileInfo.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::sync::{Arc, OnceLock};

use go_path::filepath;
use go_value::{Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::object::args;
use nh_common::paths::pathparser::{Path, PathParser};
use nh_hugofs::afero::File as AferoFile;
use nh_hugofs::fileinfo::{FileMeta, FileMetaInfo, new_file_meta_info};

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
        Arc::new(File {
            fim,
            unique_id: OnceLock::new(),
        })
    }

    /// Go: `source.NewContentFileInfoFrom(path, filename)` (tests): a content path parsed with a
    /// parser that treats every extension as content.
    // Go: source/fileInfo.go:NewContentFileInfoFrom
    pub fn new_content_file_info_from(path: &str, filename: &str) -> Arc<File> {
        let parser = PathParser {
            is_content_ext: Some(Arc::new(|_: &str| true)),
            ..Default::default()
        };
        let meta = FileMeta {
            filename: filename.to_string(),
            path_info: Some(Arc::new(parser.parse(
                nh_common::files::COMPONENT_FOLDER_CONTENT,
                filepath::to_slash(path),
            ))),
            ..Default::default()
        };
        File::new(new_file_meta_info("", false, meta))
    }

    /// Go: `IsContentAdapter()` — the file may produce more than one page.
    // Go: source/fileInfo.go:IsContentAdapter
    pub fn is_content_adapter(&self) -> bool {
        self.path_info().is_content_data()
    }

    /// Go: `Filename()` — the absolute path and filename on disk.
    // Go: source/fileInfo.go:Filename
    pub fn filename(&self) -> &str {
        &self.fim.meta.filename
    }

    /// Go: `Path()` — content-relative path with OS separators.
    // Go: source/fileInfo.go:Path
    pub fn path(&self) -> String {
        let p = self.p();
        filepath::join(&[&p.dir()[1..], p.name()])
    }

    /// Go: `Dir()` — the directory relative to the content root.
    // Go: source/fileInfo.go:Dir
    pub fn dir(&self) -> String {
        self.path_to_dir(self.p().dir())
    }

    /// Go: `Ext()` — the extension without the dot (e.g. "md").
    // Go: source/fileInfo.go:Ext
    pub fn ext(&self) -> String {
        self.p().ext().to_string()
    }

    /// Go: `Lang()` (deprecated in Go, which logs a deprecation notice; the port does not).
    // Go: source/fileInfo.go:Lang
    pub fn lang(&self) -> String {
        self.fim.meta.lang.clone()
    }

    /// Go: `LogicalName()` — name and extension (e.g. "page.sv.md").
    // Go: source/fileInfo.go:LogicalName
    pub fn logical_name(&self) -> String {
        self.p().name().to_string()
    }

    /// Go: `BaseFileName()` — name without extension (e.g. "page.sv").
    // Go: source/fileInfo.go:BaseFileName
    pub fn base_file_name(&self) -> String {
        self.p().name_no_ext().to_string()
    }

    /// Go: `TranslationBaseName()` — without the language segment (e.g. "page").
    // Go: source/fileInfo.go:TranslationBaseName
    pub fn translation_base_name(&self) -> String {
        self.p().name_no_identifier().to_string()
    }

    /// Go: `ContentBaseName()` — TranslationBaseName or the folder name for a bundle.
    // Go: source/fileInfo.go:ContentBaseName
    pub fn content_base_name(&self) -> String {
        self.p().base_name_no_identifier().to_string()
    }

    /// Go: `Section()`.
    // Go: source/fileInfo.go:Section
    pub fn section(&self) -> String {
        self.p().section().to_string()
    }

    /// Go: `UniqueID()` = md5 hex of the slash form of `Path()` (`biscuit/x/index.en.md`).
    // Go: source/fileInfo.go:UniqueID
    pub fn unique_id(&self) -> &str {
        self.init();
        self.unique_id.get().expect("initialised")
    }

    /// Go: `FileInfo()`.
    // Go: source/fileInfo.go:FileInfo
    pub fn file_info(&self) -> &FileMetaInfo {
        &self.fim
    }

    /// Go: `String()` = `BaseFileName()`.
    // Go: source/fileInfo.go:String
    pub fn string(&self) -> String {
        self.base_file_name()
    }

    /// Go: `Open()`.
    // Go: source/fileInfo.go:Open
    pub fn open(&self) -> Result<Box<dyn AferoFile>> {
        match &self.fim.meta.open_func {
            Some(f) => f(),
            None => Err(Error::new("OpenFunc not set")),
        }
    }

    // Go: source/fileInfo.go:init
    fn init(&self) {
        self.unique_id.get_or_init(|| {
            nh_common::hashing::md5_from_string_hex_encoded(
                filepath::to_slash(&self.path()).as_bytes(),
            )
        });
    }

    // Go: source/fileInfo.go:pathToDir
    fn path_to_dir(&self, s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }
        filepath::from_slash(&format!("{}/", &s[1..])).to_string()
    }

    fn path_info(&self) -> &Path {
        self.fim
            .meta
            .path_info
            .as_deref()
            .expect("source.File without PathInfo (Go: nil pointer dereference)")
    }

    // Go: source/fileInfo.go:p
    fn p(&self) -> &Path {
        self.path_info().unnormalized()
    }
}

/// `*source.File` as a template value; `FileObject(None)` is the typed nil (IsZero true).
pub struct FileObject(pub Option<Arc<File>>);

/// Go's panic text when a method of a nil `*source.File` dereferences it (text/template reports
/// it as `error calling X: ...`).
const NIL_DEREF: &str = "runtime error: invalid memory address or nil pointer dereference";

impl FileObject {
    fn file(&self) -> go_value::Result<&File> {
        self.0
            .as_deref()
            .ok_or_else(|| go_value::Error::new(NIL_DEREF))
    }

    fn str_method(
        &self,
        a: &[Value],
        name: &str,
        f: impl FnOnce(&File) -> String,
    ) -> go_value::Result<Value> {
        args::exactly(a, 0, name)?;
        Ok(Value::string(f(self.file()?)))
    }
}

nh_common::go_methods!(FileObject {
    "BaseFileName" => |f, _c, a| f.str_method(a, "BaseFileName", |x| x.base_file_name()),
    "ContentBaseName" => |f, _c, a| f.str_method(a, "ContentBaseName", |x| x.content_base_name()),
    "Dir" => |f, _c, a| f.str_method(a, "Dir", |x| x.dir()),
    "Ext" => |f, _c, a| f.str_method(a, "Ext", |x| x.ext()),
    "FileInfo" => |_f, _c, _a| Err(go_value::Error::new(
        "neohugo-rs: .File.FileInfo is not supported in templates",
    )),
    "Filename" => |f, _c, a| f.str_method(a, "Filename", |x| x.filename().to_string()),
    "IsContentAdapter" => |f, _c, a| {
        args::exactly(a, 0, "IsContentAdapter")?;
        Ok(Value::Bool(f.file()?.is_content_adapter()))
    },
    "IsZero" => |f, _c, a| {
        args::exactly(a, 0, "IsZero")?;
        Ok(Value::Bool(f.0.is_none()))
    },
    "Lang" => |f, _c, a| f.str_method(a, "Lang", |x| x.lang()),
    "LogicalName" => |f, _c, a| f.str_method(a, "LogicalName", |x| x.logical_name()),
    "Open" => |_f, _c, _a| Err(go_value::Error::new(
        "neohugo-rs: .File.Open is not supported in templates",
    )),
    "Path" => |f, _c, a| f.str_method(a, "Path", |x| x.path()),
    "Section" => |f, _c, a| f.str_method(a, "Section", |x| x.section()),
    "String" => |f, _c, a| f.str_method(a, "String", |x| x.string()),
    "TranslationBaseName" => |f, _c, a| f.str_method(a, "TranslationBaseName", |x| x.translation_base_name()),
    "UniqueID" => |f, _c, a| f.str_method(a, "UniqueID", |x| x.unique_id().to_string()),
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
// OK L41-43: (fi *File) IsContentAdapter() bool
// OK L46-46: (fi *File) Filename() string
// OK L50-50: (fi *File) Path() string
// OK L54-56: (fi *File) Dir() string
// OK L59-59: (fi *File) Ext() string
// OK L63-66: (fi *File) Lang() string
// OK L69-71: (fi *File) LogicalName() string
// OK L74-76: (fi *File) BaseFileName() string
// OK L80-80: (fi *File) TranslationBaseName() string
// OK L84-86: (fi *File) ContentBaseName() string
// OK L89-91: (fi *File) Section() string
// OK L94-97: (fi *File) UniqueID() string
// OK L100-100: (fi *File) FileInfo() hugofs.FileMetaInfo
// OK L102-102: (fi *File) String() string
// OK L105-109: (fi *File) Open() (hugio.ReadSeekCloser, error)
// OK L111-113: (fi *File) IsZero() bool
// OK L117-121: (fi *File) init()
// OK L123-128: (fi *File) pathToDir(s string) string
// OK L130-132: (fi *File) p() *paths.Path
// OK L141-148: NewContentFileInfoFrom(path, filename string) *File
// OK L150-154: NewFileInfo(fi hugofs.FileMetaInfo) *File
// ---------------------------------------------------------------------------
