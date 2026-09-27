//! Go's `unicode/utf8` package (go1.27.1), over `&[u8]`.
//!
//! Go has separate `[]byte` and `string` entry points (`DecodeRune` /
//! `DecodeRuneInString`, ...) with identical semantics; both names are kept
//! here and take `&[u8]`.
//!
//! Also provides the Go *language* conversions that decode or encode UTF-8
//! (`for i, r := range s`, `[]rune(s)`, `string(runes)`), which the Go runtime
//! implements with the same invalid-byte rules: an invalid byte decodes to
//! `RuneError` with width 1, and an invalid rune encodes as `RuneError`.

use crate::Rune;

// Numbers fundamental to the encoding.

/// the "error" Rune or "Unicode replacement character"
pub const RUNE_ERROR: Rune = 0xFFFD;
/// characters below RuneSelf are represented as themselves in a single byte.
pub const RUNE_SELF: Rune = 0x80;
/// Maximum valid Unicode code point.
pub const MAX_RUNE: Rune = 0x10FFFF;
/// maximum number of bytes of a UTF-8 encoded Unicode character.
pub const UTF_MAX: usize = 4;

// Code points in the surrogate range are not valid for UTF-8.
const SURROGATE_MIN: Rune = 0xD800;
const SURROGATE_MAX: Rune = 0xDFFF;

#[allow(dead_code)]
const T1: u8 = 0b0000_0000;
const TX: u8 = 0b1000_0000;
const T2: u8 = 0b1100_0000;
const T3: u8 = 0b1110_0000;
const T4: u8 = 0b1111_0000;
#[allow(dead_code)]
const T5: u8 = 0b1111_1000;

const MASKX: u8 = 0b0011_1111;
const MASK2: u8 = 0b0001_1111;
const MASK3: u8 = 0b0000_1111;
const MASK4: u8 = 0b0000_0111;

const RUNE1_MAX: Rune = (1 << 7) - 1;
const RUNE2_MAX: Rune = (1 << 11) - 1;
const RUNE3_MAX: Rune = (1 << 16) - 1;

// The default lowest and highest continuation byte.
const LOCB: u8 = 0b1000_0000;
const HICB: u8 = 0b1011_1111;

// These names of these constants are chosen to give nice alignment in the
// table below. The first nibble is an index into acceptRanges or F for
// special one-byte cases. The second nibble is the Rune length or the
// Status for the special one-byte case.
const XX: u8 = 0xF1; // invalid: size 1
const AS: u8 = 0xF0; // ASCII: size 1
const S1: u8 = 0x02; // accept 0, size 2
const S2: u8 = 0x13; // accept 1, size 3
const S3: u8 = 0x03; // accept 0, size 3
const S4: u8 = 0x23; // accept 2, size 3
const S5: u8 = 0x34; // accept 3, size 4
const S6: u8 = 0x04; // accept 0, size 4
const S7: u8 = 0x44; // accept 4, size 4

const RUNE_ERROR_BYTE0: u8 = T3 | (RUNE_ERROR >> 12) as u8;
const RUNE_ERROR_BYTE1: u8 = TX | ((RUNE_ERROR >> 6) as u8 & MASKX);
const RUNE_ERROR_BYTE2: u8 = TX | (RUNE_ERROR as u8 & MASKX);

/// first is information about the first byte in a UTF-8 sequence.
#[rustfmt::skip]
static FIRST: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x00-0x0F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x10-0x1F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x20-0x2F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x30-0x3F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x40-0x4F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x50-0x5F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x60-0x6F
    AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, AS, // 0x70-0x7F
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, // 0x80-0x8F
    XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, // 0x90-0x9F
    XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, // 0xA0-0xAF
    XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, // 0xB0-0xBF
    XX, XX, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, // 0xC0-0xCF
    S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, S1, // 0xD0-0xDF
    S2, S3, S3, S3, S3, S3, S3, S3, S3, S3, S3, S3, S3, S4, S3, S3, // 0xE0-0xEF
    S5, S6, S6, S6, S7, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, // 0xF0-0xFF
];

