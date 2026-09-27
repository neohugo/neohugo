//! Port of `helpers/url.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


//! Go `helpers/url.go`: `urlize`, `absURL`/`absLangURL`, `relURL`/`relLangURL` (canonifyURLs=true
//! skips AddContextRoot in RelURL — see specs/i18n-lang-misc.md §3.2).

use crate::pathspec::PathSpec;

impl PathSpec {
    /// Go: `URLize(uri)` = `URLEscape(MakePathSanitized(uri))`.
    // Go: helpers/url.go:URLize
    pub fn urlize(&self, uri: &str) -> String { todo!() }
    // Go: helpers/url.go:URLizeFilename
    pub fn urlize_filename(&self, filename: &str) -> String { todo!() }
    /// Go: `URLEscape(uri)` = `url.Parse(uri).String()` (panics on error in Go).
    // Go: helpers/url.go:URLEscape
    pub fn url_escape(&self, uri: &str) -> String { todo!() }
    /// Go: `AbsURL(in, addLanguage)` — parse error -> `in` unchanged (why `%!s(<nil>)` survives).
    // Go: helpers/url.go:AbsURL
    pub fn abs_url(&self, input: &str, add_language: bool) -> String { todo!() }
    // Go: helpers/url.go:IsAbsURL
    pub fn is_abs_url(&self, input: &str) -> bool { todo!() }
    // Go: helpers/url.go:RelURL
    pub fn rel_url(&self, input: &str, add_language: bool) -> String { todo!() }
    // Go: helpers/url.go:PrependBasePath
    pub fn prepend_base_path(&self, rel: &str, is_abs: bool) -> String { todo!() }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/url.go (189 lines; 7/8 funcs executed)
// EX L30-32: (p *PathSpec) URLize(uri string) string
//    L36-38: (p *PathSpec) URLizeFilename(filename string) string
// EX L41-50: (p *PathSpec) URLEscape(uri string) string
// EX L53-92: (p *PathSpec) AbsURL(in string, addLanguage bool) string
// EX L94-102: (p *PathSpec) getBaseURLRoot(path string) string
// EX L104-114: (p *PathSpec) IsAbsURL(in string) (bool, error)
// EX L116-174: (p *PathSpec) RelURL(in string, addLanguage bool) string
// EX L177-189: (p *PathSpec) PrependBasePath(rel string, isAbs bool) string
// ---------------------------------------------------------------------------
