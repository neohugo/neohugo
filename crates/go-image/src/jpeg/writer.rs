//! Port of Go 1.27.1 `image/jpeg/writer.go`.

use std::io::Write;

use super::dct::{BLOCK_SIZE, Block, fdct};
use super::{DHT_MARKER, DQT_MARKER, Error, SOF0_MARKER, UNZIG};
use crate::color;
use crate::geom::{Point, pt};
use crate::image::{Gray, Image, RGBA};
use crate::ycbcr::YCbCr;

/// div returns a/b rounded to the nearest integer, instead of rounded to zero.
///
/// Go: image/jpeg/writer.go:div
#[inline]
fn div(a: i32, b: i32) -> i32 {
    if a >= 0 {
        return a.wrapping_add(b >> 1).wrapping_div(b);
    }
    -(a.wrapping_neg().wrapping_add(b >> 1).wrapping_div(b))
}

// bitCount counts the number of bits needed to hold an integer.
// Go: image/jpeg/writer.go:bitCount
static BIT_COUNT: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 1usize;
    while i < 256 {
        let mut n = 0u8;
        let mut v = i;
        while v > 0 {
            n += 1;
            v >>= 1;
        }
        t[i] = n;
        i += 1;
    }
    t
};

// Go: image/jpeg/writer.go:quantIndex
const QUANT_INDEX_LUMINANCE: usize = 0;
#[allow(dead_code)]
const QUANT_INDEX_CHROMINANCE: usize = 1;
const N_QUANT_INDEX: usize = 2;

// unscaledQuant are the unscaled quantization tables in zig-zag order. Each
// encoder copies and scales the tables according to its quality parameter.
// The values are derived from section K.1 of the spec, after converting from
// natural to zig-zag order.
// Go: image/jpeg/writer.go:unscaledQuant
static UNSCALED_QUANT: [[u8; BLOCK_SIZE]; N_QUANT_INDEX] = [
    // Luminance.
    [
        16, 11, 12, 14, 12, 10, 16, 14, 13, 14, 18, 17, 16, 19, 24, 40, 26, 24, 22, 22, 24, 49, 35,
        37, 29, 40, 58, 51, 61, 60, 57, 51, 56, 55, 64, 72, 92, 78, 64, 68, 87, 69, 55, 56, 80,
        109, 81, 87, 95, 98, 103, 104, 103, 62, 77, 113, 121, 112, 100, 120, 92, 101, 103, 99,
    ],
    // Chrominance.
    [
        17, 18, 18, 24, 21, 24, 47, 26, 26, 47, 99, 66, 56, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    ],
];

// Go: image/jpeg/writer.go:huffIndex
type HuffIndex = usize;
const N_HUFF_INDEX: usize = 4;

/// huffmanSpec specifies a Huffman encoding.
///
/// Go: image/jpeg/writer.go:huffmanSpec
struct HuffmanSpec {
    // count[i] is the number of codes of length i+1 bits.
    count: [u8; 16],
    // value[i] is the decoded value of the i'th codeword.
    value: &'static [u8],
}

