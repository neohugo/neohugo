//! Go: tpl/internal/go_templates/htmltemplate/css.go

use go_unicode::{Rune, bytes, utf8};
use go_value::Value;

use super::FILTER_FAILSAFE;
use super::content::{ContentType, stringify};

// Go: css.go:endsWithCSSKeyword
/// endsWithCSSKeyword reports whether b ends with an ident that
/// case-insensitively matches the lower-case kw.
pub(crate) fn ends_with_css_keyword(b: &[u8], kw: &[u8]) -> bool {
    let i = b.len() as isize - kw.len() as isize;
    if i < 0 {
        // Too short.
        return false;
    }
    let i = i as usize;
    if i != 0 {
        let (r, _) = utf8::decode_last_rune(&b[..i]);
        if is_css_nmchar(r) {
            // Too long.
            return false;
        }
    }
    // Many CSS keywords, such as "!important" can have characters encoded,
    // but the URI production does not allow that according to
    // https://www.w3.org/TR/css3-syntax/#TOK-URI
    // This does not attempt to recognize encoded keywords. For example,
    // given "\75\72\6c" and "url" this return false.
    bytes::to_lower(&b[i..]) == kw
}

// Go: css.go:isCSSNmchar
/// isCSSNmchar reports whether rune is allowed anywhere in a CSS identifier.
pub(crate) fn is_css_nmchar(r: Rune) -> bool {
    // Based on the CSS3 nmchar production but ignores multi-rune escape
    // sequences.
    // https://www.w3.org/TR/css3-syntax/#SUBTOK-nmchar
    'a' as Rune <= r && r <= 'z' as Rune
        || 'A' as Rune <= r && r <= 'Z' as Rune
        || '0' as Rune <= r && r <= '9' as Rune
        || r == '-' as Rune
        || r == '_' as Rune
        // Non-ASCII cases below.
        || 0x80 <= r && r <= 0xd7ff
        || 0xe000 <= r && r <= 0xfffd
        || 0x10000 <= r && r <= 0x10ffff
}

// Go: css.go:decodeCSS
/// decodeCSS decodes CSS3 escapes given a sequence of stringchars.
/// If there is no change, it returns the input, otherwise it returns a slice
/// backed by a new array.
/// <https://www.w3.org/TR/css3-syntax/#SUBTOK-stringchar> defines stringchar.
pub(crate) fn decode_css(s: &[u8]) -> Vec<u8> {
    let i = bytes::index_byte(s, b'\\');
    if i == -1 {
        return s.to_vec();
    }
    // The UTF-8 sequence for a codepoint is never longer than 1 + the
    // number hex digits need to represent that codepoint, so len(s) is an
    // upper bound on the output length.
    let mut b: Vec<u8> = Vec::with_capacity(s.len());
    let mut s = s;
    while !s.is_empty() {
        let mut i = bytes::index_byte(s, b'\\');
        if i == -1 {
            i = s.len() as isize;
        }
        let i = i as usize;
        b.extend_from_slice(&s[..i]);
        s = &s[i..];
        if s.len() < 2 {
            break;
        }
        // https://www.w3.org/TR/css3-syntax/#SUBTOK-escape
        // escape ::= unicode | '\' [#x20-#x7E#x80-#xD7FF#xE000-#xFFFD#x10000-#x10FFFF]
        if is_hex(s[1]) {
            // https://www.w3.org/TR/css3-syntax/#SUBTOK-unicode
            //   unicode ::= '\' [0-9a-fA-F]{1,6} wc?
            let mut j = 2;
            while j < s.len() && j < 7 && is_hex(s[j]) {
                j += 1;
            }
            let mut r = hex_decode(&s[1..j]);
            if r > go_unicode::MAX_RUNE {
                r /= 16;
                j -= 1;
            }
            let mut buf = [0u8; 4];
            let n = utf8::encode_rune(&mut buf, r);
            // The optional space at the end allows a hex
            // sequence to be followed by a literal hex.
            // string(decodeCSS([]byte(`\A B`))) == "\nB"
            b.extend_from_slice(&buf[..n]);
            s = skip_css_space(&s[j..]);
        } else {
            // `\\` decodes to `\` and `\"` to `"`.
            let (_, n) = utf8::decode_rune(&s[1..]);
            b.extend_from_slice(&s[1..1 + n]);
            s = &s[1 + n..];
        }
    }
    b
}

// Go: css.go:isHex
/// isHex reports whether the given character is a hex digit.
pub(crate) fn is_hex(c: u8) -> bool {
    b'0' <= c && c <= b'9' || b'a' <= c && c <= b'f' || b'A' <= c && c <= b'F'
}

// Go: css.go:hexDecode
/// hexDecode decodes a short hex digit sequence: "10" -> 16.
/// The caller guarantees that every byte is a hex digit (Go panics
/// otherwise; so does this).
pub(crate) fn hex_decode(s: &[u8]) -> Rune {
    let mut n: Rune = 0;
    for &c in s {
        n = n.wrapping_shl(4);
        match c {
            b'0'..=b'9' => n |= (c - b'0') as Rune,
            b'a'..=b'f' => n |= (c - b'a') as Rune + 10,
            b'A'..=b'F' => n |= (c - b'A') as Rune + 10,
            _ => panic!("Bad hex digit in {}", super::transition::fmt_q(s, None)),
        }
    }
    n
}

