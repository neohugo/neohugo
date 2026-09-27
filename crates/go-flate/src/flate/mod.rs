//! Port of go1.27.1 `compress/flate`.
//!
//! The compressor is a line-by-line port of Go 1.27.1's (klauspost-derived)
//! encoder, so its output is byte-identical to Go's for every level, write
//! chunking, `Flush`, `Reset` and dictionary. The decompressor is a port of
//! `inflate.go` / `dict_decoder.go` (same error values and offsets as Go).

mod deflate;
mod deflatefast;
mod dict_decoder;
mod huffman_bit_writer;
mod huffman_code;
mod inflate;
mod level1;
mod level2;
mod level3;
mod level4;
mod level5;
mod level6;
mod token;

pub use deflate::{Writer, new_writer, new_writer_dict};
pub(crate) use inflate::read_full as read_full_internal;
pub use inflate::{Decompressor, new_reader, new_reader_dict};

pub const NO_COMPRESSION: i32 = 0;
pub const BEST_SPEED: i32 = 1;
pub const BEST_COMPRESSION: i32 = 9;
pub const DEFAULT_COMPRESSION: i32 = -1;

/// HuffmanOnly disables Lempel-Ziv match searching and only performs Huffman
/// entropy encoding. This mode is useful in compressing data that has
/// already been compressed with an LZ style algorithm (e.g. Snappy or LZ4)
/// that lacks an entropy encoder. Compression gains are achieved when
/// certain bytes in the input stream occur more frequently than others.
///
/// Note that HuffmanOnly produces a compressed output that is
/// RFC 1951 compliant. That is, any valid DEFLATE decompressor will
/// continue to be able to decompress this output.
pub const HUFFMAN_ONLY: i32 = -2;

pub(crate) const MIN_MATCH_LENGTH: i32 = 4; // The smallest match that the compressor looks for
pub(crate) const MAX_MATCH_LENGTH: i32 = 258; // The longest match for the compressor
pub(crate) const MAX_STORE_BLOCK_SIZE: i32 = 65535;

pub(crate) use deflatefast::BASE_MATCH_LENGTH;
pub(crate) use huffman_bit_writer::END_BLOCK_MARKER;

/// Internal entry points used by the crate's tests to exercise the
/// huffman bit writer directly (ports of Go's internal `_test.go` tables).
#[doc(hidden)]
pub mod internal {
    use std::io::Write;

    use super::huffman_bit_writer::{HuffmanBitWriter, new_huffman_bit_writer};
    use super::token::Tokens;
    use crate::error::Error;

    /// Wraps Go's unexported `huffmanBitWriter` for tests.
    pub struct BitWriter<W: Write>(HuffmanBitWriter<W>);

    /// Wraps Go's unexported `tokens` for tests.
    pub struct TokenList(Box<Tokens>);

    impl TokenList {
        /// Go: `indexTokens(in []token) tokens`.
        pub fn index(input: &[u32]) -> TokenList {
            TokenList(Box::new(Tokens::from_unindexed(input)))
        }
    }

    impl<W: Write> BitWriter<W> {
        pub fn new(w: W) -> Self {
            BitWriter(new_huffman_bit_writer(w))
        }
        pub fn set_log_new_table_penalty(&mut self, v: u32) {
            self.0.log_new_table_penalty = v;
        }
        pub fn write_block_huff(&mut self, eof: bool, input: &[u8], sync: bool) {
            self.0.write_block_huff(eof, input, sync)
        }
        pub fn write_block(&mut self, tokens: &mut TokenList, eof: bool, input: Option<&[u8]>) {
            self.0.write_block(&mut tokens.0, eof, input)
        }
        pub fn write_block_dynamic(
            &mut self,
            tokens: &mut TokenList,
            eof: bool,
            input: Option<&[u8]>,
            sync: bool,
        ) {
            self.0.write_block_dynamic(&mut tokens.0, eof, input, sync)
        }
        pub fn flush(&mut self) {
            self.0.flush()
        }
        pub fn reset(&mut self, w: W) -> W {
            self.0.reset(w)
        }
        pub fn err(&self) -> Option<Error> {
            self.0.err.clone()
        }
        pub fn get_ref(&self) -> &W {
            &self.0.writer
        }
        pub fn get_mut(&mut self) -> &mut W {
            &mut self.0.writer
        }
    }
}
