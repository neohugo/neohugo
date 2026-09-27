//! Port of go1.27.1 `compress/flate/huffman_bit_writer.go`.

use std::io::Write;
use std::sync::OnceLock;

use super::MAX_STORE_BLOCK_SIZE;
use super::deflatefast::store_le64;
use super::huffman_code::{
    Hcode, HuffmanEncoder, LITERAL_COUNT, fixed_literal_encoding, fixed_offset_encoding, histogram,
    new_huffman_encoder,
};
use super::token::{
    MATCH_OFFSET_ONLY_MASK, Token, Tokens, length_code, token_length, token_offset,
};
use crate::error::Error;

/// The largest offset code.
pub(crate) const OFFSET_CODE_COUNT: usize = 30;

/// The special code used to mark the end of a block.
pub(crate) const END_BLOCK_MARKER: usize = 256;

/// The first length code.
pub(crate) const LENGTH_CODES_START: usize = 257;

/// The number of codegen codes.
pub(crate) const CODEGEN_CODE_COUNT: usize = 19;
const BAD_CODE: u8 = 255;

/// maxPredefinedTokens is the maximum number of tokens
/// where we check if fixed size is smaller.
const MAX_PREDEFINED_TOKENS: u16 = 250;

/// bufferFlushSize indicates the buffer size
/// after which bytes are flushed to the writer.
/// Should preferably be a multiple of 6, since
/// we accumulate 6 bytes between writes to the buffer.
const BUFFER_FLUSH_SIZE: u8 = 246;

/// lengthExtraBitsMinCode is the minimum length code that emits extra bits.
const LENGTH_EXTRA_BITS_MIN_CODE: u8 = 8;

/// lengthExtraBits[i] is the number of extra bits needed by
/// length code i + lengthCodesStart.
pub(crate) static LENGTH_EXTRA_BITS: [u8; 32] = [
    /* 257 */ 0, 0, 0, /* 260 */ 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, /* 270 */ 2, 2, 2, 3,
    3, 3, 3, 4, 4, 4, /* 280 */ 4, 5, 5, 5, 5, 0, 0, 0, 0,
];

/// lengthBase[i] is the length indicated by length code i + lengthCodesStart.
static LENGTH_BASE: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128,
    160, 192, 224, 255, 0, 0, 0,
];

/// offsetExtraBitsMinCode is the minimum offset code that emits extra bits.
const OFFSET_EXTRA_BITS_MIN_CODE: u32 = 4;

/// offsetExtraBits[i] is the number of extra bits for offset code i.
pub(crate) static OFFSET_EXTRA_BITS: [i8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, /* extended window */ 14, 14,
];

/// offsetCombined combines offset lookup of extra bits and offset code in a single table.
static OFFSET_COMBINED: [u32; 32] = [
    0x0, 0x0, 0x0, 0x0, 0x401, 0x601, 0x802, 0xc02, 0x1003, 0x1803, 0x2004, 0x3004, 0x4005, 0x6005,
    0x8006, 0xc006, 0x10007, 0x18007, 0x20008, 0x30008, 0x40009, 0x60009, 0x8000a, 0xc000a,
    0x10000b, 0x18000b, 0x20000c, 0x30000c, 0x40000d, 0x60000d, 0x0, 0x0,
];

/// codegenOrder is the order in which codegen code sizes are written.
static CODEGEN_ORDER: [u32; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Go's `x << s` for unsigned x: shifts of 64 or more give 0
/// (arm64 `reg8SizeMask64 = 0xff` keeps the full shift count).
#[inline(always)]
fn shl64(x: u64, s: u8) -> u64 {
    if s >= 64 { 0 } else { x << s }
}

/// Which code tables `writeTokens` uses: Go passes `codes` slices of either
/// the fixed encoders or the writer's own encoders.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeSel {
    Fixed,
    Own,
}

/// huffmanBitWriter encodes tokens and values to a stream.
/// The huffmanBitWriter supports reusing huffman tables and will combine
/// blocks, if compression is less than creating a new table.
///
/// An incoming block estimates the output size of a new table using a
/// 'fresh' by calculating the optimal size and adding a penalty.
/// A Huffman table is not optimal, which is why we add a penalty,
/// and generating a new table is slower for both compression and decompression.
pub(crate) struct HuffmanBitWriter<W: Write> {
    // writer is the underlying writer.
    // Do not use it directly; use the write method, which ensures
    // that Write errors are sticky.
    pub(crate) writer: W,

    // Data waiting to be written is bytes[0:nbytes]
    // and then the low nbits of bits.
    bits: u64,
    nbits: u8,
    nbytes: u8,

    // If wroteHuffman is set, a table for outputting only literals
    // has been generated and offsets are invalid.
    wrote_huffman: bool,
    literal_encoding: Box<HuffmanEncoder>,
    tmp_lit_encoding: Box<HuffmanEncoder>,
    offset_encoding: Box<HuffmanEncoder>,
    codegen_encoding: Box<HuffmanEncoder>,
    pub(crate) err: Option<Error>,

    // If prevHeader is non-zero the Huffman table can be reused.
    // It also indicates that an EOB has not yet been emitted, so if a new table
    // is generated, an EOB with the previous table must be written.
    prev_header: i64,

