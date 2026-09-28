//! Port of `markup/goldmark/autoid.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/autoid.go`: heading IDs (`github` type: keep letters/digits/`_` lower-cased
//! with Go `unicode` tables, `-`/space -> `-`, drop the rest; dedup with `-1`, `-2`, ...).

use std::any::Any;
use std::collections::HashSet;

use go_unicode::utf8;
use goldmark::ast::NodeKind;

use super::goldmark_config::{AUTO_ID_TYPE_BLACKFRIDAY, AUTO_ID_TYPE_GITHUB_ASCII};

/// Go: `sanitizeAnchorNameString(s, idType)` (exposed by the converter as
/// `SanitizeAnchorName`).
// Go: markup/goldmark/autoid.go:sanitizeAnchorNameString
pub fn sanitize_anchor_name(s: &str, id_type: &str) -> String {
    // The result is always valid UTF-8 (runes are re-encoded).
    String::from_utf8(sanitize_anchor_name_bytes(s.as_bytes(), id_type))
        .expect("sanitized anchor names are valid UTF-8")
}

// Go: markup/goldmark/autoid.go:sanitizeAnchorName
pub fn sanitize_anchor_name_bytes(b: &[u8], id_type: &str) -> Vec<u8> {
    sanitize_anchor_name_with_hook(b, id_type, None)
}

// Go: markup/goldmark/autoid.go:sanitizeAnchorNameWithHook
fn sanitize_anchor_name_with_hook(
    b: &[u8],
    id_type: &str,
    hook: Option<&mut dyn FnMut(&mut Vec<u8>)>,
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    if id_type == AUTO_ID_TYPE_BLACKFRIDAY {
        buf.extend_from_slice(crate::blackfriday::sanitized_anchor_name(b).as_bytes());
    } else {
        let ascii_only = id_type == AUTO_ID_TYPE_GITHUB_ASCII;

        let removed;
        let mut b = b;
        if ascii_only {
            // Normalize it to preserve accents if possible.
            removed = remove_accents(b);
            b = &removed;
        }

        let mut b = go_unicode::bytes::trim_space(b);

        while !b.is_empty() {
            let (r, size) = utf8::decode_rune(b);
            if ascii_only && size != 1 {
            } else if r == '-' as i32 || r == ' ' as i32 {
                utf8::append_rune(&mut buf, '-' as i32);
            } else if is_alpha_numeric(r) {
                utf8::append_rune(&mut buf, go_unicode::to_lower(r));
            }

            b = &b[size..];
        }
    }

    if let Some(hook) = hook {
        hook(&mut buf);
    }

    buf
}

// Go: markup/goldmark/autoid.go:isAlphaNumeric
fn is_alpha_numeric(r: i32) -> bool {
    r == '_' as i32 || go_unicode::is_letter(r) || go_unicode::is_digit(r)
}

/// Go `text.RemoveAccents(b)` (x/text NFD, remove unicode.Mn, NFC) as far as
/// `sanitizeAnchorNameWithHook` can observe it: every ASCII byte and the space-ness of every
/// other rune are exact, other non-ASCII runes may differ. Generated data:
/// `autoid_accents.rs` (see tools/go-oracle/nh-markup/gentables, which also checks that the
/// per-rune model is exact). `nh_common::text::remove_accents` (a full x/text port) could
/// replace this; see PORTING.md divergence 4.
fn remove_accents(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let (r, size) = utf8::decode_rune(&b[i..]);
        if size == 1 && r == utf8::RUNE_ERROR {
            // runes.Remove replaces an invalid byte with U+FFFD.
            utf8::append_rune(&mut out, utf8::RUNE_ERROR);
        } else if go_unicode::is(go_unicode::tables::MN, r) {
            // Removed.
        } else if let Ok(k) =
            super::autoid_accents::REMOVE_ACCENTS.binary_search_by_key(&(r as u32), |e| e.0)
        {
            out.extend_from_slice(super::autoid_accents::REMOVE_ACCENTS[k].1.as_bytes());
        } else {
            out.extend_from_slice(&b[i..i + size]);
        }
        i += size;
    }
    out
}

