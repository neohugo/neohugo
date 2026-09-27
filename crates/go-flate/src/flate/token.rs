//! Port of go1.27.1 `compress/flate/token.go`.

use super::huffman_bit_writer::{LENGTH_EXTRA_BITS, OFFSET_CODE_COUNT, OFFSET_EXTRA_BITS};
use super::huffman_code::LITERAL_COUNT;
use super::{BASE_MATCH_LENGTH, END_BLOCK_MARKER};

// Token is a compound value:
// bits 0-16  xoffset = offset - MIN_OFFSET_SIZE, or literal - 16 bits
// bits 16-22 offset code - 5 bits
// bits 22-30 xlength = length - MIN_MATCH_LENGTH - 8 bits
// bits 30-32 type, 0 = literal  1=EOF  2=Match   3=Unused - 2 bits
pub(crate) const LENGTH_SHIFT: u32 = 22;
pub(crate) const OFFSET_MASK: u32 = (1 << LENGTH_SHIFT) - 1;
#[allow(dead_code)]
pub(crate) const TYPE_MASK: u32 = 3 << 30;
pub(crate) const MATCH_TYPE: u32 = 1 << 30;
pub(crate) const MATCH_OFFSET_ONLY_MASK: u32 = 0xffff;

// The length code for length X (MIN_MATCH_LENGTH <= X <= MAX_MATCH_LENGTH)
// is lengthCodes[length - MIN_MATCH_LENGTH]
pub(crate) static LENGTH_CODES: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 12, 12, 13, 13, 13, 13, 14, 14, 14,
    14, 15, 15, 15, 15, 16, 16, 16, 16, 16, 16, 16, 16, 17, 17, 17, 17, 17, 17, 17, 17, 18, 18, 18,
    18, 18, 18, 18, 18, 19, 19, 19, 19, 19, 19, 19, 19, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20,
    20, 20, 20, 20, 20, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25,
    25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 26, 26, 26,
    26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
    26, 26, 26, 26, 26, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 28,
];

// lengthCodes1 is length codes, but starting at 1.
pub(crate) static LENGTH_CODES1: [u8; 256] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 13, 13, 14, 14, 14, 14, 15, 15,
    15, 15, 16, 16, 16, 16, 17, 17, 17, 17, 17, 17, 17, 17, 18, 18, 18, 18, 18, 18, 18, 18, 19, 19,
    19, 19, 19, 19, 19, 19, 20, 20, 20, 20, 20, 20, 20, 20, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25,
    25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
    26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 29,
];

pub(crate) static OFFSET_CODES: [u32; 256] = [
    0, 1, 2, 3, 4, 4, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
    14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
    14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15,
];

// offsetCodes14 are offsetCodes, but with 14 added.
pub(crate) static OFFSET_CODES14: [u32; 256] = [
    14, 15, 16, 17, 18, 18, 19, 19, 20, 20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22,
    23, 23, 23, 23, 23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25, 26, 26, 26, 26, 26, 26, 26, 26,
    26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29,
    29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29, 29,
];

/// A token is a token that will be written to output stream.
/// It is either a literal or a match with offset and length.
pub(crate) type Token = u32;

/// tokens are compound values as described above.
/// Histograms are created as tokens are added.
/// A full block is allocated.
pub(crate) struct Tokens {
    pub(crate) extra_hist: [u16; 32], // codes 256->maxnumlit
    pub(crate) off_hist: [u16; 32],   // offset codes
    pub(crate) lit_hist: [u16; 256],  // codes 0->255
    pub(crate) n_filled: i64,
    pub(crate) n: u16, // Must be able to contain maxStoreBlockSize
    pub(crate) tokens: Box<[Token; 65536]>,
}

impl Default for Tokens {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokens {
    pub(crate) fn new() -> Tokens {
        Tokens {
            extra_hist: [0; 32],
            off_hist: [0; 32],
            lit_hist: [0; 256],
            n_filled: 0,
            n: 0,
            tokens: vec![0u32; 65536]
                .into_boxed_slice()
                .try_into()
                .expect("65536 tokens"),
        }
    }

    // Go: compress/flate/token.go:(*tokens).Reset
    /// Reset resets the tokens and histograms.
    pub(crate) fn reset(&mut self) {
        if self.n == 0 {
            return;
        }
        self.n = 0;
        self.n_filled = 0;
        self.lit_hist = [0; 256];
        self.extra_hist = [0; 32];
        self.off_hist = [0; 32];
    }

    // Go: compress/flate/token.go:indexTokens
    /// indexTokens creates tokens from a slice of unindexed tokens.
    #[allow(dead_code)]
    pub(crate) fn from_unindexed(input: &[Token]) -> Tokens {
        let mut t = Tokens::new();
        t.index_tokens(input);
        t
    }

