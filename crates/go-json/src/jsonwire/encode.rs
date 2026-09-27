//! Port of `encoding/json/internal/jsonwire/encode.go` (go1.27.1).

use go_unicode::utf8;

use super::{ValueFlags, append_unquote, consume_number, consume_string, is_invalid_utf8};
use crate::goerr::Err;
use crate::jsonflags::{self, Flags};

// Go: encode.go:escapeASCII
/// escapeASCII reports whether the ASCII character needs to be escaped.
/// It conservatively assumes EscapeForHTML.
#[rustfmt::skip]
pub(crate) static ESCAPE_ASCII: [u8; 128] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // escape control characters
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // escape control characters
    0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, // escape '"' and '&'
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, // escape '<' and '>'
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, // escape '\\'
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// Go: encode.go:NeedEscape
/// NeedEscape reports whether src needs escaping of any characters.
/// It conservatively assumes EscapeForHTML and EscapeForJS.
/// It reports true for inputs with invalid UTF-8.
pub(crate) fn need_escape(src: &[u8]) -> bool {
    let mut i = 0;
    while src.len() > i {
        let c = src[i];
        if c < utf8::RUNE_SELF as u8 {
            if ESCAPE_ASCII[c as usize] > 0 {
                return true;
            }
            i += 1;
        } else {
            let (r, rn) = utf8::decode_rune(&src[i..]);
            if r == utf8::RUNE_ERROR || r == 0x2028 || r == 0x2029 {
                return true;
            }
            i += rn;
        }
    }
    false
}

// Go: encode.go:AppendQuote
/// AppendQuote appends src to dst as a JSON string per RFC 7159, section 7.
///
/// It takes in flags and respects the following:
///   - EscapeForHTML escapes '<', '>', and '&'.
///   - EscapeForJS escapes '\u2028' and '\u2029'.
///   - AllowInvalidUTF8 avoids reporting an error for invalid UTF-8.
///
/// Regardless of which flags are specified, it guarantees that the output is
/// valid JSON string (i.e., it is a valid UTF-8 string with no invalid escapes).
/// (dst is appended to even when an error is returned, as in Go.)
pub(crate) fn append_quote(dst: &mut Vec<u8>, src: &[u8], flags: &Flags) -> Option<Err> {
    let mut i = 0;
    let mut n = 0;
    let mut has_invalid_utf8 = false;
    dst.reserve(1 + src.len() + 1);
    dst.push(b'"');
    while src.len() > n {
        let c = src[n];
        if c < utf8::RUNE_SELF as u8 {
            // Handle single-byte ASCII.
            n += 1;
            if ESCAPE_ASCII[c as usize] == 0 {
                continue; // no escaping possibly needed
            }
            // Handle escaping of single-byte ASCII.
            if !(c == b'<' || c == b'>' || c == b'&') || flags.get(jsonflags::ESCAPE_FOR_HTML) {
                dst.extend_from_slice(&src[i..n - 1]);
                append_escaped_ascii(dst, c);
                i = n;
            }
        } else {
            // Handle multi-byte Unicode.
            let (r, rn) = utf8::decode_rune(&src[n..]);
            n += rn;
            if r != utf8::RUNE_ERROR && r != 0x2028 && r != 0x2029 {
                continue; // no escaping possibly needed
            }
            // Handle escaping of multi-byte Unicode.
            if is_invalid_utf8(r, rn) {
                has_invalid_utf8 = true;
                dst.extend_from_slice(&src[i..n - rn]);
                dst.extend_from_slice("\u{FFFD}".as_bytes());
                i = n;
            } else if (r == 0x2028 || r == 0x2029) && flags.get(jsonflags::ESCAPE_FOR_JS) {
                dst.extend_from_slice(&src[i..n - rn]);
                append_escaped_unicode(dst, r);
                i = n;
            }
        }
    }
    dst.extend_from_slice(&src[i..n]);
    dst.push(b'"');
    if has_invalid_utf8 && !flags.get(jsonflags::ALLOW_INVALID_UTF8) {
        return Some(Err::InvalidUtf8);
    }
    None
}

