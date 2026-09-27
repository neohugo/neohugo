//! Port of go1.27.1 `compress/flate/deflatefast.go` and `load_store.go`.

use super::level1::FastEncL1;
use super::level2::FastEncL2;
use super::level3::FastEncL3;
use super::level4::FastEncL4;
use super::level5::FastEncL5;
use super::level6::FastEncL6;
use super::token::Tokens;
use super::{MAX_MATCH_LENGTH, MAX_STORE_BLOCK_SIZE};

/// tableBits is the number of bits used in the hash table.
pub(crate) const TABLE_BITS: u8 = 15;

/// tableSize is the size of the hash table.
pub(crate) const TABLE_SIZE: usize = 1 << TABLE_BITS;

/// hashLongBytes is the number of bytes used for long table hashes.
pub(crate) const HASH_LONG_BYTES: u8 = 7;

/// baseMatchOffset is the smallest match offset.
pub(crate) const BASE_MATCH_OFFSET: i32 = 1;

/// baseMatchLength is the smallest match length per RFC section 3.2.5.
pub(crate) const BASE_MATCH_LENGTH: i32 = 3;

/// maxMatchOffset is the largest match offset.
pub(crate) const MAX_MATCH_OFFSET: i32 = 1 << 15;

/// allocHistory is the size to preallocate for history.
pub(crate) const ALLOC_HISTORY: usize = MAX_STORE_BLOCK_SIZE as usize * 5;

/// bufferReset is the buffer offset at which the history is reset.
pub(crate) const BUFFER_RESET: i32 =
    ((1i64 << 31) - ALLOC_HISTORY as i64 - MAX_STORE_BLOCK_SIZE as i64 - 1) as i32;

// fastEncL1 to fastEncL6 provides specialized encoders for levels 1-6
// that each provide a different speed/size/memory strategies.
//
// Level 1: Single small table, 5 byte hashes, sparse indexing.
// Level 2: Single big table, 5 byte hashes, indexing ~ every 2 bytes.
// Level 3: Single medium table, 5 byte hashes, 2 candidates per table entry.
// Level 4: Two tables, 4/7 byte hashes, 1 candidate per table entry.
// Level 5: Two tables, 4/7 byte hashes, 2 candidates per 7-byte table entry.
// Level 6: Two tables, 4/7 byte hashes, full indexing, checks for repeats.
//
// Skipping on contiguous non-matches also decreases as levels go up.

/// fastEnc is the interface implemented by the level 1-6 fast encoders.
/// (Go interface → Rust enum.)
pub(crate) enum FastEnc {
    L1(Box<FastEncL1>),
    L2(Box<FastEncL2>),
    L3(Box<FastEncL3>),
    L4(Box<FastEncL4>),
    L5(Box<FastEncL5>),
    L6(Box<FastEncL6>),
}

impl FastEnc {
    /// encode src into dst.
    pub(crate) fn encode(&mut self, dst: &mut Tokens, src: &[u8]) {
        match self {
            FastEnc::L1(e) => e.encode(dst, src),
            FastEnc::L2(e) => e.encode(dst, src),
            FastEnc::L3(e) => e.encode(dst, src),
            FastEnc::L4(e) => e.encode(dst, src),
            FastEnc::L5(e) => e.encode(dst, src),
            FastEnc::L6(e) => e.encode(dst, src),
        }
    }

    /// reset the encoder so matches are not made with previous data.
    pub(crate) fn reset(&mut self) {
        match self {
            FastEnc::L1(e) => e.fg.reset(),
            FastEnc::L2(e) => e.fg.reset(),
            FastEnc::L3(e) => e.fg.reset(),
            FastEnc::L4(e) => e.fg.reset(),
            FastEnc::L5(e) => e.fg.reset(),
            FastEnc::L6(e) => e.fg.reset(),
        }
    }
}

// Go: compress/flate/deflatefast.go:newFastEnc
/// newFastEnc returns a fastEnc encoder for the given compression level (1-6).
pub(crate) fn new_fast_enc(level: i32) -> FastEnc {
    let fg = FastGen {
        hist: Vec::new(),
        hist_cap: 0,
        cur: MAX_STORE_BLOCK_SIZE,
    };
    match level {
        1 => FastEnc::L1(Box::new(FastEncL1::new(fg))),
        2 => FastEnc::L2(Box::new(FastEncL2::new(fg))),
        3 => FastEnc::L3(Box::new(FastEncL3::new(fg))),
        4 => FastEnc::L4(Box::new(FastEncL4::new(fg))),
        5 => FastEnc::L5(Box::new(FastEncL5::new(fg))),
        6 => FastEnc::L6(Box::new(FastEncL6::new(fg))),
        _ => panic!("invalid level specified"),
    }
}

