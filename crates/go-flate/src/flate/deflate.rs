//! Port of go1.27.1 `compress/flate/deflate.go`.

use std::io::Write;

use super::deflatefast::{FastEnc, MAX_MATCH_OFFSET, match_len, new_fast_enc};
use super::huffman_bit_writer::{
    HuffmanBitWriter, LENGTH_EXTRA_BITS, OFFSET_EXTRA_BITS, new_huffman_bit_writer,
};
use super::huffman_code::{HuffmanEncoder, new_huffman_encoder};
use super::token::{LENGTH_CODES, Tokens, offset_code};
use super::{
    DEFAULT_COMPRESSION, HUFFMAN_ONLY, MAX_MATCH_LENGTH, MAX_STORE_BLOCK_SIZE, MIN_MATCH_LENGTH,
    NO_COMPRESSION,
};
use crate::error::Error;

pub(crate) const LOG_WINDOW_SIZE: u32 = 15;
pub(crate) const WINDOW_SIZE: i32 = 1 << LOG_WINDOW_SIZE;
pub(crate) const WINDOW_MASK: i32 = WINDOW_SIZE - 1;
pub(crate) const MIN_OFFSET_SIZE: i32 = 1; // The shortest offset that makes any sense

// The maximum number of tokens we will encode at the time.
// Smaller sizes usually creates less optimal blocks.
// Bigger can make context switching slow.
// We use this for levels 7-9, so we make it big.
pub(crate) const MAX_FLATE_BLOCK_TOKENS: u16 = 1 << 15;
pub(crate) const HASH_BITS: u8 = 17; // After 17 performance degrades
pub(crate) const HASH_SIZE: usize = 1 << HASH_BITS;
pub(crate) const HASH_MASK: u32 = (1 << HASH_BITS) - 1;
pub(crate) const MAX_HASH_OFFSET: i32 = 1 << 28;

#[allow(dead_code)]
pub(crate) const SKIP_NEVER: i32 = i32::MAX;

/// compressionLevel holds the parameters for levels 7-9.
#[derive(Clone, Copy, Default)]
struct CompressionLevel {
    #[allow(dead_code)]
    good: i32, // "good enough" match length
    lazy: i32,  // don't try to find a later, better match above this length
    nice: i32,  // stop looking for a better match above this length
    chain: i32, // maximum number of hash chain entries to search
    level: i32,
}

const fn cl(good: i32, lazy: i32, nice: i32, chain: i32, level: i32) -> CompressionLevel {
    CompressionLevel {
        good,
        lazy,
        nice,
        chain,
        level,
    }
}

static LEVELS: [CompressionLevel; 10] = [
    cl(0, 0, 0, 0, 0), // 0
    // Level 1-6 uses specialized algorithm - values not used
    cl(0, 0, 0, 0, 1),
    cl(0, 0, 0, 0, 2),
    cl(0, 0, 0, 0, 3),
    cl(0, 0, 0, 0, 4),
    cl(0, 0, 0, 0, 5),
    cl(0, 0, 0, 0, 6),
    // Levels 7-9 use increasingly more lazy matching
    // and increasingly stringent conditions for "good enough".
    cl(8, 12, 16, 24, 7),
    cl(16, 30, 40, 64, 8),
    cl(32, 258, 258, 1024, 9),
];

/// advancedState contains state for levels 7-9, with bigger hash tables, etc.
struct AdvancedState {
    // deflate state
    length: i32,
    offset: i32,
    max_insert_index: i32,
    chain_head: i32,
    hash_offset: i32,

    literal_counter: u16, // consecutive literal count; overflows to reset after 64KB.

    // input window: unprocessed data is window[index:windowEnd]
    index: i32,
    hash_match: [u32; (MAX_MATCH_LENGTH + MIN_MATCH_LENGTH) as usize],

    // Input hash chains
    // hashHead[hashValue] contains the largest inputIndex with the specified hash value
    // If hashHead[hashValue] is within the current window, then
    // hashPrev[hashHead[hashValue] & windowMask] contains the previous index
    // with the same hash value.
    hash_head: Vec<i32>, // [hashSize]int32
    hash_prev: Vec<i32>, // [windowSize]int32
}

impl AdvancedState {
    fn new() -> AdvancedState {
        AdvancedState {
            length: 0,
            offset: 0,
            max_insert_index: 0,
            chain_head: 0,
            hash_offset: 0,
            literal_counter: 0,
            index: 0,
            hash_match: [0; (MAX_MATCH_LENGTH + MIN_MATCH_LENGTH) as usize],
            hash_head: vec![0; HASH_SIZE],
            hash_prev: vec![0; WINDOW_SIZE as usize],
        }
    }
}

/// Go's `fill`/`step` function pointers, as an enum.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// fill = fillBlock, step = store
    Store,
    /// fill = fillBlock, step = deflateHuff
    Huff,
    /// fill = fillBlock, step = deflateFast
    Fast,
    /// fill = fillDeflate, step = deflateLazy
    Lazy,
}

pub(crate) struct Compressor<W: Write> {
    cl: CompressionLevel,

