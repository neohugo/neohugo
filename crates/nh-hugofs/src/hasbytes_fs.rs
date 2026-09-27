//! Port of `hugofs/hasbytes_fs.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


//! Go `hugofs.hasBytesFs`: wraps the publish fs; for files whose name passes `should_check`
//! (text media type suffixes) every written byte is scanned for `patterns` (`__h_pp_l1`,
//! `__hdeferred/`) and matches are reported on Close (-> BuildState.filenamesWithPostPrefix).

use std::sync::Arc;

use crate::afero::Fs;

/// Go: `hugofs.NewHasBytesReceiver(delegate, shouldCheck, hasBytesCallback, patterns...)`.
// Go: hugofs/hasbytes_fs.go:NewHasBytesReceiver
pub fn new_has_bytes_receiver(
    delegate: Arc<dyn Fs>,
    should_check: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    has_bytes_callback: Arc<dyn Fn(&str, &[u8]) + Send + Sync>,
    patterns: Vec<Vec<u8>>,
) -> Arc<dyn Fs> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/hasbytes_fs.go (102 lines; 6/8 funcs executed)
//   types: hasBytesFs, hasBytesFile
// EX L35-37: NewHasBytesReceiver(delegate afero.Fs, shouldCheck func(name string) bool, hasBytesCallback func(name string, match []byte), patterns ...[]byte) af...
//    L39-41: (fs *hasBytesFs) UnwrapFilesystem() afero.Fs
// EX L43-49: (fs *hasBytesFs) Create(name string) (afero.File, error)
// EX L51-57: (fs *hasBytesFs) OpenFile(name string, flag int, perm os.FileMode) (afero.File, error)
// EX L59-75: (fs *hasBytesFs) wrapFile(f afero.File) afero.File
//    L77-79: (fs *hasBytesFs) Name() string
// EX L87-93: (h *hasBytesFile) Write(p []byte) (n int, err error)
// EX L95-102: (h *hasBytesFile) Close() error
// ---------------------------------------------------------------------------