/// fastGen maintains the table for matches,
/// and the previous byte block for level 1 and up.
/// This is the generic implementation.
///
/// Deviation: Go's `hist` is a slice whose capacity drives `addBlock`;
/// `hist_cap` records that capacity (0 or allocHistory) exactly.
pub(crate) struct FastGen {
    pub(crate) hist: Vec<u8>,
    pub(crate) hist_cap: usize,
    pub(crate) cur: i32,
}

impl FastGen {
    // Go: compress/flate/deflatefast.go:(*fastGen).addBlock
    /// addBlock appends src to the history and returns the offset where src starts in e.hist.
    pub(crate) fn add_block(&mut self, src: &[u8]) -> i32 {
        // check if we have space already
        if self.hist.len() + src.len() > self.hist_cap {
            if self.hist_cap == 0 {
                self.hist = Vec::with_capacity(ALLOC_HISTORY);
                self.hist_cap = ALLOC_HISTORY;
            } else {
                if self.hist_cap < (MAX_MATCH_OFFSET * 2) as usize {
                    panic!("unexpected buffer size");
                }
                // Move down
                let offset = self.hist.len() as i32 - MAX_MATCH_OFFSET;
                self.hist
                    .copy_within(offset as usize..(offset + MAX_MATCH_OFFSET) as usize, 0);
                self.cur += offset;
                self.hist.truncate(MAX_MATCH_OFFSET as usize);
            }
        }
        let s = self.hist.len() as i32;
        self.hist.extend_from_slice(src);
        s
    }

    // Go: compress/flate/deflatefast.go:(*fastGen).reset
    /// reset resets the encoding table to prepare for a new compression stream.
    pub(crate) fn reset(&mut self) {
        if self.hist_cap < ALLOC_HISTORY {
            self.hist = Vec::with_capacity(ALLOC_HISTORY);
            self.hist_cap = ALLOC_HISTORY;
        }
        // We offset current position so everything will be out of reach.
        // If we are above the buffer reset it will be cleared anyway since len(hist) == 0.
        if self.cur <= BUFFER_RESET {
            self.cur += MAX_MATCH_OFFSET + self.hist.len() as i32;
        }
        self.hist.clear();
    }
}

// Go: compress/flate/deflatefast.go:(*fastGen).matchLenLimited
/// matchLenLimited returns the match length between offsets s and t in src.
/// The maximum length returned is maxMatchLength - 4.
/// It is assumed that s > t, that t >= 0 and s < len(src).
#[inline(always)]
pub(crate) fn match_len_limited(s: usize, t: usize, src: &[u8]) -> i32 {
    let a = &src[s..std::cmp::min(s + MAX_MATCH_LENGTH as usize - 4, src.len())];
    let b = &src[t..];
    match_len(a, b) as i32
}

// Go: compress/flate/deflatefast.go:(*fastGen).matchLenLong
/// matchLenLong returns the match length between offsets s and t in src.
/// It is assumed that s > t, that t >= 0 and s < len(src).
#[inline(always)]
pub(crate) fn match_len_long(s: usize, t: usize, src: &[u8]) -> i32 {
    match_len(&src[s..], &src[t..]) as i32
}

/// tableEntry stores the offset of a hash match in the input history.
/// (Go: `struct{ offset int32 }`.)
pub(crate) type TableEntry = i32;

/// tableEntryPrev stores the current and previous offsets for a hash entry.
#[derive(Clone, Copy, Default)]
pub(crate) struct TableEntryPrev {
    pub(crate) cur: TableEntry,
    pub(crate) prev: TableEntry,
}

const PRIME3BYTES: u32 = 506832829;
const PRIME4BYTES: u32 = 2654435761;
const PRIME5BYTES: u64 = 889523592379;
const PRIME6BYTES: u64 = 227718039650203;
const PRIME7BYTES: u64 = 58295818150454627;
const PRIME8BYTES: u64 = 0xcf1bbcdcb7a56463;

