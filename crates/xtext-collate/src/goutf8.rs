//! The subset of Go's `unicode/utf8` and `unicode` used by colltab.
//!
//! `unicode.Ideographic` and `unicode.Nd` are dumped from the golden Go
//! toolchain (go1.27.1) by the oracle generator into `data/unicode.bin`.

use std::sync::OnceLock;

use crate::blob::Blob;

pub const RUNE_ERROR: i32 = 0xFFFD;
pub const RUNE_SELF: u8 = 0x80;

// Go: unicode/utf8/utf8.go:RuneStart
#[inline]
pub fn rune_start(b: u8) -> bool {
    b & 0xC0 != 0x80
}

// Go: unicode/utf8/utf8.go:DecodeRune
/// Decodes the first rune of `p` with Go's rules: an invalid or incomplete
/// encoding yields `(RuneError, 1)`, an empty input `(RuneError, 0)`.
pub fn decode_rune(p: &[u8]) -> (i32, usize) {
    let n = p.len();
    if n < 1 {
        return (RUNE_ERROR, 0);
    }
    let p0 = p[0];
    if p0 < 0x80 {
        return (p0 as i32, 1);
    }
    // Lead byte classification (Go's `first` table).
    let (sz, lo, hi) = match p0 {
        0xC2..=0xDF => (2, 0x80, 0xBF),
        0xE0 => (3, 0xA0, 0xBF),
        0xE1..=0xEC | 0xEE..=0xEF => (3, 0x80, 0xBF),
        0xED => (3, 0x80, 0x9F),
        0xF0 => (4, 0x90, 0xBF),
        0xF1..=0xF3 => (4, 0x80, 0xBF),
        0xF4 => (4, 0x80, 0x8F),
        _ => return (RUNE_ERROR, 1),
    };
    if n < sz {
        return (RUNE_ERROR, 1);
    }
    let b1 = p[1];
    if b1 < lo || hi < b1 {
        return (RUNE_ERROR, 1);
    }
    if sz <= 2 {
        return ((((p0 & 0x1F) as i32) << 6) | (b1 & 0x3F) as i32, 2);
    }
    let b2 = p[2];
    if !(0x80..=0xBF).contains(&b2) {
        return (RUNE_ERROR, 1);
    }
    if sz <= 3 {
        return (
            (((p0 & 0x0F) as i32) << 12) | (((b1 & 0x3F) as i32) << 6) | (b2 & 0x3F) as i32,
            3,
        );
    }
    let b3 = p[3];
    if !(0x80..=0xBF).contains(&b3) {
        return (RUNE_ERROR, 1);
    }
    (
        (((p0 & 0x07) as i32) << 18)
            | (((b1 & 0x3F) as i32) << 12)
            | (((b2 & 0x3F) as i32) << 6)
            | (b3 & 0x3F) as i32,
        4,
    )
}

// Go: unicode/utf8/utf8.go:EncodeRune (only used for valid scalar values).
pub fn encode_rune(buf: &mut [u8], r: i32) -> usize {
    let r = r as u32;
    if r < 0x80 {
        buf[0] = r as u8;
        1
    } else if r < 0x800 {
        buf[0] = 0xC0 | (r >> 6) as u8;
        buf[1] = 0x80 | (r & 0x3F) as u8;
        2
    } else if r < 0x10000 {
        buf[0] = 0xE0 | (r >> 12) as u8;
        buf[1] = 0x80 | ((r >> 6) & 0x3F) as u8;
        buf[2] = 0x80 | (r & 0x3F) as u8;
        3
    } else {
        buf[0] = 0xF0 | (r >> 18) as u8;
        buf[1] = 0x80 | ((r >> 12) & 0x3F) as u8;
        buf[2] = 0x80 | ((r >> 6) & 0x3F) as u8;
        buf[3] = 0x80 | (r & 0x3F) as u8;
        4
    }
}

struct UnicodeTables {
    ideographic: Vec<[u32; 3]>,
    nd: Vec<[u32; 3]>,
    version: &'static str,
}

fn tables() -> &'static UnicodeTables {
    static T: OnceLock<UnicodeTables> = OnceLock::new();
    T.get_or_init(|| {
        let b = Blob::parse(include_bytes!("../data/unicode.bin"));
        let ranges = |name: &str| {
            b.u32s(name)
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .collect::<Vec<_>>()
        };
        UnicodeTables {
            ideographic: ranges("Ideographic"),
            nd: ranges("Nd"),
            version: b.str("Version"),
        }
    })
}

/// The Unicode version of Go's `unicode` package the tables were dumped from.
pub fn unicode_version() -> &'static str {
    tables().version
}

// Go: unicode/letter.go:Is (membership semantics of a RangeTable).
fn is(ranges: &[[u32; 3]], r: i32) -> bool {
    if r < 0 {
        return false;
    }
    let r = r as u32;
    // Ranges are sorted and disjoint; binary search like is16/is32.
    let (mut lo, mut hi) = (0usize, ranges.len());
    while lo < hi {
        let m = lo + (hi - lo) / 2;
        let [rlo, rhi, stride] = ranges[m];
        if rlo <= r && r <= rhi {
            return stride == 1 || (r - rlo) % stride == 0;
        }
        if r < rlo {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    false
}

/// `unicode.Is(unicode.Ideographic, r)`.
pub fn is_ideographic(r: i32) -> bool {
    is(&tables().ideographic, r)
}

/// `unicode.In(r, unicode.Nd)`.
pub fn is_nd(r: i32) -> bool {
    is(&tables().nd, r)
}
