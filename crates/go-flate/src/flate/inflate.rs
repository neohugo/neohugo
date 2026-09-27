//! Port of go1.27.1 `compress/flate/inflate.go`.
//!
//! Go's `flate.Reader` (io.Reader + io.ByteReader) is modelled by
//! `std::io::BufRead`: `ReadByte` is `fill_buf` + `consume(1)` and `Read` is
//! `BufRead`'s `read`. When Go is given a plain `io.Reader`, `makeReader`
//! wraps it in `bufio.NewReader` (4096 bytes); do the same with
//! `std::io::BufReader::with_capacity(4096, r)` to get the same read pattern
//! on the underlying reader.

use std::io::BufRead;
use std::sync::OnceLock;

use super::END_BLOCK_MARKER;
use super::dict_decoder::DictDecoder;
use crate::error::Error;

const MAX_CODE_LEN: usize = 16; // max length of Huffman code
// The next three numbers come from the RFC section 3.2.7, with the
// additional proviso in section 3.2.5 which implies that distance codes
// 30 and 31 should never occur in compressed data.
const MAX_NUM_LIT: usize = 286;
const MAX_NUM_DIST: usize = 30;
const NUM_CODES: usize = 19; // number of codes in Huffman meta-code

/// maxMatchOffset from deflatefast.go (the dictionary size).
const MAX_MATCH_OFFSET: usize = 1 << 15;

// The data structure for decoding Huffman tables is based on that of
// zlib. There is a lookup table of a fixed bit width (huffmanChunkBits),
// For codes smaller than the table width, there are multiple entries
// (each combination of trailing bits has the same value). For codes
// larger than the table width, the table contains a link to an overflow
// table. The width of each entry in the link table is the maximum code
// size minus the chunk width.
//
// Note that you can do a lookup in the table even without all bits
// filled. Since the extra bits are zero, and the DEFLATE Huffman codes
// have the property that shorter codes come before longer ones, the
// bit length estimate in the result is a lower bound on the actual
// number of bits.
//
// See the following:
//	https://github.com/madler/zlib/raw/master/doc/algorithm.txt

// chunk & 15 is number of bits
// chunk >> 4 is value, including table link

const HUFFMAN_CHUNK_BITS: u32 = 9;
const HUFFMAN_NUM_CHUNKS: usize = 1 << HUFFMAN_CHUNK_BITS;
const HUFFMAN_COUNT_MASK: u32 = 15;
const HUFFMAN_VALUE_SHIFT: u32 = 4;

#[derive(Clone)]
struct HuffmanDecoder {
    min: usize,                             // the minimum code length
    chunks: Box<[u32; HUFFMAN_NUM_CHUNKS]>, // chunks as described above
    links: Vec<Vec<u32>>,                   // overflow links
    link_mask: u32,                         // mask the width of the link table
}

impl Default for HuffmanDecoder {
    fn default() -> Self {
        HuffmanDecoder {
            min: 0,
            chunks: Box::new([0; HUFFMAN_NUM_CHUNKS]),
            links: Vec::new(),
            link_mask: 0,
        }
    }
}

