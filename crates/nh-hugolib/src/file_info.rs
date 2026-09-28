//! Port of `hugolib/fileInfo.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).

use std::sync::Arc;

use nh_common::Result;
use nh_helpers::source::file_info::File;
use nh_hugofs::afero::File as AferoFile;

/// Go: `fileInfo` — a `source.File` with an optional language override.
#[derive(Clone)]
pub struct FileInfo {
    pub file: Arc<File>,
    pub overridden_lang: String,
}

impl FileInfo {
    // Go: hugolib/fileInfo.go:Open
    pub fn open(&self) -> Result<Box<dyn AferoFile>> {
        self.file
            .file_info()
            .meta()
            .open()
            .map_err(|err| err.wrap("fileInfo"))
    }

    // Go: hugolib/fileInfo.go:Lang
    pub fn lang(&self) -> String {
        if !self.overridden_lang.is_empty() {
            return self.overridden_lang.clone();
        }
        self.file.lang()
    }

    // Go: hugolib/fileInfo.go:String
    pub fn string(&self) -> String {
        self.file.path()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/fileInfo.go (51 lines; 0/3 funcs executed)
//   types: fileInfo
// OK L30-37: (fi *fileInfo) Open() (afero.File, error)
// OK L39-44: (fi *fileInfo) Lang() string
// OK L46-51: (fi *fileInfo) String() string
// ---------------------------------------------------------------------------
