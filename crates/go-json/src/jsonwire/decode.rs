//! Port of `encoding/json/internal/jsonwire/decode.go` (go1.27.1).
//!
//! Go returns `(n int, err error)` pairs where `n` is meaningful even with
//! an error; those functions return `(usize, Option<Err>)` here.

use go_unicode::utf8;

use super::{new_invalid_character_error, new_invalid_escape_sequence_error, quote_rune};
use crate::goerr::Err;

// Go: decode.go:ValueFlags
/// ValueFlags is a set of flags describing a JSON value.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct ValueFlags(u32);

/// string cannot be naively treated as valid UTF-8
const STRING_NON_VERBATIM: u32 = 1;
/// string not formatted according to RFC 8785, section 3.2.2.2.
const STRING_NON_CANONICAL: u32 = 2;

#[cfg(test)]
pub(crate) const STRING_NON_VERBATIM_FOR_TESTS: u32 = STRING_NON_VERBATIM;
#[cfg(test)]
pub(crate) const STRING_NON_CANONICAL_FOR_TESTS: u32 = STRING_NON_CANONICAL;

#[cfg(test)]
impl ValueFlags {
    pub(crate) fn from_bits_for_tests(bits: u32) -> ValueFlags {
        ValueFlags(bits)
    }
}

#[cfg(test)]
pub(crate) fn parse_hex_uint16_for_tests(b: &[u8]) -> (u16, bool) {
    parse_hex_uint16(b)
}

impl ValueFlags {
    pub(crate) fn join(&mut self, f2: ValueFlags) {
        self.0 |= f2.0;
    }
    fn join_bits(&mut self, bits: u32) {
        self.0 |= bits;
    }
    pub(crate) fn is_verbatim(&self) -> bool {
        self.0 & STRING_NON_VERBATIM == 0
    }
    pub(crate) fn is_canonical(&self) -> bool {
        self.0 & STRING_NON_CANONICAL == 0
    }
}

// Go: decode.go:ConsumeWhitespace
/// ConsumeWhitespace consumes leading JSON whitespace per RFC 7159, section 2.
pub(crate) fn consume_whitespace(b: &[u8]) -> usize {
    let mut n = 0;
    while b.len() > n && (b[n] == b' ' || b[n] == b'\t' || b[n] == b'\r' || b[n] == b'\n') {
        n += 1;
    }
    n
}

// Go: decode.go:ConsumeNull
/// ConsumeNull consumes the next JSON null literal per RFC 7159, section 3.
/// It returns 0 if it is invalid, in which case consumeLiteral should be used.
pub(crate) fn consume_null(b: &[u8]) -> usize {
    const LITERAL: &[u8] = b"null";
    if b.len() >= LITERAL.len() && &b[..LITERAL.len()] == LITERAL {
        return LITERAL.len();
    }
    0
}

// Go: decode.go:ConsumeFalse
pub(crate) fn consume_false(b: &[u8]) -> usize {
    const LITERAL: &[u8] = b"false";
    if b.len() >= LITERAL.len() && &b[..LITERAL.len()] == LITERAL {
        return LITERAL.len();
    }
    0
}

// Go: decode.go:ConsumeTrue
pub(crate) fn consume_true(b: &[u8]) -> usize {
    const LITERAL: &[u8] = b"true";
    if b.len() >= LITERAL.len() && &b[..LITERAL.len()] == LITERAL {
        return LITERAL.len();
    }
    0
}

// Go: decode.go:ConsumeLiteral
/// ConsumeLiteral consumes the next JSON literal per RFC 7159, section 3.
/// If the input appears truncated, it returns io.ErrUnexpectedEOF.
pub(crate) fn consume_literal(b: &[u8], lit: &str) -> (usize, Option<Err>) {
    let litb = lit.as_bytes();
    let mut i = 0;
    while i < b.len() && i < litb.len() {
        if b[i] != litb[i] {
            let where_ = format!(
                "in literal {} (expecting {})",
                lit,
                go_strconv::quote_rune(litb[i] as i32)
            );
            return (i, Some(new_invalid_character_error(&b[i..], &where_)));
        }
        i += 1;
    }
    if b.len() < litb.len() {
        return (b.len(), Some(Err::UnexpectedEof));
    }
    (litb.len(), None)
}