    h: Option<Box<HuffmanEncoder>>,    // huffman encoder, with state
    pub(crate) w: HuffmanBitWriter<W>, // writer for blocks

    // compression algorithm
    mode: Mode,

    window: Vec<u8>,    // current window - size depends on encoder level
    window_end: i32,    // filled bytes in window
    block_start: i32,   // window index where current tokens start
    err: Option<Error>, // stateful error
    closed: bool,       // err == errWriterClosed (kept separately for identity checks)

    // queued output tokens
    tokens: Box<Tokens>,               // tokens store for each block
    fast: Option<FastEnc>,             // encoder to use for blocks
    state: Option<Box<AdvancedState>>, // chained encoder for level 7-9

    sync: bool,           // requesting flush
    byte_available: bool, // if true, still need to process window[index-1].
}

// Go: compress/flate/deflate.go:hash4
/// hash4 returns a hash representation of the first 4 bytes
/// of the supplied slice.
/// The caller must ensure that len(b) >= 4.
#[inline(always)]
fn hash4(b: &[u8]) -> u32 {
    hash4u(u32::from_le_bytes(b[..4].try_into().unwrap()), HASH_BITS)
}

// Go: compress/flate/deflate.go:hash4u
/// hash4 returns the hash of u to fit in a hash table with h bits.
/// Preferably h should be a constant and should always be <32.
#[inline(always)]
fn hash4u(u: u32, h: u8) -> u32 {
    u.wrapping_mul(2654435761) >> (32 - h)
}

// Go: compress/flate/deflate.go:bulkHash4
/// bulkHash4 sets dst[i] = hash4(b[i:i+4]) for all i <= len(b)-4.
fn bulk_hash4(b: &[u8], dst: &mut [u32]) {
    if b.len() < 4 {
        return;
    }
    let mut hb = u32::from_le_bytes(b[..4].try_into().unwrap());

    dst[0] = hash4u(hb, HASH_BITS);
    let end = b.len() - 4 + 1;
    for i in 1..end {
        hb = (hb >> 8) | (b[i + 3] as u32) << 24;
        dst[i] = hash4u(hb, HASH_BITS);
    }
}

impl<W: Write> Compressor<W> {
    // Go: compress/flate/deflate.go:(*compressor).init
    /// init a new encode with new writer and compression level.
    pub(crate) fn new(w: W, mut level: i32) -> Result<Compressor<W>, Error> {
        let mut d = Compressor {
            cl: CompressionLevel::default(),
            h: None,
            w: new_huffman_bit_writer(w),
            mode: Mode::Store,
            window: Vec::new(),
            window_end: 0,
            block_start: 0,
            err: None,
            closed: false,
            tokens: Box::new(Tokens::new()),
            fast: None,
            state: None,
            sync: false,
            byte_available: false,
        };

        if level == NO_COMPRESSION {
            d.window = vec![0; MAX_STORE_BLOCK_SIZE as usize];
            d.mode = Mode::Store;
        } else if level == HUFFMAN_ONLY {
            d.w.log_new_table_penalty = 10;
            d.window = vec![0; 32 << 10];
            d.mode = Mode::Huff;
        } else if level == DEFAULT_COMPRESSION || (1..=6).contains(&level) {
            if level == DEFAULT_COMPRESSION {
                level = 6;
            }
            d.w.log_new_table_penalty = 7;
            d.fast = Some(new_fast_enc(level));
            d.window = vec![0; MAX_STORE_BLOCK_SIZE as usize];
            d.mode = Mode::Fast;
        } else if (7..=9).contains(&level) {
            d.w.log_new_table_penalty = 8;
            d.state = Some(Box::new(AdvancedState::new()));
            d.cl = LEVELS[level as usize];
            d.init_deflate();
            d.mode = Mode::Lazy;
        } else {
            return Err(Error::InvalidLevel(level as i64));
        }
        d.cl.level = level;
        Ok(d)
    }

    #[inline]
    fn step(&mut self) {
        match self.mode {
            Mode::Store => self.store(),
            Mode::Huff => self.deflate_huff(),
            Mode::Fast => self.deflate_fast(),
            Mode::Lazy => self.deflate_lazy(),
        }
    }

    #[inline]
    fn fill(&mut self, b: &[u8]) -> usize {
        match self.mode {
            Mode::Lazy => self.fill_deflate(b),
            _ => self.fill_block(b),
        }
    }

