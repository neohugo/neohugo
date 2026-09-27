//! Port of go1.27.1 `src/net/url/encoding_table.go` and the reference
//! implementation it is generated from (`src/net/url/gen_encoding_table.go`).
//!
//! Go checks in the generated 256-entry table; here the same table is
//! computed at compile time by a `const fn` port of the generator's
//! `shouldEscape`/`ishex`. `tests/oracle.rs` compares every entry against the
//! table produced by the Go oracle.

/// Go `type encoding uint8`.
pub(crate) type Encoding = u8;

pub(crate) const ENCODE_PATH: Encoding = 1 << 0;
pub(crate) const ENCODE_PATH_SEGMENT: Encoding = 1 << 1;
pub(crate) const ENCODE_HOST: Encoding = 1 << 2;
pub(crate) const ENCODE_ZONE: Encoding = 1 << 3;
pub(crate) const ENCODE_USER_PASSWORD: Encoding = 1 << 4;
pub(crate) const ENCODE_QUERY_COMPONENT: Encoding = 1 << 5;
pub(crate) const ENCODE_FRAGMENT: Encoding = 1 << 6;
// hexChar is actually NOT an encoding mode, but there are only seven
// encoding modes. We might as well abuse the otherwise unused most
// significant bit in uint8 to indicate whether a character is
// hexadecimal.
pub(crate) const HEX_CHAR: Encoding = 1 << 7;

const ALL_MODES: [Encoding; 7] = [
    ENCODE_PATH,
    ENCODE_PATH_SEGMENT,
    ENCODE_HOST,
    ENCODE_ZONE,
    ENCODE_USER_PASSWORD,
    ENCODE_QUERY_COMPONENT,
    ENCODE_FRAGMENT,
];

// Go: net/url/gen_encoding_table.go:shouldEscape
/// Return true if the specified character should be escaped when
/// appearing in a URL string, according to RFC 3986.
const fn gen_should_escape(c: u8, mode: Encoding) -> bool {
    // §2.3 Unreserved characters (alphanum)
    if c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit() {
        return false;
    }

    if mode == ENCODE_HOST || mode == ENCODE_ZONE {
        // §3.2.2 Host allows
        //	sub-delims = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
        // as part of reg-name.
        // We add : because we include :port as part of host.
        // We add [ ] because we include [ipv6]:port as part of host.
        // We add < > because they're the only characters left that
        // we could possibly allow, and Parse will reject them if we
        // escape them (because hosts can't use %-encoding for
        // ASCII bytes).
        match c {
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':'
            | b'[' | b']' | b'<' | b'>' | b'"' => return false,
            _ => {}
        }
    }

    match c {
        b'-' | b'_' | b'.' | b'~' => {
            // §2.3 Unreserved characters (mark)
            return false;
        }
        b'$' | b'&' | b'+' | b',' | b'/' | b':' | b';' | b'=' | b'?' | b'@' => {
            // §2.2 Reserved characters (reserved)
            // Different sections of the URL allow a few of
            // the reserved characters to appear unescaped.
            match mode {
                ENCODE_PATH => {
                    // §3.3
                    // The RFC allows : @ & = + $ but saves / ; , for assigning
                    // meaning to individual path segments. This package
                    // only manipulates the path as a whole, so we allow those
                    // last three as well. That leaves only ? to escape.
                    return c == b'?';
                }
                ENCODE_PATH_SEGMENT => {
                    // §3.3
                    // The RFC allows : @ & = + $ but saves / ; , for assigning
                    // meaning to individual path segments.
                    return c == b'/' || c == b';' || c == b',' || c == b'?';
                }
                ENCODE_USER_PASSWORD => {
                    // §3.2.1
                    // The RFC allows ';', ':', '&', '=', '+', '$', and ',' in
                    // userinfo, so we must escape only '@', '/', and '?'.
                    // The parsing of userinfo treats ':' as special so we must escape
                    // that too.
                    return c == b'@' || c == b'/' || c == b'?' || c == b':';
                }
                ENCODE_QUERY_COMPONENT => {
                    // §3.4
                    // The RFC reserves (so we must escape) everything.
                    return true;
                }
                ENCODE_FRAGMENT => {
                    // §4.1
                    // The RFC text is silent but the grammar allows
                    // everything, so escape nothing.
                    return false;
                }
                _ => {}
            }
        }
        _ => {}
    }

    if mode == ENCODE_FRAGMENT {
        // RFC 3986 §2.2 allows not escaping sub-delims. A subset of sub-delims are
        // included in reserved from RFC 2396 §2.2. The remaining sub-delims do not
        // need to be escaped. To minimize potential breakage, we apply two restrictions:
        // (1) we always escape sub-delims outside of the fragment, and (2) we always
        // escape single quote to avoid breaking callers that had previously assumed that
        // single quotes would be escaped. See issue #19917.
        match c {
            b'!' | b'(' | b')' | b'*' => return false,
            _ => {}
        }
    }

    // Everything else must be escaped.
    true
}

// Go: net/url/gen_encoding_table.go:ishex
const fn gen_ishex(c: u8) -> bool {
    c.is_ascii_digit() || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

// Go: net/url/gen_encoding_table.go:generateTable
const fn build_table() -> [Encoding; 256] {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let c = i as u8;
        let mut v: Encoding = 0;
        if gen_ishex(c) {
            // Set the hexChar bit if this char is hexadecimal.
            v |= HEX_CHAR;
        }
        let mut k = 0;
        while k < ALL_MODES.len() {
            if !gen_should_escape(c, ALL_MODES[k]) {
                // Set this encoding mode's bit if this char should NOT be
                // escaped.
                v |= ALL_MODES[k];
            }
            k += 1;
        }
        t[i] = v;
        i += 1;
    }
    t
}

/// Go `net/url.table` (encoding_table.go).
pub(crate) static TABLE: [Encoding; 256] = build_table();

/// The raw table, for differential testing against Go's `encoding_table.go`.
pub fn encoding_table() -> &'static [u8; 256] {
    &TABLE
}