// Go: decode.go:ConsumeSimpleString
/// ConsumeSimpleString consumes the next JSON string per RFC 7159, section 7
/// but is limited to the grammar for an ASCII string without escape sequences.
/// It returns 0 if it is invalid or more complicated than a simple string,
/// in which case consumeString should be called.
pub(crate) fn consume_simple_string(b: &[u8]) -> usize {
    let mut n = 0;
    if !b.is_empty() && b[0] == b'"' {
        n += 1;
        while b.len() > n && b[n] < utf8::RUNE_SELF as u8 && super::ESCAPE_ASCII[b[n] as usize] == 0
        {
            n += 1;
        }
        if b.len() > n && b[n] == b'"' {
            n += 1;
            return n;
        }
    }
    0
}

// Go: decode.go:ConsumeString
/// ConsumeString consumes the next JSON string per RFC 7159, section 7.
/// If validateUTF8 is false, then this allows the presence of invalid UTF-8
/// characters within the string itself.
/// It reports the number of bytes consumed and whether an error was encountered.
/// If the input appears truncated, it returns io.ErrUnexpectedEOF.
pub(crate) fn consume_string(
    flags: &mut ValueFlags,
    b: &[u8],
    validate_utf8: bool,
) -> (usize, Option<Err>) {
    consume_string_resumable(flags, b, 0, validate_utf8)
}

#[inline]
fn no_escape(c: u8) -> bool {
    c < utf8::RUNE_SELF as u8 && b' ' <= c && c != b'\\' && c != b'"'
}