// Go: css.go:skipCSSSpace
/// skipCSSSpace returns a suffix of c, skipping over a single space.
pub(crate) fn skip_css_space(c: &[u8]) -> &[u8] {
    if c.is_empty() {
        return c;
    }
    // wc ::= #x9 | #xA | #xC | #xD | #x20
    match c[0] {
        b'\t' | b'\n' | b'\x0c' | b' ' => &c[1..],
        b'\r' => {
            // This differs from CSS3's wc production because it contains a
            // probable spec error whereby wc contains all the single byte
            // sequences in nl (newline) but not CRLF.
            if c.len() >= 2 && c[1] == b'\n' {
                return &c[2..];
            }
            &c[1..]
        }
        _ => c,
    }
}

// Go: css.go:isCSSSpace
/// isCSSSpace reports whether b is a CSS space char as defined in wc.
pub(crate) fn is_css_space(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

// Go: css.go:cssEscaper
/// cssEscaper escapes HTML and CSS special characters using \<hex>+ escapes.
pub(crate) fn css_escaper(args: &[Value]) -> Vec<u8> {
    let (s, _) = stringify(args);
    let mut b: Vec<u8> = Vec::new();
    let mut written = 0usize;
    let mut i = 0usize;
    while i < s.len() {
        // See comment in htmlEscaper.
        let (r, w) = utf8::decode_rune(&s[i..]);
        let repl: &[u8] = match css_replacement(r) {
            Some(repl) => repl,
            None => {
                i += w;
                continue;
            }
        };
        if written == 0 {
            b.reserve(s.len());
        }
        b.extend_from_slice(&s[written..i]);
        b.extend_from_slice(repl);
        written = i + w;
        if repl != b"\\\\" && (written == s.len() || is_hex(s[written]) || is_css_space(s[written]))
        {
            b.push(b' ');
        }
        i += w;
    }
    if written == 0 {
        return s;
    }
    b.extend_from_slice(&s[written..]);
    b
}

// Go: css.go:cssReplacementTable
fn css_replacement(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"\\0",
        0x09 => b"\\9",
        0x0a => b"\\a",
        0x0c => b"\\c",
        0x0d => b"\\d",
        // Encode HTML specials as hex so the output can be embedded
        // in HTML attributes without further encoding.
        0x22 => b"\\22", // '"'
        0x26 => b"\\26", // '&'
        0x27 => b"\\27", // '\''
        0x28 => b"\\28", // '('
        0x29 => b"\\29", // ')'
        0x2b => b"\\2b", // '+'
        0x2f => b"\\2f", // '/'
        0x3a => b"\\3a", // ':'
        0x3b => b"\\3b", // ';'
        0x3c => b"\\3c", // '<'
        0x3e => b"\\3e", // '>'
        0x5c => b"\\\\", // '\\'
        0x7b => b"\\7b", // '{'
        0x7d => b"\\7d", // '}'
        _ => return None,
    })
}

// Go: css.go:expressionBytes, mozBindingBytes
const EXPRESSION_BYTES: &[u8] = b"expression";
const MOZ_BINDING_BYTES: &[u8] = b"mozbinding";

// Go: css.go:cssValueFilter
/// cssValueFilter allows innocuous CSS values in the output including CSS
/// quantities (10px or 25%), ID or class literals (#foo, .bar), keyword
/// values (inherit, blue), and colors (#888).
/// It filters out unsafe values, such as those that affect token boundaries,
/// and anything that might execute scripts.
pub(crate) fn css_value_filter(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Css {
        return s;
    }
    let b = decode_css(&s);
    let mut id: Vec<u8> = Vec::with_capacity(64);

    // CSS3 error handling is specified as honoring string boundaries per
    // https://www.w3.org/TR/css3-syntax/#error-handling :
    //     Malformed declarations. User agents must handle unexpected
    //     tokens encountered while parsing a declaration by reading until
    //     the end of the declaration, while observing the rules for
    //     matching pairs of (), [], {}, "", and '', and correctly handling
    //     escapes. For example, a malformed declaration may be missing a
    //     property, colon (:) or value.
    // So we need to make sure that values do not have mismatched bracket
    // or quote characters to prevent the browser from restarting parsing
    // inside a string that might embed JavaScript source.
    for (i, &c) in b.iter().enumerate() {
        match c {
            0 | b'"' | b'\'' | b'(' | b')' | b'/' | b';' | b'@' | b'[' | b'\\' | b']' | b'`'
            | b'{' | b'}' | b'<' | b'>' => return FILTER_FAILSAFE.as_bytes().to_vec(),
            b'-' => {
                // Disallow <!-- or -->.
                // -- should not appear in valid identifiers.
                if i != 0 && b[i - 1] == b'-' {
                    return FILTER_FAILSAFE.as_bytes().to_vec();
                }
            }
            _ => {
                if (c as Rune) < utf8::RUNE_SELF && is_css_nmchar(c as Rune) {
                    id.push(c);
                }
            }
        }
    }
    let id = bytes::to_lower(&id);
    if bytes::contains(&id, EXPRESSION_BYTES) || bytes::contains(&id, MOZ_BINDING_BYTES) {
        return FILTER_FAILSAFE.as_bytes().to_vec();
    }
    b
}