    // Go: compress/flate/token.go:(*tokens).indexTokens
    /// indexTokens clears and sets t from a slice of unindexed tokens.
    #[allow(dead_code)]
    pub(crate) fn index_tokens(&mut self, input: &[Token]) {
        self.reset();
        for &tok in input {
            if tok < MATCH_TYPE {
                self.add_literal(token_literal(tok));
                continue;
            }
            self.add_match(
                token_length(tok) as u32,
                token_offset(tok) & MATCH_OFFSET_ONLY_MASK,
            );
        }
    }

    // Go: compress/flate/token.go:(*tokens).AddLiteral
    /// AddLiteral adds a single literal to the tokens.
    #[inline(always)]
    pub(crate) fn add_literal(&mut self, lit: u8) {
        self.tokens[self.n as usize] = lit as Token;
        self.lit_hist[lit as usize] = self.lit_hist[lit as usize].wrapping_add(1);
        self.n = self.n.wrapping_add(1);
    }

    // Go: compress/flate/token.go:(*tokens).EstimatedBits
    /// EstimatedBits returns an estimated minimum size for the
    /// optimal compression of t.
    /// Minimum 1 bit is assigned per symbol.
    /// Maximum 15 bits are assigned per symbol.
    ///
    /// FMA (arm64, verified with `go tool objdump`): each
    /// `shannon += min(15, max(1, -mFastLog2(n*invTotal))) * n` is a single
    /// FMADDS; `shannon += 15` is a plain FADDS.
    pub(crate) fn estimated_bits(&self) -> i64 {
        let mut shannon = 0f32;
        let mut bits: i64 = 0;
        let mut n_matches: i64 = 0;
        let total = self.n as i64 + self.n_filled;
        if total > 0 {
            let inv_total = 1.0f32 / total as f32;
            for &v in self.lit_hist.iter() {
                if v > 0 {
                    let n = v as f32;
                    shannon = (-m_fast_log2(n * inv_total))
                        .max(1.0)
                        .min(15.0)
                        .mul_add(n, shannon);
                }
            }
            // Just add 15 for EOB
            shannon += 15.0;
            for (i, &v) in self.extra_hist[1..LITERAL_COUNT - 256].iter().enumerate() {
                if v > 0 {
                    let n = v as f32;
                    shannon = (-m_fast_log2(n * inv_total))
                        .max(1.0)
                        .min(15.0)
                        .mul_add(n, shannon);
                    bits += LENGTH_EXTRA_BITS[i & 31] as i64 * v as i64;
                    n_matches += v as i64;
                }
            }
        }
        if n_matches > 0 {
            let inv_total = 1.0f32 / n_matches as f32;
            for (i, &v) in self.off_hist[..OFFSET_CODE_COUNT].iter().enumerate() {
                if v > 0 {
                    let n = v as f32;
                    shannon = (-m_fast_log2(n * inv_total))
                        .max(1.0)
                        .min(15.0)
                        .mul_add(n, shannon);
                    bits += OFFSET_EXTRA_BITS[i & 31] as i64 * v as i64;
                }
            }
        }
        // Go int(float32): FCVTZS, truncating and saturating like Rust `as`.
        (shannon as i64).wrapping_add(bits)
    }

    // Go: compress/flate/token.go:(*tokens).AddMatch
    /// AddMatch adds a match to the tokens.
    #[inline(always)]
    pub(crate) fn add_match(&mut self, xlength: u32, mut xoffset: u32) {
        let o_code = offset_code(xoffset);
        xoffset |= o_code << 16;

        let lc = LENGTH_CODES1[xlength as u8 as usize] as usize;
        self.extra_hist[lc] = self.extra_hist[lc].wrapping_add(1);
        let oc = (o_code & 31) as usize;
        self.off_hist[oc] = self.off_hist[oc].wrapping_add(1);
        self.tokens[self.n as usize] = MATCH_TYPE | xlength << LENGTH_SHIFT | xoffset;
        self.n = self.n.wrapping_add(1);
    }

    // Go: compress/flate/token.go:(*tokens).AddMatchLong
    /// AddMatchLong adds a match to the tokens, potentially longer than max match length.
    /// Length should NOT have the base subtracted, only offset should.
    #[inline(always)]
    pub(crate) fn add_match_long(&mut self, mut xlength: i32, mut xoffset: u32) {
        let oc = offset_code(xoffset);
        xoffset |= oc << 16;
        while xlength > 0 {
            let mut xl = xlength;
            if xl > 258 {
                // We need to have at least baseMatchLength left over for next loop.
                if xl > 258 + BASE_MATCH_LENGTH {
                    xl = 258;
                } else {
                    xl = 258 - BASE_MATCH_LENGTH;
                }
            }
            xlength -= xl;
            xl -= BASE_MATCH_LENGTH;
            let lc = LENGTH_CODES1[xl as u8 as usize] as usize;
            self.extra_hist[lc] = self.extra_hist[lc].wrapping_add(1);
            let o = (oc & 31) as usize;
            self.off_hist[o] = self.off_hist[o].wrapping_add(1);
            self.tokens[self.n as usize] = MATCH_TYPE | (xl as u32) << LENGTH_SHIFT | xoffset;
            self.n = self.n.wrapping_add(1);
        }
    }