// theHuffmanSpec is the Huffman encoding specifications.
//
// This encoder uses the same Huffman encoding for all images. It is also the
// same Huffman encoding used by section K.3 of the spec.
// Go: image/jpeg/writer.go:theHuffmanSpec
static THE_HUFFMAN_SPEC: [HuffmanSpec; N_HUFF_INDEX] = [
    // Luminance DC.
    HuffmanSpec {
        count: [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
        value: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    },
    // Luminance AC.
    HuffmanSpec {
        count: [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125],
        value: &[
            0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51,
            0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1,
            0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
            0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57,
            0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75,
            0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92,
            0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
            0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
            0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
            0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2,
            0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
        ],
    },
    // Chrominance DC.
    HuffmanSpec {
        count: [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
        value: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    },
    // Chrominance AC.
    HuffmanSpec {
        count: [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119],
        value: &[
            0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07,
            0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09,
            0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25,
            0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56,
            0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74,
            0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
            0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba,
            0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6,
            0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2,
            0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
        ],
    },
];

/// huffmanLUT is a compiled look-up table representation of a huffmanSpec.
/// Each value maps to a uint32 of which the 8 most significant bits hold the
/// codeword size in bits and the 24 least significant bits hold the codeword.
/// The maximum codeword size is 16 bits.
///
/// Go: image/jpeg/writer.go:huffmanLUT.init
fn huffman_lut_init(s: &HuffmanSpec) -> Vec<u32> {
    let mut max_value = 0usize;
    for &v in s.value {
        if v as usize > max_value {
            max_value = v as usize;
        }
    }
    let mut h = vec![0u32; max_value + 1];
    let (mut code, mut k) = (0u32, 0usize);
    for i in 0..s.count.len() {
        let n_bits = ((i + 1) as u32) << 24;
        for _j in 0..s.count[i] {
            h[s.value[k] as usize] = n_bits | code;
            code += 1;
            k += 1;
        }
        code <<= 1;
    }
    h
}

// theHuffmanLUT are compiled representations of theHuffmanSpec.
// Go: image/jpeg/writer.go:theHuffmanLUT (built in init()).
fn the_huffman_lut() -> &'static [Vec<u32>; 4] {
    static LUT: std::sync::OnceLock<[Vec<u32>; 4]> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        [
            huffman_lut_init(&THE_HUFFMAN_SPEC[0]),
            huffman_lut_init(&THE_HUFFMAN_SPEC[1]),
            huffman_lut_init(&THE_HUFFMAN_SPEC[2]),
            huffman_lut_init(&THE_HUFFMAN_SPEC[3]),
        ]
    })
}

/// encoder encodes an image to the JPEG format.
///
/// Go: image/jpeg/writer.go:encoder. Go wraps w in a bufio.Writer unless it
/// already has Flush/WriteByte; the port buffers internally (4096 bytes, as
/// bufio's default) — the bytes written are identical.
struct Encoder<'w> {
    // w is the writer to write to. err is the first error encountered during
    // writing. All attempted writes after the first error become no-ops.
    w: &'w mut dyn Write,
    wbuf: Vec<u8>,
    err: Option<std::io::Error>,
    // buf is a scratch buffer.
    buf: [u8; 16],
    // bits and nBits are accumulated bits to write to w.
    bits: u32,
    n_bits: u32,
    // quant is the scaled quantization tables, in zig-zag order.
    quant: [[u8; BLOCK_SIZE]; N_QUANT_INDEX],
    lut: &'static [Vec<u32>; 4],
}

// Size of the internal output buffer (Go uses a 4096-byte bufio.Writer; the
// size only changes how the byte stream is chunked into Write calls).
const WBUF_SIZE: usize = 1 << 16;