/// Go: `idFactory` (per document), the `parser.IDs` of Hugo's parser context.
#[derive(Default)]
pub struct IdFactory {
    pub id_type: String,
    pub vals: HashSet<Vec<u8>>,
    pub duplicates: Vec<Vec<u8>>,
}

impl IdFactory {
    // Go: markup/goldmark/autoid.go:newIDFactory
    pub fn new(id_type: &str) -> Self {
        IdFactory {
            id_type: id_type.to_string(),
            ..Default::default()
        }
    }

    /// Go returns the map keys in random order followed by the duplicates; the only caller
    /// (the TOC transformer) sorts them. The port returns the keys in byte order.
    // Go: markup/goldmark/autoid.go:StringValues
    pub fn string_values(&self) -> Vec<Vec<u8>> {
        let mut values: Vec<Vec<u8>> = self.vals.iter().cloned().collect();
        values.sort();
        values.extend(self.duplicates.iter().cloned());
        values
    }

    // Go: markup/goldmark/autoid.go:Generate
    pub fn generate_id(&mut self, value: &[u8], kind: NodeKind) -> Vec<u8> {
        let id_type = self.id_type.clone();
        let mut result = Vec::new();
        let mut hook = |buf: &mut Vec<u8>| {
            if buf.is_empty() {
                if kind == goldmark::ast::KIND_HEADING {
                    buf.extend_from_slice(b"heading");
                } else if kind == *goldmark::extension::ast::KIND_DEFINITION_TERM {
                    buf.extend_from_slice(b"term");
                } else {
                    buf.extend_from_slice(b"id");
                }
            }

            if self.vals.contains(buf.as_slice()) {
                // Append a hyphen and a number, starting with 1.
                buf.push(b'-');
                let pos = buf.len();
                let mut i: i64 = 1;
                loop {
                    buf.extend_from_slice(go_strconv::itoa(i).as_bytes());
                    if !self.vals.contains(buf.as_slice()) {
                        break;
                    }
                    buf.truncate(pos);
                    i += 1;
                }
            }
            self.put_bytes(buf);
            result = buf.clone();
        };
        sanitize_anchor_name_with_hook(value, &id_type, Some(&mut hook));
        result
    }

    // Go: markup/goldmark/autoid.go:put
    fn put_bytes(&mut self, s: &[u8]) {
        if self.vals.contains(s) {
            self.duplicates.push(s.to_vec());
        } else {
            self.vals.insert(s.to_vec());
        }
    }

    // Go: markup/goldmark/autoid.go:Put
    pub fn put(&mut self, value: &[u8]) {
        self.put_bytes(value);
    }
}

impl goldmark::parser::IDs for IdFactory {
    fn generate(&mut self, value: &[u8], kind: NodeKind) -> Vec<u8> {
        self.generate_id(value, kind)
    }

