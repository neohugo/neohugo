//! Port of `hugofs/filename_filter_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).

use std::any::Any;
use std::sync::Arc;

use go_value::Time;
use nh_common::Result;
use nh_common::glob::filename_filter::FilenameFilter;

use crate::afero::{File, Fs, forward_file_io};
use crate::fileinfo::FileMetaInfo;
use crate::oserror;

/// Go: `hugofs.newFilenameFilterFs(fs, base, filter)` (mount includeFiles/excludeFiles).
// Go: hugofs/filename_filter_fs.go:newFilenameFilterFs
pub fn new_filename_filter_fs(fs: Arc<dyn Fs>, base: &str, filter: FilenameFilter) -> Arc<dyn Fs> {
    Arc::new(FilenameFilterFs {
        fs,
        base: base.to_string(),
        filter: Arc::new(filter),
    })
}

/// Go: `hugofs.filenameFilterFs` — a filesystem that filters by filename.
pub struct FilenameFilterFs {
    base: String,
    fs: Arc<dyn Fs>,
    filter: Arc<FilenameFilter>,
}

impl Fs for FilenameFilterFs {
    // Go: hugofs/filename_filter_fs.go:Name
    fn name(&self) -> &str {
        "FinameFilterFS"
    }
    // Go: hugofs/filename_filter_fs.go:UnwrapFilesystem
    fn unwrap_filesystem(&self) -> Option<Arc<dyn Fs>> {
        Some(self.fs.clone())
    }
    // Go: hugofs/filename_filter_fs.go:Open
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        let fi = self.fs.stat(name)?;

        if !self.filter.matches(name, fi.is_dir()) {
            return Err(oserror::err_not_exist());
        }

        let f = self.fs.open(name)?;

        if !fi.is_dir() {
            return Ok(f);
        }

        Ok(Box::new(FilenameFilterDir {
            file: f,
            base: self.base.clone(),
            filter: self.filter.clone(),
        }))
    }
    // Go: hugofs/filename_filter_fs.go:OpenFile
    fn open_file(&self, name: &str, _flag: i32, _perm: u32) -> Result<Box<dyn File>> {
        self.open(name)
    }
    // Go: hugofs/filename_filter_fs.go:Stat
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        let fi = self.fs.stat(name)?;
        if !self.filter.matches(name, fi.is_dir()) {
            return Err(oserror::err_not_exist());
        }
        Ok(fi)
    }
    // Go: hugofs/filename_filter_fs.go:Chmod
    fn chmod(&self, _n: &str, _m: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Chtimes
    fn chtimes(&self, _n: &str, _a: &Time, _m: &Time) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Chown
    fn chown(&self, _n: &str, _uid: i32, _gid: i32) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Remove
    fn remove(&self, _n: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:RemoveAll
    fn remove_all(&self, _p: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Rename
    fn rename(&self, _o: &str, _n: &str) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Create
    fn create(&self, _n: &str) -> Result<Box<dyn File>> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:Mkdir
    fn mkdir(&self, _n: &str, _p: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    // Go: hugofs/filename_filter_fs.go:MkdirAll
    fn mkdir_all(&self, _n: &str, _p: u32) -> Result<()> {
        Err(oserror::eperm())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `hugofs.filenameFilterDir`.
struct FilenameFilterDir {
    file: Box<dyn File>,
    base: String,
    filter: Arc<FilenameFilter>,
}

forward_file_io!(FilenameFilterDir, file);

impl File for FilenameFilterDir {
    fn name(&self) -> String {
        self.file.name()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        self.file.stat()
    }
    // Go: hugofs/filename_filter_fs.go:(f *filenameFilterDir) ReadDir
    fn read_dir(&mut self, n: i32) -> Result<Vec<FileMetaInfo>> {
        let des = self.file.read_dir(n)?;
        let mut out = Vec::with_capacity(des.len());
        for de in des {
            let rel = de
                .meta()
                .filename
                .strip_prefix(self.base.as_str())
                .unwrap_or(&de.meta().filename)
                .to_string();
            if self.filter.matches(&rel, de.is_dir()) {
                out.push(de);
            }
        }
        Ok(out)
    }
    // Go: hugofs/filename_filter_fs.go:(f *filenameFilterDir) Readdirnames
    fn readdirnames(&mut self, count: i32) -> Result<Vec<String>> {
        Ok(self
            .read_dir(count)?
            .iter()
            .map(|d| d.name().to_string())
            .collect())
    }
    fn close(&mut self) -> Result<()> {
        self.file.close()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/filename_filter_fs.go (172 lines; 0/19 funcs executed)
//   types: filenameFilterFs, filenameFilterDir
// OK L29-35: newFilenameFilterFs(fs afero.Fs, base string, filter *glob.FilenameFilter) afero.Fs
// OK L45-47: (fs *filenameFilterFs) UnwrapFilesystem() afero.Fs
// OK L49-73: (fs *filenameFilterFs) Open(name string) (afero.File, error)
// OK L75-77: (fs *filenameFilterFs) OpenFile(name string, flag int, perm os.FileMode) (afero.File, error)
// OK L79-88: (fs *filenameFilterFs) Stat(name string) (os.FileInfo, error)
// OK L96-111: (f *filenameFilterDir) ReadDir(n int) ([]fs.DirEntry, error)
// OK L113-115: (f *filenameFilterDir) Readdir(count int) ([]os.FileInfo, error) (Go panics; use ReadDir)
// OK L117-128: (f *filenameFilterDir) Readdirnames(count int) ([]string, error)
// OK L130-132: (fs *filenameFilterFs) Chmod(n string, m os.FileMode) error
// OK L134-136: (fs *filenameFilterFs) Chtimes(n string, a, m time.Time) error
// OK L138-140: (fs *filenameFilterFs) Chown(n string, uid, gid int) error
// OK L142-144: (fs *filenameFilterFs) ReadDir(name string) ([]os.FileInfo, error) (Go panics; not an afero.Fs method)
// OK L146-148: (fs *filenameFilterFs) Remove(n string) error
// OK L150-152: (fs *filenameFilterFs) RemoveAll(p string) error
// OK L154-156: (fs *filenameFilterFs) Rename(o, n string) error
// OK L158-160: (fs *filenameFilterFs) Create(n string) (afero.File, error)
// OK L162-164: (fs *filenameFilterFs) Name() string
// OK L166-168: (fs *filenameFilterFs) Mkdir(n string, p os.FileMode) error
// OK L170-172: (fs *filenameFilterFs) MkdirAll(n string, p os.FileMode) error
// ---------------------------------------------------------------------------