    // Go: compress/flate/deflate.go:(*compressor).fillDeflate
    /// fillDeflate will add b to the current window for levels 7-9.
    fn fill_deflate(&mut self, b: &[u8]) -> usize {
        let s = self.state.as_deref_mut().unwrap();
        if s.index >= 2 * WINDOW_SIZE - (MIN_MATCH_LENGTH + MAX_MATCH_LENGTH) {
            // shift the window by windowSize
            self.window
                .copy_within(WINDOW_SIZE as usize..2 * WINDOW_SIZE as usize, 0);
            s.index -= WINDOW_SIZE;
            self.window_end -= WINDOW_SIZE;
            if self.block_start >= WINDOW_SIZE {
                self.block_start -= WINDOW_SIZE;
            } else {
                self.block_start = i32::MAX;
            }
            s.hash_offset += WINDOW_SIZE;
            if s.hash_offset > MAX_HASH_OFFSET {
                let delta = s.hash_offset - 1;
                s.hash_offset -= delta;
                s.chain_head -= delta;
                for v in s.hash_prev.iter_mut() {
                    *v = std::cmp::max(v.wrapping_sub(delta), 0);
                }
                for v in s.hash_head.iter_mut() {
                    *v = std::cmp::max(v.wrapping_sub(delta), 0);
                }
            }
        }
        let dst = &mut self.window[self.window_end as usize..];
        let n = std::cmp::min(dst.len(), b.len());
        dst[..n].copy_from_slice(&b[..n]);
        self.window_end += n as i32;
        n
    }

    // Go: compress/flate/deflate.go:(*compressor).writeBlock
    /// writeBlock will write tokens to output.
    /// The provided index is where the block starts in d.window.
    fn write_block(&mut self, index: i32, eof: bool) -> Option<Error> {
        if index > 0 || eof {
            let mut window: Option<&[u8]> = None;
            if self.block_start <= index {
                window = Some(&self.window[self.block_start as usize..index as usize]);
            }
            self.block_start = index;
            self.w
                .write_block_dynamic(&mut self.tokens, eof, window, self.sync);
            return self.w.err.clone();
        }
        None
    }

    // Go: compress/flate/deflate.go:(*compressor).writeBlockSkip
    /// writeBlockSkip writes the current block and uses the number of tokens
    /// to determine if the block should be stored when there are no matches, or
    /// only Huffman encoded.
    #[allow(dead_code)]
    fn write_block_skip(&mut self, index: i32, eof: bool) -> Option<Error> {
        if index > 0 || eof {
            if self.block_start <= index {
                let window = &self.window[self.block_start as usize..index as usize];
                // If we removed less than a 64th of all literals
                // we huffman compress the block.
                if self.tokens.n as usize > window.len() - (window.len() >> 6) {
                    self.w.write_block_huff(eof, window, self.sync);
                } else {
                    // Write a dynamic huffman block.
                    self.w
                        .write_block_dynamic(&mut self.tokens, eof, Some(window), self.sync);
                }
            } else {
                self.w.write_block(&mut self.tokens, eof, None);
            }
            self.block_start = index;
            return self.w.err.clone();
        }
        None
    }

    // Go: compress/flate/deflate.go:(*compressor).fillWindow
    /// fillWindow will fill the current window with the supplied
    /// dictionary and calculate all hashes.
    /// This is much faster than doing a full encode.
    /// Should only be used after a start/reset.
    pub(crate) fn fill_window(&mut self, mut b: &[u8]) {
        // Do not fill window if we are in store-only or huffman mode.
        if self.cl.level <= 0 {
            return;
        }
        if let Some(fast) = self.fast.as_mut() {
            // encode the last data, but discard the result
            if b.len() > MAX_MATCH_OFFSET as usize {
                b = &b[b.len() - MAX_MATCH_OFFSET as usize..];
            }
            fast.encode(&mut self.tokens, b);
            self.tokens.reset();
            return;
        }
        let s = self.state.as_deref_mut().unwrap();
        // If we are given too much, cut it.
        if b.len() > WINDOW_SIZE as usize {
            b = &b[b.len() - WINDOW_SIZE as usize..];
        }
        // Add all to window.
        let dst = &mut self.window[self.window_end as usize..];
        let n = std::cmp::min(dst.len(), b.len());
        dst[..n].copy_from_slice(&b[..n]);
        let n = n as i32;

        // Calculate 256 hashes at the time (more L1 cache hits)
        let loops = (n + 256 - MIN_MATCH_LENGTH) / 256;
        for j in 0..loops {
            let startindex = j * 256;
            let end = std::cmp::min(startindex + 256 + MIN_MATCH_LENGTH - 1, n);
            let tocheck = &self.window[startindex as usize..end as usize];
            let dst_size = tocheck.len() as i64 - MIN_MATCH_LENGTH as i64 + 1;

            if dst_size <= 0 {
                continue;
            }

            let dst = &mut s.hash_match[..dst_size as usize];
            bulk_hash4(tocheck, dst);
            for (i, &val) in s.hash_match[..dst_size as usize].iter().enumerate() {
                let di = i as i32 + startindex;
                let new_h = (val & HASH_MASK) as usize;
                // Get previous value with the same hash.
                // Our chain should point to the previous value.
                s.hash_prev[(di & WINDOW_MASK) as usize] = s.hash_head[new_h];
                // Set the head of the hash chain to us.
                s.hash_head[new_h] = di + s.hash_offset;
            }
        }
        // Update window information.
        self.window_end += n;
        s.index = n;
    }