    // logNewTablePenalty is a log2 penalty reduction for creating new tables.
    // The initial penalty is 100%.
    // Adding 1 will cut the penalty in half.
    pub(crate) log_new_table_penalty: u32,
    bytes: [u8; 256 + 8],
    literal_freq: [u16; LENGTH_CODES_START + 32],
    offset_freq: [u16; 32],
    codegen_freq: [u16; CODEGEN_CODE_COUNT],

    // codegen must have an extra space for the final symbol.
    codegen: [u8; LITERAL_COUNT + OFFSET_CODE_COUNT + 1],
}

// Go: compress/flate/huffman_bit_writer.go:newHuffmanBitWriter
/// newHuffmanBitWriter creates a new huffmanBitWriter that will write to w.
pub(crate) fn new_huffman_bit_writer<W: Write>(w: W) -> HuffmanBitWriter<W> {
    HuffmanBitWriter {
        writer: w,
        bits: 0,
        nbits: 0,
        nbytes: 0,
        wrote_huffman: false,
        literal_encoding: Box::new(new_huffman_encoder(LITERAL_COUNT)),
        tmp_lit_encoding: Box::new(new_huffman_encoder(LITERAL_COUNT)),
        codegen_encoding: Box::new(new_huffman_encoder(CODEGEN_CODE_COUNT)),
        offset_encoding: Box::new(new_huffman_encoder(OFFSET_CODE_COUNT)),
        err: None,
        prev_header: 0,
        log_new_table_penalty: 0,
        bytes: [0; 256 + 8],
        literal_freq: [0; LENGTH_CODES_START + 32],
        offset_freq: [0; 32],
        codegen_freq: [0; CODEGEN_CODE_COUNT],
        codegen: [0; LITERAL_COUNT + OFFSET_CODE_COUNT + 1],
    }
}

// Go: compress/flate/huffman_bit_writer.go:huffOffset
/// huffOffset is a static offset encoder used for Huffman-only encoding.
/// It can be reused since we will not be encoding offset values.
fn huff_offset() -> &'static HuffmanEncoder {
    static ENC: OnceLock<HuffmanEncoder> = OnceLock::new();
    ENC.get_or_init(|| {
        let mut offset_freq = [0u16; 32];
        offset_freq[0] = 1;
        let mut h = new_huffman_encoder(OFFSET_CODE_COUNT);
        h.generate(&offset_freq[..OFFSET_CODE_COUNT], 15);
        h
    })
}

/// Writes `b` to `writer` unless `err` is set, recording the error.
/// Go: `_, w.err = w.writer.Write(b)` — one Go `Write` call is one
/// `write_all` call on the Rust writer.
#[inline]
pub(crate) fn write_to<W: Write>(writer: &mut W, err: &mut Option<Error>, b: &[u8]) {
    if let Err(e) = crate::go_write(writer, b) {
        *err = Some(Error::io(e));
    }
}

