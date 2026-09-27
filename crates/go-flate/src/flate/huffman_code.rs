//! Port of go1.27.1 `compress/flate/huffman_code.go`.

use std::sync::OnceLock;

pub(crate) const MAX_BITS_LIMIT: usize = 16;
/// number of valid literals
pub(crate) const LITERAL_COUNT: usize = 286;

/// hcode is a huffman code with a bit code and bit length.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Hcode(pub(crate) u32);

impl Hcode {
    // Go: compress/flate/huffman_code.go:(hcode).len
    /// len returns the length of the code in bits.
    #[inline(always)]
    pub(crate) fn len(self) -> u8 {
        self.0 as u8
    }

    // Go: compress/flate/huffman_code.go:(hcode).code64
    /// code64 returns the code as a uint64.
    #[inline(always)]
    pub(crate) fn code64(self) -> u64 {
        (self.0 >> 8) as u64
    }

    // Go: compress/flate/huffman_code.go:(hcode).zero
    /// zero returns true if the code is unset.
    #[inline(always)]
    pub(crate) fn zero(self) -> bool {
        self.0 == 0
    }

    // Go: compress/flate/huffman_code.go:(*hcode).set
    /// set sets the code and length of an hcode.
    #[inline(always)]
    pub(crate) fn set(&mut self, code: u16, length: u8) {
        *self = newhcode(code, length);
    }
}

// Go: compress/flate/huffman_code.go:newhcode
/// newhcode combines a code and length into an hcode.
#[inline(always)]
pub(crate) fn newhcode(code: u16, length: u8) -> Hcode {
    Hcode(length as u32 | (code as u32) << 8)
}

/// literalNode represents a literal node in the huffman tree.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct LiteralNode {
    literal: u16,
    freq: u16,
}

// Go: compress/flate/huffman_code.go:maxNode
/// maxNode returns a literalNode with the maximum possible literal and frequency.
fn max_node() -> LiteralNode {
    LiteralNode {
        literal: u16::MAX,
        freq: u16::MAX,
    }
}

/// A levelInfo describes the state of the constructed tree for a given depth.
#[derive(Clone, Copy, Default)]
struct LevelInfo {
    // Our level.  for better printing
    level: i32,
    // The frequency of the last node at this level
    last_freq: i32,
    // The frequency of the next character to add to this level
    next_char_freq: i32,
    // The frequency of the next pair (from level below) to add to this level.
    // Only valid if the "needed" value of the next lower level is 0.
    next_pair_freq: i32,
    // The number of chains remaining to generate for this level before moving
    // up to the next level
    needed: i32,
}

/// huffmanEncoder provides a fast way to generate Huffman codes for a given
/// frequency table.  It is based on the algorithm described in RFC 1951,
/// section 3.2.2.
///
/// Deviation: Go's `codes` slice has `len == size` and `cap == next power of
/// two`; Go code reads/writes past `len` (within `cap`) through reslicing
/// (`h.codes[:len(freq)]`, `lenCodes[lengthCodesStart:][:32]`). Here `codes`
/// has length `cap` and `size` records Go's `len`.
#[derive(Clone)]
pub(crate) struct HuffmanEncoder {
    pub(crate) codes: Vec<Hcode>,
    #[allow(dead_code)]
    pub(crate) size: usize,
    bit_count: [i32; 17],
    // freqcache is a reusable buffer with the longest possible frequency table.
    // Possible lengths are codegenCodeCount, offsetCodeCount and literalCount.
    // The largest of these is literalCount, so we allocate for that case.
    freqcache: Box<[LiteralNode; LITERAL_COUNT + 1]>,
}

// Go: compress/flate/huffman_code.go:newHuffmanEncoder
/// newHuffmanEncoder returns a new huffmanEncoder with the given size.
pub(crate) fn new_huffman_encoder(size: usize) -> HuffmanEncoder {
    // Make capacity to next power of two.
    let c = 32 - ((size - 1) as u32).leading_zeros();
    HuffmanEncoder {
        codes: vec![Hcode(0); 1usize << c],
        size,
        bit_count: [0; 17],
        freqcache: Box::new([LiteralNode::default(); LITERAL_COUNT + 1]),
    }
}

// Go: compress/flate/huffman_code.go:reverseBits
/// reverseBits returns the b-bit reversal of x.
/// It shifts x into the top b bits, reverses all 16, leaving the result in the low b bits.
#[inline(always)]
pub(crate) fn reverse_bits(x: u16, b: u8) -> u16 {
    (x << ((16u8.wrapping_sub(b)) & 15)).reverse_bits()
}

