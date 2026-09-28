//! Port of `hugofs/hasbytes_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

//! Go `hugofs.hasBytesFs`: wraps the publish fs; for files whose name passes `should_check`
//! (text media type suffixes) every written byte is scanned for `patterns` (`__h_pp_l1`,
//! `__hdeferred/`) and matches are reported on Close (-> BuildState.filenamesWithPostPrefix).

use std::any::Any;
use std::io::Write;
use std::sync::Arc;

use nh_common::Result;
use nh_common::hugio::HasBytesWriter;

use crate::afero::{File, Fs};
use crate::fileinfo::FileMetaInfo;
use crate::fs::is_write;

/// `shouldCheck func(name string) bool`.
pub type ShouldCheckFunc = Arc<dyn Fn(&str) -> bool + Send + Sync>;
/// `hasBytesCallback func(name string, match []byte)`.
pub type HasBytesCallback = Arc<dyn Fn(&str, &[u8]) + Send + Sync>;

/// Go: `hugofs.hasBytesFs`.
pub struct HasBytesFs {
    fs: Arc<dyn Fs>,
    should_check: ShouldCheckFunc,
    has_bytes_callback: HasBytesCallback,
    patterns: Vec<Vec<u8>>,
}

/// Go: `hugofs.NewHasBytesReceiver(delegate, shouldCheck, hasBytesCallback, patterns...)`.
// Go: hugofs/hasbytes_fs.go:NewHasBytesReceiver
pub fn new_has_bytes_receiver(
    delegate: Arc<dyn Fs>,
    should_check: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    has_bytes_callback: Arc<dyn Fn(&str, &[u8]) + Send + Sync>,
    patterns: Vec<Vec<u8>>,
) -> Arc<dyn Fs> {
    Arc::new(HasBytesFs {
        fs: delegate,
        should_check,
        has_bytes_callback,
        patterns,
    })
}

impl HasBytesFs {
    // Go: hugofs/hasbytes_fs.go:wrapFile
    fn wrap_file(&self, f: Box<dyn File>) -> Box<dyn File> {
        if !(self.should_check)(&f.name()) {
            return f;
        }
        Box::new(HasBytesFile {
            file: f,
            hbw: HasBytesWriter::new(self.patterns.clone()),
            has_bytes_callback: self.has_bytes_callback.clone(),
        })
    }
}

impl Fs for HasBytesFs {
    // Go: hugofs/hasbytes_fs.go:Name
    fn name(&self) -> &str {
        "hasBytesFs"
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.fs.as_ref())
    }
    // Go: hugofs/hasbytes_fs.go:UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        Some(self.fs.clone())
    }
    // Go: hugofs/hasbytes_fs.go:Create
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        let f = self.fs.create(name)?;
        Ok(self.wrap_file(f))
    }
    // Go: hugofs/hasbytes_fs.go:OpenFile
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        let f = self.fs.open_file(name, flag, perm)?;
        if is_write(flag) {
            return Ok(self.wrap_file(f));
        }
        Ok(f)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `hugofs.hasBytesFile`.
struct HasBytesFile {
    has_bytes_callback: HasBytesCallback,
    hbw: HasBytesWriter,
    file: Box<dyn File>,
}

impl std::io::Read for HasBytesFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Seek for HasBytesFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Write for HasBytesFile {
    // Go: hugofs/hasbytes_fs.go:(h *hasBytesFile) Write
    fn write(&mut self, p: &[u8]) -> std::io::Result<usize> {
        // Go's File.Write writes all of p or fails.
        self.file.write_all(p)?;
        Ok(self.hbw.write(p))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl File for HasBytesFile {
    fn name(&self) -> String {
        self.file.name()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        self.file.stat()
    }
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        self.file.read_dir(count)
    }
    // Go: hugofs/hasbytes_fs.go:(h *hasBytesFile) Close
    fn close(&mut self) -> Result<()> {
        let name = self.file.name();
        for p in &self.hbw.patterns {
            if p.matched {
                (self.has_bytes_callback)(&name, &p.pattern);
            }
        }
        self.file.close()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/hasbytes_fs.go (102 lines; 6/8 funcs executed)
//   types: hasBytesFs, hasBytesFile
// OK L35-37: NewHasBytesReceiver(delegate afero.Fs, shouldCheck func(name string) bool, hasBytesCallback func(name string, match []byte), patterns ...[]byte) af...
// OK L39-41: (fs *hasBytesFs) UnwrapFilesystem() afero.Fs
// OK L43-49: (fs *hasBytesFs) Create(name string) (afero.File, error)
// OK L51-57: (fs *hasBytesFs) OpenFile(name string, flag int, perm os.FileMode) (afero.File, error)
// OK L59-75: (fs *hasBytesFs) wrapFile(f afero.File) afero.File
// OK L77-79: (fs *hasBytesFs) Name() string
// OK L87-93: (h *hasBytesFile) Write(p []byte) (n int, err error)
// OK L95-102: (h *hasBytesFile) Close() error
// ---------------------------------------------------------------------------