impl Encoder<'_> {
    // Hands the buffered bytes to w unless an earlier write failed (after the
    // first error all writes are no-ops, as in Go).
    fn spill(&mut self) {
        if self.err.is_none() {
            if let Err(e) = self.w.write_all(&self.wbuf) {
                self.err = Some(e);
            }
        }
        self.wbuf.clear();
    }

    // Go: image/jpeg/writer.go:encoder.flush
    fn flush(&mut self) {
        self.spill();
        if self.err.is_none() {
            if let Err(e) = self.w.flush() {
                self.err = Some(e);
            }
        }
    }

    // Go: image/jpeg/writer.go:encoder.write
    fn write(&mut self, p: &[u8]) {
        if self.wbuf.len() + p.len() > WBUF_SIZE {
            self.spill();
        }
        self.wbuf.extend_from_slice(p);
    }

    // Go: image/jpeg/writer.go:encoder.writeByte
    #[inline(always)]
    fn write_byte(&mut self, b: u8) {
        if self.wbuf.len() >= WBUF_SIZE {
            self.spill();
        }
        self.wbuf.push(b);
    }

    /// emit emits the least significant nBits bits of bits to the bit-stream.
    /// The precondition is bits < 1<<nBits && nBits <= 16.
    ///
    /// Go: image/jpeg/writer.go:encoder.emit
    fn emit(&mut self, mut bits: u32, mut n_bits: u32) {
        n_bits = n_bits.wrapping_add(self.n_bits);
        bits = crate::go_shl_u32(bits, 32u32.wrapping_sub(n_bits));
        bits |= self.bits;
        while n_bits >= 8 {
            let b = (bits >> 24) as u8;
            self.write_byte(b);
            if b == 0xff {
                self.write_byte(0x00);
            }
            bits <<= 8;
            n_bits -= 8;
        }
        (self.bits, self.n_bits) = (bits, n_bits);
    }

    /// emitHuff emits the given value with the given Huffman encoder.
    ///
    /// Go: image/jpeg/writer.go:encoder.emitHuff
    #[inline]
    fn emit_huff(&mut self, h: HuffIndex, value: i32) {
        let x = self.lut[h][value as usize];
        self.emit(x & ((1 << 24) - 1), x >> 24);
    }

    /// emitHuffRLE emits a run of runLength copies of value encoded with the
    /// given Huffman encoder.
    ///
    /// Go: image/jpeg/writer.go:encoder.emitHuffRLE
    fn emit_huff_rle(&mut self, h: HuffIndex, run_length: i32, value: i32) {
        let (mut a, mut b) = (value, value);
        if a < 0 {
            (a, b) = (value.wrapping_neg(), value.wrapping_sub(1));
        }
        let n_bits: u32 = if a < 0x100 {
            BIT_COUNT[a as usize] as u32
        } else {
            8 + BIT_COUNT[(a >> 8) as usize] as u32
        };
        self.emit_huff(h, run_length << 4 | n_bits as i32);
        if n_bits > 0 {
            self.emit(
                (b as u32) & (crate::go_shl_u32(1, n_bits).wrapping_sub(1)),
                n_bits,
            );
        }
    }

    /// writeMarkerHeader writes the header for a marker with the given length.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeMarkerHeader
    fn write_marker_header(&mut self, marker: u8, markerlen: i64) {
        self.buf[0] = 0xff;
        self.buf[1] = marker;
        self.buf[2] = (markerlen >> 8) as u8;
        self.buf[3] = (markerlen & 0xff) as u8;
        let b = self.buf;
        self.write(&b[..4]);
    }

    /// writeDQT writes the Define Quantization Table marker.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeDQT
    fn write_dqt(&mut self) {
        let markerlen = 2 + (N_QUANT_INDEX as i64) * (1 + BLOCK_SIZE as i64);
        self.write_marker_header(DQT_MARKER, markerlen);
        for i in 0..N_QUANT_INDEX {
            self.write_byte(i as u8);
            let q = self.quant[i];
            self.write(&q);
        }
    }

    /// writeSOF0 writes the Start Of Frame (Baseline Sequential) marker.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeSOF0
    fn write_sof0(&mut self, size: Point, n_component: usize) {
        let markerlen = 8 + 3 * n_component as i64;
        self.write_marker_header(SOF0_MARKER, markerlen);
        self.buf[0] = 8; // 8-bit color.
        self.buf[1] = (size.y >> 8) as u8;
        self.buf[2] = (size.y & 0xff) as u8;
        self.buf[3] = (size.x >> 8) as u8;
        self.buf[4] = (size.x & 0xff) as u8;
        self.buf[5] = n_component as u8;
        if n_component == 1 {
            self.buf[6] = 1;
            // No subsampling for grayscale image.
            self.buf[7] = 0x11;
            self.buf[8] = 0x00;
        } else {
            for i in 0..n_component {
                self.buf[3 * i + 6] = (i + 1) as u8;
                // We use 4:2:0 chroma subsampling.
                self.buf[3 * i + 7] = b"\x22\x11\x11"[i];
                self.buf[3 * i + 8] = b"\x00\x01\x01"[i];
            }
        }
        let b = self.buf;
        self.write(&b[..3 * (n_component - 1) + 9]);
    }

    /// writeDHT writes the Define Huffman Table marker.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeDHT
    fn write_dht(&mut self, n_component: usize) {
        let mut markerlen = 2i64;
        let mut specs: &[HuffmanSpec] = &THE_HUFFMAN_SPEC[..];
        if n_component == 1 {
            // Drop the Chrominance tables.
            specs = &specs[..2];
        }
        for s in specs {
            markerlen += 1 + 16 + s.value.len() as i64;
        }
        self.write_marker_header(DHT_MARKER, markerlen);
        for (i, s) in specs.iter().enumerate() {
            self.write_byte(b"\x00\x10\x01\x11"[i]);
            self.write(&s.count);
            self.write(s.value);
        }
    }

    /// writeBlock writes a block of pixel data using the given quantization
    /// table, returning the post-quantized DC value of the DCT-transformed
    /// block. b is in natural (not zig-zag) order.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeBlock
    fn write_block(&mut self, b: &mut Block, q: usize, prev_dc: i32) -> i32 {
        fdct(b);
        // Emit the DC delta.
        let dc = div(b[0], 8 * self.quant[q][0] as i32);
        self.emit_huff_rle(2 * q, 0, dc.wrapping_sub(prev_dc));
        // Emit the AC components.
        let (h, mut run_length) = (2 * q + 1, 0i32);
        for zig in 1..BLOCK_SIZE {
            let ac = div(b[UNZIG[zig]], 8 * self.quant[q][zig] as i32);
            if ac == 0 {
                run_length += 1;
            } else {
                while run_length > 15 {
                    self.emit_huff(h, 0xf0);
                    run_length -= 16;
                }
                self.emit_huff_rle(h, run_length, ac);
                run_length = 0;
            }
        }
        if run_length > 0 {
            self.emit_huff(h, 0x00);
        }
        dc
    }

    /// writeSOS writes the StartOfScan marker.
    ///
    /// Go: image/jpeg/writer.go:encoder.writeSOS
    fn write_sos(&mut self, m: &dyn Image) {
        let gray = m.as_any().downcast_ref::<Gray>();
        if gray.is_some() {
            self.write(&SOS_HEADER_Y);
        } else {
            self.write(&SOS_HEADER_YCBCR);
        }
        // Scratch buffers to hold the YCbCr values.
        // The blocks are in natural (not zig-zag) order.
        let mut b: Block = [0; BLOCK_SIZE];
        let mut cb: [Block; 4] = [[0; BLOCK_SIZE]; 4];
        let mut cr: [Block; 4] = [[0; BLOCK_SIZE]; 4];
        // DC components are delta-encoded.
        let (mut prev_dc_y, mut prev_dc_cb, mut prev_dc_cr) = (0i32, 0i32, 0i32);
        let bounds = m.bounds();
        // TODO(wathiede): switch on m.ColorModel() instead of type.
        if let Some(m) = gray {
            let mut y = bounds.min.y;
            while y < bounds.max.y {
                let mut x = bounds.min.x;
                while x < bounds.max.x {
                    let p = pt(x, y);
                    gray_to_y(m, p, &mut b);
                    prev_dc_y = self.write_block(&mut b, QUANT_INDEX_LUMINANCE, prev_dc_y);
                    x += 8;
                }
                y += 8;
            }
        } else {
            let rgba = m.as_any().downcast_ref::<RGBA>();
            let ycbcr = m.as_any().downcast_ref::<YCbCr>();
            let mut y = bounds.min.y;
            while y < bounds.max.y {
                let mut x = bounds.min.x;
                while x < bounds.max.x {
                    for i in 0..4usize {
                        let x_off = ((i & 1) * 8) as i64;
                        let y_off = ((i & 2) * 4) as i64;
                        let p = pt(x + x_off, y + y_off);
                        if let Some(rgba) = rgba {
                            rgba_to_ycbcr(rgba, p, &mut b, &mut cb[i], &mut cr[i]);
                        } else if let Some(ycbcr) = ycbcr {
                            ycbcr_to_ycbcr(ycbcr, p, &mut b, &mut cb[i], &mut cr[i]);
                        } else {
                            to_ycbcr(m, p, &mut b, &mut cb[i], &mut cr[i]);
                        }
                        prev_dc_y = self.write_block(&mut b, 0, prev_dc_y);
                    }
                    scale(&mut b, &cb);
                    prev_dc_cb = self.write_block(&mut b, 1, prev_dc_cb);
                    scale(&mut b, &cr);
                    prev_dc_cr = self.write_block(&mut b, 1, prev_dc_cr);
                    x += 16;
                }
                y += 16;
            }
        }
        // Pad the last byte with 1's.
        self.emit(0x7f, 7);
    }
}