impl HuffmanDecoder {
    // Go: compress/flate/inflate.go:(*huffmanDecoder).init
    /// Initialize Huffman decoding tables from array of code lengths.
    /// Following this function, h is guaranteed to be initialized into a complete
    /// tree (i.e., neither over-subscribed nor under-subscribed). The exception is a
    /// degenerate case where the tree has only a single symbol with length 1. Empty
    /// trees are permitted.
    fn init(&mut self, lengths: &[i64]) -> bool {
        if self.min != 0 {
            *self = HuffmanDecoder::default();
        }

        // Count number of codes of each length,
        // compute min and max length.
        let mut count = [0i64; MAX_CODE_LEN];
        let (mut min, mut max) = (0i64, 0i64);
        for &n in lengths {
            if n == 0 {
                continue;
            }
            if min == 0 || n < min {
                min = n;
            }
            if n > max {
                max = n;
            }
            count[n as usize] += 1;
        }

        // Empty tree. The decompressor.huffSym function will fail later if the tree
        // is used. Technically, an empty tree is only valid for the HDIST tree and
        // not the HCLEN and HLIT tree. However, a stream with an empty HCLEN tree
        // is guaranteed to fail since it will attempt to use the tree to decode the
        // codes for the HLIT and HDIST trees. Similarly, an empty HLIT tree is
        // guaranteed to fail later since the compressed data section must be
        // composed of at least one symbol (the end-of-block marker).
        if max == 0 {
            return true;
        }

        let mut code: i64 = 0;
        let mut nextcode = [0i64; MAX_CODE_LEN];
        for i in min..=max {
            code <<= 1;
            nextcode[i as usize] = code;
            code += count[i as usize];
        }

        // Check that the coding is complete (i.e., that we've
        // assigned all 2-to-the-max possible bit sequences).
        // Exception: To be compatible with zlib, we also need to
        // accept degenerate single-code codings. See also
        // TestDegenerateHuffmanCoding.
        if code != 1i64 << max && !(code == 1 && max == 1) {
            return false;
        }

        self.min = min as usize;
        if max > HUFFMAN_CHUNK_BITS as i64 {
            let num_links = 1usize << (max as u32 - HUFFMAN_CHUNK_BITS);
            self.link_mask = (num_links - 1) as u32;

            // create link tables
            let link = (nextcode[HUFFMAN_CHUNK_BITS as usize + 1] >> 1) as usize;
            self.links = vec![Vec::new(); HUFFMAN_NUM_CHUNKS - link];
            for j in link..HUFFMAN_NUM_CHUNKS {
                let mut reverse = (j as u16).reverse_bits() as usize;
                reverse >>= 16 - HUFFMAN_CHUNK_BITS;
                let off = j - link;
                self.chunks[reverse] =
                    (off << HUFFMAN_VALUE_SHIFT) as u32 | (HUFFMAN_CHUNK_BITS + 1);
                self.links[off] = vec![0u32; num_links];
            }
        }

        for (i, &n) in lengths.iter().enumerate() {
            if n == 0 {
                continue;
            }
            let code = nextcode[n as usize];
            nextcode[n as usize] += 1;
            let chunk = ((i as u32) << HUFFMAN_VALUE_SHIFT) | n as u32;
            let mut reverse = (code as u16).reverse_bits() as usize;
            reverse >>= 16 - n as u32;
            if n <= HUFFMAN_CHUNK_BITS as i64 {
                let mut off = reverse;
                while off < self.chunks.len() {
                    // We should never need to overwrite
                    // an existing chunk. Also, 0 is
                    // never a valid chunk, because the
                    // lower 4 "count" bits should be
                    // between 1 and 15.
                    self.chunks[off] = chunk;
                    off += 1 << n as u32;
                }
            } else {
                let j = reverse & (HUFFMAN_NUM_CHUNKS - 1);
                let value = (self.chunks[j] >> HUFFMAN_VALUE_SHIFT) as usize;
                let linktab = &mut self.links[value];
                reverse >>= HUFFMAN_CHUNK_BITS;
                let mut off = reverse;
                while off < linktab.len() {
                    linktab[off] = chunk;
                    off += 1 << (n as u32 - HUFFMAN_CHUNK_BITS);
                }
            }
        }

        true
    }
}

// Go: compress/flate/inflate.go:fixedHuffmanDecoderInit
fn fixed_huffman_decoder() -> &'static HuffmanDecoder {
    static FIXED: OnceLock<HuffmanDecoder> = OnceLock::new();
    FIXED.get_or_init(|| {
        // These come from the RFC section 3.2.6.
        let mut bits = [0i64; 288];
        for b in bits.iter_mut().take(144) {
            *b = 8;
        }
        for b in bits.iter_mut().take(256).skip(144) {
            *b = 9;
        }
        for b in bits.iter_mut().take(280).skip(256) {
            *b = 7;
        }
        for b in bits.iter_mut().take(288).skip(280) {
            *b = 8;
        }
        let mut h = HuffmanDecoder::default();
        h.init(&bits);
        h
    })
}

/// Go's `f.step` function pointer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    NextBlock,
    HuffmanBlock,
    CopyData,
}

/// Which decoder `f.hl` points to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hl {
    Fixed,
    H1,
}

/// Input side of the decompressor: the reader plus the bit buffer.
struct BitIn<R: BufRead> {
    // Input source.
    r: R,
    roffset: i64,

    // Input bits, in top of b.
    b: u32,
    nb: u32,
}

/// Go `io.ByteReader.ReadByte` on a `BufRead`.
fn read_byte<R: BufRead>(r: &mut R) -> Result<u8, Error> {
    loop {
        match r.fill_buf() {
            Ok(buf) => {
                if buf.is_empty() {
                    return Err(Error::Eof);
                }
                let c = buf[0];
                r.consume(1);
                return Ok(c);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Error::io(e)),
        }
    }
}

