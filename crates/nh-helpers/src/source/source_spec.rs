//! Port of `source/sourceSpec.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::sync::Arc;

use nh_common::glob::filename_filter::{FilenameFilter, match_opt};
use nh_hugofs::afero::{Fs, OsFs};

use crate::pathspec::PathSpec;

/// Go: `source.SourceSpec`.
#[derive(Clone)]
pub struct SourceSpec {
    pub path_spec: Arc<PathSpec>,
    pub source_fs: Arc<dyn Fs>,
    /// Go `shouldInclude func(filename string) bool` (the inclusion filter + `cfg.IgnoreFile`).
    inclusion_filter: Option<FilenameFilter>,
}

impl std::ops::Deref for SourceSpec {
    type Target = PathSpec;
    /// Go embeds `*helpers.PathSpec`.
    fn deref(&self) -> &PathSpec {
        &self.path_spec
    }
}

impl SourceSpec {
    /// Go: `NewSourceSpec(ps, inclusionFilter, fs)` (`None` = Go's nil filter, which matches
    /// everything).
    // Go: source/sourceSpec.go:NewSourceSpec
    pub fn new(
        ps: Arc<PathSpec>,
        inclusion_filter: Option<FilenameFilter>,
        source_fs: Arc<dyn Fs>,
    ) -> Arc<SourceSpec> {
        Arc::new(SourceSpec {
            path_spec: ps,
            source_fs,
            inclusion_filter,
        })
    }

    // Go: source/sourceSpec.go:NewSourceSpec (the shouldInclude closure)
    fn should_include(&self, filename: &str) -> bool {
        if !match_opt(self.inclusion_filter.as_ref(), filename, false) {
            return false;
        }
        if self.path_spec.cfg.ignore_file(filename) {
            return false;
        }
        true
    }

    /// Go: `IgnoreFile(filename)` — base names starting with `.` or `#`, or ending with `~`
    /// (plus the inclusion filter and the `ignoreFiles` config regexps). Not applied to the
    /// static copy.
    // Go: source/sourceSpec.go:IgnoreFile
    pub fn ignore_file(&self, filename: &str) -> bool {
        if filename.is_empty() {
            return self.source_fs.as_any().is::<OsFs>();
        }

        let base = go_path::filepath::base(filename).as_bytes();

        if !base.is_empty() {
            let first = base[0];
            let last = base[base.len() - 1];
            if first == b'.' || first == b'#' || last == b'~' {
                return true;
            }
        }

        if !self.should_include(filename) {
            return true;
        }

        // runtime.GOOS == "windows": not ported (unix only).
        false
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: source/sourceSpec.go (90 lines; 2/2 funcs executed)
//   types: SourceSpec
// OK L39-52: NewSourceSpec(ps *helpers.PathSpec, inclusionFilter *glob.FilenameFilter, fs afero.Fs) *SourceSpec
// OK L55-90: (s *SourceSpec) IgnoreFile(filename string) bool
// ---------------------------------------------------------------------------