    // Go: compress/flate/deflate.go:(*compressor).findMatch
    /// findMatch finds the longest match starting at pos in the hash chain starting
    /// at prevHead. It searches up to d.chain entries in the chain.
    fn find_match(
        &self,
        s: &AdvancedState,
        pos: i32,
        prev_head: i32,
        lookahead: i32,
    ) -> (i32, i32, bool) {
        let min_match_look = std::cmp::min(lookahead, MAX_MATCH_LENGTH);

        let win = &self.window[0..(pos + min_match_look) as usize];

        // We quit when we get a match that's at least nice long
        let nice = std::cmp::min(self.cl.nice, win.len() as i32 - pos);

        // If we've got a match that's good enough, only look in 1/4 the chain.
        let mut tries = self.cl.chain;
        let mut length = MIN_MATCH_LENGTH - 1;
        let mut ok = false;

        let mut w_end = win[(pos + length) as usize];
        let w_pos = &win[pos as usize..];
        let min_index = std::cmp::max(pos - WINDOW_SIZE, 0);
        let mut offset = 0;

        // Minimum gain to accept a match.
        let mut c_gain: i64 = 4;

        // Some like it higher (CSV), some like it lower (JSON)
        const BASE_COST: i64 = 3;
        // Base is 4 bytes at with an additional cost.
        // Matches must be better than this.

        let mut i = prev_head;
        while tries > 0 {
            'next: {
                if w_end == win[(i + length) as usize] {
                    let n =
                        match_len(&win[i as usize..(i + min_match_look) as usize], w_pos) as i32;
                    if n > length {
                        if self.cl.chain >= 100 {
                            // Calculate gain. Estimates the gains of the new match compared to emitting as literals.
                            let h = self.h.as_deref().expect("d.h");
                            let new_gain = h.bit_length_raw(&w_pos[..n as usize])
                                - OFFSET_EXTRA_BITS[offset_code((pos - i) as u32) as usize] as i64
                                - BASE_COST
                                - LENGTH_EXTRA_BITS[LENGTH_CODES[((n - 3) & 255) as usize] as usize]
                                    as i64;
                            if new_gain <= c_gain {
                                break 'next; // goto next
                            }
                            c_gain = new_gain;
                        }
                        length = n;
                        offset = pos - i;
                        ok = true;
                        if n >= nice {
                            // The match is good enough that we don't try to find a better one.
                            return (length, offset, ok);
                        }
                        w_end = win[(pos + n) as usize];
                    }
                }
            }
            // next:
            if i <= min_index {
                // hashPrev[i & windowMask] has already been overwritten, so stop now.
                break;
            }
            i = s.hash_prev[(i & WINDOW_MASK) as usize] - s.hash_offset;
            if i < min_index {
                break;
            }
            tries -= 1;
        }
        (length, offset, ok)
    }

    // Go: compress/flate/deflate.go:(*compressor).writeStoredBlock
    /// writeStoredBlock writes an uncompressed block to the stream.
    fn write_stored_block(&mut self, end: usize) -> Option<Error> {
        self.w.write_stored_header(end, false);
        if self.w.err.is_some() {
            return self.w.err.clone();
        }
        self.w.write_bytes(&self.window[..end]);
        self.w.err.clone()
    }

    // Go: compress/flate/deflate.go:(*compressor).initDeflate
    /// initDeflate initializes d for levels 7-9.
    fn init_deflate(&mut self) {
        self.window = vec![0; 2 * WINDOW_SIZE as usize];
        self.byte_available = false;
        self.err = None;
        self.closed = false;
        let Some(s) = self.state.as_deref_mut() else {
            return;
        };
        s.index = 0;
        s.hash_offset = 1;
        s.length = MIN_MATCH_LENGTH - 1;
        s.offset = 0;
        s.chain_head = -1;
    }

    /// Go: `s.index < s.maxInsertIndex` hash insertion at s.index, shared by
    /// several places in the lazy matcher.
    #[inline(always)]
    fn insert_hash(&self, s: &mut AdvancedState) {
        let h = hash4(&self.window[s.index as usize..]) as usize;
        let ch = s.hash_head[h];
        s.chain_head = ch;
        s.hash_prev[(s.index & WINDOW_MASK) as usize] = ch;
        s.hash_head[h] = s.index + s.hash_offset;
    }

    // Go: compress/flate/deflate.go:(*compressor).tryBetterMatchAtEnd
    /// tryBetterMatchAtEnd checks whether a better match exists at the end of the
    /// previous match and, if so, emits the skipped literals and adjusts the match.
    /// Returns the (possibly updated) prevLength and prevOffset.
    fn try_better_match_at_end(
        &mut self,
        s: &mut AdvancedState,
        mut prev_length: i32,
        mut prev_offset: i32,
        lookahead: i32,
    ) -> (i32, i32) {
        // We start checking at checkOff from the current match position.
        // This allows up to two additional literals, but that could be
        // compensated by a higher quality match.
        // If the match looks better, we extend backwards.
        const CHECK_OFF: i32 = 2;

        if prev_length >= MAX_MATCH_LENGTH - CHECK_OFF {
            return (prev_length, prev_offset);
        }
        let prev_index = s.index - 1;
        if prev_index + prev_length >= s.max_insert_index {
            return (prev_length, prev_offset);
        }

        let end = std::cmp::min(lookahead, MAX_MATCH_LENGTH + CHECK_OFF) + prev_index;
        let min_index = std::cmp::max(s.index - WINDOW_SIZE, 0);

        let h = hash4(&self.window[(prev_index + prev_length) as usize..]) as usize;
        let ch2 = s.hash_head[h] - s.hash_offset - prev_length;
        if prev_index - ch2 == prev_offset || ch2 <= min_index + CHECK_OFF {
            return (prev_length, prev_offset);
        }

        let length = match_len(
            &self.window[(prev_index + CHECK_OFF) as usize..end as usize],
            &self.window[(ch2 + CHECK_OFF) as usize..],
        ) as i32;
        if length <= prev_length {
            return (prev_length, prev_offset);
        }

        prev_length = length;
        prev_offset = prev_index - ch2;

        let mut i = CHECK_OFF - 1;
        while i >= 0 {
            if prev_length >= MAX_MATCH_LENGTH
                || self.window[(prev_index + i) as usize] != self.window[(ch2 + i) as usize]
            {
                for j in 0..i + 1 {
                    let lit = self.window[(prev_index + j) as usize];
                    self.tokens.add_literal(lit);
                    if self.tokens.n == MAX_FLATE_BLOCK_TOKENS {
                        self.err = self.write_block(s.index, false);
                        if self.err.is_some() {
                            return (prev_length, prev_offset);
                        }
                        self.tokens.reset();
                    }
                    s.index += 1;
                    if s.index < s.max_insert_index {
                        self.insert_hash(s);
                    }
                }
                break;
            }
            prev_length += 1;
            i -= 1;
        }
        (prev_length, prev_offset)
    }

    // Go: compress/flate/deflate.go:(*compressor).skipLiterals
    /// skipLiterals emits extra literal bytes during long runs of incompressible data,
    /// skipping ahead to avoid futile match searches. Returns false on write error.
    fn skip_literals(&mut self, s: &mut AdvancedState) -> bool {
        let mut n = s.literal_counter as i32 - self.cl.chain;
        if n <= 0 {
            return true;
        }
        n = 1 + (n >> 6);
        for _ in 0..n {
            if s.index >= self.window_end - 1 {
                break;
            }
            let lit = self.window[(s.index - 1) as usize];
            self.tokens.add_literal(lit);
            if self.tokens.n == MAX_FLATE_BLOCK_TOKENS {
                self.err = self.write_block(s.index, false);
                if self.err.is_some() {
                    return false;
                }
                self.tokens.reset();
            }
            if s.index < s.max_insert_index {
                self.insert_hash(s);
            }
            s.index += 1;
        }
        let lit = self.window[(s.index - 1) as usize];
        self.tokens.add_literal(lit);
        self.byte_available = false;
        if self.tokens.n == MAX_FLATE_BLOCK_TOKENS {
            self.err = self.write_block(s.index, false);
            if self.err.is_some() {
                return false;
            }
            self.tokens.reset();
        }
        true
    }

    // Go: compress/flate/deflate.go:(*compressor).deflateLazy
    /// deflateLazy encodes the current window using lazy matching.
    /// Lazy matching defers emitting a match to see if the next position yields a better one.
    /// Unique to levels 7-9 is that more than 2 matches are potentially checked
    /// until a good/nice one is found.
    fn deflate_lazy(&mut self) {
        let mut s = self.state.take().expect("advanced state");
        self.deflate_lazy_inner(&mut s);
        self.state = Some(s);
    }

    fn deflate_lazy_inner(&mut self, s: &mut AdvancedState) {
        if self.window_end - s.index < MIN_MATCH_LENGTH + MAX_MATCH_LENGTH && !self.sync {
            return;
        }
        if self.window_end != s.index && self.cl.chain > 100 {
            // Get literal huffman coder.
            // This is used to estimate the cost of emitting a literal.
            if self.h.is_none() {
                self.h = Some(Box::new(new_huffman_encoder(
                    MAX_FLATE_BLOCK_TOKENS as usize,
                )));
            }
            let mut tmp = [0u16; 256];
            let to_index = &self.window[s.index as usize..self.window_end as usize];
            let to_index =
                &to_index[..std::cmp::min(to_index.len(), MAX_FLATE_BLOCK_TOKENS as usize)];
            for &v in to_index {
                tmp[v as usize] = tmp[v as usize].wrapping_add(1);
            }
            self.h.as_deref_mut().unwrap().generate(&tmp[..], 15);
        }

        s.max_insert_index = self.window_end - (MIN_MATCH_LENGTH - 1);

        loop {
            let lookahead = self.window_end - s.index;
            if lookahead < MIN_MATCH_LENGTH + MAX_MATCH_LENGTH {
                if !self.sync {
                    return;
                }
                if lookahead == 0 {
                    // Flush current output block if any.
                    if self.byte_available {
                        // There is still one pending token that needs to be flushed
                        let lit = self.window[(s.index - 1) as usize];
                        self.tokens.add_literal(lit);
                        self.byte_available = false;
                    }
                    if self.tokens.n > 0 {
                        self.err = self.write_block(s.index, false);
                        if self.err.is_some() {
                            return;
                        }
                        self.tokens.reset();
                    }
                    return;
                }
            }
            if s.index < s.max_insert_index {
                self.insert_hash(s);
            }
            let mut prev_length = s.length;
            let mut prev_offset = s.offset;
            s.length = MIN_MATCH_LENGTH - 1;
            s.offset = 0;
            let min_index = std::cmp::max(s.index - WINDOW_SIZE, 0);

            if s.chain_head - s.hash_offset >= min_index
                && lookahead > prev_length
                && prev_length < self.cl.lazy
            {
                let (new_length, new_offset, ok) =
                    self.find_match(s, s.index, s.chain_head - s.hash_offset, lookahead);
                if ok {
                    s.length = new_length;
                    s.offset = new_offset;
                }
            }

            if prev_length >= MIN_MATCH_LENGTH && s.length <= prev_length {
                (prev_length, prev_offset) =
                    self.try_better_match_at_end(s, prev_length, prev_offset, lookahead);
                if self.err.is_some() {
                    return;
                }

                // There was a match at the previous step, and the current match is
                // not better. Output the previous match.
                self.tokens.add_match(
                    (prev_length - 3) as u32,
                    (prev_offset - MIN_OFFSET_SIZE) as u32,
                );

                // Insert in the hash table all strings up to the end of the match.
                // index and index-1 are already inserted. If there is not enough
                // lookahead, the last two strings are not inserted into the hash
                // table.
                let new_index = s.index + prev_length - 1;
                let mut end = std::cmp::min(new_index, s.max_insert_index);
                end += MIN_MATCH_LENGTH - 1;
                let startindex = std::cmp::min(s.index + 1, s.max_insert_index);
                let tocheck = &self.window[startindex as usize..end as usize];
                let dst_size = tocheck.len() as i64 - MIN_MATCH_LENGTH as i64 + 1;
                if dst_size > 0 {
                    let dst = &mut s.hash_match[..dst_size as usize];
                    bulk_hash4(tocheck, dst);
                    for i in 0..dst_size as usize {
                        let val = s.hash_match[i];
                        let di = i as i32 + startindex;
                        let new_h = (val & HASH_MASK) as usize;
                        s.hash_prev[(di & WINDOW_MASK) as usize] = s.hash_head[new_h];
                        s.hash_head[new_h] = di + s.hash_offset;
                    }
                }

                s.index = new_index;
                self.byte_available = false;
                s.length = MIN_MATCH_LENGTH - 1;
                if self.tokens.n == MAX_FLATE_BLOCK_TOKENS {
                    self.err = self.write_block(s.index, false);
                    if self.err.is_some() {
                        return;
                    }
                    self.tokens.reset();
                }
                s.literal_counter = 0;
                continue;
            }
            if s.length >= MIN_MATCH_LENGTH {
                s.literal_counter = 0;
            }
            if self.byte_available {
                s.literal_counter = s.literal_counter.wrapping_add(1);
                let lit = self.window[(s.index - 1) as usize];
                self.tokens.add_literal(lit);
                if self.tokens.n == MAX_FLATE_BLOCK_TOKENS {
                    self.err = self.write_block(s.index, false);
                    if self.err.is_some() {
                        return;
                    }
                    self.tokens.reset();
                }
                s.index += 1;
                if !self.skip_literals(s) {
                    return;
                }
            } else {
                s.index += 1;
                self.byte_available = true;
            }
        }
    }

    // Go: compress/flate/deflate.go:(*compressor).store
    /// store will store the current window if it has filled or if we are in sync.
    fn store(&mut self) {
        if self.window_end > 0 && (self.window_end == MAX_STORE_BLOCK_SIZE || self.sync) {
            self.err = self.write_stored_block(self.window_end as usize);
            self.window_end = 0;
        }
    }

    // Go: compress/flate/deflate.go:(*compressor).fillBlock
    /// fillBlock appends b to d.window, returning the number of bytes copied.
    /// If n < len(b), the window is filled.
    fn fill_block(&mut self, b: &[u8]) -> usize {
        let dst = &mut self.window[self.window_end as usize..];
        let n = std::cmp::min(dst.len(), b.len());
        dst[..n].copy_from_slice(&b[..n]);
        self.window_end += n as i32;
        n
    }

    // Go: compress/flate/deflate.go:(*compressor).deflateHuff
    /// deflateHuff compresses and stores the current window
    /// (if it has filled or if we are in sync or flush).
    /// It uses Huffman-only encoding.
    fn deflate_huff(&mut self) {
        if (self.window_end as usize) < self.window.len() && !self.sync || self.window_end == 0 {
            return;
        }
        self.w
            .write_block_huff(false, &self.window[..self.window_end as usize], self.sync);
        self.err = self.w.err.clone();
        self.window_end = 0;
    }

    // Go: compress/flate/deflate.go:(*compressor).deflateFast
    /// deflateFast encodes the current window
    /// if it has filled or if we are doing sync/flush.
    /// It uses the level 1-6 fast encoding.
    fn deflate_fast(&mut self) {
        // We only compress if we have maxStoreBlockSize.
        if (self.window_end as usize) < self.window.len() {
            if !self.sync {
                return;
            }
            // Handle extremely small sizes.
            if self.window_end < 128 {
                if self.window_end == 0 {
                    return;
                }
                if self.window_end <= 32 {
                    self.err = self.write_stored_block(self.window_end as usize);
                } else {
                    self.w
                        .write_block_huff(false, &self.window[..self.window_end as usize], true);
                    self.err = self.w.err.clone();
                }
                self.tokens.reset();
                self.window_end = 0;
                self.fast.as_mut().unwrap().reset();
                return;
            }
        }

        let we = self.window_end as usize;
        self.fast
            .as_mut()
            .unwrap()
            .encode(&mut self.tokens, &self.window[..we]);
        // If we made zero matches, store the block as is.
        if self.tokens.n == 0 {
            self.err = self.write_stored_block(we);
            // If we removed less than 1/16th, huffman compress the block.
        } else if self.tokens.n as i32 > self.window_end - (self.window_end >> 4) {
            self.w
                .write_block_huff(false, &self.window[..we], self.sync);
            self.err = self.w.err.clone();
        } else {
            self.w.write_block_dynamic(
                &mut self.tokens,
                false,
                Some(&self.window[..we]),
                self.sync,
            );
            self.err = self.w.err.clone();
        }
        self.tokens.reset();
        self.window_end = 0;
    }

    // Go: compress/flate/deflate.go:(*compressor).write
    /// write adds b to the compressor.
    /// It can only return a short length if an error occurs.
    pub(crate) fn write(&mut self, mut b: &[u8]) -> Result<usize, Error> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        let n = b.len();
        while !b.is_empty() {
            if self.window_end as usize == self.window.len() || self.sync {
                self.step();
            }
            let m = self.fill(b);
            b = &b[m..];
            if let Some(e) = &self.err {
                return Err(e.clone());
            }
        }
        match &self.err {
            Some(e) => Err(e.clone()),
            None => Ok(n),
        }
    }

    // Go: compress/flate/deflate.go:(*compressor).syncFlush
    /// syncFlush will flush the compressor by writing
    /// any remaining window and writing a stored block
    /// to byte-align the output.
    pub(crate) fn sync_flush(&mut self) -> Result<(), Error> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        self.sync = true;
        self.step();
        if self.err.is_none() {
            self.w.write_stored_header(0, false);
            self.w.flush();
            self.err = self.w.err.clone();
        }
        self.sync = false;
        match &self.err {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    // Go: compress/flate/deflate.go:(*compressor).reset
    /// reset resets the compressor with a new output writer.
    pub(crate) fn reset(&mut self, w: W) -> W {
        let old = self.w.reset(w);
        self.sync = false;
        self.err = None;
        self.closed = false;
        self.window_end = 0;
        // We only need to reset a few things for fast encoders.
        if let Some(fast) = self.fast.as_mut() {
            fast.reset();
            self.tokens.reset();
            return old;
        }
        if self.cl.chain == 0 {
            return old;
        }
        let s = self.state.as_deref_mut().unwrap();
        s.chain_head = -1;
        s.hash_head.fill(0);
        s.hash_prev.fill(0);
        s.hash_offset = 1;
        s.index = 0;
        self.block_start = 0;
        self.byte_available = false;
        self.tokens.reset();
        s.length = MIN_MATCH_LENGTH - 1;
        s.offset = 0;
        s.literal_counter = 0;
        s.max_insert_index = 0;
        old
    }

    // Go: compress/flate/deflate.go:(*compressor).close
    /// close flushes any uncompressed data and writes an EOF block.
    pub(crate) fn close(&mut self) -> Result<(), Error> {
        if self.closed {
            // d.err == errWriterClosed
            return Ok(());
        }
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        self.sync = true;
        self.step();
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        self.w.write_stored_header(0, true);
        if let Some(e) = &self.w.err {
            return Err(e.clone());
        }
        self.w.flush();
        if let Some(e) = &self.w.err {
            return Err(e.clone());
        }
        self.err = Some(Error::WriterClosed);
        self.closed = true;
        // Go: d.w.reset(nil). The Rust port keeps the writer so callers can
        // retrieve it; it is never written to again (d.err is set).
        self.w.reset_state();
        Ok(())
    }
}