/// toYCbCr converts the 8x8 region of m whose top-left corner is p to its
/// YCbCr values.
///
/// Go: image/jpeg/writer.go:toYCbCr
fn to_ycbcr(
    m: &dyn Image,
    p: Point,
    y_block: &mut Block,
    cb_block: &mut Block,
    cr_block: &mut Block,
) {
    let b = m.bounds();
    let xmax = b.max.x - 1;
    let ymax = b.max.y - 1;
    for j in 0..8i64 {
        for i in 0..8i64 {
            let (r, g, b, _) = m.at((p.x + i).min(xmax), (p.y + j).min(ymax)).rgba();
            let (yy, cb, cr) = color::rgb_to_ycbcr((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8);
            let k = (8 * j + i) as usize;
            y_block[k] = yy as i32;
            cb_block[k] = cb as i32;
            cr_block[k] = cr as i32;
        }
    }
}

/// grayToY stores the 8x8 region of m whose top-left corner is p in yBlock.
///
/// Go: image/jpeg/writer.go:grayToY
fn gray_to_y(m: &Gray, p: Point, y_block: &mut Block) {
    let b = m.bounds();
    let xmax = b.max.x - 1;
    let ymax = b.max.y - 1;
    let pix = &m.pix;
    for j in 0..8i64 {
        for i in 0..8i64 {
            let idx = m.pix_offset((p.x + i).min(xmax), (p.y + j).min(ymax));
            y_block[(8 * j + i) as usize] = pix[idx as usize] as i32;
        }
    }
}

/// rgbaToYCbCr is a specialized version of toYCbCr for image.RGBA images.
///
/// Go: image/jpeg/writer.go:rgbaToYCbCr
fn rgba_to_ycbcr(
    m: &RGBA,
    p: Point,
    y_block: &mut Block,
    cb_block: &mut Block,
    cr_block: &mut Block,
) {
    let b = m.bounds();
    let xmax = b.max.x - 1;
    let ymax = b.max.y - 1;
    for j in 0..8usize {
        let mut sj = p.y + j as i64;
        if sj > ymax {
            sj = ymax;
        }
        let offset = (sj - b.min.y) * m.stride - b.min.x * 4;
        for i in 0..8usize {
            let mut sx = p.x + i as i64;
            if sx > xmax {
                sx = xmax;
            }
            let o = (offset + sx * 4) as usize;
            let pix = &m.pix[o..o + 3];
            let (yy, cb, cr) = color::rgb_to_ycbcr(pix[0], pix[1], pix[2]);
            y_block[8 * j + i] = yy as i32;
            cb_block[8 * j + i] = cb as i32;
            cr_block[8 * j + i] = cr as i32;
        }
    }
}

/// yCbCrToYCbCr is a specialized version of toYCbCr for image.YCbCr images.
///
/// Go: image/jpeg/writer.go:yCbCrToYCbCr
fn ycbcr_to_ycbcr(
    m: &YCbCr,
    p: Point,
    y_block: &mut Block,
    cb_block: &mut Block,
    cr_block: &mut Block,
) {
    let b = m.bounds();
    let xmax = b.max.x - 1;
    let ymax = b.max.y - 1;
    for j in 0..8i64 {
        let mut sy = p.y + j;
        if sy > ymax {
            sy = ymax;
        }
        for i in 0..8i64 {
            let mut sx = p.x + i;
            if sx > xmax {
                sx = xmax;
            }
            let yi = m.y_offset(sx, sy) as usize;
            let ci = m.c_offset(sx, sy) as usize;
            let k = (8 * j + i) as usize;
            y_block[k] = m.y[yi] as i32;
            cb_block[k] = m.cb[ci] as i32;
            cr_block[k] = m.cr[ci] as i32;
        }
    }
}

/// scale scales the 16x16 region represented by the 4 src blocks to the 8x8
/// dst block.
///
/// Go: image/jpeg/writer.go:scale
fn scale(dst: &mut Block, src: &[Block; 4]) {
    for i in 0..4usize {
        let dst_off = (i & 2) << 4 | (i & 1) << 2;
        for y in 0..4usize {
            for x in 0..4usize {
                let j = 16 * y + 2 * x;
                let sum = src[i][j] + src[i][j + 1] + src[i][j + 8] + src[i][j + 9];
                dst[8 * y + x + dst_off] = (sum + 2) >> 2;
            }
        }
    }
}

// sosHeaderY is the SOS marker "\xff\xda" followed by 8 bytes:
//   - the marker length "\x00\x08",
//   - the number of components "\x01",
//   - component 1 uses DC table 0 and AC table 0 "\x01\x00",
//   - the bytes "\x00\x3f\x00". Section B.2.3 of the spec says that for
//     sequential DCTs, those bytes (8-bit Ss, 8-bit Se, 4-bit Ah, 4-bit Al)
//     should be 0x00, 0x3f, 0x00<<4 | 0x00.
// Go: image/jpeg/writer.go:sosHeaderY
const SOS_HEADER_Y: [u8; 10] = [0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00];

// sosHeaderYCbCr is the SOS marker "\xff\xda" followed by 12 bytes:
//   - the marker length "\x00\x0c",
//   - the number of components "\x03",
//   - component 1 uses DC table 0 and AC table 0 "\x01\x00",
//   - component 2 uses DC table 1 and AC table 1 "\x02\x11",
//   - component 3 uses DC table 1 and AC table 1 "\x03\x11",
//   - the bytes "\x00\x3f\x00".
// Go: image/jpeg/writer.go:sosHeaderYCbCr
const SOS_HEADER_YCBCR: [u8; 14] = [
    0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00,
];

/// DefaultQuality is the default quality encoding parameter.
///
/// Go: image/jpeg/writer.go:DefaultQuality
pub const DEFAULT_QUALITY: i64 = 75;

/// Options are the encoding parameters.
/// Quality ranges from 1 to 100 inclusive, higher is better.
///
/// Go: image/jpeg/writer.go:Options
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub quality: i64,
}

