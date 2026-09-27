//! Port of `hugofs/glob.go`.
//!
//! Owner: Wave B task T05 (hugofs-vfs).


use std::sync::Arc;

use nh_common::Result;

use crate::afero::Fs;
use crate::fileinfo::FileMetaInfo;

/// Go: `hugofs.Glob(fs, pattern, handle)` — walks matching files (`resources.Match`, `.Resources.Match`).
// Go: hugofs/glob.go:Glob
pub fn glob(fs: Arc<dyn Fs>, pattern: &str, handle: &mut dyn FnMut(&FileMetaInfo) -> Result<bool>) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob.go (90 lines; 0/1 funcs executed)
//    L28-90: Glob(fs afero.Fs, pattern string, handle func(fi FileMetaInfo) (bool, error)) error
// ---------------------------------------------------------------------------