// Go: decode.go:ConsumeStringResumable
/// ConsumeStringResumable is identical to consumeString but supports resuming
/// from a previous call that returned io.ErrUnexpectedEOF.
pub(crate) fn consume_string_resumable(
    flags: &mut ValueFlags,
    b: &[u8],
    resume_offset: usize,
    validate_utf8: bool,
) -> (usize, Option<Err>) {
    let mut resume_offset = resume_offset;
    // Consume the leading double quote.
    let mut n = 0;
    if resume_offset > 0 {
        n = resume_offset; // already handled the leading quote
    } else if b.is_empty() {
        return (n, Some(Err::UnexpectedEof));
    } else if b[0] == b'"' {
        n += 1;
    } else {
        return (
            n,
            Some(new_invalid_character_error(
                &b[n..],
                "at start of string (expecting '\"')",
            )),
        );
    }

    // Consume every character in the string.
    while b.len() > n {
        // Optimize for long sequences of unescaped characters.
        while b.len() > n && no_escape(b[n]) {
            n += 1;
        }
        if b.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }

        // Check for terminating double quote.
        if b[n] == b'"' {
            n += 1;
            return (n, None);
        }

        let (r, rn) = utf8::decode_rune(&b[n..]);
        if rn > 1 {
            // Handle UTF-8 encoded byte sequence.
            // Due to specialized handling of ASCII above, we know that
            // all normal sequences at this point must be 2 bytes or larger.
            n += rn;
        } else if r == '\\' as i32 {
            // Handle escape sequence.
            flags.join_bits(STRING_NON_VERBATIM);
            resume_offset = n;
            if b.len() < n + 2 {
                return (resume_offset, Some(Err::UnexpectedEof));
            }
            match b[n + 1] {
                b'/' => {
                    // Forward slash is the only character with 3 representations.
                    // Per RFC 8785, section 3.2.2.2., this must not be escaped.
                    flags.join_bits(STRING_NON_CANONICAL);
                    n += 2;
                }
                b'"' | b'\\' | b'b' | b'f' | b'n' | b'r' | b't' => n += 2,
                b'u' => {
                    if b.len() < n + 6 {
                        if has_escaped_utf16_prefix(&b[n..], false) {
                            return (resume_offset, Some(Err::UnexpectedEof));
                        }
                        flags.join_bits(STRING_NON_CANONICAL);
                        return (n, Some(new_invalid_escape_sequence_error(&b[n..])));
                    }
                    let (v1, ok) = parse_hex_uint16(&b[n + 2..n + 6]);
                    if !ok {
                        flags.join_bits(STRING_NON_CANONICAL);
                        return (n, Some(new_invalid_escape_sequence_error(&b[n..n + 6])));
                    }
                    // Only certain control characters can use the \uFFFF notation
                    // for canonical formatting (per RFC 8785, section 3.2.2.2.).
                    match v1 {
                        // \uFFFF notation not permitted for these characters.
                        0x08 | 0x0C | 0x0A | 0x0D | 0x09 => flags.join_bits(STRING_NON_CANONICAL),
                        _ => {
                            // \uFFFF notation only permitted for control characters.
                            if v1 >= b' ' as u16 {
                                flags.join_bits(STRING_NON_CANONICAL);
                            } else {
                                // \uFFFF notation must be lower case.
                                for &c in &b[n + 2..n + 6] {
                                    if b'A' <= c && c <= b'F' {
                                        flags.join_bits(STRING_NON_CANONICAL);
                                    }
                                }
                            }
                        }
                    }
                    n += 6;

                    let r = v1 as i32;
                    if validate_utf8 && go_unicode::utf16::is_surrogate(r) {
                        if b.len() < n + 6 {
                            if has_escaped_utf16_prefix(&b[n..], true) {
                                return (resume_offset, Some(Err::UnexpectedEof));
                            }
                            flags.join_bits(STRING_NON_CANONICAL);
                            return (n - 6, Some(new_invalid_escape_sequence_error(&b[n - 6..])));
                        }
                        let (v2, ok) = parse_hex_uint16(&b[n + 2..n + 6]);
                        if b[n] != b'\\' || b[n + 1] != b'u' || !ok {
                            flags.join_bits(STRING_NON_CANONICAL);
                            return (
                                n - 6,
                                Some(new_invalid_escape_sequence_error(&b[n - 6..n + 6])),
                            );
                        } else if go_unicode::utf16::decode_rune(v1 as i32, v2 as i32)
                            == utf8::RUNE_ERROR
                        {
                            flags.join_bits(STRING_NON_CANONICAL);
                            return (
                                n - 6,
                                Some(new_invalid_escape_sequence_error(&b[n - 6..n + 6])),
                            );
                        } else {
                            n += 6;
                        }
                    }
                }
                _ => {
                    flags.join_bits(STRING_NON_CANONICAL);
                    return (n, Some(new_invalid_escape_sequence_error(&b[n..n + 2])));
                }
            }
        } else if r == utf8::RUNE_ERROR {
            // Handle invalid UTF-8.
            if !utf8::full_rune(&b[n..]) {
                return (n, Some(Err::UnexpectedEof));
            }
            flags.join_bits(STRING_NON_VERBATIM | STRING_NON_CANONICAL);
            if validate_utf8 {
                return (n, Some(Err::InvalidUtf8));
            }
            n += 1;
        } else if r < ' ' as i32 {
            // Handle invalid control characters.
            flags.join_bits(STRING_NON_VERBATIM | STRING_NON_CANONICAL);
            return (
                n,
                Some(new_invalid_character_error(
                    &b[n..],
                    "in string (expecting non-control character)",
                )),
            );
        } else {
            panic!("BUG: unhandled character {}", quote_rune(&b[n..]));
        }
    }
    (n, Some(Err::UnexpectedEof))
}