/// Encode writes the Image m to w in JPEG 4:2:0 baseline format with the
/// given options. Default parameters are used if `None` (Go: nil *Options)
/// is passed.
///
/// Go: image/jpeg/writer.go:Encode
pub fn encode<W: Write + ?Sized>(
    w: &mut W,
    m: &dyn Image,
    o: Option<&Options>,
) -> Result<(), Error> {
    let b = m.bounds();
    if b.dx() >= 1 << 16 || b.dy() >= 1 << 16 {
        return Err(Error::Encode("jpeg: image is too large to encode"));
    }
    let mut ww = WriteAdapter(w);
    let mut e = Encoder {
        w: &mut ww,
        wbuf: Vec::with_capacity(WBUF_SIZE),
        err: None,
        buf: [0; 16],
        bits: 0,
        n_bits: 0,
        quant: [[0; BLOCK_SIZE]; N_QUANT_INDEX],
        lut: the_huffman_lut(),
    };
    // Clip quality to [1, 100].
    let mut quality = DEFAULT_QUALITY;
    if let Some(o) = o {
        quality = o.quality;
        if quality < 1 {
            quality = 1;
        } else if quality > 100 {
            quality = 100;
        }
    }
    // Convert from a quality rating to a scaling factor.
    let scale = if quality < 50 {
        5000 / quality
    } else {
        200 - quality * 2
    };
    // Initialize the quantization tables.
    for i in 0..N_QUANT_INDEX {
        for j in 0..BLOCK_SIZE {
            let mut x = UNSCALED_QUANT[i][j] as i64;
            x = (x * scale + 50) / 100;
            if x < 1 {
                x = 1;
            } else if x > 255 {
                x = 255;
            }
            e.quant[i][j] = x as u8;
        }
    }
    // Compute number of components based on input image type.
    // TODO(wathiede): switch on m.ColorModel() instead of type.
    let n_component = if m.as_any().is::<Gray>() { 1 } else { 3 };
    // Write the Start Of Image marker.
    e.buf[0] = 0xff;
    e.buf[1] = 0xd8;
    let bb = e.buf;
    e.write(&bb[..2]);
    // Write the quantization tables.
    e.write_dqt();
    // Write the image dimensions.
    e.write_sof0(b.size(), n_component);
    // Write the Huffman tables.
    e.write_dht(n_component);
    // Write the image data.
    e.write_sos(m);
    // Write the End Of Image marker.
    e.buf[0] = 0xff;
    e.buf[1] = 0xd9;
    let bb = e.buf;
    e.write(&bb[..2]);
    e.flush();
    match e.err {
        Some(err) => Err(Error::Io(err)),
        None => Ok(()),
    }
}