/// acceptRange gives the range of valid values for the second byte in a UTF-8
/// sequence.
#[derive(Clone, Copy)]
struct AcceptRange {
    lo: u8, // lowest value for second byte.
    hi: u8, // highest value for second byte.
}

const fn ar(lo: u8, hi: u8) -> AcceptRange {
    AcceptRange { lo, hi }
}

/// acceptRanges has size 16 to avoid bounds checks in the code that uses it.
static ACCEPT_RANGES: [AcceptRange; 16] = [
    ar(LOCB, HICB),
    ar(0xA0, HICB),
    ar(LOCB, 0x9F),
    ar(0x90, HICB),
    ar(LOCB, 0x8F),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
    ar(0, 0),
];

// Go: src/unicode/utf8/utf8.go:FullRune
/// FullRune reports whether the bytes in p begin with a full UTF-8 encoding of a rune.
/// An invalid encoding is considered a full Rune since it will convert as a width-1 error rune.
pub fn full_rune(p: &[u8]) -> bool {
    let n = p.len();
    if n == 0 {
        return false;
    }
    let x = FIRST[p[0] as usize];
    if n >= (x & 7) as usize {
        return true; // ASCII, invalid or valid.
    }
    // Must be short or invalid.
    let accept = ACCEPT_RANGES[(x >> 4) as usize];
    if n > 1 && (p[1] < accept.lo || accept.hi < p[1]) {
        return true;
    } else if n > 2 && (p[2] < LOCB || HICB < p[2]) {
        return true;
    }
    false
}

// Go: src/unicode/utf8/utf8.go:FullRuneInString
/// FullRuneInString is like FullRune but its input is a string.
pub fn full_rune_in_string(s: &[u8]) -> bool {
    full_rune(s)
}

// Go: src/unicode/utf8/utf8.go:DecodeRune
/// DecodeRune unpacks the first UTF-8 encoding in p and returns the rune and
/// its width in bytes. If p is empty it returns (RuneError, 0). Otherwise, if
/// the encoding is invalid, it returns (RuneError, 1). Both are impossible
/// results for correct, non-empty UTF-8.
///
/// An encoding is invalid if it is incorrect UTF-8, encodes a rune that is
/// out of range, or is not the shortest possible UTF-8 encoding for the
/// value. No other validation is performed.
#[inline]
pub fn decode_rune(p: &[u8]) -> (Rune, usize) {
    // Inlineable fast path for ASCII characters; see #48195.
    if let Some(&b) = p.first()
        && (b as Rune) < RUNE_SELF
    {
        return (b as Rune, 1);
    }
    decode_rune_slow(p)
}

// Go: src/unicode/utf8/utf8.go:decodeRuneSlow
fn decode_rune_slow(p: &[u8]) -> (Rune, usize) {
    let n = p.len();
    if n < 1 {
        return (RUNE_ERROR, 0);
    }
    let p0 = p[0];
    let x = FIRST[p0 as usize];
    if x >= AS {
        // The following code simulates an additional check for x == xx and
        // handling the ASCII and invalid cases accordingly. This mask-and-or
        // approach prevents an additional branch.
        let mask = ((x as Rune) << 31) >> 31; // Create 0x0000 or 0xFFFF.
        return ((p[0] as Rune & !mask) | (RUNE_ERROR & mask), 1);
    }
    let sz = (x & 7) as usize;
    let accept = ACCEPT_RANGES[(x >> 4) as usize];
    if n < sz {
        return (RUNE_ERROR, 1);
    }
    let b1 = p[1];
    if b1 < accept.lo || accept.hi < b1 {
        return (RUNE_ERROR, 1);
    }
    if sz <= 2 {
        // <= instead of == to help the compiler eliminate some bounds checks
        return ((((p0 & MASK2) as Rune) << 6) | (b1 & MASKX) as Rune, 2);
    }
    let b2 = p[2];
    if !(LOCB..=HICB).contains(&b2) {
        return (RUNE_ERROR, 1);
    }
    if sz <= 3 {
        return (
            (((p0 & MASK3) as Rune) << 12) | (((b1 & MASKX) as Rune) << 6) | (b2 & MASKX) as Rune,
            3,
        );
    }
    let b3 = p[3];
    if !(LOCB..=HICB).contains(&b3) {
        return (RUNE_ERROR, 1);
    }
    (
        (((p0 & MASK4) as Rune) << 18)
            | (((b1 & MASKX) as Rune) << 12)
            | (((b2 & MASKX) as Rune) << 6)
            | (b3 & MASKX) as Rune,
        4,
    )
}

