//! Port of `transform/urlreplacers/absurlreplacer.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `absurlreplacer.go` — the canonifyURLs lexer. Port LITERALLY including its quirks (stale
//! `nextPos`, `nextPos=0` initial value, srcset whitespace collapsing, unquoted href rewriting,
//! case-sensitive prefixes); see specs/output-publishing.md §3.3 for the test vectors.
//!
//! The lexer works on the whole input (`ft.From().Bytes()`), never on a stream, so chunking of
//! the publisher's source cannot change its output.

use go_unicode::{bytes as gobytes, utf8};
use nh_common::{Error, Result};

use crate::chain::FromTo;

/// Go: `absurllexer`.
struct AbsUrlLexer<'a> {
    /// the source to absurlify
    content: &'a [u8],
    /// the target for the new absurlified content
    w: &'a mut Vec<u8>,
    /// path may be set to a "." relative path
    path: &'a [u8],
    /// The root path, without leading slash.
    root: &'a [u8],
    /// input position
    pos: usize,
    /// item start position
    start: usize,
    quotes: &'a [&'a [u8]],
}

/// Which candidate check a prefix triggers (Go: `prefix.f`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateFn {
    Base,
    Srcset,
}

/// Go: `prefix`.
struct Prefix {
    disabled: bool,
    b: &'static [u8],
    f: CandidateFn,
    /// Go's zero value is 0 (not -1): every prefix "matches" at position 0 on the first round.
    next_pos: isize,
}

impl Prefix {
    // Go: transform/urlreplacers/absurlreplacer.go:find
    fn find(&mut self, bs: &[u8], start: usize) -> bool {
        if self.disabled {
            return false;
        }

        if self.next_pos == -1 {
            let idx = gobytes::index(&bs[start..], self.b);

            if idx == -1 {
                self.disabled = true;
                // Find the closest match
                return false;
            }

            self.next_pos = start as isize + idx + self.b.len() as isize;
        }

        true
    }
}

// Go: transform/urlreplacers/absurlreplacer.go:newPrefixState
fn new_prefix_state() -> Vec<Prefix> {
    let p = |b: &'static [u8], f| Prefix {
        disabled: false,
        b,
        f,
        next_pos: 0,
    };
    vec![
        p(b"src=", CandidateFn::Base),
        p(b"href=", CandidateFn::Base),
        p(b"url=", CandidateFn::Base),
        p(b"action=", CandidateFn::Base),
        p(b"srcset=", CandidateFn::Srcset),
    ]
}

/// Go: `relURLPrefix`.
const REL_URL_PREFIX: &[u8] = b"/";
/// Go: `relURLPrefixLen`.
const REL_URL_PREFIX_LEN: usize = REL_URL_PREFIX.len();

/// Go's runtime panic for `s[low:high]` with `low > high`.
fn slice_bounds_panic(low: usize, high: usize) -> Error {
    Error::new(format!(
        "runtime error: slice bounds out of range [{low}:{high}]"
    ))
}

impl AbsUrlLexer<'_> {
    // Go: transform/urlreplacers/absurlreplacer.go:emit
    fn emit(&mut self) -> Result<()> {
        if self.start > self.pos {
            // Go panics here (the stale-nextPos rewind with a quote at the new position).
            return Err(slice_bounds_panic(self.start, self.pos));
        }
        self.w
            .extend_from_slice(&self.content[self.start..self.pos]);
        self.start = self.pos;
        Ok(())
    }

    // Go: transform/urlreplacers/absurlreplacer.go:consumeQuote
    fn consume_quote(&mut self) -> Result<Option<&'static [u8]>> {
        for &q in self.quotes {
            if self.content[self.pos..].starts_with(q) {
                self.pos += q.len();
                self.emit()?;
                return Ok(Some(quote_static(q)));
            }
        }
        Ok(None)
    }

    // Go: transform/urlreplacers/absurlreplacer.go:posAfterURL
    fn pos_after_url(&self, q: &[u8]) -> isize {
        if !q.is_empty() {
            // look for end quote
            return gobytes::index(&self.content[self.pos..], q);
        }

        gobytes::index_func(&self.content[self.pos..], |r| {
            r == '>' as i32 || go_unicode::is_space(r)
        })
    }

    // Go: transform/urlreplacers/absurlreplacer.go:replace
    fn replace(&mut self) -> Result<()> {
        let content_length = self.content.len();

        let mut prefixes = new_prefix_state();

        while self.pos < content_length {
            // Go: `var match *prefix` (index and its nextPos).
            let mut matched: Option<(usize, isize)> = None;

            for (i, p) in prefixes.iter_mut().enumerate() {
                if !p.find(self.content, self.pos) {
                    continue;
                }

                if matched.is_none_or(|(_, m)| p.next_pos < m) {
                    matched = Some((i, p.next_pos));
                }
            }

            match matched {
                None => {
                    // Done!
                    self.pos = content_length;
                    break;
                }
                Some((i, _)) => {
                    // A stale nextPos (< the current position) is used as it is.
                    self.pos = prefixes[i].next_pos as usize;
                    prefixes[i].next_pos = -1;
                    match prefixes[i].f {
                        CandidateFn::Base => check_candidate_base(self)?,
                        CandidateFn::Srcset => check_candidate_srcset(self)?,
                    }
                }
            }
        }
        // Done!
        if self.pos > self.start {
            self.emit()?;
        }
        Ok(())
    }
}

/// Maps a quote of the lexer's quote list to its static spelling (the quotes are always one of
/// the two static lists of `newAbsURLReplacer`).
fn quote_static(q: &[u8]) -> &'static [u8] {
    match q {
        b"\"" => b"\"",
        b"'" => b"'",
        b"&#34;" => b"&#34;",
        b"&#39;" => b"&#39;",
        _ => unreachable!("absurl: unknown quote"),
    }
}