/// Go `io.ReadFull`: returns the number of bytes read and `io.EOF` if none
/// were read, `io.ErrUnexpectedEOF` if some but not all were.
pub(crate) fn read_full<R: std::io::Read>(r: &mut R, buf: &mut [u8]) -> (usize, Option<Error>) {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => {
                return (
                    n,
                    Some(if n == 0 {
                        Error::Eof
                    } else {
                        Error::UnexpectedEof
                    }),
                );
            }
            Ok(m) => n += m,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return (n, Some(Error::io(e))),
        }
    }
    (n, None)
}

// Go: compress/flate/inflate.go:noEOF
/// noEOF returns err, unless err == io.EOF, in which case it returns io.ErrUnexpectedEOF.
fn no_eof(e: Error) -> Error {
    if let Error::Eof = e {
        return Error::UnexpectedEof;
    }
    e
}

impl<R: BufRead> BitIn<R> {
    // Go: compress/flate/inflate.go:(*decompressor).moreBits
    fn more_bits(&mut self) -> Result<(), Error> {
        let c = read_byte(&mut self.r).map_err(no_eof)?;
        self.roffset += 1;
        self.b |= shl32(c as u32, self.nb);
        self.nb += 8;
        Ok(())
    }

    // Go: compress/flate/inflate.go:(*decompressor).huffSym
    /// Read the next Huffman-encoded symbol from f according to h.
    ///
    /// On a corrupt symbol Go also stores the error in `f.err`; the caller
    /// receives `Err(CorruptInput)` and stores it.
    fn huff_sym(&mut self, h: &HuffmanDecoder) -> Result<i64, Error> {
        // Since a huffmanDecoder can be empty or be composed of a degenerate tree
        // with single element, huffSym must error on these two edge cases. In both
        // cases, the chunks slice will be 0 for the invalid sequence, leading it
        // satisfy the n == 0 check below.
        let mut n = h.min as u32;
        // Optimization. Compiler isn't smart enough to keep f.b,f.nb in registers,
        // but is smart enough to keep local variables in registers, so use nb and b,
        // inline call to moreBits and reassign b,nb back to f on return.
        let (mut nb, mut b) = (self.nb, self.b);
        loop {
            while nb < n {
                let c = match read_byte(&mut self.r) {
                    Ok(c) => c,
                    Err(e) => {
                        self.b = b;
                        self.nb = nb;
                        return Err(no_eof(e));
                    }
                };
                self.roffset += 1;
                b |= (c as u32) << (nb & 31);
                nb += 8;
            }
            let mut chunk = h.chunks[(b & (HUFFMAN_NUM_CHUNKS as u32 - 1)) as usize];
            n = chunk & HUFFMAN_COUNT_MASK;
            if n > HUFFMAN_CHUNK_BITS {
                chunk = h.links[(chunk >> HUFFMAN_VALUE_SHIFT) as usize]
                    [((b >> HUFFMAN_CHUNK_BITS) & h.link_mask) as usize];
                n = chunk & HUFFMAN_COUNT_MASK;
            }
            if n <= nb {
                if n == 0 {
                    self.b = b;
                    self.nb = nb;
                    return Err(Error::CorruptInput(self.roffset));
                }
                self.b = b >> (n & 31);
                self.nb = nb - n;
                return Ok((chunk >> HUFFMAN_VALUE_SHIFT) as i64);
            }
        }
    }
}

/// Go's `x << s` for uint32 (shift counts >= 32 give 0).
#[inline(always)]
fn shl32(x: u32, s: u32) -> u32 {
    if s >= 32 { 0 } else { x << s }
}

/// Decompress state (Go's unexported `decompressor`, returned by
/// `NewReader` as an `io.ReadCloser`).
pub struct Decompressor<R: BufRead> {
    input: BitIn<R>,

    // Huffman decoders for literal/length, distance.
    h1: HuffmanDecoder,
    h2: HuffmanDecoder,

    // Length arrays used to define Huffman codes.
    bits: Box<[i64; MAX_NUM_LIT + MAX_NUM_DIST]>,
    codebits: Box<[i64; NUM_CODES]>,

    // Output history, buffer.
    dict: DictDecoder,

    // Temporary buffer (avoids repeated allocation).
    buf: [u8; 4],

    // Next step in the decompression,
    // and decompression state.
    step: Step,
    step_state: i32,
    final_: bool,
    err: Option<Error>,
    to_read: (usize, usize), // range of dict.hist
    hl: Hl,
    hd_fixed: bool, // f.hd == nil
    copy_len: i64,
    copy_dist: i64,
}

