//! Port of the go1.27.1 GIF encoder (`image/gif/writer.go`: `Encode`, `EncodeAll` for one
//! frame, the encoder, `blockWriter`, `encodeColorTable`) and of the LZW compressor it uses
//! (`compress/lzw/writer.go`, LSB order), for `Image.EncodeTo`'s GIF target
//! (`gif.Encode(w, img, &gif.Options{NumColors: 256})`).
//!
//! Only the single-frame `Encode` path is reachable from Hugo without a GIF decoder (the
//! animated `Giphy` path needs `gif.DecodeAll`, which is not ported). The output is the
//! same byte stream; Go's `bufio.Writer` around the destination only changes the chunking of the
//! writes.

use std::io::Write;

use go_image::color::{Color, Palette};
use go_image::draw::{self, Drawer};
use go_image::{Image, Paletted, Point};
use nh_common::{Error, Result};

const FCOLOR_TABLE: u8 = 0x80;
const S_EXTENSION: u8 = 0x21;
const S_IMAGE_DESCRIPTOR: u8 = 0x2C;
const S_TRAILER: u8 = 0x3B;
const GC_LABEL: u8 = 0xF9;
const GC_BLOCK_SIZE: u8 = 0x04;

// Go: image/gif/writer.go:log2
fn log2(x: usize) -> usize {
    if x < 2 {
        return 0;
    }
    // bits.Len(uint(x-1)) - 1
    (usize::BITS - (x - 1).leading_zeros()) as usize - 1
}

/// Go: `gif.encoder` (one frame, `LoopCount` 0, no disposal).
struct Encoder<'w> {
    w: &'w mut Vec<u8>,
    /// buf is a scratch buffer. It must be at least 256 for the blockWriter.
    buf: [u8; 256],
    global_ct: usize,
    global_color_table: [u8; 3 * 256],
    local_color_table: [u8; 3 * 256],
    err: Option<String>,
}

