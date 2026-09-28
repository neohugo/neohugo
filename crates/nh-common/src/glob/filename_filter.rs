//! Port of `hugofs/glob/filename_filter.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

use super::glob::Glob;
use crate::herrors::Result;

/// Go: `glob.FilenameFilter` — mount include/exclude files filter.
#[derive(Clone, Debug, Default)]
pub struct FilenameFilter {
    pub shoulds: Vec<Glob>,
    pub should_nots: Vec<Glob>,
    pub is_windows: bool,
}

impl FilenameFilter {
    // Go: hugofs/glob/filename_filter.go:NewFilenameFilter
    pub fn new(inclusions: &[String], exclusions: &[String]) -> Result<Option<FilenameFilter>> {
        todo!()
    }

    // Go: hugofs/glob/filename_filter.go:Match
    pub fn matches(&self, filename: &str, is_dir: bool) -> bool {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob/filename_filter.go (182 lines; 5/7 funcs executed)
//   types: FilenameFilter
//    L34-41: normalizeFilenameGlobPattern(s string) string
// EX L46-85: NewFilenameFilter(inclusions, exclusions []string) (*FilenameFilter, error)
//    L88-94: MustNewFilenameFilter(inclusions, exclusions []string) *FilenameFilter
// EX L97-99: NewFilenameFilterForInclusionFunc(shouldInclude func(filename string) bool) *FilenameFilter
// EX L102-117: (f *FilenameFilter) Match(filename string, isDir bool) bool
// EX L120-132: (f *FilenameFilter) Append(other *FilenameFilter) *FilenameFilter
// EX L134-182: (f *FilenameFilter) doMatch(filename string, isDir bool) bool
// ---------------------------------------------------------------------------