impl<W: Write> HuffmanBitWriter<W> {
    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).reset
    /// reset the huffmanBitWriter state and replace the output.
    pub(crate) fn reset(&mut self, writer: W) -> W {
        let old = std::mem::replace(&mut self.writer, writer);
        self.reset_state();
        old
    }

    /// The state part of Go's reset (used by `close`, which calls
    /// `d.w.reset(nil)`; the Rust port keeps the writer so it can be
    /// retrieved).
    pub(crate) fn reset_state(&mut self) {
        self.bits = 0;
        self.nbits = 0;
        self.nbytes = 0;
        self.err = None;
        self.prev_header = 0;
        self.wrote_huffman = false;
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).canReuse
    /// canReuse checks if the current generated tables can be
    /// reused for the provided tokens.
    fn can_reuse(&self, t: &Tokens) -> bool {
        let a = &t.off_hist[..OFFSET_CODE_COUNT];
        let b = &self.offset_encoding.codes[..a.len()];
        for (i, &v) in a.iter().enumerate() {
            if v != 0 && b[i].zero() {
                return false;
            }
        }

        let a = &t.extra_hist[..LITERAL_COUNT - 256];
        let b = &self.literal_encoding.codes[256..LITERAL_COUNT];
        let b = &b[..a.len()];
        for (i, &v) in a.iter().enumerate() {
            if v != 0 && b[i].zero() {
                return false;
            }
        }

        let a = &t.lit_hist[..256];
        let b = &self.literal_encoding.codes[..a.len()];
        for (i, &v) in a.iter().enumerate() {
            if v != 0 && b[i].zero() {
                return false;
            }
        }
        true
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).flush
    /// flush flushes the currently encoded data.
    /// An EOB will be written if the current block hasn't been ended.
    pub(crate) fn flush(&mut self) {
        if self.err.is_some() {
            self.nbits = 0;
            return;
        }
        if self.prev_header > 0 {
            // We owe an EOB
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
        }
        let mut n = self.nbytes as usize;
        while self.nbits != 0 {
            self.bytes[n] = self.bits as u8;
            self.bits >>= 8;
            if self.nbits > 8 {
                // Avoid underflow
                self.nbits -= 8;
            } else {
                self.nbits = 0;
            }
            n += 1;
        }
        self.bits = 0;
        if n > 0 {
            self.write_buf(n);
        }
        self.nbytes = 0;
    }

    /// Go: `w.write(w.bytes[:n])`.
    #[inline]
    fn write_buf(&mut self, n: usize) {
        if self.err.is_some() {
            return;
        }
        write_to(&mut self.writer, &mut self.err, &self.bytes[..n]);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).write
    /// write writes the provided bytes directly to the output,
    /// ignoring all queued bytes.
    fn write(&mut self, b: &[u8]) {
        if self.err.is_some() {
            return;
        }
        write_to(&mut self.writer, &mut self.err, b);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeBits
    /// writeBits writes nb bits from b to the stream.
    #[inline(always)]
    fn write_bits(&mut self, b: i32, nb: u8) {
        self.bits |= (b as i64 as u64) << (self.nbits & 63);
        self.nbits = self.nbits.wrapping_add(nb);
        if self.nbits >= 48 {
            self.flush_bits();
        }
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeBytes
    /// writeBytes writes the provided bytes to the stream.
    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        if self.err.is_some() {
            return;
        }
        let mut n = self.nbytes as usize;
        if self.nbits & 7 != 0 {
            self.err = Some(Error::Internal(
                "writeBytes with unfinished bits".to_string(),
            ));
            return;
        }
        while self.nbits != 0 {
            self.bytes[n] = self.bits as u8;
            self.bits >>= 8;
            self.nbits -= 8;
            n += 1;
        }
        if n != 0 {
            self.write_buf(n);
        }
        self.nbytes = 0;
        self.write(bytes);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).generateCodegen
    /// RFC 1951 3.2.7 specifies a special run-length encoding for specifying
    /// the literal and offset lengths arrays (which are concatenated into a single
    /// array).  This method generates that run-length encoding.
    ///
    /// The result is written into the codegen array, and the frequencies
    /// of each code is written into the codegenFreq array.
    /// Codes 0-15 are single byte codes. Codes 16-18 are followed by additional
    /// information. Code badCode is an end marker
    ///
    ///    numLiterals      The number of literals in literalEncoding
    ///    numOffsets       The number of offsets in offsetEncoding
    ///    litenc, offenc   The literal and offset encoder to use
    ///
    /// `off_enc_huff` selects `huffOffset()` instead of `w.offsetEncoding`;
    /// the literal encoder is always `w.literalEncoding` at the call sites.
    fn generate_codegen(&mut self, num_literals: usize, num_offsets: usize, off_enc_huff: bool) {
        self.codegen_freq = [0; CODEGEN_CODE_COUNT];
        // Note that we are using codegen both as a temporary variable for holding
        // a copy of the frequencies, and as the place where we put the result.
        // This is fine because the output is always shorter than the input used
        // so far.
        let lit_enc: &HuffmanEncoder = &self.literal_encoding;
        let off_enc: &HuffmanEncoder = if off_enc_huff {
            huff_offset()
        } else {
            &self.offset_encoding
        };
        let codegen = &mut self.codegen; // cache
        // Copy the concatenated code sizes to codegen. Put a marker at the end.
        for i in 0..num_literals {
            codegen[i] = lit_enc.codes[i].len();
        }
        for i in 0..num_offsets {
            codegen[num_literals + i] = off_enc.codes[i].len();
        }
        codegen[num_literals + num_offsets] = BAD_CODE;

        let codegen_freq = &mut self.codegen_freq;
        let mut size = codegen[0];
        let mut count: i32 = 1;
        let mut out_index = 0usize;
        let mut in_index = 1usize;
        while size != BAD_CODE {
            // INVARIANT: We have seen "count" copies of size that have not yet
            // had output generated for them.
            let next_size = codegen[in_index];
            if next_size == size {
                count += 1;
                in_index += 1;
                continue;
            }
            // We need to generate codegen indicating "count" of size.
            if size != 0 {
                codegen[out_index] = size;
                out_index += 1;
                codegen_freq[size as usize] = codegen_freq[size as usize].wrapping_add(1);
                count -= 1;
                while count >= 3 {
                    let n = std::cmp::min(6, count);
                    codegen[out_index] = 16;
                    out_index += 1;
                    codegen[out_index] = (n - 3) as u8;
                    out_index += 1;
                    codegen_freq[16] = codegen_freq[16].wrapping_add(1);
                    count -= n;
                }
            } else {
                while count >= 11 {
                    let n = std::cmp::min(138, count);
                    codegen[out_index] = 18;
                    out_index += 1;
                    codegen[out_index] = (n - 11) as u8;
                    out_index += 1;
                    codegen_freq[18] = codegen_freq[18].wrapping_add(1);
                    count -= n;
                }
                if count >= 3 {
                    // count >= 3 && count <= 10
                    codegen[out_index] = 17;
                    out_index += 1;
                    codegen[out_index] = (count - 3) as u8;
                    out_index += 1;
                    codegen_freq[17] = codegen_freq[17].wrapping_add(1);
                    count = 0;
                }
            }
            count -= 1;
            while count >= 0 {
                codegen[out_index] = size;
                out_index += 1;
                codegen_freq[size as usize] = codegen_freq[size as usize].wrapping_add(1);
                count -= 1;
            }
            // Set up invariant for next time through the loop.
            size = next_size;
            count = 1;
            in_index += 1;
        }
        // Marker indicating the end of the codegen.
        codegen[out_index] = BAD_CODE;
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).codegens
    /// codegens returns current number of non-zero codegens.
    fn codegens(&self) -> usize {
        let mut num_codegens = self.codegen_freq.len();
        while num_codegens > 4 && self.codegen_freq[CODEGEN_ORDER[num_codegens - 1] as usize] == 0 {
            num_codegens -= 1;
        }
        num_codegens
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).headerSize
    /// headerSize returns the size of the header with the current encodings.
    fn header_size(&self) -> (i64, usize) {
        let mut num_codegens = self.codegen_freq.len();
        while num_codegens > 4 && self.codegen_freq[CODEGEN_ORDER[num_codegens - 1] as usize] == 0 {
            num_codegens -= 1;
        }
        (
            3 + 5
                + 5
                + 4
                + (3 * num_codegens as i64)
                + self.codegen_encoding.bit_length(&self.codegen_freq[..])
                + self.codegen_freq[16] as i64 * 2
                + self.codegen_freq[17] as i64 * 3
                + self.codegen_freq[18] as i64 * 7,
            num_codegens,
        )
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).dynamicReuseSize
    /// dynamicSize returns the size of dynamically encoded data in bits.
    fn dynamic_reuse_size(&self) -> i64 {
        self.literal_encoding.bit_length(&self.literal_freq[..])
            + self.offset_encoding.bit_length(&self.offset_freq[..])
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).dynamicSize
    /// dynamicSize returns the size of dynamically encoded data in bits.
    fn dynamic_size(&self, extra_bits: i64) -> (i64, usize) {
        let (header, num_codegens) = self.header_size();
        let size = header
            + self.literal_encoding.bit_length(&self.literal_freq[..])
            + self.offset_encoding.bit_length(&self.offset_freq[..])
            + extra_bits;
        (size, num_codegens)
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).extraBitSize
    /// extraBitSize returns the number of bits that will be written
    /// as "extra" bits on matches.
    fn extra_bit_size(&self) -> i64 {
        let mut total: i64 = 0;
        for (i, &n) in self.literal_freq[257..LITERAL_COUNT].iter().enumerate() {
            total += n as i64 * LENGTH_EXTRA_BITS[i & 31] as i64;
        }
        for (i, &n) in self.offset_freq[..OFFSET_CODE_COUNT].iter().enumerate() {
            total += n as i64 * OFFSET_EXTRA_BITS[i & 31] as i64;
        }
        total
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).fixedSize
    /// fixedSize returns the size of dynamically encoded data in bits.
    fn fixed_size(&self, extra_bits: i64) -> i64 {
        3 + fixed_literal_encoding().bit_length(&self.literal_freq[..])
            + fixed_offset_encoding().bit_length(&self.offset_freq[..])
            + extra_bits
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).storedSize
    /// storedSize calculates the stored size, including header.
    /// The function returns the size in bits and whether the block
    /// fits inside a single block.
    fn stored_size(&self, input: Option<&[u8]>) -> (i64, bool) {
        let Some(input) = input else {
            return (0, false);
        };
        if input.len() <= MAX_STORE_BLOCK_SIZE as usize {
            return ((input.len() as i64 + 5) * 8, true);
        }
        (0, false)
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeCode
    /// writeCode writes 'c' to the stream.
    #[inline(always)]
    fn write_code(&mut self, c: Hcode) {
        self.bits |= shl64(c.code64(), self.nbits);
        self.nbits = self.nbits.wrapping_add(c.len());
        if self.nbits >= 48 {
            self.flush_bits();
        }
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).flushBits
    /// flushBits writes accumulated bits to the byte buffer.
    fn flush_bits(&mut self) {
        let bits = self.bits;
        self.bits >>= 48;
        self.nbits -= 48;
        let mut n = self.nbytes;

        // We overwrite, but faster...
        store_le64(&mut self.bytes[n as usize..], bits);
        n += 6;

        if n >= BUFFER_FLUSH_SIZE {
            if self.err.is_some() {
                // Go: `n = 0; return` (w.nbytes is left unchanged).
                return;
            }
            self.write_buf(n as usize);
            n = 0;
        }

        self.nbytes = n;
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeDynamicHeader
    /// writeDynamicHeader writes the header of a dynamic Huffman block to the output stream.
    ///
    /// numLiterals is the number of literals specified in codegen.
    /// numOffsets is the number of offsets specified in codegen.
    /// numCodegens is the number of codegens used in codegen.
    fn write_dynamic_header(
        &mut self,
        num_literals: usize,
        num_offsets: usize,
        num_codegens: usize,
        is_eof: bool,
    ) {
        if self.err.is_some() {
            return;
        }
        let mut first_bits: i32 = 4;
        if is_eof {
            first_bits = 5;
        }
        self.write_bits(first_bits, 3);
        self.write_bits(num_literals as i32 - 257, 5);
        self.write_bits(num_offsets as i32 - 1, 5);
        self.write_bits(num_codegens as i32 - 4, 4);

        for i in 0..num_codegens {
            let value = self.codegen_encoding.codes[CODEGEN_ORDER[i] as usize].len() as u32;
            self.write_bits(value as i32, 3);
        }

        let mut i = 0usize;
        loop {
            let code_word = self.codegen[i] as u32;
            i += 1;
            if code_word == BAD_CODE as u32 {
                break;
            }
            let c = self.codegen_encoding.codes[code_word as usize];
            self.write_code(c);

            match code_word {
                16 => {
                    self.write_bits(self.codegen[i] as i32, 2);
                    i += 1;
                }
                17 => {
                    self.write_bits(self.codegen[i] as i32, 3);
                    i += 1;
                }
                18 => {
                    self.write_bits(self.codegen[i] as i32, 7);
                    i += 1;
                }
                _ => {}
            }
        }
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeStoredHeader
    /// writeStoredHeader writes a stored header.
    /// If the stored block is only used for EOF,
    /// it is replaced with a fixed huffman block.
    pub(crate) fn write_stored_header(&mut self, length: usize, is_eof: bool) {
        if self.err.is_some() {
            return;
        }
        if self.prev_header > 0 {
            // We owe an EOB
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
        }

        // To write EOF, use a fixed encoding block. 10 bits instead of 5 bytes.
        if length == 0 && is_eof {
            self.write_fixed_header(is_eof);
            // EOB: 7 bits, value: 0
            self.write_bits(0, 7);
            self.flush();
            return;
        }

        let mut flag: i32 = 0;
        if is_eof {
            flag = 1;
        }
        self.write_bits(flag, 3);
        self.flush();
        self.write_bits(length as i32, 16);
        self.write_bits(!(length as u16) as i32, 16);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeFixedHeader
    /// writeFixedHeader writes a fixed encoding header to the output stream.
    fn write_fixed_header(&mut self, is_eof: bool) {
        if self.err.is_some() {
            return;
        }
        if self.prev_header > 0 {
            // We owe an EOB
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
        }

        // Indicate that we are a fixed Huffman block
        let mut value: i32 = 2;
        if is_eof {
            value = 3;
        }
        self.write_bits(value, 3);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeBlock
    /// writeBlock writes a block of tokens using the smallest encoding.
    /// The original input can be supplied, and if the Huffman-encoded data
    /// is larger than the original bytes, the data will be written as a
    /// stored block.
    /// If the input is nil, the tokens will always be Huffman encoded.
    pub(crate) fn write_block(&mut self, tokens: &mut Tokens, eof: bool, input: Option<&[u8]>) {
        if self.err.is_some() {
            return;
        }

        tokens.add_eob();
        if self.prev_header > 0 {
            // We owe an EOB
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
        }
        let (num_literals, num_offsets) = self.index_tokens(tokens);
        self.generate();
        let mut extra_bits: i64 = 0;
        let (stored_size, storable) = self.stored_size(input);
        if storable {
            extra_bits = self.extra_bit_size();
        }

        // Figure out smallest code.
        // Fixed Huffman baseline.
        let mut sel = CodeSel::Fixed;
        let mut size: i64 = i32::MAX as i64;
        if tokens.n < MAX_PREDEFINED_TOKENS {
            size = self.fixed_size(extra_bits);
        }

        // Dynamic Huffman?

        // Generate codegen and codegenFrequencies, which indicates how to encode
        // the literalEncoding and the offsetEncoding.
        self.generate_codegen(num_literals, num_offsets, false);
        self.codegen_encoding.generate(&self.codegen_freq, 7);
        let (dynamic_size, num_codegens) = self.dynamic_size(extra_bits);

        if dynamic_size < size {
            size = dynamic_size;
            sel = CodeSel::Own;
        }

        // Stored bytes?
        if storable && stored_size <= size {
            let input = input.unwrap();
            self.write_stored_header(input.len(), eof);
            self.write_bytes(input);
            return;
        }

        // Huffman.
        if sel == CodeSel::Fixed {
            self.write_fixed_header(eof);
        } else {
            self.write_dynamic_header(num_literals, num_offsets, num_codegens, eof);
        }

        // Write the tokens.
        self.write_tokens(tokens.slice(), sel);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeBlockDynamic
    /// writeBlockDynamic encodes a block using a dynamic Huffman table.
    /// This should be used if the symbols used have a disproportionate
    /// histogram distribution.
    pub(crate) fn write_block_dynamic(
        &mut self,
        tokens: &mut Tokens,
        eof: bool,
        input: Option<&[u8]>,
        mut sync: bool,
    ) {
        if self.err.is_some() {
            return;
        }

        sync = sync || eof;
        if sync {
            tokens.add_eob();
        } else {
            // Ensure we can always write EOB.
            tokens.extra_hist[0] = 1;
        }

        // We cannot reuse pure Huffman table, and must mark as EOF.
        if (self.wrote_huffman || eof) && self.prev_header > 0 {
            // We will not try to reuse.
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
            self.wrote_huffman = false;
        }

        if self.prev_header > 0 && !self.can_reuse(tokens) {
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
        }

        let (num_literals, num_offsets) = self.index_tokens(tokens);
        let mut extra_bits: i64 = 0;
        let (ssize, storable) = self.stored_size(input);

        if storable || self.prev_header > 0 {
            extra_bits = self.extra_bit_size();
        }

        let mut size: i64;

        // Check whether we should reuse the previous Huffman table.
        if self.prev_header > 0 {
            // Estimate size for using a new table.
            // Use the previous header size as the best estimate.
            let mut new_size = self.prev_header + tokens.estimated_bits();

            // The estimated size is calculated as an optimal table.
            // We add a penalty to make it more realistic and re-use a bit more.
            new_size += self.literal_encoding.codes[END_BLOCK_MARKER].len() as i64
                + go_shr_i64(new_size, self.log_new_table_penalty);

            // Calculate the size for reusing the current table.
            let reuse_size = self.dynamic_reuse_size() + extra_bits;

            // Check if a new table is better.
            if new_size < reuse_size {
                // Write the EOB we owe.
                let c = self.literal_encoding.codes[END_BLOCK_MARKER];
                self.write_code(c);
                size = new_size;
                self.prev_header = 0;
            } else {
                size = reuse_size;
            }

            // Small blocks can be more efficient with fixed encoding.
            if tokens.n < MAX_PREDEFINED_TOKENS {
                let pre_size = self.fixed_size(extra_bits) + 7;
                if pre_size < size {
                    // Check if we get a reasonable size decrease.
                    if storable && ssize <= size {
                        let input = input.unwrap();
                        self.write_stored_header(input.len(), eof);
                        self.write_bytes(input);
                        return;
                    }
                    self.write_fixed_header(eof);
                    if !sync {
                        tokens.add_eob();
                    }
                    self.write_tokens(tokens.slice(), CodeSel::Fixed);
                    return;
                }
            }

            // Check if we get a reasonable size decrease.
            if storable && ssize <= size {
                let input = input.unwrap();
                self.write_stored_header(input.len(), eof);
                self.write_bytes(input);
                return;
            }
        }

        // We want a new block/table
        if self.prev_header == 0 {
            self.literal_freq[END_BLOCK_MARKER] = 1;

            self.generate();
            // Generate codegen and codegenFrequencies, which indicates how to encode
            // the literalEncoding and the offsetEncoding.
            self.generate_codegen(num_literals, num_offsets, false);
            self.codegen_encoding.generate(&self.codegen_freq, 7);

            let num_codegens;
            (size, num_codegens) = self.dynamic_size(extra_bits);

            // Store predefined or raw, if we don't get a reasonable improvement.
            if tokens.n < MAX_PREDEFINED_TOKENS {
                let pre_size = self.fixed_size(extra_bits);
                if pre_size <= size {
                    // Store bytes, if we don't get an improvement.
                    if storable && ssize <= pre_size {
                        let input = input.unwrap();
                        self.write_stored_header(input.len(), eof);
                        self.write_bytes(input);
                        return;
                    }
                    self.write_fixed_header(eof);
                    if !sync {
                        tokens.add_eob();
                    }
                    self.write_tokens(tokens.slice(), CodeSel::Fixed);
                    return;
                }
            }

            if storable && ssize <= size {
                // Store bytes, if we don't get an improvement.
                let input = input.unwrap();
                self.write_stored_header(input.len(), eof);
                self.write_bytes(input);
                return;
            }

            // Write Huffman table.
            self.write_dynamic_header(num_literals, num_offsets, num_codegens, eof);
            if !sync {
                (self.prev_header, _) = self.header_size();
            }
            self.wrote_huffman = false;
        }

        if sync {
            self.prev_header = 0;
        }
        // Write the tokens.
        self.write_tokens(tokens.slice(), CodeSel::Own);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).indexTokens
    /// indexTokens indexes a slice of tokens, updates literalFreq and offsetFreq,
    /// and generates literalEncoding and offsetEncoding.
    /// It returns the number of literal and offset tokens.
    fn index_tokens(&mut self, t: &Tokens) -> (usize, usize) {
        self.literal_freq[..256].copy_from_slice(&t.lit_hist);
        self.literal_freq[256..288].copy_from_slice(&t.extra_hist);
        self.offset_freq = t.off_hist;

        if t.n == 0 {
            return (0, 0);
        }
        // get the number of literals
        let mut num_literals = self.literal_freq.len();
        while self.literal_freq[num_literals - 1] == 0 {
            num_literals -= 1;
        }
        // get the number of offsets
        let mut num_offsets = self.offset_freq.len();
        while num_offsets > 0 && self.offset_freq[num_offsets - 1] == 0 {
            num_offsets -= 1;
        }
        if num_offsets == 0 {
            // We haven't found a single match. If we want to go with the dynamic encoding,
            // we should count at least one offset to be sure that the offset huffman tree could be encoded.
            self.offset_freq[0] = 1;
            num_offsets = 1;
        }
        (num_literals, num_offsets)
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).generate
    /// generate literalEncoding and offsetEncoding based on respective histograms.
    fn generate(&mut self) {
        self.literal_encoding
            .generate(&self.literal_freq[..LITERAL_COUNT], 15);
        self.offset_encoding
            .generate(&self.offset_freq[..OFFSET_CODE_COUNT], 15);
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeTokens
    /// writeTokens writes a slice of tokens to the output.
    /// Codes for literal and offset encoding must be supplied.
    fn write_tokens(&mut self, mut tokens: &[Token], sel: CodeSel) {
        if self.err.is_some() {
            return;
        }
        if tokens.is_empty() {
            return;
        }

        // Only last token should be endBlockMarker.
        let mut defer_eob = false;
        if tokens[tokens.len() - 1] == END_BLOCK_MARKER as Token {
            tokens = &tokens[..tokens.len() - 1];
            defer_eob = true;
        }

        let (len_codes, off_codes): (&[Hcode], &[Hcode]) = match sel {
            CodeSel::Fixed => (
                &fixed_literal_encoding().codes,
                &fixed_offset_encoding().codes,
            ),
            CodeSel::Own => (&self.literal_encoding.codes, &self.offset_encoding.codes),
        };

        // Create slices up to the next power of two to avoid bounds checks.
        let lits = &len_codes[..256];
        let offs = &off_codes[..32];
        let lengths = &len_codes[LENGTH_CODES_START..];
        let lengths = &lengths[..32];
        let eob_code = len_codes[END_BLOCK_MARKER];

        // Go 1.16 LOVES having these on stack.
        let (mut bits, mut nbits, mut nbytes) = (self.bits, self.nbits, self.nbytes);

        macro_rules! flush48 {
            () => {
                if nbits >= 48 {
                    store_le64(&mut self.bytes[nbytes as usize..], bits);
                    bits >>= 48;
                    nbits -= 48;
                    nbytes += 6;
                    if nbytes >= BUFFER_FLUSH_SIZE {
                        if self.err.is_some() {
                            // Go: `nbytes = 0; return` without restoring w.bits/w.nbits/w.nbytes.
                            return;
                        }
                        write_to(
                            &mut self.writer,
                            &mut self.err,
                            &self.bytes[..nbytes as usize],
                        );
                        nbytes = 0;
                    }
                }
            };
        }

        for &t in tokens {
            if t < 256 {
                let c = lits[t as usize];
                bits |= c.code64() << (nbits & 63);
                nbits = nbits.wrapping_add(c.len());
                flush48!();
                continue;
            }

            // Write the length
            let length = token_length(t);
            let len_code = length_code(length) & 31;
            // inlined 'w.writeCode(lengths[lengthCode])'
            let c = lengths[len_code as usize];
            bits |= c.code64() << (nbits & 63);
            nbits = nbits.wrapping_add(c.len());
            flush48!();

            if len_code >= LENGTH_EXTRA_BITS_MIN_CODE {
                let extra_length_bits = LENGTH_EXTRA_BITS[len_code as usize];
                //w.writeBits(extraLength, extraLengthBits)
                let extra_length = length.wrapping_sub(LENGTH_BASE[len_code as usize]) as i32;
                bits |= (extra_length as i64 as u64) << (nbits & 63);
                nbits = nbits.wrapping_add(extra_length_bits);
                flush48!();
            }
            // Write the offset
            let offset = token_offset(t);
            let off_code = (offset >> 16) & 31;
            // inlined 'w.writeCode(offs[offCode])'
            let c = offs[off_code as usize];
            bits |= c.code64() << (nbits & 63);
            nbits = nbits.wrapping_add(c.len());
            flush48!();

            if off_code >= OFFSET_EXTRA_BITS_MIN_CODE {
                let offset_comb = OFFSET_COMBINED[off_code as usize];
                bits |= ((offset.wrapping_sub(offset_comb >> 8) & MATCH_OFFSET_ONLY_MASK) as u64)
                    << (nbits & 63);
                nbits = nbits.wrapping_add(offset_comb as u8);
                flush48!();
            }
        }
        // Restore...
        self.bits = bits;
        self.nbits = nbits;
        self.nbytes = nbytes;

        if defer_eob {
            self.write_code(eob_code);
        }
    }

    // Go: compress/flate/huffman_bit_writer.go:(*huffmanBitWriter).writeBlockHuff
    /// writeBlockHuff encodes a block of bytes as either
    /// Huffman-encoded literals or uncompressed bytes if the
    /// results gain very little from compression.
    ///
    /// FMA (arm64, verified with `go tool objdump`): `avg` is compiled as a
    /// multiplication by 1/256 fused into `float64(v) - avg` (FMSUBD, exact
    /// either way) and `abs += diff * diff` is FMADDD.
    pub(crate) fn write_block_huff(&mut self, eof: bool, mut input: &[u8], sync: bool) {
        if self.err.is_some() {
            return;
        }

        // Clear histogram
        self.literal_freq = [0; LENGTH_CODES_START + 32];
        if !self.wrote_huffman {
            self.offset_freq = [0; 32];
        }

        const NUM_LITERALS: usize = END_BLOCK_MARKER + 1;
        const NUM_OFFSETS: usize = 1;

        // Estimate size of literal encoding.
        const GUESS_HEADER_SIZE_BITS: i64 = 70 * 8; // 70 bytes; see https://stackoverflow.com/a/25454430
        histogram(input, &mut self.literal_freq[..NUM_LITERALS]);
        let (ssize, storable) = self.stored_size(Some(input));
        if storable && input.len() > 1024 {
            // Quick check for incompressible content.
            // The following checks if all frequencies lie
            // close to the average frequency.
            // If so, we quickly store the data uncompressed.
            // This will typically only trigger on random data.
            // Most other data will typically exit after only a few iterations.
            let mut abs = 0f64;
            let len_f = input.len() as f64;
            let max = (input.len() * 2) as f64;
            for &v in self.literal_freq[..256].iter() {
                // Go: diff := float64(v) - avg, with avg = float64(len)/256
                // compiled as FMSUBD(float64(v), float64(len), 1/256).
                let diff = (-len_f).mul_add(1.0 / 256.0, v as f64);
                abs = diff.mul_add(diff, abs);
                if abs >= max {
                    break;
                }
            }
            if abs < max {
                // No chance we can compress this...
                self.write_stored_header(input.len(), eof);
                self.write_bytes(input);
                return;
            }
        }
        self.literal_freq[END_BLOCK_MARKER] = 1;
        self.tmp_lit_encoding
            .generate(&self.literal_freq[..NUM_LITERALS], 15);
        let mut est_bits = self
            .tmp_lit_encoding
            .can_encode_len(&self.literal_freq[..NUM_LITERALS]);
        if est_bits < i32::MAX as i64 {
            est_bits += self.prev_header;
            if self.prev_header == 0 {
                est_bits += GUESS_HEADER_SIZE_BITS;
            }
            est_bits += go_shr_i64(est_bits, self.log_new_table_penalty);
        }

        // Store bytes, if we don't get a reasonable improvement.
        if storable && ssize <= est_bits {
            self.write_stored_header(input.len(), eof);
            self.write_bytes(input);
            return;
        }

        if self.prev_header > 0 {
            let reuse_size = self
                .literal_encoding
                .can_encode_len(&self.literal_freq[..256]);
            if est_bits < reuse_size {
                // We owe an EOB
                let c = self.literal_encoding.codes[END_BLOCK_MARKER];
                self.write_code(c);
                self.prev_header = 0;
            }
        }

        if self.prev_header == 0 {
            // Use the temp encoding, so swap.
            std::mem::swap(&mut self.literal_encoding, &mut self.tmp_lit_encoding);
            // Generate codegen and codegenFrequencies, which indicates how to encode
            // the literalEncoding and the offsetEncoding.
            self.generate_codegen(NUM_LITERALS, NUM_OFFSETS, true);
            self.codegen_encoding.generate(&self.codegen_freq, 7);
            let num_codegens = self.codegens();

            // Huffman.
            self.write_dynamic_header(NUM_LITERALS, NUM_OFFSETS, num_codegens, eof);
            self.wrote_huffman = true;
            (self.prev_header, _) = self.header_size();
        }

        let encoding = &self.literal_encoding.codes[..256];
        // Go 1.16 LOVES having these on stack. At least 1.5x the speed.
        let (mut bits, mut nbits, mut nbytes) = (self.bits, self.nbits, self.nbytes);

        // Unroll, write 3 codes/loop.
        // Fastest number of unrolls.
        while input.len() > 3 {
            // We must have at least 48 bits free.
            if nbits >= 8 {
                let n = nbits >> 3;
                store_le64(&mut self.bytes[nbytes as usize..], bits);
                bits >>= (n * 8) & 63;
                nbits -= n * 8;
                nbytes += n;
            }
            if nbytes >= BUFFER_FLUSH_SIZE {
                if self.err.is_some() {
                    // Go: `nbytes = 0; return` without restoring state.
                    return;
                }
                write_to(
                    &mut self.writer,
                    &mut self.err,
                    &self.bytes[..nbytes as usize],
                );
                nbytes = 0;
            }
            let (a, b) = (encoding[input[0] as usize], encoding[input[1] as usize]);
            bits |= a.code64() << (nbits & 63);
            bits |= b.code64() << (nbits.wrapping_add(a.len()) & 63);
            let c = encoding[input[2] as usize];
            nbits = nbits.wrapping_add(b.len().wrapping_add(a.len()));
            bits |= c.code64() << (nbits & 63);
            nbits = nbits.wrapping_add(c.len());
            input = &input[3..];
        }

        // Remaining...
        for &t in input {
            if nbits >= 48 {
                store_le64(&mut self.bytes[nbytes as usize..], bits);
                bits >>= 48;
                nbits -= 48;
                nbytes += 6;
                if nbytes >= BUFFER_FLUSH_SIZE {
                    if self.err.is_some() {
                        return;
                    }
                    write_to(
                        &mut self.writer,
                        &mut self.err,
                        &self.bytes[..nbytes as usize],
                    );
                    nbytes = 0;
                }
            }
            // Bitwriting inlined, ~30% speedup
            let c = encoding[t as usize];
            bits |= c.code64() << (nbits & 63);

            nbits = nbits.wrapping_add(c.len());
        }
        // Restore...
        self.bits = bits;
        self.nbits = nbits;
        self.nbytes = nbytes;

        // Flush if needed to have space.
        if self.nbits >= 48 {
            self.flush_bits();
        }

        if eof || sync {
            let c = self.literal_encoding.codes[END_BLOCK_MARKER];
            self.write_code(c);
            self.prev_header = 0;
            self.wrote_huffman = false;
        }
    }
}

/// Go's `x >> s` for a signed int and an unsigned shift count
/// (counts >= 64 give 0 or -1).
#[inline(always)]
fn go_shr_i64(x: i64, s: u32) -> i64 {
    if s >= 64 { x >> 63 } else { x >> s }
}