/// handle URLs in src and href.
// Go: transform/urlreplacers/absurlreplacer.go:checkCandidateBase
fn check_candidate_base(l: &mut AbsUrlLexer<'_>) -> Result<()> {
    l.consume_quote()?;

    if !l.content[l.pos..].starts_with(REL_URL_PREFIX) {
        return Ok(());
    }

    // check for schemaless URLs
    let pos_after = l.pos + REL_URL_PREFIX_LEN;
    if pos_after >= l.content.len() {
        return Ok(());
    }
    let (r, _) = utf8::decode_rune(&l.content[pos_after..]);
    if r == '/' as i32 {
        // schemaless: skip
        return Ok(());
    }
    if l.pos > l.start {
        l.emit()?;
    }
    l.pos += REL_URL_PREFIX_LEN;
    l.w.extend_from_slice(l.path);
    if !l.root.is_empty() && l.content[l.pos..].starts_with(l.root) {
        l.pos += l.root.len();
    }
    l.start = l.pos;
    Ok(())
}

/// handle URLs in srcset.
// Go: transform/urlreplacers/absurlreplacer.go:checkCandidateSrcset
fn check_candidate_srcset(l: &mut AbsUrlLexer<'_>) -> Result<()> {
    let q = match l.consume_quote()? {
        Some(q) => q,
        // srcset needs to be quoted.
        None => return Ok(()),
    };

    // special case, not frequent (me think)
    if !l.content[l.pos..].starts_with(REL_URL_PREFIX) {
        return Ok(());
    }

    // check for schemaless URLs
    let pos_after = l.pos + REL_URL_PREFIX_LEN;
    if pos_after >= l.content.len() {
        return Ok(());
    }
    let (r, _) = utf8::decode_rune(&l.content[pos_after..]);
    if r == '/' as i32 {
        // schemaless: skip
        return Ok(());
    }

    let pos_end = l.pos_after_url(q);

    // safe guard
    if !(0..=2000).contains(&pos_end) {
        return Ok(());
    }

    if l.pos > l.start {
        l.emit()?;
    }

    let section = &l.content[l.pos..l.pos + pos_end as usize + 1];

    let fields = gobytes::fields(section);
    let n_fields = fields.len();
    for (i, f) in fields.into_iter().enumerate() {
        if f[0] == b'/' {
            l.w.extend_from_slice(l.path);
            let mut n = 1;
            if !l.root.is_empty() && f[n..].starts_with(l.root) {
                n += l.root.len();
            }
            l.w.extend_from_slice(&f[n..]);
        } else {
            l.w.extend_from_slice(f);
        }

        if i < n_fields - 1 {
            l.w.push(b' ');
        }
    }

    l.pos += section.len();
    l.start = l.pos;
    Ok(())
}

/// Go: `doReplace(path, ct, quotes)`. Returns an error where Go panics (see PORTING.md).
// Go: transform/urlreplacers/absurlreplacer.go:doReplace
fn do_replace(path: &str, ft: &mut FromTo<'_>, quotes: &[&[u8]]) -> Result<()> {
    let mut root: Vec<u8> = Vec::new();
    if let Ok(u) = go_url::parse(path.as_bytes()) {
        // paths.TrimLeading(u.Path) = strings.TrimPrefix(u.Path, "/"), on the raw bytes (a
        // percent-decoded path may be any bytes).
        root = u.path.strip_prefix(b"/").unwrap_or(&u.path).to_vec();
    }
    let mut lexer = AbsUrlLexer {
        content: ft.from,
        w: ft.to,
        path: path.as_bytes(),
        root: &root,
        pos: 0,
        start: 0,
        quotes,
    };

    lexer.replace()
}

/// Go: `absURLReplacer.htmlQuotes`.
const HTML_QUOTES: [&[u8]; 2] = [b"\"", b"'"];
/// Go: `absURLReplacer.xmlQuotes`.
const XML_QUOTES: [&[u8]; 2] = [b"&#34;", b"&#39;"];

/// Go: `absURLReplacer.replaceInHTML`.
// Go: transform/urlreplacers/absurlreplacer.go:replaceInHTML
pub fn replace_in_html(path: &str, ft: &mut FromTo<'_>) -> Result<()> {
    do_replace(path, ft, &HTML_QUOTES)
}

/// Go: `absURLReplacer.replaceInXML`.
// Go: transform/urlreplacers/absurlreplacer.go:replaceInXML
pub fn replace_in_xml(path: &str, ft: &mut FromTo<'_>) -> Result<()> {
    do_replace(path, ft, &XML_QUOTES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/urlreplacers/absurlreplacer.go (274 lines; 12/12 funcs executed)
//   types: absurllexer, prefix, absURLReplacer
// OK L53-71: (p *prefix) find(bs []byte, start int) bool
// OK L73-81: newPrefixState() []*prefix
// OK L83-86: (l *absurllexer) emit()
// OK L93-102: (l *absurllexer) consumeQuote() []byte
// OK L105-131: checkCandidateBase(l *absurllexer)
// OK L133-142: (l *absurllexer) posAfterURL(q []byte) int
// OK L145-202: checkCandidateSrcset(l *absurllexer)
// OK L205-238: (l *absurllexer) replace()
// OK L240-254: doReplace(path string, ct transform.FromTo, quotes [][]byte)
// OK L261-266: newAbsURLReplacer() *absURLReplacer
// OK L268-270: (au *absURLReplacer) replaceInHTML(path string, ct transform.FromTo)
// OK L272-274: (au *absURLReplacer) replaceInXML(path string, ct transform.FromTo)
// ---------------------------------------------------------------------------