// Go: compress/flate/inflate.go:codeOrder
static CODE_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

const STATE_INIT: i32 = 0; // Zero value must be stateInit
const STATE_DICT: i32 = 1;

impl<R: BufRead> Decompressor<R> {
    fn run_step(&mut self) {
        match self.step {
            Step::NextBlock => self.next_block(),
            Step::HuffmanBlock => self.huffman_block(),
            Step::CopyData => self.copy_data(),
        }
    }

    // Go: compress/flate/inflate.go:(*decompressor).nextBlock
    fn next_block(&mut self) {
        while self.input.nb < 1 + 2 {
            if let Err(e) = self.input.more_bits() {
                self.err = Some(e);
                return;
            }
        }
        self.final_ = self.input.b & 1 == 1;
        self.input.b >>= 1;
        let typ = self.input.b & 3;
        self.input.b >>= 2;
        self.input.nb -= 1 + 2;
        match typ {
            0 => self.data_block(),
            1 => {
                // compressed, fixed Huffman tables
                self.hl = Hl::Fixed;
                self.hd_fixed = true;
                self.huffman_block();
            }
            2 => {
                // compressed, dynamic Huffman tables
                if let Err(e) = self.read_huffman() {
                    self.err = Some(e);
                    return;
                }
                self.hl = Hl::H1;
                self.hd_fixed = false;
                self.huffman_block();
            }
            _ => {
                // 3 is reserved.
                self.err = Some(Error::CorruptInput(self.input.roffset));
            }
        }
    }

    // Go: compress/flate/inflate.go:(*decompressor).Read
    /// Read reads decompressed data into b. Go semantics: returns the number
    /// of bytes read and an error (`Error::Eof` at the end of the stream);
    /// both may be set in the same call.
    pub fn read(&mut self, b: &mut [u8]) -> (usize, Option<Error>) {
        loop {
            if self.to_read.1 > self.to_read.0 {
                let (start, end) = self.to_read;
                let n = std::cmp::min(b.len(), end - start);
                b[..n].copy_from_slice(&self.dict.hist[start..start + n]);
                self.to_read.0 += n;
                if self.to_read.0 == self.to_read.1 {
                    return (n, self.err.clone());
                }
                return (n, None);
            }
            if let Some(e) = &self.err {
                return (0, Some(e.clone()));
            }
            self.run_step();
            if self.err.is_some() && self.to_read.0 == self.to_read.1 {
                self.to_read = self.dict.read_flush(); // Flush what's left in case of error
            }
        }
    }

    // Go: compress/flate/inflate.go:(*decompressor).Close
    pub fn close(&mut self) -> Result<(), Error> {
        match &self.err {
            None | Some(Error::Eof) => Ok(()),
            Some(e) => Err(e.clone()),
        }
    }