// Go: compress/flate/huffman_code.go:generateFixedLiteralEncoding
/// generateFixedLiteralEncoding returns the encoder for the fixed literal table.
fn generate_fixed_literal_encoding() -> HuffmanEncoder {
    let mut h = new_huffman_encoder(LITERAL_COUNT);
    let codes = &mut h.codes;
    for ch in 0..LITERAL_COUNT as u16 {
        let bits: u16;
        let size: u8;
        if ch < 144 {
            // size 8, 000110000  .. 10111111
            bits = ch + 48;
            size = 8;
        } else if ch < 256 {
            // size 9, 110010000 .. 111111111
            bits = ch + 400 - 144;
            size = 9;
        } else if ch < 280 {
            // size 7, 0000000 .. 0010111
            bits = ch - 256;
            size = 7;
        } else {
            // size 8, 11000000 .. 11000111
            bits = ch + 192 - 280;
            size = 8;
        }
        codes[ch as usize] = newhcode(reverse_bits(bits, size), size);
    }
    h
}

// Go: compress/flate/huffman_code.go:generateFixedOffsetEncoding
fn generate_fixed_offset_encoding() -> HuffmanEncoder {
    let mut h = new_huffman_encoder(30);
    for ch in 0..30usize {
        h.codes[ch] = newhcode(reverse_bits(ch as u16, 5), 5);
    }
    h
}

// Go: compress/flate/huffman_code.go:fixedLiteralEncoding (sync.OnceValue)
pub(crate) fn fixed_literal_encoding() -> &'static HuffmanEncoder {
    static ENC: OnceLock<HuffmanEncoder> = OnceLock::new();
    ENC.get_or_init(generate_fixed_literal_encoding)
}

// Go: compress/flate/huffman_code.go:fixedOffsetEncoding (sync.OnceValue)
pub(crate) fn fixed_offset_encoding() -> &'static HuffmanEncoder {
    static ENC: OnceLock<HuffmanEncoder> = OnceLock::new();
    ENC.get_or_init(generate_fixed_offset_encoding)
}

