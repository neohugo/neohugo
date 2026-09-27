//! Go's `unicode/utf16` package (go1.27.1).

use crate::Rune;

const REPLACEMENT_CHAR: Rune = 0xFFFD; // Unicode replacement character
const MAX_RUNE: Rune = 0x10FFFF; // Maximum valid Unicode code point.

// 0xd800-0xdc00 encodes the high 10 bits of a pair.
// 0xdc00-0xe000 encodes the low 10 bits of a pair.
// the value is those 20 bits plus 0x10000.
const SURR1: Rune = 0xd800;
const SURR2: Rune = 0xdc00;
const SURR3: Rune = 0xe000;

const SURR_SELF: Rune = 0x10000;

// Go: src/unicode/utf16/utf16.go:IsSurrogate
/// IsSurrogate reports whether the specified Unicode code point
/// can appear in a surrogate pair.
pub fn is_surrogate(r: Rune) -> bool {
    (SURR1..SURR3).contains(&r)
}

// Go: src/unicode/utf16/utf16.go:DecodeRune
/// DecodeRune returns the UTF-16 decoding of a surrogate pair.
/// If the pair is not a valid UTF-16 surrogate pair, DecodeRune returns
/// the Unicode replacement code point U+FFFD.
pub fn decode_rune(r1: Rune, r2: Rune) -> Rune {
    if (SURR1..SURR2).contains(&r1) && (SURR2..SURR3).contains(&r2) {
        return (((r1 - SURR1) << 10) | (r2 - SURR2)) + SURR_SELF;
    }
    REPLACEMENT_CHAR
}

// Go: src/unicode/utf16/utf16.go:EncodeRune
/// EncodeRune returns the UTF-16 surrogate pair r1, r2 for the given rune.
/// If the rune is not a valid Unicode code point or does not need encoding,
/// EncodeRune returns U+FFFD, U+FFFD.
pub fn encode_rune(mut r: Rune) -> (Rune, Rune) {
    if !(SURR_SELF..=MAX_RUNE).contains(&r) {
        return (REPLACEMENT_CHAR, REPLACEMENT_CHAR);
    }
    r -= SURR_SELF;
    (SURR1 + ((r >> 10) & 0x3ff), SURR2 + (r & 0x3ff))
}

// Go: src/unicode/utf16/utf16.go:RuneLen
/// RuneLen returns the number of 16-bit words in the UTF-16 encoding of the rune.
/// It returns -1 if the rune is not a valid value to encode in UTF-16.
pub fn rune_len(r: Rune) -> isize {
    if (0..SURR1).contains(&r) || (SURR3..SURR_SELF).contains(&r) {
        1
    } else if (SURR_SELF..=MAX_RUNE).contains(&r) {
        2
    } else {
        -1
    }
}

// Go: src/unicode/utf16/utf16.go:Encode
/// Encode returns the UTF-16 encoding of the Unicode code point sequence s.
pub fn encode(s: &[Rune]) -> Vec<u16> {
    let mut a = Vec::with_capacity(s.len());
    for &v in s {
        match rune_len(v) {
            1 => {
                // normal rune
                a.push(v as u16);
            }
            2 => {
                // needs surrogate sequence
                let (r1, r2) = encode_rune(v);
                a.push(r1 as u16);
                a.push(r2 as u16);
            }
            _ => {
                a.push(REPLACEMENT_CHAR as u16);
            }
        }
    }
    a
}

// Go: src/unicode/utf16/utf16.go:AppendRune
/// AppendRune appends the UTF-16 encoding of the Unicode code point r
/// to the end of p. If the rune is not a valid Unicode code point, it
/// appends the encoding of U+FFFD.
pub fn append_rune(a: &mut Vec<u16>, r: Rune) {
    if (0..SURR1).contains(&r) || (SURR3..SURR_SELF).contains(&r) {
        // normal rune
        a.push(r as u16);
    } else if (SURR_SELF..=MAX_RUNE).contains(&r) {
        // needs surrogate sequence
        let (r1, r2) = encode_rune(r);
        a.push(r1 as u16);
        a.push(r2 as u16);
    } else {
        a.push(REPLACEMENT_CHAR as u16);
    }
}

// Go: src/unicode/utf16/utf16.go:Decode
/// Decode returns the Unicode code point sequence represented
/// by the UTF-16 encoding s.
pub fn decode(s: &[u16]) -> Vec<Rune> {
    let mut buf = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let r = s[i] as Rune;
        let ar = if !(SURR1..SURR3).contains(&r) {
            // normal rune
            r
        } else if (SURR1..SURR2).contains(&r)
            && i + 1 < s.len()
            && (SURR2..SURR3).contains(&(s[i + 1] as Rune))
        {
            // valid surrogate sequence
            let d = decode_rune(r, s[i + 1] as Rune);
            i += 1;
            d
        } else {
            // invalid surrogate sequence
            REPLACEMENT_CHAR
        };
        buf.push(ar);
        i += 1;
    }
    buf
}
