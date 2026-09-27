// Minimal port of the go1.27.1 `unicode/utf8` functions strconv uses
// (DecodeRuneInString, AppendRune, ValidRune, ValidString), kept private so
// go-strconv stays a leaf crate. Semantics are Go's: an invalid byte decodes
// to (RuneError, 1).

pub(crate) const RUNE_ERROR: i32 = 0xFFFD;
pub(crate) const RUNE_SELF: i32 = 0x80;
pub(crate) const MAX_RUNE: i32 = 0x10FFFF;

const SURROGATE_MIN: i32 = 0xD800;
const SURROGATE_MAX: i32 = 0xDFFF;

const T2: u8 = 0b1100_0000;
const T3: u8 = 0b1110_0000;
const T4: u8 = 0b1111_0000;
const TX: u8 = 0b1000_0000;

const MASKX: u8 = 0b0011_1111;
const MASK2: u8 = 0b0001_1111;
const MASK3: u8 = 0b0000_1111;
const MASK4: u8 = 0b0000_0111;

const RUNE1_MAX: u32 = (1 << 7) - 1;
const RUNE2_MAX: u32 = (1 << 11) - 1;
const RUNE3_MAX: u32 = (1 << 16) - 1;

const LOCB: u8 = 0b1000_0000;
const HICB: u8 = 0b1011_1111;

const XX: u8 = 0xF1; // invalid: size 1
const AS: u8 = 0xF0; // ASCII: size 1
const S1: u8 = 0x02; // accept 0, size 2
const S2: u8 = 0x13; // accept 1, size 3
const S3: u8 = 0x03; // accept 0, size 3
const S4: u8 = 0x23; // accept 2, size 3
const S5: u8 = 0x34; // accept 3, size 4
const S6: u8 = 0x04; // accept 0, size 4
const S7: u8 = 0x44; // accept 4, size 4

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

#[derive(Clone, Copy)]
struct AcceptRange {
    lo: u8, // lowest value for second byte.
    hi: u8, // highest value for second byte.
}

static ACCEPT_RANGES: [AcceptRange; 16] = [
    AcceptRange { lo: LOCB, hi: HICB },
    AcceptRange { lo: 0xA0, hi: HICB },
    AcceptRange { lo: LOCB, hi: 0x9F },
    AcceptRange { lo: 0x90, hi: HICB },
    AcceptRange { lo: LOCB, hi: 0x8F },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
    AcceptRange { lo: 0, hi: 0 },
];

// Go: unicode/utf8/utf8.go:DecodeRuneInString
/// DecodeRuneInString unpacks the first UTF-8 encoding in s and returns the
/// rune and its width in bytes. If s is empty it returns (RuneError, 0).
/// Otherwise, if the encoding is invalid, it returns (RuneError, 1).
pub(crate) fn decode_rune(s: &[u8]) -> (i32, usize) {
    let n = s.len();
    if n < 1 {
        return (RUNE_ERROR, 0);
    }
    let s0 = s[0];
    let x = FIRST[s0 as usize];
    if x >= AS {
        if x == AS {
            return (s0 as i32, 1);
        }
        return (RUNE_ERROR, 1);
    }
    let sz = (x & 7) as usize;
    let accept = ACCEPT_RANGES[(x >> 4) as usize];
    if n < sz {
        return (RUNE_ERROR, 1);
    }
    let s1 = s[1];
    if s1 < accept.lo || accept.hi < s1 {
        return (RUNE_ERROR, 1);
    }
    if sz <= 2 {
        return (((s0 & MASK2) as i32) << 6 | (s1 & MASKX) as i32, 2);
    }
    let s2 = s[2];
    if !(LOCB..=HICB).contains(&s2) {
        return (RUNE_ERROR, 1);
    }
    if sz <= 3 {
        return (
            ((s0 & MASK3) as i32) << 12 | ((s1 & MASKX) as i32) << 6 | (s2 & MASKX) as i32,
            3,
        );
    }
    let s3 = s[3];
    if !(LOCB..=HICB).contains(&s3) {
        return (RUNE_ERROR, 1);
    }
    (
        ((s0 & MASK4) as i32) << 18
            | ((s1 & MASKX) as i32) << 12
            | ((s2 & MASKX) as i32) << 6
            | (s3 & MASKX) as i32,
        4,
    )
}

// Go: unicode/utf8/utf8.go:AppendRune
/// AppendRune appends the UTF-8 encoding of r to the end of p.
/// If the rune is out of range, it appends the encoding of RuneError.
pub(crate) fn append_rune(p: &mut Vec<u8>, r: i32) {
    let i = r as u32;
    if i <= RUNE1_MAX {
        p.push(r as u8);
    } else if i <= RUNE2_MAX {
        p.extend_from_slice(&[T2 | (r >> 6) as u8, TX | (r as u8) & MASKX]);
    } else if i < SURROGATE_MIN as u32 || (SURROGATE_MAX as u32) < i && i <= RUNE3_MAX {
        p.extend_from_slice(&[
            T3 | (r >> 12) as u8,
            TX | ((r >> 6) as u8) & MASKX,
            TX | (r as u8) & MASKX,
        ]);
    } else if i > RUNE3_MAX && i <= MAX_RUNE as u32 {
        p.extend_from_slice(&[
            T4 | (r >> 18) as u8,
            TX | ((r >> 12) as u8) & MASKX,
            TX | ((r >> 6) as u8) & MASKX,
            TX | (r as u8) & MASKX,
        ]);
    } else {
        p.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
    }
}

// Go: unicode/utf8/utf8.go:ValidRune
/// ValidRune reports whether r can be legally encoded as UTF-8.
/// Code points that are out of range or a surrogate half are illegal.
pub(crate) fn valid_rune(r: i32) -> bool {
    (0..SURROGATE_MIN).contains(&r) || (SURROGATE_MAX < r && r <= MAX_RUNE)
}

// Go: unicode/utf8/utf8.go:ValidString
/// ValidString reports whether s consists entirely of valid UTF-8-encoded runes.
/// (Go's word-at-a-time ASCII skipping does not change the result and is omitted.)
pub(crate) fn valid_string(s: &[u8]) -> bool {
    let mut s = s;
    while !s.is_empty() {
        let s0 = s[0];
        if (s0 as i32) < RUNE_SELF {
            s = &s[1..];
            continue;
        }
        let x = FIRST[s0 as usize];
        let size = (x & 7) as usize;
        let accept = ACCEPT_RANGES[(x >> 4) as usize];
        match size {
            2 => {
                if s.len() < 2 || s[1] < accept.lo || accept.hi < s[1] {
                    return false;
                }
                s = &s[2..];
            }
            3 => {
                if s.len() < 3 || s[1] < accept.lo || accept.hi < s[1] || s[2] < LOCB || HICB < s[2]
                {
                    return false;
                }
                s = &s[3..];
            }
            4 => {
                if s.len() < 4
                    || s[1] < accept.lo
                    || accept.hi < s[1]
                    || s[2] < LOCB
                    || HICB < s[2]
                    || s[3] < LOCB
                    || HICB < s[3]
                {
                    return false;
                }
                s = &s[4..];
            }
            _ => return false, // illegal starter byte
        }
    }
    true
}