/// A Writer takes data written to it and writes the compressed
/// form of that data to an underlying writer (see [`new_writer`]).
pub struct Writer<W: Write> {
    d: Compressor<W>,
    dict: Vec<u8>,
}

// Go: compress/flate/deflate.go:NewWriter
/// NewWriter returns a new [`Writer`] compressing data at the given level.
/// Following zlib, levels range from 1 ([`BEST_SPEED`](super::BEST_SPEED)) to 9
/// ([`BEST_COMPRESSION`](super::BEST_COMPRESSION)); higher levels typically run
/// slower but compress more. Level 0 ([`NO_COMPRESSION`]) does not attempt any
/// compression; it only adds the necessary DEFLATE framing.
/// Level -1 ([`DEFAULT_COMPRESSION`]) uses the default compression level.
/// Level -2 ([`HUFFMAN_ONLY`]) will use Huffman compression only, giving
/// a very fast compression for all types of input, but sacrificing considerable
/// compression efficiency.
///
/// If level is in the range [-2, 9] then the error returned will be nil.
/// Otherwise the error returned will be non-nil.
///
/// Every Go `Write` call on the underlying `io.Writer` is exactly one
/// `write_all` call on `w`, with the same bytes.
pub fn new_writer<W: Write>(w: W, level: i32) -> Result<Writer<W>, Error> {
    let d = Compressor::new(w, level)?;
    Ok(Writer {
        d,
        dict: Vec::new(),
    })
}

