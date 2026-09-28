//! Port of `hugofs/glob/glob.go`.
//!
//! plus a gobwas/glob port (only the syntax Hugo uses)
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `hugofs/glob` over [`super::gobwas`], the port of `github.com/gobwas/glob` v0.2.3
//! (`*`, `**`, `?`, `[...]`, `{a,b}` with `/` as separator; patterns and inputs are lower-cased).

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use go_path::{filepath, path};
use go_unicode::strings;

use super::gobwas::{self, Matcher};
use crate::herrors::{Error, Result};

/// Go: `hugofs/glob.isWindows` (always false here).
pub const IS_WINDOWS: bool = false;

/// A compiled glob (Go: `glob.Glob`): a `globDecorator` from [`get_glob`], a `globSlice` from
/// [`or`] or a `MatchesFunc` from [`matches_func`].
#[derive(Clone)]
pub struct Glob {
    /// The pattern given to [`get_glob`] (empty for `Or` and `MatchesFunc`).
    pub pattern: String,
    kind: GlobKind,
}

#[derive(Clone)]
enum GlobKind {
    /// Go: `globDecorator`.
    Decorator { g: Arc<Matcher>, is_windows: bool },
    /// Go: `globSlice`.
    Slice(Vec<Glob>),
    /// Go: `MatchesFunc`.
    Func(Arc<dyn Fn(&str) -> bool + Send + Sync>),
}

impl fmt::Debug for Glob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            GlobKind::Decorator { g, .. } => f
                .debug_struct("Glob")
                .field("pattern", &self.pattern)
                .field("matcher", &g.string())
                .finish(),
            GlobKind::Slice(gs) => f.debug_tuple("Or").field(gs).finish(),
            GlobKind::Func(_) => f.write_str("MatchesFunc"),
        }
    }
}

impl Glob {
    /// Go: `Glob.Match(s)`.
    pub fn matches(&self, s: &str) -> bool {
        match &self.kind {
            GlobKind::Func(f) => f(s),
            _ => self.matches_bytes(s.as_bytes()),
        }
    }

    /// [`Glob::matches`] over Go string bytes (a `MatchesFunc` sees them as UTF-8; invalid input
    /// never matches it).
    pub fn matches_bytes(&self, s: &[u8]) -> bool {
        match &self.kind {
            // Go: hugofs/glob/glob.go:(globDecorator).Match
            GlobKind::Decorator { g, is_windows } => {
                let _ = is_windows; // filepath.ToSlash is the identity off Windows.
                let s = strings::to_lower(s);
                g.is_match(&s)
            }
            // Go: hugofs/glob/glob.go:(globSlice).Match
            GlobKind::Slice(globs) => globs.iter().any(|g| g.matches_bytes(s)),
            // Go: hugofs/glob/glob.go:(MatchesFunc).Match
            GlobKind::Func(f) => std::str::from_utf8(s).is_ok_and(|s| f(s)),
        }
    }

    /// The compiled gobwas matcher (for a glob from [`get_glob`]).
    pub fn matcher(&self) -> Option<&Matcher> {
        match &self.kind {
            GlobKind::Decorator { g, .. } => Some(g),
            _ => None,
        }
    }
}

/// Go: `glob.Or(globs...)` — a Glob matching if any of the given globs match.
// Go: hugofs/glob/glob.go:Or
pub fn or(globs: Vec<Glob>) -> Glob {
    Glob {
        pattern: String::new(),
        kind: GlobKind::Slice(globs),
    }
}

/// Go: `glob.MatchesFunc` — a Glob from a function.
// Go: hugofs/glob/glob.go:(MatchesFunc).Match
pub fn matches_func(f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Glob {
    Glob {
        pattern: String::new(),
        kind: GlobKind::Func(Arc::new(f)),
    }
}

/// Go: `globCache`.
struct GlobCache {
    is_windows: bool,
    cache: Mutex<HashMap<String, Result<Glob>>>,
}

impl GlobCache {
    // Go: hugofs/glob/glob.go:(*globCache).GetGlob
    fn get_glob(&self, pattern: &str) -> Result<Glob> {
        if let Some(eg) = self.cache.lock().unwrap().get(pattern) {
            return eg.clone();
        }

        let original = pattern;
        let pattern = filepath::to_slash(pattern);
        let lower = strings::to_lower(pattern.as_bytes());
        let eg = match gobwas::compile(&lower, &['/' as gobwas::Rune]) {
            Ok(g) => Ok(Glob {
                pattern: original.to_string(),
                kind: GlobKind::Decorator {
                    g: Arc::new(g),
                    is_windows: self.is_windows,
                },
            }),
            Err(e) => Err(Error::new(e)),
        };

        self.cache
            .lock()
            .unwrap()
            .insert(pattern.to_string(), eg.clone());

        eg
    }
}

fn default_glob_cache() -> &'static GlobCache {
    static C: OnceLock<GlobCache> = OnceLock::new();
    C.get_or_init(|| GlobCache {
        is_windows: IS_WINDOWS,
        cache: Mutex::new(HashMap::new()),
    })
}

