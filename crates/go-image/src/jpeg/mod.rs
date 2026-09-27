//! Port of Go 1.27.1 `image/jpeg` (reader.go, scan.go, huffman.go, dct.go,
//! writer.go).
//!
//! Package jpeg implements a JPEG image decoder and encoder.
//! JPEG is defined in ITU-T T.81: https://www.w3.org/Graphics/JPEG/itu-t81.pdf.

mod dct;
mod huffman;
mod scan;
mod writer;

pub use writer::{DEFAULT_QUALITY, Options, encode};

use std::fmt;
use std::io::Read;

use crate::color::Model;
use crate::format::{BoxError, Config};
use crate::image::{CMYK, Gray, Image, RGBA};
use crate::imageutil;
use crate::ycbcr::YCbCr;

use dct::{BLOCK_SIZE, Block};
use huffman::Huffman;

/// Errors returned by the decoder and encoder.
///
/// Go: image/jpeg `FormatError`, `UnsupportedError`, `io.ErrUnexpectedEOF`,
/// errors from the underlying reader/writer, and the encoder's
/// `errors.New("jpeg: image is too large to encode")`. `Display` matches Go's
/// `Error()` strings.
#[derive(Debug)]
pub enum Error {
    /// A FormatError reports that the input is not a valid JPEG.
    Format(&'static str),
    /// An UnsupportedError reports that the input uses a valid but
    /// unimplemented JPEG feature.
    Unsupported(&'static str),
    /// io.ErrUnexpectedEOF
    UnexpectedEof,
    /// An error from the underlying reader or writer.
    Io(std::io::Error),
    /// An encoder error created with errors.New.
    Encode(&'static str),
}

impl Error {
    pub(crate) fn is_format(&self, msg: &str) -> bool {
        matches!(self, Error::Format(m) if *m == msg)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format(s) => write!(f, "invalid JPEG format: {}", s),
            Error::Unsupported(s) => write!(f, "unsupported JPEG feature: {}", s),
            Error::UnexpectedEof => f.write_str("unexpected EOF"),
            Error::Io(e) => write!(f, "{}", e),
            Error::Encode(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::Format(a), Error::Format(b)) => a == b,
            (Error::Unsupported(a), Error::Unsupported(b)) => a == b,
            (Error::UnexpectedEof, Error::UnexpectedEof) => true,
            (Error::Encode(a), Error::Encode(b)) => a == b,
            _ => false,
        }
    }
}

// Go: image/jpeg/reader.go:errUnsupportedSubsamplingRatio
const ERR_UNSUPPORTED_SUBSAMPLING_RATIO: Error =
    Error::Unsupported("luma/chroma subsampling ratio");

// errMissingFF00 means that readByteStuffedByte encountered an 0xff byte (a
// marker byte) that wasn't the expected byte-stuffed sequence 0xff, 0x00.
// Go: image/jpeg/reader.go:errMissingFF00
pub(crate) const ERR_MISSING_FF00: &str = "missing 0xff00 sequence";

// errShortHuffmanData means that an unexpected EOF occurred while decoding
// Huffman data.
// Go: image/jpeg/huffman.go:errShortHuffmanData
pub(crate) const ERR_SHORT_HUFFMAN_DATA: &str = "short Huffman data";

/// Component specification, specified in section B.2.2.
///
/// Go: image/jpeg/reader.go:component
#[derive(Clone, Copy, Default)]
pub(crate) struct Component {
    h: i64,        // Horizontal sampling factor.
    v: i64,        // Vertical sampling factor.
    c: u8,         // Component identifier.
    tq: u8,        // Quantization table destination selector.
    expand_h: i64, // Horizontal expansion factor for non-standard subsampling.
    expand_v: i64, // Vertical expansion factor for non-standard subsampling.
}

pub(crate) const DC_TABLE: usize = 0;
pub(crate) const AC_TABLE: usize = 1;
pub(crate) const MAX_TC: usize = 1;
pub(crate) const MAX_TH: usize = 3;
pub(crate) const MAX_TQ: usize = 3;

pub(crate) const MAX_COMPONENTS: usize = 4;

pub(crate) const SOF0_MARKER: u8 = 0xc0; // Start Of Frame (Baseline Sequential).
pub(crate) const SOF1_MARKER: u8 = 0xc1; // Start Of Frame (Extended Sequential).
pub(crate) const SOF2_MARKER: u8 = 0xc2; // Start Of Frame (Progressive).
pub(crate) const DHT_MARKER: u8 = 0xc4; // Define Huffman Table.
pub(crate) const RST0_MARKER: u8 = 0xd0; // ReSTart (0).
pub(crate) const RST7_MARKER: u8 = 0xd7; // ReSTart (7).
pub(crate) const SOI_MARKER: u8 = 0xd8; // Start Of Image.
pub(crate) const EOI_MARKER: u8 = 0xd9; // End Of Image.
pub(crate) const SOS_MARKER: u8 = 0xda; // Start Of Scan.
pub(crate) const DQT_MARKER: u8 = 0xdb; // Define Quantization Table.
pub(crate) const DRI_MARKER: u8 = 0xdd; // Define Restart Interval.
pub(crate) const COM_MARKER: u8 = 0xfe; // COMment.
// "APPlication specific" markers aren't part of the JPEG spec per se,
// but in practice, their use is described at
// https://www.sno.phy.queensu.ca/~phil/exiftool/TagNames/JPEG.html
pub(crate) const APP0_MARKER: u8 = 0xe0;
pub(crate) const APP14_MARKER: u8 = 0xee;
pub(crate) const APP15_MARKER: u8 = 0xef;

// See https://www.sno.phy.queensu.ca/~phil/exiftool/TagNames/JPEG.html#Adobe
const ADOBE_TRANSFORM_UNKNOWN: u8 = 0;
#[allow(dead_code)]
const ADOBE_TRANSFORM_YCBCR: u8 = 1;
#[allow(dead_code)]
const ADOBE_TRANSFORM_YCBCRK: u8 = 2;

/// unzig maps from the zig-zag ordering to the natural ordering. For example,
/// unzig[3] is the column and row of the fourth element in zig-zag order. The
/// value is 16, which means first column (16%8 == 0) and third row (16/8 == 2).
///
/// Go: image/jpeg/reader.go:unzig
pub(crate) const UNZIG: [usize; BLOCK_SIZE] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// bits holds the unprocessed bits that have been taken from the byte-stream.
/// The n least significant bits of a form the unread bits, to be read in MSB
/// to LSB order.
///
/// Go: image/jpeg/reader.go:bits
#[derive(Clone, Copy, Default)]
pub(crate) struct Bits {
    pub(crate) a: u32, // accumulator.
    pub(crate) m: u32, // mask. m==1<<(n-1) when n>0, with m==0 when n==0.
    pub(crate) n: i32, // the number of unread bits in a.
}

/// bytes is a byte buffer, similar to a bufio.Reader, except that it has to
/// be able to unread more than 1 byte, due to byte stuffing.
///
/// Go: image/jpeg/reader.go:decoder.bytes
pub(crate) struct Bytes {
    // buf[i:j] are the buffered bytes read from the underlying
    // io.Reader that haven't yet been passed further on.
    pub(crate) buf: [u8; 4096],
    pub(crate) i: usize,
    pub(crate) j: usize,
    // nUnreadable is the number of bytes to back up i after
    // overshooting. It can be 0, 1 or 2.
    pub(crate) n_unreadable: usize,
}

/// The input half of Go's `decoder` struct: `r`, `bits` and `bytes`.
pub(crate) struct Input<'r> {
    r: &'r mut dyn Read,
    pub(crate) bits: Bits,
    pub(crate) bytes: Bytes,
}

/// Go: image/jpeg/reader.go:decoder (minus the fields in [`Input`]).
pub(crate) struct Decoder<'r> {
    pub(crate) inp: Input<'r>,
    pub(crate) width: i64,
    pub(crate) height: i64,

