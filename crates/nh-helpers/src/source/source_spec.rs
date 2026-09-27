//! Port of `source/sourceSpec.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::sync::Arc;

use nh_common::Result;
use nh_hugofs::afero::Fs;

use crate::pathspec::PathSpec;

/// Go: `source.SourceSpec`.
#[derive(Clone)]
pub struct SourceSpec {
    pub path_spec: Arc<PathSpec>,
    pub source_fs: Arc<dyn Fs>,
}

impl SourceSpec {
    // Go: source/sourceSpec.go:NewSourceSpec
    pub fn new(ps: Arc<PathSpec>, source_fs: Arc<dyn Fs>) -> Arc<SourceSpec> {
        todo!()
    }

    /// Go: `IgnoreFile(filename)` — base names starting with `.` or `#`, or ending with `~`
    /// (plus `ignoreFiles` config regexps). Not applied to the static copy.
    // Go: source/sourceSpec.go:IgnoreFile
    pub fn ignore_file(&self, filename: &str) -> bool {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: source/sourceSpec.go (90 lines; 2/2 funcs executed)
//   types: SourceSpec
// EX L39-52: NewSourceSpec(ps *helpers.PathSpec, inclusionFilter *glob.FilenameFilter, fs afero.Fs) *SourceSpec
// EX L55-90: (s *SourceSpec) IgnoreFile(filename string) bool
// ---------------------------------------------------------------------------