// Go: compress/flate/deflatefast.go:hashLen
/// hashLen returns a hash of the first n bytes of u, using b output bits.
/// It expects 3 <= n <= 8; other values are treated as n == 4.
/// The bit length b must be <= 32.
/// b and n should be constants in speed-critical use.
#[inline(always)]
pub(crate) fn hash_len(u: u64, b: u8, n: u8) -> u32 {
    match n {
        3 => ((u << 8) as u32).wrapping_mul(PRIME3BYTES) >> (32 - b),
        5 => ((u << (64 - 40)).wrapping_mul(PRIME5BYTES) >> (64 - b)) as u32,
        6 => ((u << (64 - 48)).wrapping_mul(PRIME6BYTES) >> (64 - b)) as u32,
        7 => ((u << (64 - 56)).wrapping_mul(PRIME7BYTES) >> (64 - b)) as u32,
        8 => (u.wrapping_mul(PRIME8BYTES) >> (64 - b)) as u32,
        _ => (u as u32).wrapping_mul(PRIME4BYTES) >> (32 - b),
    }
}

// Go: compress/flate/deflatefast.go:matchLen
/// matchLen returns the maximum common prefix length of a and b.
/// a must be the shortest of the two.
#[inline(always)]
pub(crate) fn match_len(a: &[u8], b: &[u8]) -> usize {
    let mut n = 0usize;
    let mut left = a.len();
    while left >= 8 {
        let diff = load_le64(a, n) ^ load_le64(b, n);
        if diff != 0 {
            return n + (diff.trailing_zeros() as usize >> 3);
        }
        n += 8;
        left -= 8;
    }

    let a = &a[n..];
    let b = &b[n..];
    let b = &b[..a.len()];
    for i in 0..a.len() {
        if a[i] != b[i] {
            break;
        }
        n += 1;
    }
    n
}

// Go: compress/flate/load_store.go:loadLE8
#[inline(always)]
pub(crate) fn load_le8(b: &[u8], i: i32) -> u8 {
    b[i as usize]
}

// Go: compress/flate/load_store.go:loadLE32
/// loadLE32 will load from b at index i.
#[inline(always)]
pub(crate) fn load_le32<I: Idx>(b: &[u8], i: I) -> u32 {
    let i = i.idx();
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}

// Go: compress/flate/load_store.go:loadLE64
/// loadLE64 will load from b at index i.
#[inline(always)]
pub(crate) fn load_le64<I: Idx>(b: &[u8], i: I) -> u64 {
    let i = i.idx();
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}

// Go: compress/flate/load_store.go:storeLE64
/// storeLE64 will store v at start of b.
#[inline(always)]
pub(crate) fn store_le64(b: &mut [u8], v: u64) {
    b[..8].copy_from_slice(&v.to_le_bytes());
}

/// Go's generic `indexer` constraint for load helpers. A negative index
/// panics (Go panics with an index-out-of-range error).
pub(crate) trait Idx: Copy {
    fn idx(self) -> usize;
}

impl Idx for i32 {
    #[inline(always)]
    fn idx(self) -> usize {
        usize::try_from(self).expect("index out of range")
    }
}