    fn put(&mut self, value: &[u8]) {
        IdFactory::put(self, value);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goldmark::goldmark_config::AUTO_ID_TYPE_GITHUB;

    // Go: markup/goldmark/autoid_test.go:TestSanitizeAnchorName
    #[test]
    fn sanitize_anchor_name_github() {
        // Tests generated manually on github.com
        let tests = "God is good: 神真美好
Number 32
Question?
1+2=3
Special !\"#$%&(parens)=?´* chars
Resumé
One-Hyphen
Multiple--Hyphens
Trailing hyphen-
Many   spaces  here
Forward/slash
Backward\\slash
Under_score
Nonbreaking\u{a0}Space
Tab\tSpace";
        let expect = "god-is-good-神真美好
number-32
question
123
special-parens-chars
resumé
one-hyphen
multiple--hyphens
trailing-hyphen-
many---spaces--here
forwardslash
backwardslash
under_score
nonbreakingspace
tabspace";
        let mut testlines: Vec<&str> = tests.split('\n').collect();
        let mut expectlines: Vec<&str> = expect.split('\n').collect();
        testlines.push("Trailing Space ");
        expectlines.push("trailing-space");
        assert_eq!(testlines.len(), expectlines.len());
        for (input, expect) in testlines.iter().zip(expectlines) {
            assert_eq!(
                sanitize_anchor_name_bytes(input.as_bytes(), AUTO_ID_TYPE_GITHUB),
                expect.as_bytes(),
                "{input}"
            );
            assert_eq!(
                sanitize_anchor_name(input, AUTO_ID_TYPE_GITHUB),
                expect,
                "{input}"
            );
        }
    }

    // Go: markup/goldmark/autoid_test.go:TestSanitizeAnchorNameAsciiOnly
    #[test]
    fn sanitize_anchor_name_ascii_only() {
        assert_eq!(
            sanitize_anchor_name("god is神真美好 good", AUTO_ID_TYPE_GITHUB_ASCII),
            "god-is-good"
        );
        assert_eq!(
            sanitize_anchor_name("Resumé", AUTO_ID_TYPE_GITHUB_ASCII),
            "resume"
        );
    }

    // Go: markup/goldmark/autoid_test.go:TestSanitizeAnchorNameBlackfriday
    #[test]
    fn sanitize_anchor_name_blackfriday() {
        assert_eq!(
            sanitize_anchor_name("Let's try this, shall we?", AUTO_ID_TYPE_BLACKFRIDAY),
            "let-s-try-this-shall-we"
        );
    }

    // Go: markup/goldmark/autoid_test.go:BenchmarkSanitizeAnchorName* (the length checks)
    #[test]
    fn sanitize_anchor_name_lengths() {
        let input = "God is good: 神真美好".as_bytes();
        assert_eq!(
            sanitize_anchor_name_bytes(input, AUTO_ID_TYPE_GITHUB).len(),
            24
        );
        assert_eq!(
            sanitize_anchor_name_bytes(input, AUTO_ID_TYPE_GITHUB_ASCII).len(),
            12
        );
        assert_eq!(
            sanitize_anchor_name_bytes(input, AUTO_ID_TYPE_BLACKFRIDAY).len(),
            24
        );
    }

    #[test]
    fn id_factory_dedup() {
        let mut ids = IdFactory::new(AUTO_ID_TYPE_GITHUB);
        let h = goldmark::ast::KIND_HEADING;
        assert_eq!(ids.generate_id(b"Dup", h), b"dup");
        assert_eq!(ids.generate_id(b"Dup", h), b"dup-1");
        assert_eq!(ids.generate_id(b"dup-1", h), b"dup-1-1");
        assert_eq!(ids.generate_id(b"!!!", h), b"heading");
        assert_eq!(ids.generate_id(b"", goldmark::ast::KIND_TEXT), b"id");
        assert_eq!(
            ids.generate_id(b"?", *goldmark::extension::ast::KIND_DEFINITION_TERM),
            b"term"
        );
        ids.put(b"dup");
        assert_eq!(
            ids.string_values(),
            vec![
                b"dup".to_vec(),
                b"dup-1".to_vec(),
                b"dup-1-1".to_vec(),
                b"heading".to_vec(),
                b"id".to_vec(),
                b"term".to_vec(),
                b"dup".to_vec()
            ]
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/autoid.go (160 lines; 6/9 funcs executed)
//   types: idFactory, stringValuesProvider
// OK L36-38: sanitizeAnchorNameString(s string, idType string) string
// OK L40-42: sanitizeAnchorName(b []byte, idType string) []byte
// OK L44-85: sanitizeAnchorNameWithHook(b []byte, idType string, hook func(buf *bytes.Buffer)) []byte
// OK L87-89: isAlphaNumeric(r rune) bool
// OK L99-104: newIDFactory(idType string) *idFactory
// OK L112-119: (ids *idFactory) StringValues() []string
// OK L121-148: (ids *idFactory) Generate(value []byte, kind ast.NodeKind) []byte
// OK L150-156: (ids *idFactory) put(s string)
// OK L158-160: (ids *idFactory) Put(value []byte)
// ---------------------------------------------------------------------------