    // Go: compress/flate/inflate.go:(*decompressor).readHuffman
    fn read_huffman(&mut self) -> Result<(), Error> {
        let f = &mut self.input;
        // HLIT[5], HDIST[5], HCLEN[4].
        while f.nb < 5 + 5 + 4 {
            f.more_bits()?;
        }
        let nlit = (f.b & 0x1F) as usize + 257;
        if nlit > MAX_NUM_LIT {
            return Err(Error::CorruptInput(f.roffset));
        }
        f.b >>= 5;
        let ndist = (f.b & 0x1F) as usize + 1;
        if ndist > MAX_NUM_DIST {
            return Err(Error::CorruptInput(f.roffset));
        }
        f.b >>= 5;
        let nclen = (f.b & 0xF) as usize + 4;
        // numCodes is 19, so nclen is always valid.
        f.b >>= 4;
        f.nb -= 5 + 5 + 4;

        // (HCLEN+4)*3 bits: code lengths in the magic codeOrder order.
        for i in 0..nclen {
            while f.nb < 3 {
                f.more_bits()?;
            }
            self.codebits[CODE_ORDER[i]] = (f.b & 0x7) as i64;
            f.b >>= 3;
            f.nb -= 3;
        }
        for i in nclen..CODE_ORDER.len() {
            self.codebits[CODE_ORDER[i]] = 0;
        }
        if !self.h1.init(&self.codebits[..]) {
            return Err(Error::CorruptInput(f.roffset));
        }

        // HLIT + 257 code lengths, HDIST + 1 code lengths,
        // using the code length Huffman code.
        let n = nlit + ndist;
        let mut i = 0usize;
        while i < n {
            let x = f.huff_sym(&self.h1)?;
            if x < 16 {
                // Actual length.
                self.bits[i] = x;
                i += 1;
                continue;
            }
            // Repeat previous length or zero.
            let mut rep: usize;
            let nb: u32;
            let b: i64;
            match x {
                16 => {
                    rep = 3;
                    nb = 2;
                    if i == 0 {
                        return Err(Error::CorruptInput(f.roffset));
                    }
                    b = self.bits[i - 1];
                }
                17 => {
                    rep = 3;
                    nb = 3;
                    b = 0;
                }
                18 => {
                    rep = 11;
                    nb = 7;
                    b = 0;
                }
                _ => return Err(Error::Internal("unexpected length code".to_string())),
            }
            while f.nb < nb {
                f.more_bits()?;
            }
            rep += (f.b & ((1u32 << nb) - 1)) as usize;
            f.b >>= nb;
            f.nb -= nb;
            if i + rep > n {
                return Err(Error::CorruptInput(f.roffset));
            }
            for _ in 0..rep {
                self.bits[i] = b;
                i += 1;
            }
        }

        if !self.h1.init(&self.bits[0..nlit]) || !self.h2.init(&self.bits[nlit..nlit + ndist]) {
            return Err(Error::CorruptInput(f.roffset));
        }

        // As an optimization, we can initialize the min bits to read at a time
        // for the HLIT tree to the length of the EOB marker since we know that
        // every block must terminate with one. This preserves the property that
        // we never read any extra bytes after the end of the DEFLATE stream.
        if (self.h1.min as i64) < self.bits[END_BLOCK_MARKER] {
            self.h1.min = self.bits[END_BLOCK_MARKER] as usize;
        }

        Ok(())
    }

    // Go: compress/flate/inflate.go:(*decompressor).huffmanBlock
    /// Decode a single Huffman block from f.
    /// hl and hd are the Huffman states for the lit/length values
    /// and the distance values, respectively. If hd == nil, using the
    /// fixed distance encoding associated with fixed Huffman blocks.
    fn huffman_block(&mut self) {
        let mut copy_history = self.step_state == STATE_DICT;
        loop {
            if !copy_history {
                // readLiteral:
                // Read literal and/or (length, distance) according to RFC section 3.2.3.
                let hl: &HuffmanDecoder = match self.hl {
                    Hl::Fixed => fixed_huffman_decoder(),
                    Hl::H1 => &self.h1,
                };
                let v = match self.input.huff_sym(hl) {
                    Ok(v) => v,
                    Err(e) => {
                        self.err = Some(e);
                        return;
                    }
                };
                let n: u32; // number of bits extra
                let mut length: i64;
                if v < 256 {
                    self.dict.write_byte(v as u8);
                    if self.dict.avail_write() == 0 {
                        self.to_read = self.dict.read_flush();
                        self.step = Step::HuffmanBlock;
                        self.step_state = STATE_INIT;
                        return;
                    }
                    continue; // goto readLiteral
                } else if v == 256 {
                    self.finish_block();
                    return;
                // otherwise, reference to older data
                } else if v < 265 {
                    length = v - (257 - 3);
                    n = 0;
                } else if v < 269 {
                    length = v * 2 - (265 * 2 - 11);
                    n = 1;
                } else if v < 273 {
                    length = v * 4 - (269 * 4 - 19);
                    n = 2;
                } else if v < 277 {
                    length = v * 8 - (273 * 8 - 35);
                    n = 3;
                } else if v < 281 {
                    length = v * 16 - (277 * 16 - 67);
                    n = 4;
                } else if v < 285 {
                    length = v * 32 - (281 * 32 - 131);
                    n = 5;
                } else if v < MAX_NUM_LIT as i64 {
                    length = 258;
                    n = 0;
                } else {
                    self.err = Some(Error::CorruptInput(self.input.roffset));
                    return;
                }
                if n > 0 {
                    while self.input.nb < n {
                        if let Err(e) = self.input.more_bits() {
                            self.err = Some(e);
                            return;
                        }
                    }
                    length += (self.input.b & ((1u32 << n) - 1)) as i64;
                    self.input.b >>= n;
                    self.input.nb -= n;
                }

                let mut dist: i64;
                if self.hd_fixed {
                    while self.input.nb < 5 {
                        if let Err(e) = self.input.more_bits() {
                            self.err = Some(e);
                            return;
                        }
                    }
                    dist = (((self.input.b & 0x1F) << 3) as u8).reverse_bits() as i64;
                    self.input.b >>= 5;
                    self.input.nb -= 5;
                } else {
                    match self.input.huff_sym(&self.h2) {
                        Ok(d) => dist = d,
                        Err(e) => {
                            self.err = Some(e);
                            return;
                        }
                    }
                }

                if dist < 4 {
                    dist += 1;
                } else if dist < MAX_NUM_DIST as i64 {
                    let nb = ((dist - 2) as u64 >> 1) as u32;
                    // have 1 bit in bottom of dist, need nb more.
                    let mut extra = (dist & 1) << nb;
                    while self.input.nb < nb {
                        if let Err(e) = self.input.more_bits() {
                            self.err = Some(e);
                            return;
                        }
                    }
                    extra |= (self.input.b & ((1u32 << nb) - 1)) as i64;
                    self.input.b >>= nb;
                    self.input.nb -= nb;
                    dist = (1i64 << (nb + 1)) + 1 + extra;
                } else {
                    self.err = Some(Error::CorruptInput(self.input.roffset));
                    return;
                }

                // No check on length; encoding can be prescient.
                if dist > self.dict.hist_size() as i64 {
                    self.err = Some(Error::CorruptInput(self.input.roffset));
                    return;
                }

                self.copy_len = length;
                self.copy_dist = dist;
                copy_history = true;
                continue; // goto copyHistory
            }

            // copyHistory:
            // Perform a backwards copy according to RFC section 3.2.3.
            {
                let mut cnt = self
                    .dict
                    .try_write_copy(self.copy_dist as usize, self.copy_len as usize);
                if cnt == 0 {
                    cnt = self
                        .dict
                        .write_copy(self.copy_dist as usize, self.copy_len as usize);
                }
                self.copy_len -= cnt as i64;

                if self.dict.avail_write() == 0 || self.copy_len > 0 {
                    self.to_read = self.dict.read_flush();
                    self.step = Step::HuffmanBlock; // We need to continue this work
                    self.step_state = STATE_DICT;
                    return;
                }
                copy_history = false; // goto readLiteral
            }
        }
    }