// Go: decode.go:AppendUnquote
/// AppendUnquote appends the unescaped form of a JSON string in src to dst.
/// Any invalid UTF-8 within the string will be replaced with utf8.RuneError,
/// but the error will be specified as having encountered such an error.
/// The input must be an entire JSON string with no surrounding whitespace.
/// (dst is appended to even when an error is returned, as in Go.)
pub(crate) fn append_unquote(dst: &mut Vec<u8>, src: &[u8]) -> Option<Err> {
    dst.reserve(src.len());
    let mut err: Option<Err> = None;

    // Consume the leading double quote.
    let (mut i, mut n);
    if src.is_empty() {
        return Some(Err::UnexpectedEof);
    } else if src[0] == b'"' {
        i = 1;
        n = 1;
    } else {
        return Some(new_invalid_character_error(
            src,
            "at start of string (expecting '\"')",
        ));
    }

    // Consume every character in the string.
    while src.len() > n {
        // Optimize for long sequences of unescaped characters.
        while src.len() > n && no_escape(src[n]) {
            n += 1;
        }
        if src.len() <= n {
            dst.extend_from_slice(&src[i..n]);
            return Some(Err::UnexpectedEof);
        }

        // Check for terminating double quote.
        if src[n] == b'"' {
            dst.extend_from_slice(&src[i..n]);
            n += 1;
            if n < src.len() {
                err = Some(new_invalid_character_error(&src[n..], "after string value"));
            }
            return err;
        }

        let (r, rn) = utf8::decode_rune(&src[n..]);
        if rn > 1 {
            // Handle UTF-8 encoded byte sequence.
            n += rn;
        } else if r == '\\' as i32 {
            dst.extend_from_slice(&src[i..n]);

            // Handle escape sequence.
            if src.len() < n + 2 {
                return Some(Err::UnexpectedEof);
            }
            match src[n + 1] {
                c @ (b'"' | b'\\' | b'/') => {
                    dst.push(c);
                    n += 2;
                }
                b'b' => {
                    dst.push(0x08);
                    n += 2;
                }
                b'f' => {
                    dst.push(0x0C);
                    n += 2;
                }
                b'n' => {
                    dst.push(b'\n');
                    n += 2;
                }
                b'r' => {
                    dst.push(b'\r');
                    n += 2;
                }
                b't' => {
                    dst.push(b'\t');
                    n += 2;
                }
                b'u' => {
                    if src.len() < n + 6 {
                        if has_escaped_utf16_prefix(&src[n..], false) {
                            return Some(Err::UnexpectedEof);
                        }
                        return Some(new_invalid_escape_sequence_error(&src[n..]));
                    }
                    let (v1, ok) = parse_hex_uint16(&src[n + 2..n + 6]);
                    if !ok {
                        return Some(new_invalid_escape_sequence_error(&src[n..n + 6]));
                    }
                    n += 6;

                    // Check whether this is a surrogate half.
                    let mut r = v1 as i32;
                    if go_unicode::utf16::is_surrogate(r) {
                        r = utf8::RUNE_ERROR; // assume failure unless the following succeeds
                        if src.len() < n + 6 {
                            if has_escaped_utf16_prefix(&src[n..], true) {
                                utf8::append_rune(dst, r);
                                return Some(Err::UnexpectedEof);
                            }
                            err = Some(new_invalid_escape_sequence_error(&src[n - 6..]));
                        } else {
                            let (v2, ok) = parse_hex_uint16(&src[n + 2..n + 6]);
                            if src[n] != b'\\' || src[n + 1] != b'u' || !ok {
                                err = Some(new_invalid_escape_sequence_error(&src[n - 6..n + 6]));
                            } else {
                                r = go_unicode::utf16::decode_rune(v1 as i32, v2 as i32);
                                if r == utf8::RUNE_ERROR {
                                    err =
                                        Some(new_invalid_escape_sequence_error(&src[n - 6..n + 6]));
                                } else {
                                    n += 6;
                                }
                            }
                        }
                    }

                    utf8::append_rune(dst, r);
                }
                _ => return Some(new_invalid_escape_sequence_error(&src[n..n + 2])),
            }
            i = n;
        } else if r == utf8::RUNE_ERROR {
            // Handle invalid UTF-8.
            dst.extend_from_slice(&src[i..n]);
            if !utf8::full_rune(&src[n..]) {
                return Some(Err::UnexpectedEof);
            }
            // NOTE: An unescaped string may be longer than the escaped string
            // because invalid UTF-8 bytes are being replaced.
            dst.extend_from_slice("\u{FFFD}".as_bytes());
            n += rn;
            i = n;
            err = Some(Err::InvalidUtf8);
        } else if r < ' ' as i32 {
            // Handle invalid control characters.
            dst.extend_from_slice(&src[i..n]);
            return Some(new_invalid_character_error(
                &src[n..],
                "in string (expecting non-control character)",
            ));
        } else {
            panic!("BUG: unhandled character {}", quote_rune(&src[n..]));
        }
    }
    dst.extend_from_slice(&src[i..n]);
    Some(Err::UnexpectedEof)
}

