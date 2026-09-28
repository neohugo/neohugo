//! Module `prose`.
//!
//! PORT jdkato/prose@v1.2.1 transform/title.go (AP style, including the rune-count/byte-offset bug)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).

//! Port of `github.com/jdkato/prose@v1.2.1/transform/title.go` (AP / Chicago title case), used by
//! `helpers.GetTitleFunc` (`CreateTitle`). Keep the `RuneCount`-as-byte-offset bug and the ASCII-only
//! `\s` of Go RE2 (see specs/content-model.md §6.1).
//!
//! Upstream: `github.com/jdkato/prose v1.2.1`, file `transform/title.go` (108 lines).
//!
//! Go strings are bytes: the byte-level entry points ([`TitleConverter::title_bytes`],
//! [`TitleConverter::try_title_bytes`]) take and return `&[u8]`/`Vec<u8>`, invalid UTF-8
//! included; [`TitleConverter::title`] is the `&str` convenience wrapper.
//!
//! The split regexp `[\p{N}\p{L}]+[^\s-/]*` is matched by hand (`next_match`) with Go RE2
//! semantics: leftmost-first, runes decoded with Go's rules (an invalid byte is U+FFFD of width 1),
//! `\s` = `[\t\n\f\r ]`, and `\p{L}`/`\p{N}` from the go1.27.1 tables (Unicode 17).
//!
//! `t[idx:]` is Go's only possible runtime panic here (`slice bounds out of range` if `idx` ever
//! passed `len(t)`). `idx` only advances by rune counts of matches, which never exceed their byte
//! length in the sanitized copy, so it cannot happen for any input found so far (the oracle corpus
//! has none); the bounds check is kept anyway: [`TitleConverter::try_title_bytes`] returns such a
//! panic as an error (what text/template's `safeCall` would report) and
//! [`TitleConverter::title_bytes`] panics like Go.

use std::sync::OnceLock;

use go_unicode::replacer::Replacer;
use go_unicode::{Rune, strings, utf8};

use crate::herrors::{Error, Result};

/// Go: `transform.IgnoreCase`/`APStyle`/`ChicagoStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleStyle {
    Ap,
    Chicago,
}

impl TitleStyle {
    /// Go: calling the `IgnoreFunc` (`APStyle` = `optionsAP`, `ChicagoStyle` = `optionsChicago`).
    fn ignore(self, word: &[u8], first_or_last: bool) -> bool {
        match self {
            TitleStyle::Ap => options_ap(word, first_or_last),
            TitleStyle::Chicago => options_chicago(word, first_or_last),
        }
    }
}

/// Go: `transform.TitleConverter`.
#[derive(Clone, Debug)]
pub struct TitleConverter {
    pub style: TitleStyle,
}

impl TitleConverter {
    // Go: prose transform/title.go:NewTitleConverter
    pub fn new(style: TitleStyle) -> Self {
        TitleConverter { style }
    }

    // Go: prose transform/title.go:Title
    /// `Title` over a `&str`. Panics where Go panics (see the module docs); use
    /// [`TitleConverter::try_title_bytes`] to get that panic as an error.
    pub fn title(&self, s: &str) -> String {
        bytes_to_string(self.title_bytes(s.as_bytes()))
    }

    /// `Title` over Go string bytes. Panics where Go panics (see the module docs).
    pub fn title_bytes(&self, s: &[u8]) -> Vec<u8> {
        match self.try_title_bytes(s) {
            Ok(b) => b,
            Err(e) => panic!("{}", e.message()),
        }
    }

    // Go: prose transform/title.go:Title
    /// `Title` over Go string bytes; a Go runtime panic is returned as an error with Go's
    /// panic message (`runtime error: slice bounds out of range [i:n]`).
    pub fn try_title_bytes(&self, s: &[u8]) -> Result<Vec<u8>> {
        let mut idx: i64 = 0;
        let t = sanitizer().replace(s);
        let t: &[u8] = &t;
        let end = t.len() as i64;

        // Go: splitRE.ReplaceAllStringFunc(s, func(m string) string { ... })
        let mut buf: Vec<u8> = Vec::with_capacity(s.len());
        let mut last_match_end = 0usize;
        let mut search_pos = 0usize;
        while search_pos <= s.len() {
            let Some((a0, a1)) = next_match(s, search_pos) else {
                break;
            };
            buf.extend_from_slice(&s[last_match_end..a0]);
            // Every match is non-empty, so `a1 > lastMatchEnd || a0 == 0` always holds.
            let m = &s[a0..a1];

            let sm = strings::to_lower(m);
            if idx < 0 || idx > t.len() as i64 {
                return Err(Error::new(format!(
                    "runtime error: slice bounds out of range [{}:{}]",
                    idx,
                    t.len()
                )));
            }
            let pos = strings::index(&t[idx as usize..], m) as i64 + idx;
            let prev = char_at(t, pos - 1);
            let ext = utf8::rune_count_in_string(m) as i64;
            idx = pos + ext;
            if self.style.ignore(&sm, pos == 0 || idx == end)
                && (prev == b' ' || prev == b'-' || prev == b'/')
                && char_at(t, pos - 2) != b':'
                && char_at(t, pos - 2) != b'-'
                && (char_at(t, pos + ext) != b'-' || char_at(t, pos - 1) == b'-')
            {
                buf.extend_from_slice(&sm);
            } else {
                buf.extend_from_slice(&to_title(m, prev));
            }

            last_match_end = a1;
            // A match is never empty and ends on a rune boundary, so Go's "advance at least one
            // character" rule reduces to `searchPos = a[1]`.
            search_pos = a1;
        }
        buf.extend_from_slice(&s[last_match_end..]);
        Ok(buf)
    }
}