    // Go: compress/flate/inflate.go:(*decompressor).dataBlock
    /// Copy a single uncompressed data block from input to output.
    fn data_block(&mut self) {
        // Uncompressed.
        // Discard current half-byte.
        self.input.nb = 0;
        self.input.b = 0;

        // Length then ones-complement of length.
        let (nr, err) = read_full(&mut self.input.r, &mut self.buf[0..4]);
        self.input.roffset += nr as i64;
        if let Some(e) = err {
            self.err = Some(no_eof(e));
            return;
        }
        let n = self.buf[0] as i64 | (self.buf[1] as i64) << 8;
        let nn = self.buf[2] as i64 | (self.buf[3] as i64) << 8;
        if nn as u16 != !n as u16 {
            self.err = Some(Error::CorruptInput(self.input.roffset));
            return;
        }

        if n == 0 {
            self.to_read = self.dict.read_flush();
            self.finish_block();
            return;
        }

        self.copy_len = n;
        self.copy_data();
    }

    // Go: compress/flate/inflate.go:(*decompressor).copyData
    /// copyData copies f.copyLen bytes from the underlying reader into f.hist.
    /// It pauses for reads when f.hist is full.
    fn copy_data(&mut self) {
        let copy_len = self.copy_len as usize;
        let buf = self.dict.write_slice();
        let len = std::cmp::min(buf.len(), copy_len);
        let buf = &mut buf[..len];

        let (cnt, err) = read_full(&mut self.input.r, buf);
        self.input.roffset += cnt as i64;
        self.copy_len -= cnt as i64;
        self.dict.write_mark(cnt);
        if let Some(e) = err {
            self.err = Some(no_eof(e));
            return;
        }

        if self.dict.avail_write() == 0 || self.copy_len > 0 {
            self.to_read = self.dict.read_flush();
            self.step = Step::CopyData;
            return;
        }
        self.finish_block();
    }

    // Go: compress/flate/inflate.go:(*decompressor).finishBlock
    fn finish_block(&mut self) {
        if self.final_ {
            if self.dict.avail_read() > 0 {
                self.to_read = self.dict.read_flush();
            }
            self.err = Some(Error::Eof);
        }
        self.step = Step::NextBlock;
    }