// Go: decode.go:hasEscapedUTF16Prefix
/// hasEscapedUTF16Prefix reports whether b is possibly
/// the truncated prefix of a \uFFFF escape sequence.
fn has_escaped_utf16_prefix(b: &[u8], lower_surrogate_half: bool) -> bool {
    for (i, &c) in b.iter().enumerate() {
        if i == 0 && c != b'\\' {
            return false;
        }
        if i == 1 && c != b'u' {
            return false;
        }
        if i == 2 && lower_surrogate_half && c != b'd' && c != b'D' {
            return false; // not within ['\uDC00':'\uDFFF']
        }
        if i == 3 && lower_surrogate_half && !(b'c' <= c && c <= b'f') && !(b'C' <= c && c <= b'F')
        {
            return false; // not within ['\uDC00':'\uDFFF']
        }
        if i >= 2
            && i < 6
            && !c.is_ascii_digit()
            && !(b'a' <= c && c <= b'f')
            && !(b'A' <= c && c <= b'F')
        {
            return false;
        }
    }
    true
}

// Go: decode.go:UnquoteMayCopy
/// UnquoteMayCopy returns the unescaped form of b.
/// If there are no escaped characters, the output is simply a subslice of
/// the input with the surrounding quotes removed.
/// Otherwise, a new buffer is allocated for the output.
/// It assumes the input is valid.
pub(crate) fn unquote_may_copy(b: &[u8], is_verbatim: bool) -> Vec<u8> {
    if is_verbatim {
        return b[1..b.len() - 1].to_vec();
    }
    let mut out = Vec::new();
    let _ = append_unquote(&mut out, b);
    out
}

// Go: decode.go:ConsumeSimpleNumber
/// ConsumeSimpleNumber consumes the next JSON number per RFC 7159, section 6
/// but is limited to the grammar for a positive integer.
/// It returns 0 if it is invalid or more complicated than a simple integer,
/// in which case consumeNumber should be called.
pub(crate) fn consume_simple_number(b: &[u8]) -> usize {
    let mut n = 0;
    if !b.is_empty() {
        if b[0] == b'0' {
            n += 1;
        } else if b'1' <= b[0] && b[0] <= b'9' {
            n += 1;
            while b.len() > n && b[n].is_ascii_digit() {
                n += 1;
            }
        } else {
            return 0;
        }
        if b.len() <= n || (b[n] != b'.' && b[n] != b'e' && b[n] != b'E') {
            return n;
        }
    }
    0
}

// Go: decode.go:ConsumeNumberState
pub(crate) type ConsumeNumberState = u32;

pub(crate) const CONSUME_NUMBER_INIT: ConsumeNumberState = 0;
const BEFORE_INTEGER_DIGITS: ConsumeNumberState = 1;
const WITHIN_INTEGER_DIGITS: ConsumeNumberState = 2;
const BEFORE_FRACTIONAL_DIGITS: ConsumeNumberState = 3;
const WITHIN_FRACTIONAL_DIGITS: ConsumeNumberState = 4;
const BEFORE_EXPONENT_DIGITS: ConsumeNumberState = 5;
const WITHIN_EXPONENT_DIGITS: ConsumeNumberState = 6;

// Go: decode.go:ConsumeNumber
/// ConsumeNumber consumes the next JSON number per RFC 7159, section 6.
/// It reports the number of bytes consumed and whether an error was encountered.
/// If the input appears truncated, it returns io.ErrUnexpectedEOF.
///
/// Note that JSON numbers are not self-terminating.
/// If the entire input is consumed, then the caller needs to consider whether
/// there may be subsequent unread data that may still be part of this number.
pub(crate) fn consume_number(b: &[u8]) -> (usize, Option<Err>) {
    let (n, _, err) = consume_number_resumable(b, 0, CONSUME_NUMBER_INIT);
    (n, err)
}