impl Idx for usize {
    #[inline(always)]
    fn idx(self) -> usize {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flate::MAX_MATCH_LENGTH;

    // Go: compress/flate/deflate_test.go:TestBestSpeedMatch
    #[test]
    fn best_speed_match() {
        let z = |n: usize| vec![0u8; n];
        let mml = MAX_MATCH_LENGTH - 4;
        #[allow(clippy::type_complexity)]
        let cases: Vec<(Vec<u8>, Vec<u8>, i64, i64, i32)> = vec![
            (
                vec![0, 0, 0, 1, 2],
                vec![3, 4, 5, 0, 1, 2, 3, 4, 5],
                -3,
                3,
                6,
            ),
            (
                vec![0, 0, 0, 1, 2],
                vec![2, 4, 5, 0, 1, 2, 3, 4, 5],
                -3,
                3,
                3,
            ),
            (
                vec![0, 0, 0, 1, 1],
                vec![3, 4, 5, 0, 1, 2, 3, 4, 5],
                -3,
                3,
                2,
            ),
            (
                vec![0, 0, 0, 1, 2],
                vec![2, 2, 2, 2, 1, 2, 3, 4, 5],
                -1,
                0,
                4,
            ),
            (
                vec![0, 0, 0, 1, 2, 3, 4, 5, 2, 2],
                vec![2, 2, 2, 2, 1, 2, 3, 4, 5],
                -7,
                4,
                5,
            ),
            (
                vec![9, 9, 9, 9, 9],
                vec![2, 2, 2, 2, 1, 2, 3, 4, 5],
                -1,
                0,
                0,
            ),
            (
                vec![9, 9, 9, 9, 9],
                vec![9, 2, 2, 2, 1, 2, 3, 4, 5],
                0,
                1,
                0,
            ),
            (vec![], vec![2, 2, 2, 2, 1, 2, 3, 4, 5], 0, 1, 3),
            (vec![3, 4, 5], vec![3, 4, 5], -3, 0, 3),
            (z(1000), z(1000), -1000, 0, mml),
            (z(200), z(500), -200, 0, mml),
            (z(200), z(500), 0, 1, mml),
            (z(mml as usize), z(500), -(mml as i64), 0, mml),
            (z(200), z(500), -200, 400, 100),
            (z(10), z(500), 200, 400, 100),
        ];
        for (i, (previous, current, t, s, want)) in cases.into_iter().enumerate() {
            let mut e = FastGen {
                hist: Vec::new(),
                hist_cap: 0,
                cur: 0,
            };
            e.add_block(&previous);
            e.add_block(&current);
            let got = match_len_limited(
                (s + previous.len() as i64) as usize,
                (t + previous.len() as i64) as usize,
                &e.hist,
            );
            assert_eq!(got, want, "Test {i}: match length");
        }
    }

    // Go: compress/flate/deflate_test.go:TestBestSpeedShiftOffsets
    // (Go draws testData from math/rand; any input without internal matches
    // exercises the same property.)
    #[test]
    fn best_speed_shift_offsets() {
        for level in 1..=6 {
            let mut enc = new_fast_enc(level);
            // testData may not generate internal matches.
            let mut x: u64 = 0x9e3779b97f4a7c15;
            let test_data: Vec<u8> = (0..100)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    x as u8
                })
                .collect();
            let val_or_len = |val: u16| -> u16 {
                if val == 0 {
                    test_data.len() as u16
                } else {
                    val
                }
            };
            // Encode the testdata with clean state.
            // Second part should pick up matches from the first block.
            let mut first_tokens = Tokens::new();
            let mut second_tokens = Tokens::new();
            enc.encode(&mut first_tokens, &test_data);
            enc.encode(&mut second_tokens, &test_data);
            let want_first = val_or_len(first_tokens.n);
            let want_second = val_or_len(second_tokens.n);
            assert!(
                want_first > want_second,
                "level {level}: test needs matches between inputs to be generated, {want_first} == {want_second}"
            );

            fn fg(enc: &mut FastEnc) -> &mut FastGen {
                match enc {
                    FastEnc::L1(e) => &mut e.fg,
                    FastEnc::L2(e) => &mut e.fg,
                    FastEnc::L3(e) => &mut e.fg,
                    FastEnc::L4(e) => &mut e.fg,
                    FastEnc::L5(e) => &mut e.fg,
                    FastEnc::L6(e) => &mut e.fg,
                }
            }
            // Forward the current indicator to before wraparound.
            {
                let g = fg(&mut enc);
                g.hist = Vec::new();
                g.hist_cap = 0;
                g.cur = BUFFER_RESET - test_data.len() as i32;
            }

            // Part 1 before wrap, should match clean state.
            let mut got_tokens = Tokens::new();
            enc.encode(&mut got_tokens, &test_data);
            assert_eq!(val_or_len(got_tokens.n), want_first, "level {level}");

            // Verify we are about to wrap.
            {
                let g = fg(&mut enc);
                assert_eq!(g.cur + g.hist.len() as i32, BUFFER_RESET, "level {level}");
            }

            // Part 2 should match clean state as well even if wrapped.
            got_tokens.reset();
            enc.encode(&mut got_tokens, &test_data);
            assert_eq!(val_or_len(got_tokens.n), want_second, "level {level}");

            // Verify that we wrapped.
            assert!(fg(&mut enc).cur < BUFFER_RESET, "level {level}");

            // Forward the current buffer, leaving the matches at the bottom.
            {
                let g = fg(&mut enc);
                g.cur = BUFFER_RESET;
                g.hist = Vec::new();
                g.hist_cap = 0;
            }

            // Ensure that no matches were picked up.
            got_tokens.reset();
            enc.encode(&mut got_tokens, &test_data);
            assert_eq!(val_or_len(got_tokens.n), want_first, "level {level}");
        }
    }
}