impl Encoder<'_> {
    // Go: image/gif/writer.go:(*encoder).write
    fn write(&mut self, p: &[u8]) {
        if self.err.is_some() {
            return;
        }
        self.w.extend_from_slice(p);
    }

    // Go: image/gif/writer.go:(*encoder).writeByte
    fn write_byte(&mut self, b: u8) {
        if self.err.is_some() {
            return;
        }
        self.w.push(b);
    }

    // Go: image/gif/writer.go:(*encoder).writeHeader (one frame: no NETSCAPE loop block)
    fn write_header(&mut self, width: i64, height: i64, p: &Palette) {
        if self.err.is_some() {
            return;
        }
        self.w.extend_from_slice(b"GIF89a");

        // Logical screen width and height.
        self.buf[0..2].copy_from_slice(&(width as u16).to_le_bytes());
        self.buf[2..4].copy_from_slice(&(height as u16).to_le_bytes());
        let b = self.buf;
        self.write(&b[..4]);

        if !p.0.is_empty() {
            let padded_size = log2(p.0.len()); // Size of Global Color Table: 2^(1+n).
            self.buf[0] = FCOLOR_TABLE | padded_size as u8;
            self.buf[1] = 0; // BackgroundIndex
            self.buf[2] = 0x00; // Pixel Aspect Ratio.
            let b = self.buf;
            self.write(&b[..3]);
            match encode_color_table(&mut self.global_color_table, p, padded_size) {
                Ok(n) => self.global_ct = n,
                Err(e) => {
                    if self.err.is_none() {
                        self.err = Some(e);
                    }
                    return;
                }
            }
            let t = self.global_color_table;
            self.write(&t[..self.global_ct]);
        } else {
            // All frames have a local color table, so a global color table is not needed.
            self.buf[0] = 0x00;
            self.buf[1] = 0x00; // Background Color Index.
            self.buf[2] = 0x00; // Pixel Aspect Ratio.
            let b = self.buf;
            self.write(&b[..3]);
        }
    }

    /// Go: `writeImageBlock(pm, 0, 0)` of the single frame, whose palette is the global colour
    /// table (the same slice, so `&gp[0] == &pm.Palette[0]`).
    // Go: image/gif/writer.go:(*encoder).writeImageBlock
    fn write_image_block(&mut self, pm: &Paletted, width: i64, height: i64) {
        if self.err.is_some() {
            return;
        }

        if pm.palette.0.is_empty() {
            self.err = Some("gif: cannot encode image block with empty palette".into());
            return;
        }

        let b = pm.rect;
        if b.min.x < 0 || b.max.x >= 1 << 16 || b.min.y < 0 || b.max.y >= 1 << 16 {
            self.err = Some("gif: image block is too large to encode".into());
            return;
        }
        if !b.in_(go_image::rect(0, 0, width, height)) {
            self.err = Some("gif: image block is out of bounds".into());
            return;
        }

        let mut transparent_index: i64 = -1;
        for (i, c) in pm.palette.0.iter().enumerate() {
            let (_, _, _, a) = c.rgba();
            if a == 0 {
                transparent_index = i as i64;
                break;
            }
        }

        if transparent_index != -1 {
            self.buf[0] = S_EXTENSION; // Extension Introducer.
            self.buf[1] = GC_LABEL; // Graphic Control Label.
            self.buf[2] = GC_BLOCK_SIZE; // Block Size.
            self.buf[3] = 0x01;
            self.buf[4..6].copy_from_slice(&0u16.to_le_bytes()); // Delay Time
            // Transparent color index.
            self.buf[6] = transparent_index as u8;
            self.buf[7] = 0x00; // Block Terminator.
            let bb = self.buf;
            self.write(&bb[..8]);
        }
        self.buf[0] = S_IMAGE_DESCRIPTOR;
        self.buf[1..3].copy_from_slice(&(b.min.x as u16).to_le_bytes());
        self.buf[3..5].copy_from_slice(&(b.min.y as u16).to_le_bytes());
        self.buf[5..7].copy_from_slice(&(b.dx() as u16).to_le_bytes());
        self.buf[7..9].copy_from_slice(&(b.dy() as u16).to_le_bytes());
        let bb = self.buf;
        self.write(&bb[..9]);

        let padded_size = log2(pm.palette.0.len()); // Size of Local Color Table: 2^(1+n).
        // The frame's palette is the global colour table itself.
        self.write_byte(0); // Use the global color table.

        let mut lit_width = padded_size + 1;
        if lit_width < 2 {
            lit_width = 2;
        }
        self.write_byte(lit_width as u8); // LZW Minimum Code Size.

        // blockWriter.setup
        self.buf[0] = 0;
        let mut lzww = LzwWriter::new(lit_width as u32);
        let dx = b.dx();
        if dx == pm.stride {
            let n = (dx * b.dy()) as usize;
            lzww.write(self, &pm.pix[..n]);
        } else {
            let mut i: i64 = 0;
            let mut y = b.min.y;
            while y < b.max.y {
                lzww.write(self, &pm.pix[i as usize..(i + dx) as usize]);
                i += pm.stride;
                y += 1;
            }
        }
        lzww.close(self); // flush to bw
        self.block_writer_close(); // flush to e.w
    }

    // Go: image/gif/writer.go:blockWriter.WriteByte
    fn block_write_byte(&mut self, c: u8) {
        if self.err.is_some() {
            return;
        }

        // Append c to buffered sub-block.
        self.buf[0] += 1;
        let n = self.buf[0] as usize;
        self.buf[n] = c;
        if self.buf[0] < 255 {
            return;
        }

        // Flush block
        let b = self.buf;
        self.write(&b[..256]);
        self.buf[0] = 0;
    }

    // Go: image/gif/writer.go:blockWriter.close
    fn block_writer_close(&mut self) {
        // Write the block terminator (0x00), either by itself, or along with a pending
        // sub-block.
        if self.buf[0] == 0 {
            self.write_byte(0);
        } else {
            let n = self.buf[0] as usize;
            self.buf[n + 1] = 0;
            let b = self.buf;
            self.write(&b[..n + 2]);
        }
    }
}