// Go: src/unicode/utf8/utf8.go:DecodeRuneInString
/// DecodeRuneInString is like [`decode_rune`] but its input is a string.
#[inline]
pub fn decode_rune_in_string(s: &[u8]) -> (Rune, usize) {
    decode_rune(s)
}

// Go: src/unicode/utf8/utf8.go:DecodeLastRune
/// DecodeLastRune unpacks the last UTF-8 encoding in p and returns the rune and
/// its width in bytes. If p is empty it returns (RuneError, 0). Otherwise, if
/// the encoding is invalid, it returns (RuneError, 1). Both are impossible
/// results for correct, non-empty UTF-8.
pub fn decode_last_rune(p: &[u8]) -> (Rune, usize) {
    let end = p.len() as isize;
    if end == 0 {
        return (RUNE_ERROR, 0);
    }
    let mut start = end - 1;
    let r = p[start as usize] as Rune;
    if r < RUNE_SELF {
        return (r, 1);
    }
    // guard against O(n^2) behavior when traversing
    // backwards through strings with long sequences of
    // invalid UTF-8.
    let lim = (end - UTF_MAX as isize).max(0);
    start -= 1;
    while start >= lim {
        if rune_start(p[start as usize]) {
            break;
        }
        start -= 1;
    }
    if start < 0 {
        start = 0;
    }
    let (r, size) = decode_rune(&p[start as usize..end as usize]);
    if start + size as isize != end {
        return (RUNE_ERROR, 1);
    }
    (r, size)
}

// Go: src/unicode/utf8/utf8.go:DecodeLastRuneInString
/// DecodeLastRuneInString is like [`decode_last_rune`] but its input is a string.
pub fn decode_last_rune_in_string(s: &[u8]) -> (Rune, usize) {
    decode_last_rune(s)
}

// Go: src/unicode/utf8/utf8.go:RuneLen
/// RuneLen returns the number of bytes in the UTF-8 encoding of the rune.
/// It returns -1 if the rune is not a valid value to encode in UTF-8.
pub fn rune_len(r: Rune) -> isize {
    if r < 0 {
        -1
    } else if r <= RUNE1_MAX {
        1
    } else if r <= RUNE2_MAX {
        2
    } else if (SURROGATE_MIN..=SURROGATE_MAX).contains(&r) {
        -1
    } else if r <= RUNE3_MAX {
        3
    } else if r <= MAX_RUNE {
        4
    } else {
        -1
    }
}

// Go: src/unicode/utf8/utf8.go:EncodeRune
/// EncodeRune writes into p (which must be large enough) the UTF-8 encoding of the rune.
/// If the rune is out of range, it writes the encoding of RuneError.
/// It returns the number of bytes written.
///
/// Panics (like Go) if p is too short.
#[inline]
pub fn encode_rune(p: &mut [u8], r: Rune) -> usize {
    // This function is inlineable for fast handling of ASCII.
    if r as u32 <= RUNE1_MAX as u32 {
        p[0] = r as u8;
        return 1;
    }
    encode_rune_non_ascii(p, r)
}