// Go: encode.go:appendEscapedASCII
fn append_escaped_ascii(dst: &mut Vec<u8>, c: u8) {
    match c {
        b'"' | b'\\' => dst.extend_from_slice(&[b'\\', c]),
        0x08 => dst.extend_from_slice(b"\\b"),
        0x0C => dst.extend_from_slice(b"\\f"),
        b'\n' => dst.extend_from_slice(b"\\n"),
        b'\r' => dst.extend_from_slice(b"\\r"),
        b'\t' => dst.extend_from_slice(b"\\t"),
        _ => append_escaped_utf16(dst, c as u16),
    }
}

// Go: encode.go:appendEscapedUnicode
fn append_escaped_unicode(dst: &mut Vec<u8>, r: i32) {
    let (r1, r2) = go_unicode::utf16::encode_rune(r);
    if r1 != 0xFFFD && r2 != 0xFFFD {
        append_escaped_utf16(dst, r1 as u16);
        append_escaped_utf16(dst, r2 as u16);
    } else {
        append_escaped_utf16(dst, r as u16);
    }
}

// Go: encode.go:appendEscapedUTF16
fn append_escaped_utf16(dst: &mut Vec<u8>, x: u16) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    dst.extend_from_slice(&[
        b'\\',
        b'u',
        HEX[((x >> 12) & 0xf) as usize],
        HEX[((x >> 8) & 0xf) as usize],
        HEX[((x >> 4) & 0xf) as usize],
        HEX[(x & 0xf) as usize],
    ]);
}

// Go: encode.go:ReformatString
/// ReformatString consumes a JSON string from src and appends it to dst,
/// reformatting it if necessary according to the specified flags.
/// It returns the appended output and the number of consumed input bytes.
pub(crate) fn reformat_string(
    dst: &mut Vec<u8>,
    src: &[u8],
    flags: &Flags,
) -> (usize, Option<Err>) {
    // TODO: Should this update ValueFlags as input?
    let mut val_flags = ValueFlags::default();
    let (n, err) = consume_string(
        &mut val_flags,
        src,
        !flags.get(jsonflags::ALLOW_INVALID_UTF8),
    );
    if err.is_some() {
        return (n, err);
    }

    // If the output requires no special escapes, and the input
    // is already in canonical form or should be preserved verbatim,
    // then directly copy the input to the output.
    if !flags.get(jsonflags::ANY_ESCAPE)
        && (val_flags.is_canonical() || flags.get(jsonflags::PRESERVE_RAW_STRINGS))
    {
        dst.extend_from_slice(&src[..n]); // copy the string verbatim
        return (n, None);
    }

    // Under [jsonflags.PreserveRawStrings], any pre-escaped sequences
    // remain escaped, however we still need to respect the
    // [jsonflags.EscapeForHTML] and [jsonflags.EscapeForJS] options.
    if flags.get(jsonflags::PRESERVE_RAW_STRINGS) {
        let mut i = 0;
        let mut last_append_index = 0;
        while i < n {
            let c = src[i];
            if c < utf8::RUNE_SELF as u8 {
                if (c == b'<' || c == b'>' || c == b'&') && flags.get(jsonflags::ESCAPE_FOR_HTML) {
                    dst.extend_from_slice(&src[last_append_index..i]);
                    append_escaped_ascii(dst, c);
                    last_append_index = i + 1;
                }
                i += 1;
            } else {
                let (r, rn) = utf8::decode_rune(&src[i..]);
                if (r == 0x2028 || r == 0x2029) && flags.get(jsonflags::ESCAPE_FOR_JS) {
                    dst.extend_from_slice(&src[last_append_index..i]);
                    append_escaped_unicode(dst, r);
                    last_append_index = i + rn;
                }
                i += rn;
            }
        }
        dst.extend_from_slice(&src[last_append_index..n]);
        return (n, None);
    }

    // The input contains characters that might need escaping,
    // unnecessary escape sequences, or invalid UTF-8.
    // Perform a round-trip unquote and quote to properly reformat
    // these sequences according to the current flags.
    let mut b = Vec::new();
    let _ = append_unquote(&mut b, &src[..n]);
    let _ = append_quote(dst, &b, flags);
    (n, None)
}