impl HuffmanEncoder {
    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).bitLength
    /// bitLength returns the number of bits needed to encode freq.
    pub(crate) fn bit_length(&self, freq: &[u16]) -> i64 {
        let mut total: i64 = 0;
        for (i, &f) in freq.iter().enumerate() {
            if f != 0 {
                total += f as i64 * self.codes[i].len() as i64;
            }
        }
        total
    }

    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).bitLengthRaw
    /// bitLengthRaw will return the number of bits needed to encode b.
    /// For unset codes 1 bit/entry will be added.
    pub(crate) fn bit_length_raw(&self, b: &[u8]) -> i64 {
        let mut total: i64 = 0;
        for &f in b {
            total += std::cmp::max(1, self.codes[f as usize].len() as i64);
        }
        total
    }

    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).canEncodeLen
    /// canEncodeLen returns the number of bits to encode freq.
    /// It returns math.MaxInt32 if freq cannot be encoded.
    pub(crate) fn can_encode_len(&self, freq: &[u16]) -> i64 {
        let mut total: i64 = 0;
        for (i, &f) in freq.iter().enumerate() {
            if f != 0 {
                let code = self.codes[i];
                if code.zero() {
                    return i32::MAX as i64;
                }
                total += f as i64 * code.len() as i64;
            }
        }
        total
    }

    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).bitCounts
    /// bitCounts returns an integer slice in which slice[i] is the number
    /// of literals that should be encoded using i bits.
    ///
    /// This method is only called when len(list) >= 3.
    /// The cases of 0, 1, and 2 literals are handled by special case code.
    ///
    /// list is an array of the literals with non-zero frequencies
    /// and their associated frequencies. The array is in order of increasing
    /// frequency and has as its last element a special element with frequency
    /// MaxInt32.
    ///
    /// maxBits is the maximum number of bits that should be used to encode any literal.
    /// It must be less than 16.
    ///
    /// `n` is `len(list)`; `self.freqcache[n]` is overwritten with maxNode()
    /// exactly like Go's `list = list[0 : n+1]` reslice.
    /// Returns the number of valid entries of `self.bit_count` (maxBits+1).
    fn bit_counts(&mut self, n: usize, mut max_bits: i32) -> usize {
        if max_bits >= MAX_BITS_LIMIT as i32 {
            panic!("flate: maxBits too large");
        }
        let list = &mut self.freqcache[..];
        let n32 = n as i32;
        list[n] = max_node();

        // The tree can't have greater depth than n - 1, no matter what. This
        // saves a little bit of work in some small cases
        if max_bits > n32 - 1 {
            max_bits = n32 - 1;
        }

        // Create information about each of the levels.
        // A bogus "Level 0" whose sole purpose is so that
        // level1.prev.needed==0.  This makes level1.nextPairFreq
        // be a legitimate value that never gets chosen.
        let mut levels = [LevelInfo::default(); MAX_BITS_LIMIT];
        // leafCounts[i] counts the number of literals at the left
        // of ancestors of the rightmost node at level i.
        // leafCounts[i][j] is the number of literals at the left
        // of the level j ancestor.
        let mut leaf_counts = [[0i32; MAX_BITS_LIMIT]; MAX_BITS_LIMIT];

        let _ = list[2]; // check bounds here instead of in loop
        for level in 1..=max_bits {
            // For every level, the first two items are the first two characters.
            // We initialize the levels as if we had already figured this out.
            let lv = level as usize;
            levels[lv] = LevelInfo {
                level,
                last_freq: list[1].freq as i32,
                next_char_freq: list[2].freq as i32,
                next_pair_freq: list[0].freq as i32 + list[1].freq as i32,
                needed: 0,
            };
            leaf_counts[lv][lv] = 2;
            if level == 1 {
                levels[lv].next_pair_freq = i32::MAX;
            }
        }

        // We need a total of 2*n - 2 items at top level and have already generated 2.
        levels[max_bits as usize].needed = 2 * n32 - 4;

        let mut level = max_bits as u32;
        while level < 16 {
            let lvl = level as usize;
            if levels[lvl].next_pair_freq == i32::MAX && levels[lvl].next_char_freq == i32::MAX {
                // We've run out of both leafs and pairs.
                // End all calculations for this level.
                // To make sure we never come back to this level or any lower level,
                // set nextPairFreq impossibly large.
                levels[lvl].needed = 0;
                levels[lvl + 1].next_pair_freq = i32::MAX;
                level += 1;
                continue;
            }

            let prev_freq = levels[lvl].last_freq;
            if levels[lvl].next_char_freq < levels[lvl].next_pair_freq {
                // The next item on this row is a leaf node.
                let nn = leaf_counts[lvl][lvl] + 1;
                levels[lvl].last_freq = levels[lvl].next_char_freq;
                // Lower leafCounts are the same of the previous node.
                leaf_counts[lvl][lvl] = nn;
                let e = list[nn as usize];
                if e.literal < u16::MAX {
                    levels[lvl].next_char_freq = e.freq as i32;
                } else {
                    levels[lvl].next_char_freq = i32::MAX;
                }
            } else {
                // The next item on this row is a pair from the previous row.
                // nextPairFreq isn't valid until we generate two
                // more values in the level below
                levels[lvl].last_freq = levels[lvl].next_pair_freq;
                // Take leaf counts from the lower level, except counts[level] remains the same.
                let save = leaf_counts[lvl][lvl];
                leaf_counts[lvl] = leaf_counts[lvl - 1];
                leaf_counts[lvl][lvl] = save;
                let below = (levels[lvl].level - 1) as usize;
                levels[below].needed = 2;
            }

            levels[lvl].needed -= 1;
            if levels[lvl].needed == 0 {
                // We've done everything we need to do for this level.
                // Continue calculating one level up. Fill in nextPairFreq
                // of that level with the sum of the two nodes we've just calculated on
                // this level.
                if levels[lvl].level == max_bits {
                    // All done!
                    break;
                }
                let up = (levels[lvl].level + 1) as usize;
                levels[up].next_pair_freq = prev_freq + levels[lvl].last_freq;
                level += 1;
            } else {
                // If we stole from below, move down temporarily to replenish it.
                while levels[(level - 1) as usize].needed > 0 {
                    level -= 1;
                }
            }
        }

        // Somethings is wrong if at the end, the top level is null or hasn't used
        // all of the leaves.
        if leaf_counts[max_bits as usize][max_bits as usize] != n32 {
            panic!("leafCounts[maxBits][maxBits] != n");
        }

        let bit_count = &mut self.bit_count[..(max_bits + 1) as usize];
        let mut bits = 1usize;
        let counts = &leaf_counts[max_bits as usize];
        let mut level = max_bits as usize;
        while level > 0 {
            // chain.leafCount gives the number of literals requiring at least "bits"
            // bits to encode.
            bit_count[bits] = counts[level] - counts[level - 1];
            bits += 1;
            level -= 1;
        }
        (max_bits + 1) as usize
    }

    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).assignEncodingAndSize
    /// assignEncodingAndSize assigns bit counts and encodings to the leaves
    /// as specified in RFC 1951 3.2.2.
    ///
    /// `bit_count` is `self.bit_count[..nbc]`, `list` is `self.freqcache[..list_len]`.
    fn assign_encoding_and_size(&mut self, nbc: usize, list_len: usize) {
        let mut code: u16 = 0;
        let mut list_end = list_len;
        for n in 0..nbc {
            let bits = self.bit_count[n];
            code <<= 1;
            if n == 0 || bits == 0 {
                continue;
            }
            // The literals list[len(list)-bits] .. list[len(list)-bits]
            // are encoded using "bits" bits, and get the values
            // code, code + 1, ....  The code values are
            // assigned in literal order (not frequency order).
            let start = list_end - bits as usize;
            let chunk = &mut self.freqcache[start..list_end];
            // Go: slices.SortFunc by literal. Literals are unique, so any
            // correct sort gives Go's order.
            chunk.sort_unstable_by_key(|a| a.literal);
            for i in start..list_end {
                let node = self.freqcache[i];
                self.codes[node.literal as usize] = newhcode(reverse_bits(code, n as u8), n as u8);
                code = code.wrapping_add(1);
            }
            list_end = start;
        }
    }

    // Go: compress/flate/huffman_code.go:(*huffmanEncoder).generate
    /// generate rewrites h to be the Huffman code for the given frequency count.
    /// freq[i] is the frequency of literal i, and maxBits is the maximum number
    /// of bits to use for any literal.
    pub(crate) fn generate(&mut self, freq: &[u16], max_bits: i32) {
        // list := h.freqcache[:len(freq)+1]
        let _ = &self.freqcache[..freq.len() + 1];
        // codes := h.codes[:len(freq)]
        let codes = &mut self.codes[..freq.len()];
        // Number of non-zero literals
        let mut count = 0usize;
        // Set list to be the set of all non-zero literals and their frequencies
        for (i, &f) in freq.iter().enumerate() {
            if f != 0 {
                self.freqcache[count] = LiteralNode {
                    literal: i as u16,
                    freq: f,
                };
                count += 1;
            } else {
                codes[i] = Hcode(0);
            }
        }
        self.freqcache[count] = LiteralNode::default();

        if count <= 2 {
            // Handle the small cases here, because they are awkward for the general case code. With
            // two or fewer literals, everything has bit length 1.
            for i in 0..count {
                let node = self.freqcache[i];
                // "list" is in order of increasing literal value.
                self.codes[node.literal as usize].set(i as u16, 1);
            }
            return;
        }
        // Go: slices.SortFunc with key freq<<10 + literal; keys are unique
        // (literals are unique), so any correct sort gives Go's order.
        self.freqcache[..count]
            .sort_unstable_by_key(|a| ((a.freq as i64) << 10) + a.literal as i64);

        // Get the number of literals for each bit count
        let nbc = self.bit_counts(count, max_bits);
        // And do the assignment
        self.assign_encoding_and_size(nbc, count);
    }
}

