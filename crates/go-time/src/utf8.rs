//! Minimal port of `unicode/utf8.DecodeRuneInString` (go1.27.1), used where
//! the Go `time` package ranges over a string (`for i, c := range s`).
//!
//! Kept local (instead of depending on `go-unicode`) because the `time`
//! package only needs rune decoding, and Go's own `time` package likewise
//! avoids depending on `unicode/utf8`.

/// Go: `utf8.RuneError`.
pub(crate) const RUNE_ERROR: u32 = 0xFFFD;

/// Go: `utf8.RuneSelf`.
pub(crate) const RUNE_SELF: u32 = 0x80;

// Go: unicode/utf8/utf8.go:DecodeRuneInString
/// Decodes the first rune of `s`, returning the rune and its width in bytes.
/// Invalid encodings return `(RUNE_ERROR, 1)`, an empty input `(RUNE_ERROR, 0)`.
pub(crate) fn decode_rune(s: &[u8]) -> (u32, usize) {
    let n = s.len();
    if n < 1 {
        return (RUNE_ERROR, 0);
    }
    let p0 = s[0];
    if p0 < 0x80 {
        return (p0 as u32, 1);
    }
    // Accept ranges for the second byte, per the first byte (Go's `first`
    // and `acceptRanges` tables).
    let (sz, lo, hi) = match p0 {
        0xC2..=0xDF => (2, 0x80, 0xBF),
        0xE0 => (3, 0xA0, 0xBF),
        0xE1..=0xEC => (3, 0x80, 0xBF),
        0xED => (3, 0x80, 0x9F),
        0xEE..=0xEF => (3, 0x80, 0xBF),
        0xF0 => (4, 0x90, 0xBF),
        0xF1..=0xF3 => (4, 0x80, 0xBF),
        0xF4 => (4, 0x80, 0x8F),
        _ => return (RUNE_ERROR, 1),
    };
    if n < sz {
        return (RUNE_ERROR, 1);
    }
    let s1 = s[1];
    if s1 < lo || hi < s1 {
        return (RUNE_ERROR, 1);
    }
    if sz <= 2 {
        return ((((p0 & 0x1F) as u32) << 6) | (s1 & 0x3F) as u32, 2);
    }
    let s2 = s[2];
    if !(0x80..=0xBF).contains(&s2) {
        return (RUNE_ERROR, 1);
    }
    if sz <= 3 {
        return (
            (((p0 & 0x0F) as u32) << 12) | (((s1 & 0x3F) as u32) << 6) | (s2 & 0x3F) as u32,
            3,
        );
    }
    let s3 = s[3];
    if !(0x80..=0xBF).contains(&s3) {
        return (RUNE_ERROR, 1);
    }
    (
        (((p0 & 0x07) as u32) << 18)
            | (((s1 & 0x3F) as u32) << 12)
            | (((s2 & 0x3F) as u32) << 6)
            | (s3 & 0x3F) as u32,
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        assert_eq!(decode_rune(b""), (RUNE_ERROR, 0));
        assert_eq!(decode_rune(b"a"), ('a' as u32, 1));
        assert_eq!(decode_rune("☺".as_bytes()), ('☺' as u32, 3));
        assert_eq!(decode_rune("\u{FFFD}".as_bytes()), (RUNE_ERROR, 3));
        assert_eq!(decode_rune(b"\xff"), (RUNE_ERROR, 1));
        assert_eq!(decode_rune(b"\xe2\x98"), (RUNE_ERROR, 1));
        assert_eq!(decode_rune(b"\xed\xa0\x80"), (RUNE_ERROR, 1)); // surrogate
        assert_eq!(decode_rune("𝄞".as_bytes()), ('𝄞' as u32, 4));
        assert_eq!(decode_rune("µ".as_bytes()), ('µ' as u32, 2));
    }
}
