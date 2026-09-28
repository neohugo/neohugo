//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/internal/characters` (`ascii.go`, `utf8.go`).

/// Go: `invalidAsciiTable`: control characters other than TAB, LF and CR, and DEL.
// Go: internal/characters/ascii.go:InvalidAscii
pub(crate) fn invalid_ascii(b: u8) -> bool {
    matches!(b, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)
}

/// Go: `utf8Err`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Utf8Err {
    pub(crate) index: usize,
    pub(crate) size: usize,
}

impl Utf8Err {
    // Go: internal/characters/utf8.go:Zero
    pub(crate) fn zero(&self) -> bool {
        self.size == 0
    }
}

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

/// Go: `first` — information about the first byte in a UTF-8 sequence.
fn first(b: u8) -> u8 {
    match b {
        0x00..=0x7F => AS,
        0x80..=0xC1 => XX,
        0xC2..=0xDF => S1,
        0xE0 => S2,
        0xE1..=0xEC => S3,
        0xED => S4,
        0xEE..=0xEF => S3,
        0xF0 => S5,
        0xF1..=0xF3 => S6,
        0xF4 => S7,
        0xF5..=0xFF => XX,
    }
}

/// Go: `acceptRanges` (lo, hi) for the second byte.
fn accept_range(i: u8) -> (u8, u8) {
    match i {
        0 => (LOCB, HICB),
        1 => (0xA0, HICB),
        2 => (LOCB, 0x9F),
        3 => (0x90, HICB),
        4 => (LOCB, 0x8F),
        // Go's array has 16 zero entries: {0, 0}.
        _ => (0, 0),
    }
}

/// Verified that a given string is only made of valid UTF-8 characters allowed
/// by the TOML spec (a copy of Go 1.17's `utf8.Valid`, exiting early on a character that is not
/// allowed). The returned `Utf8Err` is zero if the string is valid, or holds the byte index and
/// size of the invalid character.
// Go: internal/characters/utf8.go:Utf8TomlValidAlreadyEscaped
pub(crate) fn utf8_toml_valid_already_escaped(p: &[u8]) -> Utf8Err {
    let mut err = Utf8Err::default();
    let mut p = p;
    // Fast path. Check for and skip 8 bytes of ASCII characters per iteration.
    let mut offset = 0;
    while p.len() >= 8 {
        if p[..8].iter().any(|&b| b >= 0x80) {
            // Found a non ASCII byte (>= RuneSelf).
            break;
        }

        for (i, &b) in p[..8].iter().enumerate() {
            if invalid_ascii(b) {
                err.index = offset + i;
                err.size = 1;
                return err;
            }
        }

        p = &p[8..];
        offset += 8;
    }
    let n = p.len();
    let mut i = 0;
    while i < n {
        let pi = p[i];
        if pi < 0x80 {
            if invalid_ascii(pi) {
                err.index = offset + i;
                err.size = 1;
                return err;
            }
            i += 1;
            continue;
        }
        let x = first(pi);
        if x == XX {
            // Illegal starter byte.
            err.index = offset + i;
            err.size = 1;
            return err;
        }
        let size = (x & 7) as usize;
        if i + size > n {
            // Short or invalid.
            err.index = offset + i;
            err.size = n - i;
            return err;
        }
        let (lo, hi) = accept_range(x >> 4);
        let c = p[i + 1];
        if c < lo || hi < c {
            err.index = offset + i;
            err.size = 2;
            return err;
        } else if size == 2 {
        } else if p[i + 2] < LOCB || HICB < p[i + 2] {
            err.index = offset + i;
            err.size = 3;
            return err;
        } else if size == 3 {
        } else if p[i + 3] < LOCB || HICB < p[i + 3] {
            err.index = offset + i;
            err.size = 4;
            return err;
        }
        i += size;
    }
    err
}

/// Return the size of the next rune if valid, 0 otherwise.
// Go: internal/characters/utf8.go:Utf8ValidNext
pub(crate) fn utf8_valid_next(p: &[u8]) -> usize {
    let c = p[0];

    if c < 0x80 {
        if invalid_ascii(c) {
            return 0;
        }
        return 1;
    }

    let x = first(c);
    if x == XX {
        // Illegal starter byte.
        return 0;
    }
    let size = (x & 7) as usize;
    if size > p.len() {
        // Short or invalid.
        return 0;
    }
    let (lo, hi) = accept_range(x >> 4);
    let c1 = p[1];
    if c1 < lo || hi < c1 {
        return 0;
    } else if size == 2 {
    } else if p[2] < LOCB || HICB < p[2] {
        return 0;
    } else if size == 3 {
    } else if p[3] < LOCB || HICB < p[3] {
        return 0;
    }

    size
}