    pub(crate) img1: Option<Gray>,
    pub(crate) img3: Option<YCbCr>,
    pub(crate) black_pix: Option<Vec<u8>>,
    pub(crate) black_stride: i64,

    // For non-standard subsampling ratios (flex mode).
    pub(crate) flex: bool, // True if using non-standard subsampling that requires manual pixel expansion.
    pub(crate) max_h: i64, // Maximum horizontal and vertical sampling factors across all components.
    pub(crate) max_v: i64,

    pub(crate) ri: i64, // Restart Interval.
    pub(crate) n_comp: i64,

    // As per section 4.5, there are four modes of operation (selected by the
    // SOF? markers): sequential DCT, progressive DCT, lossless and
    // hierarchical, although this implementation does not support the latter
    // two non-DCT modes. Sequential DCT is further split into baseline and
    // extended, as per section 4.11.
    pub(crate) baseline: bool,
    pub(crate) progressive: bool,

    pub(crate) jfif: bool,
    pub(crate) adobe_transform_valid: bool,
    pub(crate) adobe_transform: u8,
    pub(crate) eob_run: u16, // End-of-Band run, specified in section G.1.2.2.

    pub(crate) comp: [Component; MAX_COMPONENTS],
    pub(crate) prog_coeffs: [Option<Vec<Block>>; MAX_COMPONENTS], // Saved state between progressive-mode scans.
    pub(crate) huff: [[Huffman; MAX_TH + 1]; MAX_TC + 1],
    pub(crate) quant: [Block; MAX_TQ + 1], // Quantization tables, in zig-zag order.
    pub(crate) tmp: [u8; 2 * BLOCK_SIZE],
}