    // Go: compress/flate/inflate.go:(*decompressor).Reset
    /// Reset discards any buffered data and resets the decompressor as if it
    /// was newly initialized with the given reader. Returns the previous reader.
    pub fn reset(&mut self, r: R, dict: &[u8]) -> R {
        let old = std::mem::replace(&mut self.input.r, r);
        self.input.roffset = 0;
        self.input.b = 0;
        self.input.nb = 0;
        self.h1 = HuffmanDecoder::default();
        self.h2 = HuffmanDecoder::default();
        self.buf = [0; 4];
        self.step = Step::NextBlock;
        self.step_state = STATE_INIT;
        self.final_ = false;
        self.err = None;
        self.to_read = (0, 0);
        self.hl = Hl::Fixed;
        self.hd_fixed = true;
        self.copy_len = 0;
        self.copy_dist = 0;
        self.dict.init(MAX_MATCH_OFFSET, dict);
        old
    }

    /// Returns a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.input.r
    }

    /// Returns a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.input.r
    }

    /// Consumes the decompressor and returns the underlying reader.
    pub fn into_inner(self) -> R {
        self.input.r
    }
}

fn new_decompressor<R: BufRead>(r: R, dict: &[u8]) -> Decompressor<R> {
    let mut f = Decompressor {
        input: BitIn {
            r,
            roffset: 0,
            b: 0,
            nb: 0,
        },
        h1: HuffmanDecoder::default(),
        h2: HuffmanDecoder::default(),
        bits: Box::new([0; MAX_NUM_LIT + MAX_NUM_DIST]),
        codebits: Box::new([0; NUM_CODES]),
        dict: DictDecoder::default(),
        buf: [0; 4],
        step: Step::NextBlock,
        step_state: STATE_INIT,
        final_: false,
        err: None,
        to_read: (0, 0),
        hl: Hl::Fixed,
        hd_fixed: true,
        copy_len: 0,
        copy_dist: 0,
    };
    f.dict.init(MAX_MATCH_OFFSET, dict);
    f
}

// Go: compress/flate/inflate.go:NewReader
/// NewReader returns a new decompressor that can be used
/// to read the uncompressed version of r.
/// The reader returns `Error::Eof` after the final block in the DEFLATE stream has
/// been encountered. Any trailing data after the final block is ignored.
///
/// `r` plays the role of Go's `flate.Reader` (io.Reader + io.ByteReader).
pub fn new_reader<R: BufRead>(r: R) -> Decompressor<R> {
    let _ = fixed_huffman_decoder();
    new_decompressor(r, &[])
}

// Go: compress/flate/inflate.go:NewReaderDict
/// NewReaderDict is like [`new_reader`] but initializes the reader
/// with a preset dictionary. The returned reader behaves as if
/// the uncompressed data stream started with the given dictionary,
/// which has already been read. NewReaderDict is typically used
/// to read data compressed by NewWriterDict.
pub fn new_reader_dict<R: BufRead>(r: R, dict: &[u8]) -> Decompressor<R> {
    let _ = fixed_huffman_decoder();
    new_decompressor(r, dict)
}

