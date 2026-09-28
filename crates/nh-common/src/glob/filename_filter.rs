//! Port of `hugofs/glob/filename_filter.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

use std::fmt;
use std::sync::Arc;

use go_path::{filepath, path};

use super::glob::{Glob, IS_WINDOWS, get_glob};
use crate::herrors::Result;

/// A Go `func(filename string) bool` inclusion predicate.
pub type ShouldInclude = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Go: `glob.FilenameFilter` — mount include/exclude files filter. `Match` returns whether a
/// filename should be included; the inclusions are checked first.
#[derive(Clone, Default)]
pub struct FilenameFilter {
    pub(crate) should_include: Option<ShouldInclude>,
    pub(crate) inclusions: Vec<Glob>,
    pub(crate) dir_inclusions: Vec<Glob>,
    pub(crate) exclusions: Vec<Glob>,
    pub is_windows: bool,

    pub(crate) nested: Vec<Option<Arc<FilenameFilter>>>,
}

impl fmt::Debug for FilenameFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilenameFilter")
            .field("should_include", &self.should_include.is_some())
            .field("inclusions", &self.inclusions)
            .field("dir_inclusions", &self.dir_inclusions)
            .field("exclusions", &self.exclusions)
            .field("nested", &self.nested)
            .finish()
    }
}

// Go: hugofs/glob/filename_filter.go:normalizeFilenameGlobPattern
fn normalize_filename_glob_pattern(s: &str) -> String {
    // Use Unix separators even on Windows.
    let s = filepath::to_slash(s);
    if !s.starts_with('/') {
        return format!("/{s}");
    }
    s.to_string()
}

impl FilenameFilter {
    /// Go: `NewFilenameFilter(inclusions, exclusions)` with Go's nil slices as `None`. Both nil
    /// gives no filter (`None`, which matches everything).
    // Go: hugofs/glob/filename_filter.go:NewFilenameFilter
    pub fn new_opt(
        inclusions: Option<&[String]>,
        exclusions: Option<&[String]>,
    ) -> Result<Option<FilenameFilter>> {
        if inclusions.is_none() && exclusions.is_none() {
            return Ok(None);
        }
        let mut filter = FilenameFilter {
            is_windows: IS_WINDOWS,
            ..Default::default()
        };

        for include in inclusions.unwrap_or_default() {
            let include = normalize_filename_glob_pattern(include);
            let g = get_glob(&include)?;
            filter.inclusions.push(g);

            // For mounts that do directory walking (e.g. content) we
            // must make sure that all directories up to this inclusion also
            // gets included.
            let dir = path::dir(&include);
            let parts: Vec<&str> = dir.split('/').collect();
            for i in 0..parts.len() {
                let pattern = format!("/{}", filepath::join(&parts[..i + 1]));
                let g = get_glob(&pattern)?;
                filter.dir_inclusions.push(g);
            }
        }

        for exclude in exclusions.unwrap_or_default() {
            let exclude = normalize_filename_glob_pattern(exclude);
            let g = get_glob(&exclude)?;
            filter.exclusions.push(g);
        }

        Ok(Some(filter))
    }

    /// Go: `NewFilenameFilter(inclusions, exclusions)` where an empty slice stands for Go's nil
    /// (a non-nil empty Go slice gives a filter that matches everything, like no filter).
    pub fn new(inclusions: &[String], exclusions: &[String]) -> Result<Option<FilenameFilter>> {
        fn opt(s: &[String]) -> Option<&[String]> {
            if s.is_empty() { None } else { Some(s) }
        }
        Self::new_opt(opt(inclusions), opt(exclusions))
    }

    /// Go: `NewFilenameFilterForInclusionFunc(shouldInclude)`.
    // Go: hugofs/glob/filename_filter.go:NewFilenameFilterForInclusionFunc
    pub fn new_for_inclusion_func(
        should_include: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> FilenameFilter {
        FilenameFilter {
            should_include: Some(Arc::new(should_include)),
            is_windows: IS_WINDOWS,
            ..Default::default()
        }
    }

    /// Go: `Match(filename, isDir)` — whether filename should be included.
    // Go: hugofs/glob/filename_filter.go:Match
    pub fn matches(&self, filename: &str, is_dir: bool) -> bool {
        if !self.do_match(filename, is_dir) {
            return false;
        }

        for nested in &self.nested {
            if !match_opt(nested.as_deref(), filename, is_dir) {
                return false;
            }
        }

        true
    }

    // Go: hugofs/glob/filename_filter.go:doMatch
    fn do_match(&self, filename: &str, is_dir: bool) -> bool {
        let filename = if !filename.starts_with('/') {
            format!("/{filename}")
        } else {
            filename.to_string()
        };

        // (f.isWindows is false: no FromSlash retry.)
        if let Some(should_include) = &self.should_include
            && should_include(&filename)
        {
            return true;
        }

        for inclusion in &self.inclusions {
            if inclusion.matches(&filename) {
                return true;
            }
        }

        if is_dir && !self.inclusions.is_empty() {
            for inclusion in &self.dir_inclusions {
                if inclusion.matches(&filename) {
                    return true;
                }
            }
        }

        for exclusion in &self.exclusions {
            if exclusion.matches(&filename) {
                return false;
            }
        }

        self.inclusions.is_empty() && self.should_include.is_none()
    }
}

/// Go: `(*FilenameFilter).Match` on a possibly nil filter (nil matches everything).
pub fn match_opt(f: Option<&FilenameFilter>, filename: &str, is_dir: bool) -> bool {
    match f {
        None => true,
        Some(f) => f.matches(filename, is_dir),
    }
}

/// Go: `(*FilenameFilter).Append(other)` — appends a filter to the chain; the receiver is
/// copied (nil receiver: `other`).
// Go: hugofs/glob/filename_filter.go:Append
pub fn append(f: Option<&FilenameFilter>, other: Option<FilenameFilter>) -> Option<FilenameFilter> {
    let Some(f) = f else {
        return other;
    };

    let mut clone = f.clone();
    clone.nested.push(other.map(Arc::new));

    Some(clone)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob/filename_filter.go (182 lines; 5/7 funcs executed)
//   types: FilenameFilter
// OK L34-41: normalizeFilenameGlobPattern(s string) string
// OK L46-85: NewFilenameFilter(inclusions, exclusions []string) (*FilenameFilter, error)
// OK L88-94: MustNewFilenameFilter(inclusions, exclusions []string) *FilenameFilter (new + unwrap)
// OK L97-99: NewFilenameFilterForInclusionFunc(shouldInclude func(filename string) bool) *FilenameFilter
// OK L102-117: (f *FilenameFilter) Match(filename string, isDir bool) bool
// OK L120-132: (f *FilenameFilter) Append(other *FilenameFilter) *FilenameFilter
// OK L134-182: (f *FilenameFilter) doMatch(filename string, isDir bool) bool
// ---------------------------------------------------------------------------