impl<'r> Input<'r> {
    /// fill fills up the d.bytes.buf buffer from the underlying io.Reader. It
    /// should only be called when there are no unread bytes in d.bytes.
    ///
    /// Go: image/jpeg/reader.go:decoder.fill
    #[inline(never)]
    fn fill(&mut self) -> Result<(), Error> {
        if self.bytes.i != self.bytes.j {
            panic!("jpeg: fill called when unread bytes exist");
        }
        // Move the last 2 bytes to the start of the buffer, in case we need
        // to call unreadByteStuffedByte.
        if self.bytes.j > 2 {
            self.bytes.buf[0] = self.bytes.buf[self.bytes.j - 2];
            self.bytes.buf[1] = self.bytes.buf[self.bytes.j - 1];
            (self.bytes.i, self.bytes.j) = (2, 2);
        }
        // Fill in the rest of the buffer.
        let n = loop {
            match self.r.read(&mut self.bytes.buf[self.bytes.j..]) {
                Ok(n) => break n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            }
        };
        self.bytes.j += n;
        if n > 0 {
            return Ok(());
        }
        // A Rust reader signals io.EOF with Ok(0).
        Err(Error::UnexpectedEof)
    }

    /// unreadByteStuffedByte undoes the most recent readByteStuffedByte call,
    /// giving a byte of data back from d.bits to d.bytes.
    ///
    /// Go: image/jpeg/reader.go:decoder.unreadByteStuffedByte
    #[inline]
    pub(crate) fn unread_byte_stuffed_byte(&mut self) {
        self.bytes.i -= self.bytes.n_unreadable;
        self.bytes.n_unreadable = 0;
        if self.bits.n >= 8 {
            self.bits.a >>= 8;
            self.bits.n -= 8;
            self.bits.m >>= 8;
        }
    }

    /// readByte returns the next byte, whether buffered or not buffered. It
    /// does not care about byte stuffing.
    ///
    /// Go: image/jpeg/reader.go:decoder.readByte
    #[inline]
    pub(crate) fn read_byte(&mut self) -> Result<u8, Error> {
        while self.bytes.i == self.bytes.j {
            self.fill()?;
        }
        let x = self.bytes.buf[self.bytes.i];
        self.bytes.i += 1;
        self.bytes.n_unreadable = 0;
        Ok(x)
    }

    /// readByteStuffedByte is like readByte but is for byte-stuffed Huffman
    /// data.
    ///
    /// Go: image/jpeg/reader.go:decoder.readByteStuffedByte
    #[inline]
    pub(crate) fn read_byte_stuffed_byte(&mut self) -> Result<u8, Error> {
        // Take the fast path if d.bytes.buf contains at least two bytes.
        if self.bytes.i + 2 <= self.bytes.j {
            let x = self.bytes.buf[self.bytes.i];
            self.bytes.i += 1;
            self.bytes.n_unreadable = 1;
            if x != 0xff {
                return Ok(x);
            }
            if self.bytes.buf[self.bytes.i] != 0x00 {
                return Err(Error::Format(ERR_MISSING_FF00));
            }
            self.bytes.i += 1;
            self.bytes.n_unreadable = 2;
            return Ok(0xff);
        }

        self.bytes.n_unreadable = 0;

        let x = self.read_byte()?;
        self.bytes.n_unreadable = 1;
        if x != 0xff {
            return Ok(x);
        }

        let x = self.read_byte()?;
        self.bytes.n_unreadable = 2;
        if x != 0x00 {
            return Err(Error::Format(ERR_MISSING_FF00));
        }
        Ok(0xff)
    }

    /// readFull reads exactly len(p) bytes into p. It does not care about
    /// byte stuffing.
    ///
    /// Go: image/jpeg/reader.go:decoder.readFull
    #[inline]
    pub(crate) fn read_full(&mut self, mut p: &mut [u8]) -> Result<(), Error> {
        // Unread the overshot bytes, if any.
        if self.bytes.n_unreadable != 0 {
            if self.bits.n >= 8 {
                self.unread_byte_stuffed_byte();
            }
            self.bytes.n_unreadable = 0;
        }

        loop {
            let avail = &self.bytes.buf[self.bytes.i..self.bytes.j];
            let n = p.len().min(avail.len());
            p[..n].copy_from_slice(&avail[..n]);
            p = &mut p[n..];
            self.bytes.i += n;
            if p.is_empty() {
                break;
            }
            self.fill()?;
        }
        Ok(())
    }