// Go: compress/flate/deflate.go:NewWriterDict
/// NewWriterDict is like [`new_writer`] but initializes the new
/// [`Writer`] with a preset dictionary. The returned [`Writer`] behaves
/// as if the dictionary had been written to it without producing
/// any compressed output. The compressed data written to w
/// can only be decompressed by a reader initialized with the
/// same dictionary.
pub fn new_writer_dict<W: Write>(w: W, level: i32, dict: &[u8]) -> Result<Writer<W>, Error> {
    let mut zw = new_writer(w, level)?;
    zw.d.fill_window(dict);
    // Clone dict so we can Reset without changing the provided slice.
    zw.dict = dict.to_vec();
    Ok(zw)
}

impl<W: Write> Writer<W> {
    // Go: compress/flate/deflate.go:(*Writer).Write
    /// Write writes data to w, which will eventually write the
    /// compressed form of data to its underlying writer.
    ///
    /// Go returns `(0, err)` on error; the Rust port returns `Err(err)`.
    pub fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        self.d.write(data)
    }

    // Go: compress/flate/deflate.go:(*Writer).Flush
    /// Flush flushes any pending data to the underlying writer.
    /// It is useful mainly in compressed network protocols, to ensure that
    /// a remote reader has enough data to reconstruct a packet.
    /// Flush does not return until the data has been written.
    /// Calling Flush when there is no pending data still causes the [`Writer`]
    /// to emit a sync marker of at least 4 bytes.
    /// If the underlying writer returns an error, Flush returns that error.
    ///
    /// In the terminology of the zlib library, Flush is equivalent to Z_SYNC_FLUSH.
    pub fn flush(&mut self) -> Result<(), Error> {
        // For more about flushing:
        // https://www.bolet.org/~pornin/deflate-flush.html
        self.d.sync_flush()
    }

    // Go: compress/flate/deflate.go:(*Writer).Close
    /// Close flushes and closes the writer.
    pub fn close(&mut self) -> Result<(), Error> {
        self.d.close()
    }

    // Go: compress/flate/deflate.go:(*Writer).Reset
    /// Reset discards the writer's state and makes it equivalent to
    /// the result of NewWriter or NewWriterDict called with dst
    /// and w's level and dictionary. Returns the previous underlying writer.
    pub fn reset(&mut self, dst: W) -> W {
        let old = self.d.reset(dst);
        let dict = std::mem::take(&mut self.dict);
        self.d.fill_window(&dict);
        self.dict = dict;
        old
    }

    /// Returns a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.d.w.writer
    }

    /// Returns a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.d.w.writer
    }

    /// Consumes the Writer (without closing it) and returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.d.w.writer
    }
}

