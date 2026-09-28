//! Port of `hugofs/glob/glob.go`.
//!
//! plus a gobwas/glob port (only the syntax Hugo uses)
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `hugofs/glob` over a port of the subset of `github.com/gobwas/glob` that Hugo uses
//! (`*`, `**`, `?`, `[...]`, `{a,b}` with `/` as separator, case-insensitive variants).

use crate::herrors::Result;

/// A compiled glob (Go: `glob.Glob`).
#[derive(Clone, Debug)]
pub struct Glob {
    pub pattern: String,
}

impl Glob {
    pub fn matches(&self, s: &str) -> bool {
        todo!()
    }
}

/// Go: `glob.GetGlob(pattern)` (cached, lower-cased, `/` separators).
// Go: hugofs/glob/glob.go:GetGlob
pub fn get_glob(pattern: &str) -> Result<Glob> {
    todo!()
}

/// Go: `glob.NormalizePath`.
pub fn normalize_path(p: &str) -> String {
    todo!()
}

/// Go: `glob.ResolveRootDir` (the static prefix of a pattern).
pub fn resolve_root_dir(p: &str) -> String {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob/glob.go (175 lines; 0/11 funcs executed)
//   types: globErr, globCache, MatchesFunc, globSlice, globDecorator
//    L52-82: (gc *globCache) GetGlob(pattern string) (glob.Glob, error)
//    L85-87: Or(globs ...glob.Glob) glob.Glob
//    L92-94: (m MatchesFunc) Match(s string) bool
//    L100-107: (g globSlice) Match(s string) bool
//    L117-123: (g globDecorator) Match(s string) bool
//    L125-127: GetGlob(pattern string) (glob.Glob, error)
//    L129-131: NormalizePath(p string) string
//    L133-135: NormalizePathNoLower(p string) string
//    L139-154: ResolveRootDir(p string) string
//    L157-165: FilterGlobParts(a []string) []string
//    L168-175: HasGlobChar(s string) bool
// ---------------------------------------------------------------------------