// Go: image/gif/writer.go:encodeColorTable
fn encode_color_table(
    dst: &mut [u8],
    p: &Palette,
    size: usize,
) -> std::result::Result<usize, String> {
    if size >= 8 {
        return Err("gif: cannot encode color table with more than 256 entries".into());
    }
    for (i, c) in p.0.iter().enumerate() {
        let (r, g, b) = match c {
            // It is most likely that the palette is full of color.RGBAs, so they get a fast
            // path.
            Color::RGBA(rgba) => (rgba.r, rgba.g, rgba.b),
            c => {
                let (rr, gg, bb, _) = c.rgba();
                ((rr >> 8) as u8, (gg >> 8) as u8, (bb >> 8) as u8)
            }
        };
        dst[3 * i] = r;
        dst[3 * i + 1] = g;
        dst[3 * i + 2] = b;
    }
    let n = 1 << (size + 1);
    if n > p.0.len() {
        // Pad with black.
        for x in &mut dst[3 * p.0.len()..3 * n] {
            *x = 0;
        }
    }
    Ok(3 * n)
}

/// Go: `gif.Encode(w, m, &gif.Options{NumColors: 256})` (nil Quantizer, FloydSteinberg drawer).
// Go: image/gif/writer.go:Encode
pub(crate) fn encode(w: &mut dyn Write, m: &dyn Image) -> Result<()> {
    // Check for bounds and size restrictions.
    let b = m.bounds();
    if b.dx() >= 1 << 16 || b.dy() >= 1 << 16 {
        return Err(Error::new("gif: image is too large to encode"));
    }

    let num_colors = 256;

    let mut pm: Option<Paletted> = m.as_any().downcast_ref::<Paletted>().cloned();
    if pm.is_none()
        && let go_image::color::Model::Palette(cp) = m.color_model()
    {
        let mut p = Paletted::new(b, cp.clone());
        for y in b.min.y..b.max.y {
            for x in b.min.x..b.max.x {
                if let Some(c) = cp.convert(m.at(x, y)) {
                    p.set(x, y, c);
                }
            }
        }
        pm = Some(p);
    }
    let mut pm = match pm {
        Some(p) if p.palette.0.len() <= num_colors => p,
        _ => {
            // Set pm to be a palettedized copy of m, including its bounds, which might not
            // start at (0, 0).
            let plan9 = go_image::color::palette::plan9();
            let mut p = Paletted::new(b, Palette(plan9.0[..num_colors].to_vec()));
            draw::FLOYD_STEINBERG.draw(&mut p, b, m, b.min);
            p
        }
    };

    // When calling Encode instead of EncodeAll, the single-frame image is translated such that
    // its top-left corner is (0, 0), so that the single frame completely fills the overall
    // GIF's bounds.
    if pm.rect.min != (Point { x: 0, y: 0 }) {
        pm.rect = pm.rect.sub(pm.rect.min);
    }

    // EncodeAll with Config{ColorModel: pm.Palette, Width: b.Dx(), Height: b.Dy()}.
    let mut out = Vec::new();
    let mut e = Encoder {
        w: &mut out,
        buf: [0; 256],
        global_ct: 0,
        global_color_table: [0; 3 * 256],
        local_color_table: [0; 3 * 256],
        err: None,
    };
    let palette = pm.palette.clone();
    e.write_header(b.dx(), b.dy(), &palette);
    e.write_image_block(&pm, b.dx(), b.dy());
    e.write_byte(S_TRAILER);
    let _ = e.local_color_table;
    if let Some(err) = e.err {
        // Go writes what it has produced before the error through its bufio.Writer only on
        // Flush, which it does not reach after an error.
        return Err(Error::new(err));
    }
    w.write_all(&out).map_err(|e| Error::new(e.to_string()))
}

// ---------------------------------------------------------------------------
// compress/lzw (LSB)

/// A code is a 12 bit value, stored as a uint32 when encoding to avoid type conversions when
/// shifting bits.
const MAX_CODE: u32 = (1 << 12) - 1;
const INVALID_CODE: u32 = u32::MAX;
/// There are 1<<12 possible codes, which is an upper bound on the number of valid hash table
/// entries at any given point in time. tableSize is 4x that.
const TABLE_SIZE: usize = 4 * (1 << 12);
const TABLE_MASK: u32 = TABLE_SIZE as u32 - 1;
/// A hash table entry is a uint32. Zero is an invalid entry since the lower 12 bits of a valid
/// entry must be a non-literal code.
const INVALID_ENTRY: u32 = 0;

/// Go: `lzw.Writer` in LSB order writing to the GIF block writer.
struct LzwWriter {
    lit_width: u32,
    n_bits: u32,
    width: u32,
    bits: u32,
    hi: u32,
    overflow: u32,
    saved_code: u32,
    table: Vec<u32>,
}