// Go: prose transform/title.go:optionsAP
fn options_ap(word: &[u8], bounding: bool) -> bool {
    !bounding && string_in_slice(word, SMALL_WORDS)
}

// Go: prose transform/title.go:optionsChicago
fn options_chicago(word: &[u8], bounding: bool) -> bool {
    !bounding && (string_in_slice(word, SMALL_WORDS) || string_in_slice(word, PREPOSITIONS))
}

// Go: prose internal/util/util.go:StringInSlice
fn string_in_slice(a: &[u8], slice: &[&str]) -> bool {
    slice.iter().any(|b| b.as_bytes() == a)
}

const SMALL_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "en", "for", "if", "in", "nor", "of", "on", "or",
    "per", "the", "to", "vs", "vs.", "via", "v", "v.",
];

const PREPOSITIONS: &[&str] = &[
    "with",
    "from",
    "into",
    "during",
    "including",
    "until",
    "against",
    "among",
    "throughout",
    "despite",
    "towards",
    "upon",
    "concerning",
    "about",
    "over",
    "through",
    "before",
    "between",
    "after",
    "since",
    "without",
    "under",
    "within",
    "along",
    "following",
    "across",
    "beyond",
    "around",
    "down",
    "near",
    "above",
];

/// Go: `var sanitizer = strings.NewReplacer(...)`: replaces a set of Unicode characters with
/// ASCII equivalents.
fn sanitizer() -> &'static Replacer {
    static R: OnceLock<Replacer> = OnceLock::new();
    R.get_or_init(|| {
        Replacer::new(&[
            "\u{201c}", "\"", "\u{201d}", "\"", "\u{2018}", "'", "\u{2019}", "'", "\u{2013}", "-",
            "\u{2014}", "-", "\u{2026}", "...",
        ])
    })
}

// Go: prose transform/title.go:charAt
/// Returns the ith character of s, if it exists. Otherwise, it returns the first character.
/// (`s` is never empty here: it is the sanitized copy of a string that has a match.)
fn char_at(s: &[u8], i: i64) -> u8 {
    if i >= 0 && i < s.len() as i64 {
        return s[i as usize];
    }
    s[0]
}

// Go: prose transform/title.go:toTitle
/// Returns a copy of the string m with its first Unicode letter mapped to its title case.
fn to_title(m: &[u8], _prev: u8) -> Vec<u8> {
    let (r, size) = utf8::decode_rune_in_string(m);
    let mut out = utf8::rune_to_string(go_unicode::to_title(r));
    out.extend_from_slice(&m[size..]);
    out
}

/// `[\p{N}\p{L}]`.
fn is_letter_or_number(r: Rune) -> bool {
    go_unicode::is(go_unicode::N, r) || go_unicode::is(go_unicode::L, r)
}

/// `[^\s-/]` with Go RE2's ASCII `\s` (`[\t\n\f\r ]`).
fn is_not_separator(r: Rune) -> bool {
    !matches!(r, 0x09 | 0x0a | 0x0c | 0x0d | 0x20 | 0x2d | 0x2f)
}

/// Go: `splitRE.FindStringIndex(s[searchPos:])` for `[\p{N}\p{L}]+[^\s-/]*` (leftmost-first).
///
/// The leftmost match starts at the first `[\p{N}\p{L}]` rune at or after `from`; since that
/// class is contained in `[^\s-/]`, the preferred (greedy) match extends to the first separator
/// rune or the end of the input.
fn next_match(s: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i < s.len() {
        let (r, w) = utf8::decode_rune_in_string(&s[i..]);
        if is_letter_or_number(r) {
            let start = i;
            let mut j = i + w;
            while j < s.len() {
                let (r, w) = utf8::decode_rune_in_string(&s[j..]);
                if !is_not_separator(r) {
                    break;
                }
                j += w;
            }
            return Some((start, j));
        }
        i += w;
    }
    None
}

/// `String` from bytes produced by a string transformation of valid UTF-8 input (always valid).
fn bytes_to_string(b: Vec<u8>) -> String {
    match String::from_utf8(b) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (third-party: jdkato/prose v1.2.1, transform package; written by T26, not
// generated — the coverage run did not instrument third-party modules). `OK` = ported. Items
// without a prefix are not called by neohugo and are not ported (no stub needed: nothing can
// reach them).
// Source: transform/title.go (108 lines)
//   types: IgnoreFunc, TitleConverter
// OK L22-33: APStyle, ChicagoStyle (the TitleStyle enum)
// OK L38-40: NewTitleConverter(style IgnoreFunc) *TitleConverter
// OK L43-61: (tc *TitleConverter) Title(s string) string
// OK L63-65: optionsAP(word string, bounding bool) bool
// OK L67-69: optionsChicago(word string, bounding bool) bool
// OK L71-80: smallWords, prepositions
// OK L82: splitRE = regexp.MustCompile(`[\p{N}\p{L}]+[^\s-/]*`) (hand-written matcher)
// OK L85-92: sanitizer = strings.NewReplacer(...)
// OK L96-101: charAt(s string, i int) byte
// OK L105-108: toTitle(m string, prev byte) string
// Source: internal/util/util.go
// OK L69-76: StringInSlice(a string, slice []string) bool
// Source: transform/transform.go (86 lines) — not used by neohugo
//    L14-30: removeCase(s string, sep string, t func(rune) rune) string
//    L32-34: Simple(s string) string
//    L37-39: Dash(s string) string
//    L42-44: Snake(s string) string
//    L47-49: Dot(s string) string
//    L52-54: Constant(s string) string
//    L57-70: Pascal(s string) string
//    L73-86: Camel(s string) string
// ---------------------------------------------------------------------------
