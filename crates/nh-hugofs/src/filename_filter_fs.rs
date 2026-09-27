//! Port of `hugofs/filename_filter_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


use std::sync::Arc;

use nh_common::glob::filename_filter::FilenameFilter;

use crate::afero::Fs;

/// Go: `hugofs.newFilenameFilterFs(fs, base, filter)` (mount includeFiles/excludeFiles).
// Go: hugofs/filename_filter_fs.go:newFilenameFilterFs
pub fn new_filename_filter_fs(fs: Arc<dyn Fs>, base: &str, filter: FilenameFilter) -> Arc<dyn Fs> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/filename_filter_fs.go (172 lines; 0/19 funcs executed)
//   types: filenameFilterFs, filenameFilterDir
//    L29-35: newFilenameFilterFs(fs afero.Fs, base string, filter *glob.FilenameFilter) afero.Fs
//    L45-47: (fs *filenameFilterFs) UnwrapFilesystem() afero.Fs
//    L49-73: (fs *filenameFilterFs) Open(name string) (afero.File, error)
//    L75-77: (fs *filenameFilterFs) OpenFile(name string, flag int, perm os.FileMode) (afero.File, error)
//    L79-88: (fs *filenameFilterFs) Stat(name string) (os.FileInfo, error)
//    L96-111: (f *filenameFilterDir) ReadDir(n int) ([]fs.DirEntry, error)
//    L113-115: (f *filenameFilterDir) Readdir(count int) ([]os.FileInfo, error)
//    L117-128: (f *filenameFilterDir) Readdirnames(count int) ([]string, error)
//    L130-132: (fs *filenameFilterFs) Chmod(n string, m os.FileMode) error
//    L134-136: (fs *filenameFilterFs) Chtimes(n string, a, m time.Time) error
//    L138-140: (fs *filenameFilterFs) Chown(n string, uid, gid int) error
//    L142-144: (fs *filenameFilterFs) ReadDir(name string) ([]os.FileInfo, error)
//    L146-148: (fs *filenameFilterFs) Remove(n string) error
//    L150-152: (fs *filenameFilterFs) RemoveAll(p string) error
//    L154-156: (fs *filenameFilterFs) Rename(o, n string) error
//    L158-160: (fs *filenameFilterFs) Create(n string) (afero.File, error)
//    L162-164: (fs *filenameFilterFs) Name() string
//    L166-168: (fs *filenameFilterFs) Mkdir(n string, p os.FileMode) error
//    L170-172: (fs *filenameFilterFs) MkdirAll(n string, p os.FileMode) error
// ---------------------------------------------------------------------------