// Go: compress/flate/huffman_code.go:histogram
pub(crate) fn histogram(b: &[u8], h: &mut [u16]) {
    if b.len() >= 8 << 10 {
        histogram_split(b, h);
        return;
    }
    let h = &mut h[..256];
    for &t in b {
        h[t as usize] = h[t as usize].wrapping_add(1);
    }
}

// Go: compress/flate/huffman_code.go:histogramSplit
fn histogram_split(mut b: &[u8], h: &mut [u16]) {
    // Walk four quarters in parallel.
    // Tested to be faster than walking halves.
    let h = &mut h[..256];
    // Make size divisible by 4
    while b.len() & 3 != 0 {
        h[b[0] as usize] = h[b[0] as usize].wrapping_add(1);
        b = &b[1..];
    }
    let n = b.len() / 4;
    let (x, y, z, w) = (&b[..n], &b[n..], &b[n + n..], &b[n + n + n..]);
    let (y, z, w) = (&y[..x.len()], &z[..x.len()], &w[..x.len()]);
    for (i, &t) in x.iter().enumerate() {
        h[t as usize] = h[t as usize].wrapping_add(1);
        h[y[i] as usize] = h[y[i] as usize].wrapping_add(1);
        h[z[i] as usize] = h[z[i] as usize].wrapping_add(1);
        h[w[i] as usize] = h[w[i] as usize].wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: compress/flate/deflate_test.go:reverseBitsTests
    #[test]
    fn test_reverse_bits() {
        let tests: &[(u16, u8, u16)] = &[
            (1, 1, 1),
            (1, 2, 2),
            (1, 3, 4),
            (1, 4, 8),
            (1, 5, 16),
            (17, 5, 17),
            (257, 9, 257),
            (29, 5, 23),
        ];
        for &(input, bit_count, out) in tests {
            assert_eq!(reverse_bits(input, bit_count), out, "{input} {bit_count}");
        }
    }
}