/// `std::io::Write` adapter. `write` is Go's `Write` (it always consumes all
/// of `buf` or fails); `flush` is Go's `Flush`, i.e. it emits a sync marker.
impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Writer::write(self, buf).map_err(Into::into)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Writer::flush(self).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: compress/flate/deflate_test.go:TestBulkHash4
    #[test]
    fn bulk_hash4_matches_hash4() {
        let outs: &[&[u8]] = &[
            &[0x3, 0x0],
            &[0x12, 0x4, 0xc, 0x0],
            &[0x0, 0x1, 0x0, 0xfe, 0xff, 0x11, 0x3, 0x0],
            &[0x0, 0x2, 0x0, 0xfd, 0xff, 0x11, 0x12, 0x3, 0x0],
            &[
                0x0, 0x8, 0x0, 0xf7, 0xff, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x3, 0x0,
            ],
            &[0x12, 0x14, 0x2, 0xc, 0x0],
            &[0x12, 0x84, 0x1, 0xc0, 0x0],
        ];
        for y in outs {
            if y.len() < MIN_MATCH_LENGTH as usize {
                continue;
            }
            let mut y2 = y.to_vec();
            y2.extend_from_slice(y);
            for j in 4..y2.len() {
                let y = &y2[..j];
                let mut dst: Vec<u32> = (0..y.len() - MIN_MATCH_LENGTH as usize + 1)
                    .map(|i| i as u32 + 100)
                    .collect();
                bulk_hash4(y, &mut dst);
                for (i, &got) in dst.iter().enumerate() {
                    assert_eq!(got, hash4(&y[i..]), "len {} index {i}", y.len());
                }
            }
        }
    }
}