// Go: decode.go:ConsumeNumberResumable
/// ConsumeNumberResumable is identical to consumeNumber but supports resuming
/// from a previous call that returned io.ErrUnexpectedEOF.
/// (Go's `goto beforeInteger/beforeFractional/beforeExponent` jumps are
/// the `start` stage selector.)
pub(crate) fn consume_number_resumable(
    b: &[u8],
    resume_offset: usize,
    state: ConsumeNumberState,
) -> (usize, ConsumeNumberState, Option<Err>) {
    let mut state = state;
    let mut resume_offset = resume_offset;
    // Jump to the right state when resuming from a partial consumption.
    let mut n = resume_offset;
    // 0: beforeInteger, 1: beforeFractional, 2: beforeExponent
    let mut start = 0;
    if state > CONSUME_NUMBER_INIT {
        match state {
            WITHIN_INTEGER_DIGITS | WITHIN_FRACTIONAL_DIGITS | WITHIN_EXPONENT_DIGITS => {
                // Consume leading digits.
                while b.len() > n && b[n].is_ascii_digit() {
                    n += 1;
                }
                if b.len() <= n {
                    return (n, state, None); // still within the same state
                }
                state += 1; // switches "withinX" to "beforeY" where Y is the state after X
            }
            _ => {}
        }
        match state {
            BEFORE_INTEGER_DIGITS => start = 0,
            BEFORE_FRACTIONAL_DIGITS => start = 1,
            BEFORE_EXPONENT_DIGITS => start = 2,
            _ => return (n, state, None),
        }
    }

    // Consume required integer component (with optional minus sign).
    if start == 0 {
        resume_offset = n;
        if !b.is_empty() && b[0] == b'-' {
            n += 1;
        }
        if b.len() <= n {
            return (
                resume_offset,
                BEFORE_INTEGER_DIGITS,
                Some(Err::UnexpectedEof),
            );
        } else if b[n] == b'0' {
            n += 1;
            state = BEFORE_FRACTIONAL_DIGITS;
        } else if b'1' <= b[n] && b[n] <= b'9' {
            n += 1;
            while b.len() > n && b[n].is_ascii_digit() {
                n += 1;
            }
            state = WITHIN_INTEGER_DIGITS;
        } else {
            return (
                n,
                state,
                Some(new_invalid_character_error(
                    &b[n..],
                    "in number (expecting digit)",
                )),
            );
        }
    }

    // Consume optional fractional component.
    if start <= 1 && b.len() > n && b[n] == b'.' {
        resume_offset = n;
        n += 1;
        if b.len() <= n {
            return (
                resume_offset,
                BEFORE_FRACTIONAL_DIGITS,
                Some(Err::UnexpectedEof),
            );
        } else if b[n].is_ascii_digit() {
            n += 1;
        } else {
            return (
                n,
                state,
                Some(new_invalid_character_error(
                    &b[n..],
                    "in number (expecting digit)",
                )),
            );
        }
        while b.len() > n && b[n].is_ascii_digit() {
            n += 1;
        }
        state = WITHIN_FRACTIONAL_DIGITS;
    }

    // Consume optional exponent component.
    if b.len() > n && (b[n] == b'e' || b[n] == b'E') {
        resume_offset = n;
        n += 1;
        if b.len() > n && (b[n] == b'-' || b[n] == b'+') {
            n += 1;
        }
        if b.len() <= n {
            return (
                resume_offset,
                BEFORE_EXPONENT_DIGITS,
                Some(Err::UnexpectedEof),
            );
        } else if b[n].is_ascii_digit() {
            n += 1;
        } else {
            return (
                n,
                state,
                Some(new_invalid_character_error(
                    &b[n..],
                    "in number (expecting digit)",
                )),
            );
        }
        while b.len() > n && b[n].is_ascii_digit() {
            n += 1;
        }
        state = WITHIN_EXPONENT_DIGITS;
    }

    (n, state, None)
}

// Go: decode.go:parseHexUint16
/// parseHexUint16 is similar to strconv.ParseUint,
/// but operates directly on []byte and is optimized for base-16.
/// See https://go.dev/issue/42429.
fn parse_hex_uint16(b: &[u8]) -> (u16, bool) {
    if b.len() != 4 {
        return (0, false);
    }
    let mut v: u16 = 0;
    for &c in &b[..4] {
        let c = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => 10 + c - b'a',
            b'A'..=b'F' => 10 + c - b'A',
            _ => return (0, false),
        };
        v = v.wrapping_mul(16).wrapping_add(c as u16);
    }
    (v, true)
}
