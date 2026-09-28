//! Port of `markup/highlight/chromalexers/chromalexers.go` and the lookup part of
//! `github.com/alecthomas/chroma/v2` v2.19.0 `registry.go` (`LexerRegistry.Get` / `Match`).
//!
//! Owner: Wave B task T06 (markup).
//!
//! Hugo only asks whether a lexer exists (`chromalexers.Get(lang) != nil` decides whether code
//! block attributes are Chroma options), so the port keeps the lexer names, aliases and file name
//! globs (generated from Go, `chromalexers_data.rs`) and answers that question.

use super::chromalexers_data::{FILENAME_GLOBS, NAMES_AND_ALIASES};

/// chroma's `ignoredSuffixes`.
const IGNORED_SUFFIXES: [&str; 13] = [
    // Editor backups
    "~",
    ".bak",
    ".old",
    ".orig",
    // Debian and derivatives apt/dpkg/ucf backups
    ".dpkg-dist",
    ".dpkg-old",
    ".ucf-dist",
    ".ucf-new",
    ".ucf-old",
    // Red Hat and derivatives rpm backups
    ".rpmnew",
    ".rpmorig",
    ".rpmsave",
    // Build system input/template files
    ".in",
];

/// Get reports whether Chroma has a lexer for the given language name (Go: `Get(name) != nil`).
// Go: markup/highlight/chromalexers/chromalexers.go:Get
pub fn get(name: &[u8]) -> bool {
    lexer_registry_get(name)
}

/// Go: `LexerRegistry.Get(name) != nil`: by name or alias (as is, then lower-cased), then by
/// file extension (`filename.<name>`) and by exact file name.
// Go: registry.go:(*LexerRegistry).Get
fn lexer_registry_get(name: &[u8]) -> bool {
    if is_name_or_alias(name) {
        return true;
    }
    let lower = go_unicode::strings::to_lower(name);
    if is_name_or_alias(&lower) {
        return true;
    }
    let mut ext = b"filename.".to_vec();
    ext.extend_from_slice(name);
    lexer_registry_match(&ext) || lexer_registry_match(name)
}

fn is_name_or_alias(name: &[u8]) -> bool {
    NAMES_AND_ALIASES
        .binary_search_by(|k| k.as_bytes().cmp(name))
        .is_ok()
}

/// Go: `LexerRegistry.Match(filename) != nil` (primary and alias file name globs, each also with
/// the ignored backup suffixes).
// Go: registry.go:(*LexerRegistry).Match
fn lexer_registry_match(filename: &[u8]) -> bool {
    let filename = base(filename);
    for glob in FILENAME_GLOBS {
        if glob_match(glob.as_bytes(), filename) {
            return true;
        }
        for suf in IGNORED_SUFFIXES {
            let mut g = glob.as_bytes().to_vec();
            g.extend_from_slice(suf.as_bytes());
            if glob_match(&g, filename) {
                return true;
            }
        }
    }
    false
}

/// Go `filepath.Match(glob, name)`; chroma panics on a bad pattern (the built-in globs are all
/// valid).
fn glob_match(pattern: &[u8], name: &[u8]) -> bool {
    go_path::filepath::match_bytes(pattern, name).expect("chroma: bad lexer file name pattern")
}

/// Go `filepath.Base` (unix) over bytes.
fn base(path: &[u8]) -> &[u8] {
    if path.is_empty() {
        return b".";
    }
    let mut p = path;
    // Strip trailing slashes.
    while !p.is_empty() && p[p.len() - 1] == b'/' {
        p = &p[..p.len() - 1];
    }
    if let Some(i) = p.iter().rposition(|&c| c == b'/') {
        p = &p[i + 1..];
    }
    if p.is_empty() {
        return b"/";
    }
    p
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST
// Source: markup/highlight/chromalexers/chromalexers.go (50 lines)
// OK L37-50: Get(name string) chroma.Lexer (existence only; the cache is not needed)
// ---------------------------------------------------------------------------