// Go: src/unicode/utf8/utf8.go:encodeRuneNonASCII
fn encode_rune_non_ascii(p: &mut [u8], r: Rune) -> usize {
    // Negative values are erroneous. Making it unsigned addresses the problem.
    let i = r as u32;
    if i <= RUNE2_MAX as u32 {
        let _ = p[1]; // eliminate bounds checks
        p[0] = T2 | (r >> 6) as u8;
        p[1] = TX | (r as u8 & MASKX);
        2
    } else if i < SURROGATE_MIN as u32 || (SURROGATE_MAX as u32) < i && i <= RUNE3_MAX as u32 {
        let _ = p[2]; // eliminate bounds checks
        p[0] = T3 | (r >> 12) as u8;
        p[1] = TX | ((r >> 6) as u8 & MASKX);
        p[2] = TX | (r as u8 & MASKX);
        3
    } else if i > RUNE3_MAX as u32 && i <= MAX_RUNE as u32 {
        let _ = p[3]; // eliminate bounds checks
        p[0] = T4 | (r >> 18) as u8;
        p[1] = TX | ((r >> 12) as u8 & MASKX);
        p[2] = TX | ((r >> 6) as u8 & MASKX);
        p[3] = TX | (r as u8 & MASKX);
        4
    } else {
        let _ = p[2]; // eliminate bounds checks
        p[0] = RUNE_ERROR_BYTE0;
        p[1] = RUNE_ERROR_BYTE1;
        p[2] = RUNE_ERROR_BYTE2;
        3
    }
}

// Go: src/unicode/utf8/utf8.go:AppendRune
/// AppendRune appends the UTF-8 encoding of r to the end of p.
/// If the rune is out of range, it appends the encoding of RuneError.
/// (Go returns the extended slice; here `p` is extended in place.)
#[inline]
pub fn append_rune(p: &mut Vec<u8>, r: Rune) {
    // This function is inlineable for fast handling of ASCII.
    if r as u32 <= RUNE1_MAX as u32 {
        p.push(r as u8);
        return;
    }
    append_rune_non_ascii(p, r)
}

// Go: src/unicode/utf8/utf8.go:appendRuneNonASCII
fn append_rune_non_ascii(p: &mut Vec<u8>, r: Rune) {
    // Negative values are erroneous. Making it unsigned addresses the problem.
    let i = r as u32;
    if i <= RUNE2_MAX as u32 {
        p.extend_from_slice(&[T2 | (r >> 6) as u8, TX | (r as u8 & MASKX)]);
    } else if i < SURROGATE_MIN as u32 || (SURROGATE_MAX as u32) < i && i <= RUNE3_MAX as u32 {
        p.extend_from_slice(&[
            T3 | (r >> 12) as u8,
            TX | ((r >> 6) as u8 & MASKX),
            TX | (r as u8 & MASKX),
        ]);
    } else if i > RUNE3_MAX as u32 && i <= MAX_RUNE as u32 {
        p.extend_from_slice(&[
            T4 | (r >> 18) as u8,
            TX | ((r >> 12) as u8 & MASKX),
            TX | ((r >> 6) as u8 & MASKX),
            TX | (r as u8 & MASKX),
        ]);
    } else {
        p.extend_from_slice(&[RUNE_ERROR_BYTE0, RUNE_ERROR_BYTE1, RUNE_ERROR_BYTE2]);
    }
}

// Go: src/unicode/utf8/utf8.go:RuneCount
/// RuneCount returns the number of runes in p. Erroneous and short
/// encodings are treated as single runes of width 1 byte.
pub fn rune_count(p: &[u8]) -> usize {
    let np = p.len();
    let mut n = 0;
    while n < np {
        let c = p[n];
        if c as Rune >= RUNE_SELF {
            // non-ASCII slow path
            return n + rune_count_in_string(&p[n..]);
        }
        n += 1;
    }
    n
}

// Go: src/unicode/utf8/utf8.go:RuneCountInString
/// RuneCountInString is like [`rune_count`] but its input is a string.
pub fn rune_count_in_string(s: &[u8]) -> usize {
    let mut n = 0;
    for _ in runes(s) {
        n += 1;
    }
    n
}

