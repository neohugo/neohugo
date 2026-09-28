//! Port of `transform/livereloadinject/livereloadinject.go`.
//!
//! Server only: a build never sets `Descriptor.LiveReloadBaseURL`, so the publisher never adds
//! this transformer; it is ported (it is small) for completeness.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! The Go regexps are matched by hand, with RE2's semantics: `\s` is `[\t\n\f\r ]`, `(?i)` on
//! the ASCII letters of `doctype`/`html`/`head` is ASCII case folding (none of them has a
//! non-ASCII simple fold), and `[^>]` matches any byte but `>` (Go decodes an invalid byte as
//! U+FFFD, which is not `>`).

use crate::chain::Transformer;

/// RE2 `\s`.
fn is_re_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// `bytes.Index(b, sep)` as an Option.
fn find(b: &[u8], sep: &[u8]) -> Option<usize> {
    b.windows(sep.len()).position(|w| w == sep)
}

/// Whether `b` starts with `lit`, ASCII case-insensitively.
fn has_prefix_fold(b: &[u8], lit: &[u8]) -> bool {
    b.len() >= lit.len() && b[..lit.len()].eq_ignore_ascii_case(lit)
}

/// Go: `ignoredSyntax.Find(b)` = `(?s)^(?:\s+|<!--.*?-->|<\?.*?\?>)*`; returns the match length.
fn ignored_syntax(b: &[u8]) -> usize {
    let mut i = 0;
    loop {
        if i < b.len() && is_re_space(b[i]) {
            while i < b.len() && is_re_space(b[i]) {
                i += 1;
            }
            continue;
        }
        if b[i..].starts_with(b"<!--")
            && let Some(j) = find(&b[i + 4..], b"-->")
        {
            i += 4 + j + 3;
            continue;
        }
        if b[i..].starts_with(b"<?")
            && let Some(j) = find(&b[i + 2..], b"?>")
        {
            i += 2 + j + 2;
            continue;
        }
        return i;
    }
}

/// `(?is)^<!doctype\s[^>]*>`.
fn doctype_tag(b: &[u8]) -> usize {
    if !has_prefix_fold(b, b"<!doctype") || b.len() < 10 || !is_re_space(b[9]) {
        return 0;
    }
    match b[10..].iter().position(|&c| c == b'>') {
        Some(j) => 10 + j + 1,
        None => 0,
    }
}

/// `(?is)^<name(?:\s[^>]*)?>` for `name` = `html` / `head`.
fn start_tag(b: &[u8], open: &[u8]) -> usize {
    if !has_prefix_fold(b, open) {
        return 0;
    }
    let n = open.len();
    if n < b.len() && is_re_space(b[n]) {
        if let Some(j) = b[n + 1..].iter().position(|&c| c == b'>') {
            return n + 1 + j + 1;
        }
        return 0;
    }
    if n < b.len() && b[n] == b'>' {
        return n + 1;
    }
    0
}

/// Go: `livereloadinject.New(baseURL)`.
// Go: transform/livereloadinject/livereloadinject.go:New
pub fn new_transformer(base_url: &go_url::Url) -> Transformer {
    let path = base_url
        .path
        .strip_suffix(b"/")
        .unwrap_or(&base_url.path)
        .to_vec();
    let port = base_url.port().to_vec();

    Box::new(move |ft| {
        let b = ft.from;

        // We find the start of the head by reading past (in order)
        // the doctype declaration, HTML start tag and head start tag,
        // all of which are optional, and any whitespace, comments, or
        // XML instructions in-between.
        let mut idx = 0;
        for tag in [
            doctype_tag as fn(&[u8]) -> usize,
            |b: &[u8]| start_tag(b, b"<html"),
            |b: &[u8]| start_tag(b, b"<head"),
        ] {
            idx += ignored_syntax(&b[idx..]);
            idx += tag(&b[idx..]);
        }

        let mut src = path.clone();
        src.extend_from_slice(b"/livereload.js?mindelay=10&v=2");
        src.extend_from_slice(b"&port=");
        src.extend_from_slice(&port);
        src.extend_from_slice(b"&path=");
        let mut lr = path.clone();
        lr.extend_from_slice(b"/livereload");
        src.extend_from_slice(lr.strip_prefix(b"/").unwrap_or(&lr));

        let mut script = b"<script src=\"".to_vec();
        script.extend_from_slice(&go_html::escape_string_bytes(&src));
        script.extend_from_slice(b"\" data-no-instant defer></script>");

        let mut c = Vec::with_capacity(b.len() + script.len());
        c.extend_from_slice(&b[..idx]);
        c.extend_from_slice(&script);
        c.extend_from_slice(&b[idx..]);

        // Go logs a warning when the write fails; writing to the buffer cannot fail.
        ft.to.extend_from_slice(&c);
        Ok(())
    })
}

/// STUB kept from the skeleton: a build never injects the livereload script (server only).
pub fn new() -> Option<crate::chain::Transformer> {
    None
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/livereloadinject/livereloadinject.go (71 lines; 0/1 funcs executed)
// OK L39-71: New(baseURL *url.URL) transform.Transformer
// ---------------------------------------------------------------------------