// Go: encode.go:AppendFloat
/// Append formats f to dst according to RFC 8785, section 3.2.2.3.
/// This matches the behavior of JavaScript's Number.prototype.toString.
pub(crate) fn append_float(dst: &mut Vec<u8>, src: f64, bits: i64) {
    let mut src = src;
    if bits == 32 {
        src = go_strconv::internal::f32_to_f64(go_strconv::internal::f64_to_f32(src));
    }

    let abs = src.abs();
    let mut fmt = b'f';
    if abs != 0.0 {
        let abs32 = go_strconv::internal::f64_to_f32(abs);
        if bits == 64 && (abs < 1e-6 || abs >= 1e21)
            || bits == 32 && (abs32 < 1e-6_f32 || abs32 >= 1e21_f32)
        {
            fmt = b'e';
        }
    }
    let base = dst.len();
    go_strconv::append_float(dst, src, fmt, -1, bits);
    if fmt == b'e' {
        // Clean up e-09 to e-9.
        let n = dst.len();
        if n - base >= 4 && dst[n - 4] == b'e' && dst[n - 3] == b'-' && dst[n - 2] == b'0' {
            dst[n - 2] = dst[n - 1];
            dst.truncate(n - 1);
        }
    }
}

// Go: encode.go:ReformatNumber
/// ReformatNumber consumes a JSON string from src and appends it to dst,
/// canonicalizing it if specified.
/// It returns the appended output and the number of consumed input bytes.
pub(crate) fn reformat_number(
    dst: &mut Vec<u8>,
    src: &[u8],
    flags: &Flags,
) -> (usize, Option<Err>) {
    let (n, err) = consume_number(src);
    if err.is_some() {
        return (n, err);
    }
    if !flags.get(jsonflags::CANONICALIZE_NUMBERS) {
        dst.extend_from_slice(&src[..n]); // copy the number verbatim
        return (n, None);
    }

    // Identify the kind of number.
    let is_float = src[..n]
        .iter()
        .any(|&c| c == b'.' || c == b'e' || c == b'E');

    // Check if need to canonicalize this kind of number.
    if &src[..n] == b"-0" {
        // canonicalize -0 as 0 regardless of kind
    } else if is_float {
        if !flags.get(jsonflags::CANONICALIZE_RAW_FLOATS) {
            dst.extend_from_slice(&src[..n]); // copy the number verbatim
            return (n, None);
        }
    } else {
        // As an optimization, we can copy integer numbers below 2⁵³ verbatim
        // since the canonical form is always identical.
        const MAX_EXACT_INTEGER_DIGITS: usize = 16; // len(strconv.AppendUint(nil, 1<<53, 10))
        if !flags.get(jsonflags::CANONICALIZE_RAW_INTS) || n < MAX_EXACT_INTEGER_DIGITS {
            dst.extend_from_slice(&src[..n]); // copy the number verbatim
            return (n, None);
        }
    }

    // Parse and reformat the number (which uses a canonical format).
    let (mut fv, _) = go_strconv::internal::parse_float(&src[..n], 64);
    if fv == 0.0 {
        fv = 0.0; // normalize negative zero as just zero
    } else if fv == f64::INFINITY {
        fv = f64::MAX;
    } else if fv == f64::NEG_INFINITY {
        fv = -f64::MAX;
    }
    append_float(dst, fv, 64);
    (n, None)
}