    // Go: compress/flate/token.go:(*tokens).AddEOB
    /// AddEOB adds an end of block marker to the tokens.
    #[inline(always)]
    pub(crate) fn add_eob(&mut self) {
        self.tokens[self.n as usize] = END_BLOCK_MARKER as Token;
        self.extra_hist[0] = self.extra_hist[0].wrapping_add(1);
        self.n = self.n.wrapping_add(1);
    }

    // Go: compress/flate/token.go:(*tokens).Slice
    /// Slice returns a slice of the tokens that references the tokens in t.
    #[inline(always)]
    pub(crate) fn slice(&self) -> &[Token] {
        &self.tokens[..self.n as usize]
    }
}

// Go: compress/flate/token.go:emitLiterals
/// emitLiterals writes a literal chunk and returns the number of bytes written.
#[inline(always)]
pub(crate) fn emit_literals(dst: &mut Tokens, lit: &[u8]) {
    for &v in lit {
        dst.tokens[dst.n as usize] = v as Token;
        dst.lit_hist[v as usize] = dst.lit_hist[v as usize].wrapping_add(1);
        dst.n = dst.n.wrapping_add(1);
    }
}

// Go: compress/flate/token.go:mFastLog2
/// mFastLog2 returns a fast approximation of log2(val).
/// From https://stackoverflow.com/a/28730362.
///
/// FMA (arm64, verified with `go tool objdump`):
/// `((-0.34484843)*uval+2.02466578)` is FMADDS, `(...)*uval - 0.67487759` is
/// FNMSUBS (= fused `x*y - z`), and the final `log2 += ...` is a plain FADDS.
#[inline(always)]
pub(crate) fn m_fast_log2(val: f32) -> f32 {
    let mut ux = val.to_bits() as i32;
    let mut log2 = (((ux >> 23) & 255) - 128) as f32;
    ux &= -0x7f800001;
    ux = ux.wrapping_add(127 << 23);
    let uval = f32::from_bits(ux as u32);
    // Constants spelled exactly as in Go so they round to the same float32.
    let t = (-0.34484843_f32).mul_add(uval, 2.02466578_f32);
    let t = t.mul_add(uval, -0.67487759_f32);
    log2 += t;
    log2
}

// Go: compress/flate/token.go:(token).typ
#[allow(dead_code)]
#[inline(always)]
pub(crate) fn token_typ(t: Token) -> u32 {
    t & TYPE_MASK
}

// Go: compress/flate/token.go:(token).literal
#[inline(always)]
pub(crate) fn token_literal(t: Token) -> u8 {
    t as u8
}

// Go: compress/flate/token.go:(token).offset
#[inline(always)]
pub(crate) fn token_offset(t: Token) -> u32 {
    t & OFFSET_MASK
}

// Go: compress/flate/token.go:(token).length
#[inline(always)]
pub(crate) fn token_length(t: Token) -> u8 {
    (t >> LENGTH_SHIFT) as u8
}

// Go: compress/flate/token.go:lengthCode
#[inline(always)]
pub(crate) fn length_code(len: u8) -> u8 {
    LENGTH_CODES[len as usize]
}

// Go: compress/flate/token.go:offsetCode
/// offsetCode returns the offset code corresponding to a specific offset.
#[inline(always)]
pub(crate) fn offset_code(off: u32) -> u32 {
    if off < OFFSET_CODES.len() as u32 {
        return OFFSET_CODES[off as u8 as usize];
    }
    OFFSET_CODES14[(off >> 7) as u8 as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The float32 constants of mFastLog2 as stored in the Go binary
    /// (`$f32.beb08ff9`, `$f32.40019420`, `$f32.3f2cc4c7`).
    #[test]
    fn m_fast_log2_constants() {
        assert_eq!((-0.34484843_f32).to_bits(), 0xbeb08ff9);
        assert_eq!(2.02466578_f32.to_bits(), 0x40019420);
        assert_eq!(0.67487759_f32.to_bits(), 0x3f2cc4c7);
    }

    #[test]
    fn m_fast_log2_values() {
        // Spot values (fused evaluation); log2(1) ~ 0, log2(0.5) ~ -1.
        assert!((m_fast_log2(1.0)).abs() < 0.01);
        assert!((m_fast_log2(0.5) + 1.0).abs() < 0.01);
    }
}