    /// ignore ignores the next n bytes.
    ///
    /// Go: image/jpeg/reader.go:decoder.ignore
    pub(crate) fn ignore(&mut self, mut n: i64) -> Result<(), Error> {
        // Unread the overshot bytes, if any.
        if self.bytes.n_unreadable != 0 {
            if self.bits.n >= 8 {
                self.unread_byte_stuffed_byte();
            }
            self.bytes.n_unreadable = 0;
        }

        loop {
            let mut m = (self.bytes.j - self.bytes.i) as i64;
            if m > n {
                m = n;
            }
            self.bytes.i += m as usize;
            n -= m;
            if n == 0 {
                break;
            }
            self.fill()?;
        }
        Ok(())
    }
}

impl<'r> Decoder<'r> {
    fn new(r: &'r mut dyn Read) -> Box<Decoder<'r>> {
        Box::new(Decoder {
            inp: Input {
                r,
                bits: Bits::default(),
                bytes: Bytes {
                    buf: [0; 4096],
                    i: 0,
                    j: 0,
                    n_unreadable: 0,
                },
            },
            width: 0,
            height: 0,
            img1: None,
            img3: None,
            black_pix: None,
            black_stride: 0,
            flex: false,
            max_h: 0,
            max_v: 0,
            ri: 0,
            n_comp: 0,
            baseline: false,
            progressive: false,
            jfif: false,
            adobe_transform_valid: false,
            adobe_transform: 0,
            eob_run: 0,
            comp: [Component::default(); MAX_COMPONENTS],
            prog_coeffs: [None, None, None, None],
            huff: Default::default(),
            quant: [[0; BLOCK_SIZE]; MAX_TQ + 1],
            tmp: [0; 2 * BLOCK_SIZE],
        })
    }

    /// Specified in section B.2.2.
    ///
    /// Go: image/jpeg/reader.go:decoder.processSOF
    fn process_sof(&mut self, n: i64) -> Result<(), Error> {
        if self.n_comp != 0 {
            return Err(Error::Format("multiple SOF markers"));
        }
        match n {
            // Grayscale image.
            9 => self.n_comp = 1,
            // YCbCr or RGB image.
            15 => self.n_comp = 3,
            // YCbCrK or CMYK image.
            18 => self.n_comp = 4,
            _ => return Err(Error::Unsupported("number of components")),
        }
        self.inp.read_full(&mut self.tmp[..n as usize])?;
        // We only support 8-bit precision.
        if self.tmp[0] != 8 {
            return Err(Error::Unsupported("precision"));
        }
        self.height = ((self.tmp[1] as i64) << 8) + self.tmp[2] as i64;
        self.width = ((self.tmp[3] as i64) << 8) + self.tmp[4] as i64;
        if self.tmp[5] as i64 != self.n_comp {
            return Err(Error::Format("SOF has wrong length"));
        }

        for i in 0..self.n_comp as usize {
            self.comp[i].c = self.tmp[6 + 3 * i];
            // Section B.2.2 states that "the value of C_i shall be different from
            // the values of C_1 through C_(i-1)".
            for j in 0..i {
                if self.comp[i].c == self.comp[j].c {
                    return Err(Error::Format("repeated component identifier"));
                }
            }

            self.comp[i].tq = self.tmp[8 + 3 * i];
            if self.comp[i].tq as usize > MAX_TQ {
                return Err(Error::Format("bad Tq value"));
            }

            let hv = self.tmp[7 + 3 * i];
            let (mut h, mut v) = ((hv >> 4) as i64, (hv & 0x0f) as i64);
            if h < 1 || 4 < h || v < 1 || 4 < v {
                return Err(Error::Format("luma/chroma subsampling ratio"));
            }
            if h == 3 || v == 3 {
                return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
            }
            match self.n_comp {
                1 => {
                    // If a JPEG image has only one component, section A.2 says "this data
                    // is non-interleaved by definition" and section A.2.2 says "[in this
                    // case...] the order of data units within a scan shall be left-to-right
                    // and top-to-bottom... regardless of the values of H_1 and V_1". Section
                    // 4.8.2 also says "[for non-interleaved data], the MCU is defined to be
                    // one data unit". Similarly, section A.1.1 explains that it is the ratio
                    // of H_i to max_j(H_j) that matters, and similarly for V. For grayscale
                    // images, H_1 is the maximum H_j for all components j, so that ratio is
                    // always 1. The component's (h, v) is effectively always (1, 1): even if
                    // the nominal (h, v) is (2, 1), a 20x5 image is encoded in three 8x8
                    // MCUs, not two 16x8 MCUs.
                    (h, v) = (1, 1);
                }
                3 => {
                    // For YCbCr images, we support both standard subsampling ratios
                    // (4:4:4, 4:4:0, 4:2:2, 4:2:0, 4:1:1, 4:1:0) and non-standard ratios
                    // where components may have different sampling factors. The only
                    // restriction is that each component's sampling factors must evenly
                    // divide the maximum factors (validated after the loop).
                }
                4 => {
                    // For 4-component images (either CMYK or YCbCrK), we only support two
                    // hv vectors: [0x11 0x11 0x11 0x11] and [0x22 0x11 0x11 0x22].
                    match i {
                        0 => {
                            if hv != 0x11 && hv != 0x22 {
                                return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
                            }
                        }
                        1 | 2 => {
                            if hv != 0x11 {
                                return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
                            }
                        }
                        3 => {
                            if self.comp[0].h != h || self.comp[0].v != v {
                                return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            (self.max_h, self.max_v) = (self.max_h.max(h), self.max_v.max(v));
            self.comp[i].h = h;
            self.comp[i].v = v;
        }

        // For 3-component images, validate that maxH and maxV are evenly divisible
        // by each component's sampling factors.
        if self.n_comp == 3 {
            for i in 0..3 {
                if self.max_h % self.comp[i].h != 0 || self.max_v % self.comp[i].v != 0 {
                    return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
                }
            }
        }

        // Compute expansion factors for each component.
        for i in 0..self.n_comp as usize {
            self.comp[i].expand_h = self.max_h / self.comp[i].h;
            self.comp[i].expand_v = self.max_v / self.comp[i].v;
        }

        Ok(())
    }

    /// Specified in section B.2.4.1.
    ///
    /// Go: image/jpeg/reader.go:decoder.processDQT
    fn process_dqt(&mut self, mut n: i64) -> Result<(), Error> {
        'outer: while n > 0 {
            n -= 1;
            let x = self.inp.read_byte()?;
            let tq = (x & 0x0f) as usize;
            if tq > MAX_TQ {
                return Err(Error::Format("bad Tq value"));
            }
            match x >> 4 {
                0 => {
                    if n < BLOCK_SIZE as i64 {
                        break 'outer;
                    }
                    n -= BLOCK_SIZE as i64;
                    self.inp.read_full(&mut self.tmp[..BLOCK_SIZE])?;
                    for i in 0..BLOCK_SIZE {
                        self.quant[tq][i] = self.tmp[i] as i32;
                    }
                }
                1 => {
                    if n < 2 * BLOCK_SIZE as i64 {
                        break 'outer;
                    }
                    n -= 2 * BLOCK_SIZE as i64;
                    self.inp.read_full(&mut self.tmp[..2 * BLOCK_SIZE])?;
                    for i in 0..BLOCK_SIZE {
                        self.quant[tq][i] =
                            (self.tmp[2 * i] as i32) << 8 | self.tmp[2 * i + 1] as i32;
                    }
                }
                _ => return Err(Error::Format("bad Pq value")),
            }
        }
        if n != 0 {
            return Err(Error::Format("DQT has wrong length"));
        }
        Ok(())
    }

    /// Specified in section B.2.4.4.
    ///
    /// Go: image/jpeg/reader.go:decoder.processDRI
    fn process_dri(&mut self, n: i64) -> Result<(), Error> {
        if n != 2 {
            return Err(Error::Format("DRI has wrong length"));
        }
        self.inp.read_full(&mut self.tmp[..2])?;
        self.ri = ((self.tmp[0] as i64) << 8) + self.tmp[1] as i64;
        Ok(())
    }

    // Go: image/jpeg/reader.go:decoder.processApp0Marker
    fn process_app0_marker(&mut self, mut n: i64) -> Result<(), Error> {
        if n < 5 {
            return self.inp.ignore(n);
        }
        self.inp.read_full(&mut self.tmp[..5])?;
        n -= 5;

        self.jfif = self.tmp[0] == b'J'
            && self.tmp[1] == b'F'
            && self.tmp[2] == b'I'
            && self.tmp[3] == b'F'
            && self.tmp[4] == b'\x00';

        if n > 0 {
            return self.inp.ignore(n);
        }
        Ok(())
    }

    // Go: image/jpeg/reader.go:decoder.processApp14Marker
    fn process_app14_marker(&mut self, mut n: i64) -> Result<(), Error> {
        if n < 12 {
            return self.inp.ignore(n);
        }
        self.inp.read_full(&mut self.tmp[..12])?;
        n -= 12;

        if self.tmp[0] == b'A'
            && self.tmp[1] == b'd'
            && self.tmp[2] == b'o'
            && self.tmp[3] == b'b'
            && self.tmp[4] == b'e'
        {
            self.adobe_transform_valid = true;
            self.adobe_transform = self.tmp[11];
        }

        if n > 0 {
            return self.inp.ignore(n);
        }
        Ok(())
    }

    /// decode reads a JPEG image from r and returns it as an image.Image.
    /// With configOnly, `Ok(None)` is Go's `nil, nil`.
    ///
    /// Go: image/jpeg/reader.go:decoder.decode
    fn decode(&mut self, config_only: bool) -> Result<Option<Box<dyn Image>>, Error> {
        // Check for the Start Of Image marker.
        self.inp.read_full(&mut self.tmp[..2])?;
        if self.tmp[0] != 0xff || self.tmp[1] != SOI_MARKER {
            return Err(Error::Format("missing SOI marker"));
        }

        // Process the remaining segments until the End Of Image marker.
        loop {
            self.inp.read_full(&mut self.tmp[..2])?;
            while self.tmp[0] != 0xff {
                // Strictly speaking, this is a format error. However, libjpeg is
                // liberal in what it accepts. As of version 9, next_marker in
                // jdmarker.c treats this as a warning (JWRN_EXTRANEOUS_DATA) and
                // continues to decode the stream. [...]
                //
                // We are therefore also liberal in what we accept. Extraneous data
                // is silently ignored.
                self.tmp[0] = self.tmp[1];
                self.tmp[1] = self.inp.read_byte()?;
            }
            let mut marker = self.tmp[1];
            if marker == 0 {
                // Treat "\xff\x00" as extraneous data.
                continue;
            }
            while marker == 0xff {
                // Section B.1.1.2 says, "Any marker may optionally be preceded by any
                // number of fill bytes, which are bytes assigned code X'FF'".
                marker = self.inp.read_byte()?;
            }
            if marker == EOI_MARKER {
                // End Of Image.
                break;
            }
            if RST0_MARKER <= marker && marker <= RST7_MARKER {
                // Figures B.2 and B.16 of the specification suggest that restart markers should
                // only occur between Entropy Coded Segments and not after the final ECS.
                // However, some encoders may generate incorrect JPEGs with a final restart
                // marker. That restart marker will be seen here instead of inside the processSOS
                // method, and is ignored as a harmless error. Restart markers have no extra data,
                // so we check for this before we read the 16-bit length of the segment.
                continue;
            }

            // Read the 16-bit length of the segment. The value includes the 2 bytes for the
            // length itself, so we subtract 2 to get the number of remaining bytes.
            self.inp.read_full(&mut self.tmp[..2])?;
            let n = ((self.tmp[0] as i64) << 8) + self.tmp[1] as i64 - 2;
            if n < 0 {
                return Err(Error::Format("short segment length"));
            }

            let r = match marker {
                SOF0_MARKER | SOF1_MARKER | SOF2_MARKER => {
                    self.baseline = marker == SOF0_MARKER;
                    self.progressive = marker == SOF2_MARKER;
                    let r = self.process_sof(n);
                    if config_only && self.jfif {
                        return r.map(|_| None);
                    }
                    r
                }
                DHT_MARKER => {
                    if config_only {
                        self.inp.ignore(n)
                    } else {
                        self.process_dht(n)
                    }
                }
                DQT_MARKER => {
                    if config_only {
                        self.inp.ignore(n)
                    } else {
                        self.process_dqt(n)
                    }
                }
                SOS_MARKER => {
                    if config_only {
                        return Ok(None);
                    }
                    self.process_sos(n)
                }
                DRI_MARKER => {
                    if config_only {
                        self.inp.ignore(n)
                    } else {
                        self.process_dri(n)
                    }
                }
                APP0_MARKER => self.process_app0_marker(n),
                APP14_MARKER => self.process_app14_marker(n),
                _ => {
                    if APP0_MARKER <= marker && marker <= APP15_MARKER || marker == COM_MARKER {
                        self.inp.ignore(n)
                    } else if marker < 0xc0 {
                        // See Table B.1 "Marker code assignments".
                        Err(Error::Format("unknown marker"))
                    } else {
                        Err(Error::Unsupported("unknown marker"))
                    }
                }
            };
            r?;
        }

        if self.progressive {
            self.reconstruct_progressive_image()?;
        }
        if let Some(img1) = self.img1.take() {
            return Ok(Some(Box::new(img1)));
        }
        if self.img3.is_some() {
            if self.black_pix.is_some() {
                return self.apply_black().map(Some);
            } else if self.is_rgb() {
                return self.convert_to_rgb().map(Some);
            }
            return Ok(Some(Box::new(self.img3.take().unwrap())));
        }
        Err(Error::Format("missing SOS marker"))
    }

    /// applyBlack combines d.img3 and d.blackPix into a CMYK image. The
    /// formula used depends on whether the JPEG image is stored as CMYK or
    /// YCbCrK, indicated by the APP14 (Adobe) metadata.
    ///
    /// Go: image/jpeg/reader.go:decoder.applyBlack
    fn apply_black(&mut self) -> Result<Box<dyn Image>, Error> {
        if !self.adobe_transform_valid {
            return Err(Error::Unsupported(
                "unknown color model: 4-component JPEG doesn't have Adobe APP14 metadata",
            ));
        }
        let img3 = self.img3.as_ref().unwrap();
        let black_pix = self.black_pix.as_ref().unwrap();

        // If the 4-component JPEG image isn't explicitly marked as "Unknown (RGB
        // or CMYK)" as per
        // https://www.sno.phy.queensu.ca/~phil/exiftool/TagNames/JPEG.html#Adobe
        // we assume that it is YCbCrK. This matches libjpeg's jdapimin.c.
        if self.adobe_transform != ADOBE_TRANSFORM_UNKNOWN {
            // Convert the YCbCr part of the YCbCrK to RGB, invert the RGB to get
            // CMY, and patch in the original K. The RGB to CMY inversion cancels
            // out the 'Adobe inversion' described in the applyBlack doc comment
            // above, so in practice, only the fourth channel (black) is inverted.
            let bounds = img3.bounds();
            let mut img = RGBA::new(bounds);
            imageutil::draw_ycbcr(&mut img, bounds, img3, bounds.min);
            let (mut i_base, mut y) = (0i64, bounds.min.y);
            while y < bounds.max.y {
                let (mut i, mut x) = (i_base + 3, bounds.min.x);
                while x < bounds.max.x {
                    img.pix[i as usize] = 255
                        - black_pix[((y - bounds.min.y) * self.black_stride + (x - bounds.min.x))
                            as usize];
                    i += 4;
                    x += 1;
                }
                i_base += img.stride;
                y += 1;
            }
            return Ok(Box::new(CMYK {
                pix: img.pix,
                stride: img.stride,
                rect: img.rect,
            }));
        }

        // The first three channels (cyan, magenta, yellow) of the CMYK
        // were decoded into d.img3, but each channel was decoded into a separate
        // []byte slice, and some channels may be subsampled. We interleave the
        // separate channels into an image.CMYK's single []byte slice containing 4
        // contiguous bytes per pixel.
        let bounds = img3.bounds();
        let mut img = CMYK::new(bounds);

        let translations: [(&[u8], i64); 4] = [
            (&img3.y, img3.y_stride),
            (&img3.cb, img3.c_stride),
            (&img3.cr, img3.c_stride),
            (black_pix, self.black_stride),
        ];
        for (t, &(src, stride)) in translations.iter().enumerate() {
            let subsample = self.comp[t].h != self.comp[0].h || self.comp[t].v != self.comp[0].v;
            let (mut i_base, mut y) = (0i64, bounds.min.y);
            while y < bounds.max.y {
                let mut sy = y - bounds.min.y;
                if subsample {
                    sy /= 2;
                }
                let (mut i, mut x) = (i_base + t as i64, bounds.min.x);
                while x < bounds.max.x {
                    let mut sx = x - bounds.min.x;
                    if subsample {
                        sx /= 2;
                    }
                    img.pix[i as usize] = 255 - src[(sy * stride + sx) as usize];
                    i += 4;
                    x += 1;
                }
                i_base += img.stride;
                y += 1;
            }
        }
        Ok(Box::new(img))
    }

    // Go: image/jpeg/reader.go:decoder.isRGB
    fn is_rgb(&self) -> bool {
        if self.jfif {
            return false;
        }
        if self.adobe_transform_valid && self.adobe_transform == ADOBE_TRANSFORM_UNKNOWN {
            // https://www.sno.phy.queensu.ca/~phil/exiftool/TagNames/JPEG.html#Adobe
            // says that 0 means Unknown (and in practice RGB) and 1 means YCbCr.
            return true;
        }
        self.comp[0].c == b'R' && self.comp[1].c == b'G' && self.comp[2].c == b'B'
    }

    // Go: image/jpeg/reader.go:decoder.convertToRGB
    fn convert_to_rgb(&mut self) -> Result<Box<dyn Image>, Error> {
        // Historically, we only supported 4:4:4, 4:4:0, 4:2:2, 4:2:0, 4:1:1 or
        // 4:1:0 chroma subsampling ratios. [...] convertToRGB still makes those
        // historical assumptions and does not support the intersection of (1)
        // atypical chroma subsampling and (2) RGB-instead-of-YCbCr.
        let (h0, h1, h2) = (self.comp[0].h, self.comp[1].h, self.comp[2].h);
        let (v0, v1, v2) = (self.comp[0].v, self.comp[1].v, self.comp[2].v);
        if (h1 != h2) || (h0 % h1 != 0) || (v1 != v2) || (v0 % v1 != 0) {
            return Err(ERR_UNSUPPORTED_SUBSAMPLING_RATIO);
        }

        let c_scale = h0 / h1;
        let img3 = self.img3.as_ref().unwrap();
        let bounds = img3.bounds();
        let mut img = RGBA::new(bounds);
        for y in bounds.min.y..bounds.max.y {
            let po = img.pix_offset(bounds.min.x, y) as usize;
            let yo = img3.y_offset(bounds.min.x, y) as usize;
            let co = img3.c_offset(bounds.min.x, y) as usize;
            let i_max = (bounds.max.x - bounds.min.x) as usize;
            for i in 0..i_max {
                img.pix[po + 4 * i] = img3.y[yo + i];
                img.pix[po + 4 * i + 1] = img3.cb[co + i / c_scale as usize];
                img.pix[po + 4 * i + 2] = img3.cr[co + i / c_scale as usize];
                img.pix[po + 4 * i + 3] = 255;
            }
        }
        Ok(Box::new(img))
    }
}

/// Decode reads a JPEG image from r and returns it as an [`Image`]: a
/// [`Gray`], [`YCbCr`], [`RGBA`] or [`CMYK`], exactly as Go does.
///
/// Go: image/jpeg/reader.go:Decode
pub fn decode<R: Read + ?Sized>(r: &mut R) -> Result<Box<dyn Image>, Error> {
    let mut rr = ReadAdapter(r);
    let mut d = Decoder::new(&mut rr);
    match d.decode(false)? {
        Some(m) => Ok(m),
        // Unreachable: decode(false) never returns (nil, nil).
        None => Err(Error::Format("missing SOS marker")),
    }
}

/// DecodeConfig returns the color model and dimensions of a JPEG image without
/// decoding the entire image.
///
/// Go: image/jpeg/reader.go:DecodeConfig
pub fn decode_config<R: Read + ?Sized>(r: &mut R) -> Result<Config, Error> {
    let mut rr = ReadAdapter(r);
    let mut d = Decoder::new(&mut rr);
    d.decode(true)?;
    match d.n_comp {
        1 => Ok(Config {
            color_model: Model::Gray,
            width: d.width,
            height: d.height,
        }),
        3 => {
            let mut cm = Model::YCbCr;
            if d.is_rgb() {
                cm = Model::RGBA;
            }
            Ok(Config {
                color_model: cm,
                width: d.width,
                height: d.height,
            })
        }
        4 => Ok(Config {
            color_model: Model::CMYK,
            width: d.width,
            height: d.height,
        }),
        _ => Err(Error::Format("missing SOF marker")),
    }
}

// Adapts `R: Read + ?Sized` to a sized `dyn Read` target.
struct ReadAdapter<'a, R: Read + ?Sized>(&'a mut R);

impl<R: Read + ?Sized> Read for ReadAdapter<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

fn decode_boxed(r: &mut dyn Read) -> Result<Box<dyn Image>, BoxError> {
    decode(r).map_err(|e| Box::new(e) as BoxError)
}

fn decode_config_boxed(r: &mut dyn Read) -> Result<Config, BoxError> {
    decode_config(r).map_err(|e| Box::new(e) as BoxError)
}

/// Registers the JPEG format with [`crate::register_format`] (Go does this in
/// the package's `init`). Idempotent.
///
/// Go: image/jpeg/reader.go:init
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        crate::format::register_format("jpeg", b"\xff\xd8", decode_boxed, decode_config_boxed);
    });
}