// Adapts `W: Write + ?Sized` to a sized `dyn Write` target.
struct WriteAdapter<'a, W: Write + ?Sized>(&'a mut W);

impl<W: Write + ?Sized> Write for WriteAdapter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // zigzag maps from the natural ordering to the zig-zag ordering.
    // Go: image/jpeg/writer_test.go:zigzag
    const ZIGZAG: [usize; BLOCK_SIZE] = [
        0, 1, 5, 6, 14, 15, 27, 28, 2, 4, 7, 13, 16, 26, 29, 42, 3, 8, 12, 17, 25, 30, 41, 43, 9,
        11, 18, 24, 31, 40, 44, 53, 10, 19, 23, 32, 39, 45, 52, 54, 20, 22, 33, 38, 46, 51, 55, 60,
        21, 34, 37, 47, 50, 56, 59, 61, 35, 36, 48, 49, 57, 58, 62, 63,
    ];

    // Go: image/jpeg/writer_test.go:TestZigUnzig
    #[test]
    fn zig_unzig() {
        for i in 0..BLOCK_SIZE {
            assert_eq!(UNZIG[ZIGZAG[i]], i);
            assert_eq!(ZIGZAG[UNZIG[i]], i);
        }
    }

    // Go: image/jpeg/writer_test.go:TestUnscaledQuant
    #[test]
    fn unscaled_quant() {
        const NATURAL: [[u8; BLOCK_SIZE]; N_QUANT_INDEX] = [
            [
                16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40,
                57, 69, 56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24,
                35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98,
                112, 100, 103, 99,
            ],
            [
                17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99,
                99, 99, 99, 47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
                99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
                99,
            ],
        ];
        for i in 0..N_QUANT_INDEX {
            for zig in 0..BLOCK_SIZE {
                assert_eq!(UNSCALED_QUANT[i][zig], NATURAL[i][UNZIG[zig]]);
            }
        }
    }

    #[test]
    fn bit_count_table() {
        assert_eq!(BIT_COUNT[0], 0);
        assert_eq!(BIT_COUNT[1], 1);
        assert_eq!(BIT_COUNT[3], 2);
        assert_eq!(BIT_COUNT[128], 8);
        assert_eq!(BIT_COUNT[255], 8);
    }
}