// Go: src/unicode/utf8/utf8.go:RuneStart
/// RuneStart reports whether the byte could be the first byte of an encoded,
/// possibly invalid rune. Second and subsequent bytes always have the top two
/// bits set to 10.
#[inline]
pub fn rune_start(b: u8) -> bool {
    b & 0xC0 != 0x80
}

// Go: src/unicode/utf8/utf8.go:Valid
/// Valid reports whether p consists entirely of valid UTF-8-encoded runes.
/// (The word-at-a-time ASCII skipping of the Go code is an optimisation with
/// no observable effect and is done here byte by byte.)
pub fn valid(mut p: &[u8]) -> bool {
    while !p.is_empty() {
        let p0 = p[0];
        if (p0 as Rune) < RUNE_SELF {
            p = &p[1..];
            continue;
        }
        let x = FIRST[p0 as usize];
        let size = (x & 7) as usize;
        let accept = ACCEPT_RANGES[(x >> 4) as usize];
        match size {
            2 => {
                if p.len() < 2 || p[1] < accept.lo || accept.hi < p[1] {
                    return false;
                }
                p = &p[2..];
            }
            3 => {
                if p.len() < 3 || p[1] < accept.lo || accept.hi < p[1] || p[2] < LOCB || HICB < p[2]
                {
                    return false;
                }
                p = &p[3..];
            }
            4 => {
                if p.len() < 4
                    || p[1] < accept.lo
                    || accept.hi < p[1]
                    || p[2] < LOCB
                    || HICB < p[2]
                    || p[3] < LOCB
                    || HICB < p[3]
                {
                    return false;
                }
                p = &p[4..];
            }
            _ => return false, // illegal starter byte
        }
    }
    true
}

// Go: src/unicode/utf8/utf8.go:ValidString
/// ValidString reports whether s consists entirely of valid UTF-8-encoded runes.
pub fn valid_string(s: &[u8]) -> bool {
    valid(s)
}

// Go: src/unicode/utf8/utf8.go:ValidRune
/// ValidRune reports whether r can be legally encoded as UTF-8.
/// Code points that are out of range or a surrogate half are illegal.
pub fn valid_rune(r: Rune) -> bool {
    (0..SURROGATE_MIN).contains(&r) || (SURROGATE_MAX < r && r <= MAX_RUNE)
}

// ---------------------------------------------------------------------------
// Go language conversions (runtime/utf8.go, runtime/string.go).

/// Iterator over `(byte_index, rune)` with the semantics of Go's
/// `for i, r := range s` (runtime `decoderune`: an invalid byte yields
/// `RuneError` and advances by one byte).
#[derive(Clone, Debug)]
pub struct Runes<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Iterator for Runes<'_> {
    type Item = (usize, Rune);

    #[inline]
    fn next(&mut self) -> Option<(usize, Rune)> {
        if self.pos >= self.s.len() {
            return None;
        }
        let i = self.pos;
        let (r, size) = decode_rune(&self.s[i..]);
        self.pos += size;
        Some((i, r))
    }
}

/// `for i, r := range s` over a Go string.
#[inline]
pub fn runes(s: &[u8]) -> Runes<'_> {
    Runes { s, pos: 0 }
}

/// Go `[]rune(s)`.
pub fn to_runes(s: &[u8]) -> Vec<Rune> {
    runes(s).map(|(_, r)| r).collect()
}

/// Go `string(rs)` for `rs []rune` (invalid runes become U+FFFD).
pub fn from_runes(rs: &[Rune]) -> Vec<u8> {
    let mut b = Vec::with_capacity(rs.len());
    for &r in rs {
        append_rune(&mut b, r);
    }
    b
}

/// Go `string(r)` for a rune `r` (invalid runes become U+FFFD).
pub fn rune_to_string(r: Rune) -> Vec<u8> {
    let mut b = Vec::with_capacity(4);
    append_rune(&mut b, r);
    b
}