/// `std::io::Read` adapter: data is returned first, a pending error on the
/// next call; `Error::Eof` becomes `Ok(0)`.
impl<R: BufRead> std::io::Read for Decompressor<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let (n, err) = Decompressor::read(self, buf);
        if n > 0 {
            return Ok(n);
        }
        match err {
            None => Ok(0),
            Some(Error::Eof) => Ok(0),
            Some(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: compress/flate/flate_test.go:TestIssue5915
    #[test]
    fn issue_5915() {
        let bits = [
            4, 0, 0, 6, 4, 3, 2, 3, 3, 4, 4, 5, 0, 0, 0, 0, 5, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 8, 6, 0, 11, 0,
            8, 0, 6, 6, 10, 8,
        ];
        let mut h = HuffmanDecoder::default();
        assert!(
            !h.init(&bits),
            "Given sequence of bits is bad, and should not succeed."
        );
    }

    // Go: compress/flate/flate_test.go:TestIssue5962
    #[test]
    fn issue_5962() {
        let bits = [
            4, 0, 0, 6, 4, 3, 2, 3, 3, 4, 4, 5, 0, 0, 0, 0, 5, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            11,
        ];
        let mut h = HuffmanDecoder::default();
        assert!(!h.init(&bits));
    }

    // Go: compress/flate/flate_test.go:TestIssue6255
    #[test]
    fn issue_6255() {
        let bits1 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11];
        let bits2 = [11, 13];
        let mut h = HuffmanDecoder::default();
        assert!(
            h.init(&bits1),
            "Given sequence of bits is good and should succeed."
        );
        assert!(
            !h.init(&bits2),
            "Given sequence of bits is bad and should not succeed."
        );
    }

    // Go: compress/flate/flate_test.go:TestInvalidEncoding
    #[test]
    fn invalid_encoding() {
        let mut h = HuffmanDecoder::default();
        assert!(h.init(&[1]), "Failed to initialize Huffman decoder");
        let data: &[u8] = &[0xff];
        let mut input = BitIn {
            r: data,
            roffset: 0,
            b: 0,
            nb: 0,
        };
        assert!(
            input.huff_sym(&h).is_err(),
            "Should have rejected invalid bit sequence"
        );
    }

    // Go: compress/flate/flate_test.go:TestReaderEarlyEOF
    /// Read returns (n, io.EOF) instead of (n, nil) + (0, io.EOF) when possible.
    #[test]
    fn reader_early_eof() {
        const WINDOW_SIZE: usize = 1 << 15;
        let test_sizes = [
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            100,
            1000,
            10000,
            100000,
            128,
            1024,
            16384,
            131072,
            // Testing multiples of windowSize triggers the case
            // where Read will fail to return an early io.EOF.
            WINDOW_SIZE,
            WINDOW_SIZE * 2,
            WINDOW_SIZE * 3,
        ];
        let max_size = *test_sizes.iter().max().unwrap();
        let mut read_buf = [0u8; 40];
        let data: Vec<u8> = (0..max_size).map(|i| i as u8).collect();
        for &sz in &test_sizes {
            for flush in [true, false] {
                let mut early_eof = true; // Do we expect early io.EOF?
                let mut w = crate::flate::new_writer(Vec::new(), 5).unwrap();
                w.write(&data[..sz]).unwrap();
                if flush {
                    // If a Flush occurs after all the actual data, the flushing
                    // semantics dictate that we will observe a (0, io.EOF) since
                    // Read must return data before it knows that the stream ended.
                    w.flush().unwrap();
                    early_eof = false;
                }
                w.close().unwrap();
                let buf = w.into_inner();

                let mut r = new_reader(&buf[..]);
                loop {
                    let (n, err) = r.read(&mut read_buf);
                    if let Some(Error::Eof) = err {
                        // If the availWrite == windowSize, then that means that the
                        // previous Read returned because the write buffer was full
                        // and it just so happened that the stream had no more data.
                        // This situation is rare, but unavoidable.
                        if r.dict.avail_write() == WINDOW_SIZE {
                            early_eof = false;
                        }
                        assert!(
                            !(n == 0 && early_eof),
                            "On size:{sz} flush:{flush}, Read() = (0, io.EOF), want (n, io.EOF)"
                        );
                        assert!(
                            !(n != 0 && !early_eof),
                            "On size:{sz} flush:{flush}, Read() = ({n}, io.EOF), want (0, io.EOF)"
                        );
                        break;
                    }
                    assert!(err.is_none(), "{err:?}");
                }
            }
        }
    }

    // Go: compress/flate/reader_test.go:TestNlitOutOfRange
    /// Bogus flate data with a Huffman table with nlit=288 must not panic.
    #[test]
    fn nlit_out_of_range() {
        let data: &[u8] = b"\xfc\xfe\x36\xe7\x5e\x1c\xef\xb3\x55\x58\x77\xb6\x56\xb5\x43\xf4\
\x6f\xf2\xd2\xe6\x3d\x99\xa0\x85\x8c\x48\xeb\xf8\xda\x83\x04\x2a\
\x75\xc4\xf8\x0f\x12\x11\xb9\xb4\x4b\x09\xa0\xbe\x8b\x91\x4c";
        assert_eq!(data.len(), 47);
        let mut r = new_reader(data);
        let mut buf = [0u8; 512];
        loop {
            let (_, err) = r.read(&mut buf);
            if let Some(e) = err {
                assert!(matches!(e, Error::CorruptInput(_)), "{e}");
                break;
            }
        }
    }

    // Go: compress/flate/flate_test.go:TestInvalidBits
    #[test]
    fn invalid_bits() {
        let oversubscribed = [1, 2, 3, 4, 4, 5];
        let incomplete = [1, 2, 4, 4];
        let mut h = HuffmanDecoder::default();
        assert!(
            !h.init(&oversubscribed),
            "Should reject oversubscribed bit-length set"
        );
        assert!(
            !h.init(&incomplete),
            "Should reject incomplete bit-length set"
        );
    }
}