impl LzwWriter {
    // Go: compress/lzw/writer.go:(*Writer).init
    fn new(lit_width: u32) -> LzwWriter {
        LzwWriter {
            lit_width,
            n_bits: 0,
            width: 1 + lit_width,
            bits: 0,
            hi: (1 << lit_width) + 1,
            overflow: 1 << (lit_width + 1),
            saved_code: INVALID_CODE,
            table: vec![INVALID_ENTRY; TABLE_SIZE],
        }
    }

    // Go: compress/lzw/writer.go:(*Writer).writeLSB
    fn write_lsb(&mut self, e: &mut Encoder<'_>, c: u32) {
        self.bits |= c << self.n_bits;
        self.n_bits += self.width;
        while self.n_bits >= 8 {
            e.block_write_byte(self.bits as u8);
            self.bits >>= 8;
            self.n_bits -= 8;
        }
    }

    /// Returns whether the codes ran out (Go's `errOutOfCodes`).
    // Go: compress/lzw/writer.go:(*Writer).incHi
    fn inc_hi(&mut self, e: &mut Encoder<'_>) -> bool {
        self.hi += 1;
        if self.hi == self.overflow {
            self.width += 1;
            self.overflow <<= 1;
        }
        if self.hi == MAX_CODE {
            let clear = 1u32 << self.lit_width;
            self.write_lsb(e, clear);
            self.width = self.lit_width + 1;
            self.hi = clear + 1;
            self.overflow = clear << 1;
            for t in self.table.iter_mut() {
                *t = INVALID_ENTRY;
            }
            return true;
        }
        false
    }

    /// (The palette indexes never exceed `1<<litWidth - 1`: the palette has at most
    /// `1<<(litWidth)` entries, so the litWidth check cannot fail.)
    // Go: compress/lzw/writer.go:(*Writer).Write
    fn write(&mut self, e: &mut Encoder<'_>, mut p: &[u8]) {
        if p.is_empty() {
            return;
        }
        let mut code = self.saved_code;
        if code == INVALID_CODE {
            // This is the first write; send a clear code.
            let clear = 1u32 << self.lit_width;
            self.write_lsb(e, clear);
            // After the starting clear code, the next code sent (for non-empty input) is always
            // a literal code.
            code = p[0] as u32;
            p = &p[1..];
        }
        'lp: for &x in p {
            let literal = x as u32;
            let key = code << 8 | literal;
            // If there is a hash table hit for this key then we continue the loop and do not
            // emit a code yet.
            let mut hash = (key >> 12 ^ key) & TABLE_MASK;
            let mut h = hash;
            let mut t = self.table[hash as usize];
            while t != INVALID_ENTRY {
                if key == t >> 12 {
                    code = t & MAX_CODE;
                    continue 'lp;
                }
                h = (h + 1) & TABLE_MASK;
                t = self.table[h as usize];
            }
            // Otherwise, write the current code, and literal becomes the start of the next
            // emitted code.
            self.write_lsb(e, code);
            code = literal;
            // Increment e.hi, the next implied code. If we run out of codes, reset the writer
            // state (including clearing the hash table) and continue.
            if self.inc_hi(e) {
                continue;
            }
            // Otherwise, insert key -> e.hi into the map that e.table represents.
            loop {
                if self.table[hash as usize] == INVALID_ENTRY {
                    self.table[hash as usize] = (key << 12) | self.hi;
                    break;
                }
                hash = (hash + 1) & TABLE_MASK;
            }
        }
        self.saved_code = code;
    }

    // Go: compress/lzw/writer.go:(*Writer).Close
    fn close(&mut self, e: &mut Encoder<'_>) {
        // Write the savedCode if valid.
        if self.saved_code != INVALID_CODE {
            self.write_lsb(e, self.saved_code);
            self.inc_hi(e);
        } else {
            // Write the starting clear code, as w.Write did not.
            let clear = 1u32 << self.lit_width;
            self.write_lsb(e, clear);
        }
        // Write the eof code.
        let eof = (1u32 << self.lit_width) + 1;
        self.write_lsb(e, eof);
        // Write the final bits.
        if self.n_bits > 0 {
            e.block_write_byte(self.bits as u8);
        }
    }
}