/// Go: `glob.GetGlob(pattern)` (cached; the pattern is lower-cased and compiled with `/` as the
/// separator; matching lower-cases its input).
// Go: hugofs/glob/glob.go:GetGlob
pub fn get_glob(pattern: &str) -> Result<Glob> {
    default_glob_cache().get_glob(pattern)
}

/// Go: `glob.NormalizePath`.
// Go: hugofs/glob/glob.go:NormalizePath
pub fn normalize_path(p: &str) -> String {
    let s = normalize_path_no_lower(p);
    String::from_utf8(strings::to_lower(s.as_bytes()).into_owned())
        .expect("ToLower of valid UTF-8 is valid UTF-8")
}

/// Go: `glob.NormalizePathNoLower` — `strings.Trim(path.Clean(filepath.ToSlash(p)), "/.")`.
// Go: hugofs/glob/glob.go:NormalizePathNoLower
pub fn normalize_path_no_lower(p: &str) -> String {
    path::clean(filepath::to_slash(p))
        .trim_matches(['/', '.'])
        .to_string()
}

/// Go: `glob.ResolveRootDir` — takes a normalized path on the form "assets/**.json" and
/// determines any root dir, i.e. any start path without any wildcards.
// Go: hugofs/glob/glob.go:ResolveRootDir
pub fn resolve_root_dir(p: &str) -> String {
    let d = path::dir(p);
    let parts: Vec<&str> = d.split('/').collect();
    let mut roots: Vec<&str> = Vec::new();
    for part in parts {
        if has_glob_char(part) {
            break;
        }
        roots.push(part);
    }

    if roots.is_empty() {
        return String::new();
    }

    roots.join("/")
}

/// Go: `glob.FilterGlobParts` — removes any string with glob wildcard.
// Go: hugofs/glob/glob.go:FilterGlobParts
pub fn filter_glob_parts(a: Vec<String>) -> Vec<String> {
    a.into_iter().filter(|x| !has_glob_char(x)).collect()
}

/// Go: `glob.HasGlobChar` — whether s contains any glob wildcards.
// Go: hugofs/glob/glob.go:HasGlobChar
pub fn has_glob_char(s: &str) -> bool {
    s.bytes().any(gobwas::special)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugofs/glob/glob.go (175 lines; 0/11 funcs executed)
//   types: globErr, globCache, MatchesFunc, globSlice, globDecorator
// OK L52-82: (gc *globCache) GetGlob(pattern string) (glob.Glob, error)
// OK L85-87: Or(globs ...glob.Glob) glob.Glob
// OK L92-94: (m MatchesFunc) Match(s string) bool
// OK L100-107: (g globSlice) Match(s string) bool
// OK L117-123: (g globDecorator) Match(s string) bool
// OK L125-127: GetGlob(pattern string) (glob.Glob, error)
// OK L129-131: NormalizePath(p string) string
// OK L133-135: NormalizePathNoLower(p string) string
// OK L139-154: ResolveRootDir(p string) string
// OK L157-165: FilterGlobParts(a []string) []string
// OK L168-175: HasGlobChar(s string) bool
// Source: github.com/gobwas/glob@v0.2.3 (in super::gobwas)
// OK glob.go: Compile, QuoteMeta (MustCompile: compile + panic, not needed)
// OK syntax/syntax.go: Parse, Special
// OK syntax/lexer/{lexer,token}.go: every function
// OK syntax/ast/{ast,parser}.go: every function (Node.String not needed)
// OK compiler/compiler.go: every function
// OK match/*.go: every matcher's Match, Index, Len, String; appendMerge, reverseSegments;
//    segments.go pools (no-op: buffers only)
// OK util/runes/runes.go: IndexRune, Equal (the ones the lexer/compiler use)
// OK util/strings/strings.go: IndexAnyRunes, LastIndexAnyRunes
// ---------------------------------------------------------------------------
