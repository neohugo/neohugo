//! Minimal private port of Go's `unicode/utf8` (go1.27.1): only `EncodeRune`.
//! The canonical port lives in the `go-unicode` crate; this copy keeps the
//! crate standalone (see PORTING.md).

const MAX_RUNE: u32 = 0x10FFFF;
const SURROGATE_MIN: u32 = 0xD800;
const SURROGATE_MAX: u32 = 0xDFFF;

const TX: u8 = 0b1000_0000;
const T2: u8 = 0b1100_0000;
const T3: u8 = 0b1110_0000;
const T4: u8 = 0b1111_0000;
const MASKX: u8 = 0b0011_1111;

const RUNE1_MAX: u32 = (1 << 7) - 1;
const RUNE2_MAX: u32 = (1 << 11) - 1;
const RUNE3_MAX: u32 = (1 << 16) - 1;

// Go: unicode/utf8/utf8.go:EncodeRune (+ encodeRuneNonASCII)
/// Writes the UTF-8 encoding of `r` into `p` and returns the number of bytes
/// written. Invalid runes (negative, surrogates, > MaxRune) encode U+FFFD.
/// Panics if `p` is too short, like Go.
pub fn encode_rune(p: &mut [u8], r: i32) -> usize {
    // Negative values are erroneous. Making it unsigned addresses the problem.
    let i = r as u32;
    if i <= RUNE1_MAX {
        p[0] = r as u8;
        return 1;
    }
    if i <= RUNE2_MAX {
        p[0] = T2 | (r >> 6) as u8;
        p[1] = TX | (r as u8) & MASKX;
        2
    } else if i < SURROGATE_MIN || SURROGATE_MAX < i && i <= RUNE3_MAX {
        p[0] = T3 | (r >> 12) as u8;
        p[1] = TX | ((r >> 6) as u8) & MASKX;
        p[2] = TX | (r as u8) & MASKX;
        3
    } else if i > RUNE3_MAX && i <= MAX_RUNE {
        p[0] = T4 | (r >> 18) as u8;
        p[1] = TX | ((r >> 12) as u8) & MASKX;
        p[2] = TX | ((r >> 6) as u8) & MASKX;
        p[3] = TX | (r as u8) & MASKX;
        4
    } else {
        p[0] = 0xEF;
        p[1] = 0xBF;
        p[2] = 0xBD;
        3
    }
}
