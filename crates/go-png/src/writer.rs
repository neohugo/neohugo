//! Port of go1.27.1 `image/png/writer.go`: the PNG encoder.

use std::io::Write;
use std::sync::Arc;

use go_flate::zlib;
use go_image::color::{self, Color, Model, Palette};
use go_image::{Gray, Image, NRGBA, Paletted, RGBA};

use crate::bufio;
use crate::error::Error;
use crate::paeth::paeth;
use crate::reader::{
    CB_G8, CB_G16, CB_P1, CB_P2, CB_P4, CB_P8, CB_TC8, CB_TC16, CB_TCA8, CB_TCA16, CT_GRAYSCALE,
    CT_PALETTED, CT_TRUE_COLOR, CT_TRUE_COLOR_ALPHA, FT_AVERAGE, FT_NONE, FT_PAETH, FT_SUB, FT_UP,
    N_FILTER, PNG_HEADER,
};

/// CompressionLevel indicates the compression level.
///
/// Go: image/png/writer.go:CompressionLevel
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CompressionLevel(pub i64);

impl CompressionLevel {
    pub const DEFAULT_COMPRESSION: CompressionLevel = CompressionLevel(0);
    pub const NO_COMPRESSION: CompressionLevel = CompressionLevel(-1);
    pub const BEST_SPEED: CompressionLevel = CompressionLevel(-2);
    pub const BEST_COMPRESSION: CompressionLevel = CompressionLevel(-3);
    // Positive CompressionLevel values are reserved to mean a numeric zlib
    // compression level, although that is not implemented yet.
}

/// Go: `png.DefaultCompression`.
pub const DEFAULT_COMPRESSION: CompressionLevel = CompressionLevel::DEFAULT_COMPRESSION;
/// Go: `png.NoCompression`.
pub const NO_COMPRESSION: CompressionLevel = CompressionLevel::NO_COMPRESSION;
/// Go: `png.BestSpeed`.
pub const BEST_SPEED: CompressionLevel = CompressionLevel::BEST_SPEED;
/// Go: `png.BestCompression`.
pub const BEST_COMPRESSION: CompressionLevel = CompressionLevel::BEST_COMPRESSION;

/// EncoderBufferPool is an interface for getting and returning temporary
/// instances of the [`EncoderBuffer`] struct. This can be used to reuse buffers
/// when encoding multiple images.
///
/// Go: image/png/writer.go:EncoderBufferPool. `get` returning `None` is
/// Go's `Get()` returning nil.
pub trait EncoderBufferPool: Send + Sync {
    fn get(&self) -> Option<Box<EncoderBuffer>>;
    fn put(&self, b: Box<EncoderBuffer>);
}

/// Encoder configures encoding PNG images.
///
/// Go: image/png/writer.go:Encoder
#[derive(Clone, Default)]
pub struct Encoder {
    pub compression_level: CompressionLevel,

    /// BufferPool optionally specifies a buffer pool to get temporary
    /// EncoderBuffers when encoding an image.
    pub buffer_pool: Option<Arc<dyn EncoderBufferPool>>,
}

/// The encoder's `io.Writer` face as seen by the `bufio.Writer` (Go:
/// `(*encoder).Write`, which turns every Write into an IDAT chunk).
///
/// The pooled zlib writer cannot hold the caller's borrowed `io.Writer`, so
/// each Write is recorded here and turned into an IDAT chunk (by the same
/// `writeChunk` code, in the same order and with the same `Write` calls on the
/// caller's writer) as soon as the zlib/bufio call that produced it returns.
/// After a failed chunk write the sink fails every later Write, exactly like
/// Go's `(*encoder).Write` once `e.err` is set.
#[derive(Default)]
struct IdatSink {
    pending: Vec<Vec<u8>>,
    err: Option<Error>,
}

impl Write for IdatSink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if let Some(e) = &self.err {
            return Err(e.clone().into_io());
        }
        self.pending.push(b.to_vec());
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type Bw = bufio::Writer<IdatSink>;

/// EncoderBuffer holds the buffers used for encoding PNG images.
///
/// Go: image/png/writer.go:EncoderBuffer (= the unexported `encoder`
/// struct; the per-call fields `enc`, `w`, `m` live in the encoding state).
#[derive(Default)]
pub struct EncoderBuffer {
    cb: i32,
    err: Option<Error>,
    header: [u8; 8],
    footer: [u8; 4],
    tmp: Vec<u8>, // Go: [4 * 256]byte
    cr: [Vec<u8>; N_FILTER],
    pr: Vec<u8>,
    /// Go's `e.zw`; Go's `e.bw` is the writer it writes to (Go shares the
    /// pointer, the port keeps the bufio.Writer inside the zlib writer).
    zw: Option<zlib::Writer<Bw>>,
    zw_level: i32,
}

impl EncoderBuffer {
    /// A zero-value buffer (Go: `new(png.EncoderBuffer)`).
    pub fn new() -> EncoderBuffer {
        EncoderBuffer::default()
    }
}

// Returns whether or not the image is fully opaque.
// Go: image/png/writer.go:opaque
fn opaque(m: &dyn Image) -> bool {
    if let Some(o) = m.try_opaque() {
        return o;
    }
    let b = m.bounds();
    for y in b.min.y..b.max.y {
        for x in b.min.x..b.max.x {
            let (_, _, _, a) = m.at(x, y).rgba();
            if a != 0xffff {
                return false;
            }
        }
    }
    true
}

// The absolute value of a byte interpreted as a signed int8.
// Go: image/png/writer.go:abs8
#[inline]
fn abs8(d: u8) -> i64 {
    if d < 128 {
        return d as i64;
    }
    256 - d as i64
}

/// The state of one `Encode` call (Go's `*encoder` with `e.enc`, `e.w`,
/// `e.m` set).
struct EncState<'a> {
    e: &'a mut EncoderBuffer,
    enc: &'a Encoder,
    w: &'a mut dyn Write,
    m: &'a dyn Image,
}

/// Go `n, err := w.Write(b)` on the caller's writer: one `write_all`.
fn w_write(w: &mut dyn Write, b: &[u8]) -> Option<Error> {
    let r = if b.is_empty() {
        w.write(b).map(|_| ())
    } else {
        w.write_all(b)
    };
    r.err().map(Error::from_io)
}

impl EncState<'_> {
    // Go: image/png/writer.go:(*encoder).writeChunk
    fn write_chunk(&mut self, b: &[u8], name: &[u8; 4]) {
        let e = &mut *self.e;
        if e.err.is_some() {
            return;
        }
        let n = b.len() as u32;
        if n as usize != b.len() {
            e.err = Some(Error::unsupported(format!(
                "{} chunk is too large: {}",
                std::str::from_utf8(name).unwrap_or("????"),
                b.len()
            )));
            return;
        }
        e.header[..4].copy_from_slice(&n.to_be_bytes());
        e.header[4] = name[0];
        e.header[5] = name[1];
        e.header[6] = name[2];
        e.header[7] = name[3];
        let mut crc = crc32fast::Hasher::new();
        crc.update(&e.header[4..8]);
        crc.update(b);
        e.footer[..4].copy_from_slice(&crc.finalize().to_be_bytes());

        e.err = w_write(self.w, &e.header[..8]);
        if e.err.is_some() {
            return;
        }
        e.err = w_write(self.w, b);
        if e.err.is_some() {
            return;
        }
        e.err = w_write(self.w, &e.footer[..4]);
    }

    // Go: image/png/writer.go:(*encoder).writeIHDR
    fn write_ihdr(&mut self) {
        let b = self.m.bounds();
        let tmp = &mut self.e.tmp;
        tmp[0..4].copy_from_slice(&(b.dx() as u32).to_be_bytes());
        tmp[4..8].copy_from_slice(&(b.dy() as u32).to_be_bytes());
        // Set bit depth and color type.
        match self.e.cb {
            CB_G8 => {
                tmp[8] = 8;
                tmp[9] = CT_GRAYSCALE;
            }
            CB_TC8 => {
                tmp[8] = 8;
                tmp[9] = CT_TRUE_COLOR;
            }
            CB_P8 => {
                tmp[8] = 8;
                tmp[9] = CT_PALETTED;
            }
            CB_P4 => {
                tmp[8] = 4;
                tmp[9] = CT_PALETTED;
            }
            CB_P2 => {
                tmp[8] = 2;
                tmp[9] = CT_PALETTED;
            }
            CB_P1 => {
                tmp[8] = 1;
                tmp[9] = CT_PALETTED;
            }
            CB_TCA8 => {
                tmp[8] = 8;
                tmp[9] = CT_TRUE_COLOR_ALPHA;
            }
            CB_G16 => {
                tmp[8] = 16;
                tmp[9] = CT_GRAYSCALE;
            }
            CB_TC16 => {
                tmp[8] = 16;
                tmp[9] = CT_TRUE_COLOR;
            }
            CB_TCA16 => {
                tmp[8] = 16;
                tmp[9] = CT_TRUE_COLOR_ALPHA;
            }
            _ => {}
        }
        tmp[10] = 0; // default compression method
        tmp[11] = 0; // default filter method
        tmp[12] = 0; // non-interlaced
        let chunk: [u8; 13] = tmp[..13].try_into().unwrap();
        self.write_chunk(&chunk, b"IHDR");
    }

    // Go: image/png/writer.go:(*encoder).writePLTEAndTRNS
    fn write_plte_and_trns(&mut self, p: &Palette) {
        if p.len() < 1 || p.len() > 256 {
            self.e.err = Some(Error::format(format!("bad palette length: {}", p.len())));
            return;
        }
        let mut last: i64 = -1;
        let tmp = &mut self.e.tmp;
        for (i, &c) in p.iter().enumerate() {
            let c1 = match color::nrgba_model(c) {
                Color::NRGBA(c1) => c1,
                _ => unreachable!("NRGBAModel returns color.NRGBA"),
            };
            tmp[3 * i] = c1.r;
            tmp[3 * i + 1] = c1.g;
            tmp[3 * i + 2] = c1.b;
            if c1.a != 0xff {
                last = i as i64;
            }
            tmp[3 * 256 + i] = c1.a;
        }
        let plte = tmp[..3 * p.len()].to_vec();
        self.write_chunk(&plte, b"PLTE");
        if last != -1 {
            let trns = self.e.tmp[3 * 256..3 * 256 + 1 + last as usize].to_vec();
            self.write_chunk(&trns, b"tRNS");
        }
    }

    /// Turns the Writes recorded by the IDAT sink into IDAT chunks (Go's
    /// `(*encoder).Write`, run synchronously from inside the bufio.Writer).
    /// Returns the error Go's `(*encoder).Write` would have returned to the
    /// bufio.Writer, and makes the sink fail from then on.
    fn drain(&mut self, sink: &mut IdatSink) -> Option<Error> {
        for b in std::mem::take(&mut sink.pending) {
            // Go: func (e *encoder) Write(b []byte) (int, error)
            self.write_chunk(&b, b"IDAT");
            if let Some(err) = &self.e.err {
                sink.err = Some(err.clone());
                return Some(err.clone());
            }
        }
        None
    }

    /// Drains the sink of the bufio.Writer held by the zlib writer.
    fn drain_zw(&mut self) -> Option<Error> {
        let mut sink = std::mem::take(
            self.e
                .zw
                .as_mut()
                .expect("zlib writer exists while writing IDATs")
                .get_mut()
                .get_mut(),
        );
        let err = self.drain(&mut sink);
        *self.e.zw.as_mut().unwrap().get_mut().get_mut() = sink;
        if let Some(err) = &err {
            // The failing Write happened inside the bufio.Writer: Go's
            // bufio.Writer records the error (b.err) and fails from then on.
            self.e.zw.as_mut().unwrap().get_mut().set_err(err.clone());
        }
        err
    }

    // Go: image/png/writer.go:(*encoder).writeImage. `w` is `e.bw`.
    fn write_image(&mut self, w: Bw, cb: i32, level: i32) -> Result<(), Error> {
        match self.e.zw.take() {
            Some(mut zw) if self.e.zw_level == level => {
                zw.reset(w);
                self.e.zw = Some(zw);
            }
            _ => {
                let zw = zlib::new_writer_level(w, level).map_err(Error::from_flate)?;
                self.e.zw = Some(zw);
                self.e.zw_level = level;
            }
        }
        let r = self.write_image_rows(cb, level);
        // Go: defer e.zw.Close() — its result is ignored.
        let _ = self.e.zw.as_mut().unwrap().close();
        // A Write error inside the deferred Close is recorded by the
        // bufio.Writer (the Flush that follows reports it) and ignored here.
        let _ = self.drain_zw();
        r
    }

    fn write_image_rows(&mut self, cb: i32, level: i32) -> Result<(), Error> {
        let m = self.m;
        let mut bits_per_pixel: i64 = 0;

        match cb {
            CB_G8 => bits_per_pixel = 8,
            CB_TC8 => bits_per_pixel = 24,
            CB_P8 => bits_per_pixel = 8,
            CB_P4 => bits_per_pixel = 4,
            CB_P2 => bits_per_pixel = 2,
            CB_P1 => bits_per_pixel = 1,
            CB_TCA8 => bits_per_pixel = 32,
            CB_TC16 => bits_per_pixel = 48,
            CB_TCA16 => bits_per_pixel = 64,
            CB_G16 => bits_per_pixel = 16,
            _ => {}
        }

        // cr[*] and pr are the bytes for the current and previous row.
        // cr[0] is unfiltered (or equivalently, filtered with the ftNone filter).
        // cr[ft], for non-zero filter types ft, are buffers for transforming cr[0] under the
        // other PNG filter types. These buffers are allocated once and re-used for each row.
        // The +1 is for the per-row filter type, which is at cr[*][0].
        let b = m.bounds();
        let sz = (1 + (bits_per_pixel * b.dx() + 7) / 8) as usize;
        for (i, cri) in self.e.cr.iter_mut().enumerate() {
            // Go: if cap(e.cr[i]) < sz { make } else { e.cr[i] = e.cr[i][:sz] }
            cri.resize(sz, 0);
            cri[0] = i as u8;
        }
        // Go: cr := e.cr (an array copy of the slice headers); pr := e.pr.
        // The row loop swaps cr[0] and pr locally; the buffers are taken out
        // of the EncoderBuffer for the duration of the loop and put back.
        let mut cr: [Vec<u8>; N_FILTER] = std::mem::take(&mut self.e.cr);
        // Go: if cap(e.pr) < sz { make } else { e.pr = e.pr[:sz]; clear(e.pr) }
        let mut pr = std::mem::take(&mut self.e.pr);
        pr.clear();
        pr.resize(sz, 0);
        let r = self.rows(m, cb, level, bits_per_pixel, &mut cr, &mut pr);
        self.e.cr = cr;
        self.e.pr = pr;
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn rows(
        &mut self,
        m: &dyn Image,
        cb: i32,
        level: i32,
        bits_per_pixel: i64,
        cr: &mut [Vec<u8>; N_FILTER],
        pr: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let b = m.bounds();
        let gray = m.downcast_ref::<Gray>();
        let rgba = m.downcast_ref::<RGBA>();
        let paletted = m.downcast_ref::<Paletted>();
        let nrgba = m.downcast_ref::<NRGBA>();
        let dx = b.dx() as usize;

        for y in b.min.y..b.max.y {
            // Convert from colors to bytes.
            let mut i = 1usize;
            match cb {
                CB_G8 => {
                    if let Some(gray) = gray {
                        let offset = ((y - b.min.y) * gray.stride) as usize;
                        cr[0][1..1 + dx].copy_from_slice(&gray.pix[offset..offset + dx]);
                    } else {
                        for x in b.min.x..b.max.x {
                            let c = match color::gray_model(m.at(x, y)) {
                                Color::Gray(c) => c,
                                _ => unreachable!("GrayModel returns color.Gray"),
                            };
                            cr[0][i] = c.y;
                            i += 1;
                        }
                    }
                }
                CB_TC8 => {
                    // We have previously verified that the alpha value is fully opaque.
                    let cr0 = &mut cr[0];
                    let (mut stride, mut pix): (i64, &[u8]) = (0, &[]);
                    if let Some(rgba) = rgba {
                        (stride, pix) = (rgba.stride, &rgba.pix);
                    } else if let Some(nrgba) = nrgba {
                        (stride, pix) = (nrgba.stride, &nrgba.pix);
                    }
                    if stride != 0 {
                        let j0 = ((y - b.min.y) * stride) as usize;
                        let j1 = j0 + dx * 4;
                        let mut j = j0;
                        while j < j1 {
                            cr0[i] = pix[j];
                            cr0[i + 1] = pix[j + 1];
                            cr0[i + 2] = pix[j + 2];
                            i += 3;
                            j += 4;
                        }
                    } else {
                        for x in b.min.x..b.max.x {
                            let (r, g, b, _) = m.at(x, y).rgba();
                            cr0[i] = (r >> 8) as u8;
                            cr0[i + 1] = (g >> 8) as u8;
                            cr0[i + 2] = (b >> 8) as u8;
                            i += 3;
                        }
                    }
                }
                CB_P8 => {
                    if let Some(paletted) = paletted {
                        let offset = ((y - b.min.y) * paletted.stride) as usize;
                        cr[0][1..1 + dx].copy_from_slice(&paletted.pix[offset..offset + dx]);
                    } else {
                        let pi = m
                            .as_paletted_image()
                            .expect("cbP8 is chosen only for image.PalettedImage");
                        for x in b.min.x..b.max.x {
                            cr[0][i] = pi.color_index_at(x, y);
                            i += 1;
                        }
                    }
                }
                CB_P4 | CB_P2 | CB_P1 => {
                    let pi = m
                        .as_paletted_image()
                        .expect("cbP* is chosen only for image.PalettedImage");

                    let mut a: u8 = 0;
                    let mut c: i64 = 0;
                    let pixels_per_byte = 8 / bits_per_pixel;
                    for x in b.min.x..b.max.x {
                        a = a << (bits_per_pixel as u32) | pi.color_index_at(x, y);
                        c += 1;
                        if c == pixels_per_byte {
                            cr[0][i] = a;
                            i += 1;
                            a = 0;
                            c = 0;
                        }
                    }
                    if c != 0 {
                        while c != pixels_per_byte {
                            a <<= bits_per_pixel as u32;
                            c += 1;
                        }
                        cr[0][i] = a;
                    }
                }
                CB_TCA8 => {
                    if let Some(nrgba) = nrgba {
                        let offset = ((y - b.min.y) * nrgba.stride) as usize;
                        cr[0][1..1 + dx * 4].copy_from_slice(&nrgba.pix[offset..offset + dx * 4]);
                    } else if let Some(rgba) = rgba {
                        let dst = &mut cr[0][1..];
                        let src = &rgba.pix[rgba.pix_offset(b.min.x, y) as usize
                            ..rgba.pix_offset(b.max.x, y) as usize];
                        for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                            if s[3] == 0x00 {
                                d[0] = 0;
                                d[1] = 0;
                                d[2] = 0;
                                d[3] = 0;
                            } else if s[3] == 0xff {
                                d.copy_from_slice(s);
                            } else {
                                // This code does the same as color.NRGBAModel.Convert(
                                // rgba.At(x, y)).(color.NRGBA) but with no extra memory
                                // allocations or interface/function call overhead.
                                //
                                // The multiplier m combines 0x101 (which converts
                                // 8-bit color to 16-bit color) and 0xffff (which, when
                                // combined with the division-by-a, converts from
                                // alpha-premultiplied to non-alpha-premultiplied).
                                const M: u32 = 0x101 * 0xffff;
                                let a = s[3] as u32 * 0x101;
                                d[0] = ((s[0] as u32).wrapping_mul(M) / a >> 8) as u8;
                                d[1] = ((s[1] as u32).wrapping_mul(M) / a >> 8) as u8;
                                d[2] = ((s[2] as u32).wrapping_mul(M) / a >> 8) as u8;
                                d[3] = s[3];
                            }
                        }
                    } else {
                        // Convert from image.Image (which is alpha-premultiplied) to PNG's non-alpha-premultiplied.
                        for x in b.min.x..b.max.x {
                            let c = match color::nrgba_model(m.at(x, y)) {
                                Color::NRGBA(c) => c,
                                _ => unreachable!("NRGBAModel returns color.NRGBA"),
                            };
                            cr[0][i] = c.r;
                            cr[0][i + 1] = c.g;
                            cr[0][i + 2] = c.b;
                            cr[0][i + 3] = c.a;
                            i += 4;
                        }
                    }
                }
                CB_G16 => {
                    for x in b.min.x..b.max.x {
                        let c = match color::gray16_model(m.at(x, y)) {
                            Color::Gray16(c) => c,
                            _ => unreachable!("Gray16Model returns color.Gray16"),
                        };
                        cr[0][i] = (c.y >> 8) as u8;
                        cr[0][i + 1] = c.y as u8;
                        i += 2;
                    }
                }
                CB_TC16 => {
                    // We have previously verified that the alpha value is fully opaque.
                    for x in b.min.x..b.max.x {
                        let (r, g, b, _) = m.at(x, y).rgba();
                        cr[0][i] = (r >> 8) as u8;
                        cr[0][i + 1] = r as u8;
                        cr[0][i + 2] = (g >> 8) as u8;
                        cr[0][i + 3] = g as u8;
                        cr[0][i + 4] = (b >> 8) as u8;
                        cr[0][i + 5] = b as u8;
                        i += 6;
                    }
                }
                CB_TCA16 => {
                    // Convert from image.Image (which is alpha-premultiplied) to PNG's non-alpha-premultiplied.
                    for x in b.min.x..b.max.x {
                        let c = match color::nrgba64_model(m.at(x, y)) {
                            Color::NRGBA64(c) => c,
                            _ => unreachable!("NRGBA64Model returns color.NRGBA64"),
                        };
                        cr[0][i] = (c.r >> 8) as u8;
                        cr[0][i + 1] = c.r as u8;
                        cr[0][i + 2] = (c.g >> 8) as u8;
                        cr[0][i + 3] = c.g as u8;
                        cr[0][i + 4] = (c.b >> 8) as u8;
                        cr[0][i + 5] = c.b as u8;
                        cr[0][i + 6] = (c.a >> 8) as u8;
                        cr[0][i + 7] = c.a as u8;
                        i += 8;
                    }
                }
                _ => {}
            }

            // Apply the filter.
            // Skip filter for NoCompression and paletted images (cbP8) as
            // "filters are rarely useful on palette images" and will result
            // in larger files (see http://www.libpng.org/pub/png/book/chapter09.html).
            let mut f = FT_NONE;
            if level != zlib::NO_COMPRESSION
                && cb != CB_P8
                && cb != CB_P4
                && cb != CB_P2
                && cb != CB_P1
            {
                // Since we skip paletted images we don't have to worry about
                // bitsPerPixel not being a multiple of 8
                let bpp = (bits_per_pixel / 8) as usize;
                f = filter(cr, pr, bpp);
            }

            // Write the compressed bytes.
            let zw = self.e.zw.as_mut().unwrap();
            if let Err(err) = zw.write(&cr[f]) {
                return Err(Error::from_flate(err));
            }
            if let Some(err) = self.drain_zw() {
                // Go: the failing (*encoder).Write happened inside zw.Write,
                // which returns that error.
                return Err(err);
            }

            // The current row for y is the previous row for y+1.
            std::mem::swap(pr, &mut cr[0]);
        }
        Ok(())
    }

    // Write the actual image data to one or more IDAT chunks.
    // Go: image/png/writer.go:(*encoder).writeIDATs
    fn write_idats(&mut self) {
        if self.e.err.is_some() {
            return;
        }
        // Go: if e.bw == nil { e.bw = bufio.NewWriterSize(e, 1<<15) } else { e.bw.Reset(e) }
        let existing = self.e.zw.as_mut().map(|zw| {
            // e.bw is the writer e.zw writes to.
            std::mem::replace(
                zw.get_mut(),
                bufio::Writer::placeholder(IdatSink::default()),
            )
        });
        let bw = match existing {
            None => bufio::Writer::new_size(IdatSink::default(), 1 << 15),
            Some(mut bw) => {
                bw.reset(IdatSink::default());
                bw
            }
        };
        let level = level_to_zlib(self.enc.compression_level);
        self.e.err = self.write_image(bw, self.e.cb, level).err();
        if self.e.err.is_some() {
            return;
        }
        // Go: e.err = e.bw.Flush()
        let r = self.e.zw.as_mut().unwrap().get_mut().flush();
        let drained = self.drain_zw();
        self.e.err = match r {
            Err(e) => Some(e),
            Ok(()) => drained,
        };
    }

    // Go: image/png/writer.go:(*encoder).writeIEND
    fn write_iend(&mut self) {
        self.write_chunk(&[], b"IEND");
    }
}

// Chooses the filter to use for encoding the current row, and applies it.
// The return value is the index of the filter and also of the row in cr that has had it applied.
// Go: image/png/writer.go:filter
fn filter(cr: &mut [Vec<u8>; N_FILTER], pr: &[u8], bpp: usize) -> usize {
    // We try all five filter types, and pick the one that minimizes the sum of absolute differences.
    // This is the same heuristic that libpng uses, although the filters are attempted in order of
    // estimated most likely to be minimal (ftUp, ftPaeth, ftNone, ftSub, ftAverage), rather than
    // in their enumeration order (ftNone, ftSub, ftUp, ftAverage, ftPaeth).
    let [c0, c1, c2, c3, c4] = cr;
    let cdat0 = &c0[1..];
    let cdat1 = &mut c1[1..];
    let cdat2 = &mut c2[1..];
    let cdat3 = &mut c3[1..];
    let cdat4 = &mut c4[1..];
    let pdat = &pr[1..];
    let n = cdat0.len();

    // The up filter.
    let mut sum: i64 = 0;
    for i in 0..n {
        cdat2[i] = cdat0[i].wrapping_sub(pdat[i]);
        sum += abs8(cdat2[i]);
    }
    let mut best = sum;
    let mut filter = FT_UP;

    // The Paeth filter.
    sum = 0;
    for i in 0..bpp {
        cdat4[i] = cdat0[i].wrapping_sub(pdat[i]);
        sum += abs8(cdat4[i]);
    }
    for i in bpp..n {
        cdat4[i] = cdat0[i].wrapping_sub(paeth(cdat0[i - bpp], pdat[i], pdat[i - bpp]));
        sum += abs8(cdat4[i]);
        if sum >= best {
            break;
        }
    }
    if sum < best {
        best = sum;
        filter = FT_PAETH;
    }

    // The none filter.
    sum = 0;
    for i in 0..n {
        sum += abs8(cdat0[i]);
        if sum >= best {
            break;
        }
    }
    if sum < best {
        best = sum;
        filter = FT_NONE;
    }

    // The sub filter.
    sum = 0;
    for i in 0..bpp {
        cdat1[i] = cdat0[i];
        sum += abs8(cdat1[i]);
    }
    for i in bpp..n {
        cdat1[i] = cdat0[i].wrapping_sub(cdat0[i - bpp]);
        sum += abs8(cdat1[i]);
        if sum >= best {
            break;
        }
    }
    if sum < best {
        best = sum;
        filter = FT_SUB;
    }

    // The average filter.
    sum = 0;
    for i in 0..bpp {
        cdat3[i] = cdat0[i].wrapping_sub(pdat[i] / 2);
        sum += abs8(cdat3[i]);
    }
    for i in bpp..n {
        cdat3[i] = cdat0[i].wrapping_sub(((cdat0[i - bpp] as i64 + pdat[i] as i64) / 2) as u8);
        sum += abs8(cdat3[i]);
        if sum >= best {
            break;
        }
    }
    if sum < best {
        filter = FT_AVERAGE;
    }

    filter
}

// This function is required because we want the zero value of
// Encoder.CompressionLevel to map to zlib.DefaultCompression.
// Go: image/png/writer.go:levelToZlib
fn level_to_zlib(l: CompressionLevel) -> i32 {
    match l {
        DEFAULT_COMPRESSION => zlib::DEFAULT_COMPRESSION,
        NO_COMPRESSION => zlib::NO_COMPRESSION,
        BEST_SPEED => zlib::BEST_SPEED,
        BEST_COMPRESSION => zlib::BEST_COMPRESSION,
        _ => zlib::DEFAULT_COMPRESSION,
    }
}

/// Encode writes the Image m to w in PNG format. Any Image may be
/// encoded, but images that are not [`NRGBA`] might be encoded lossily.
///
/// Go: image/png/writer.go:Encode
pub fn encode(w: &mut dyn Write, m: &dyn Image) -> Result<(), Error> {
    let e = Encoder::default();
    e.encode(w, m)
}

impl Encoder {
    /// Encode writes the Image m to w in PNG format.
    ///
    /// Go: image/png/writer.go:(*Encoder).Encode
    pub fn encode(&self, w: &mut dyn Write, m: &dyn Image) -> Result<(), Error> {
        // Obviously, negative widths and heights are invalid. Furthermore, the PNG
        // spec section 11.2.2 says that zero is invalid. Excessively large images are
        // also rejected.
        let (mw, mh) = (m.bounds().dx(), m.bounds().dy());
        if mw <= 0 || mh <= 0 || mw >= 1 << 32 || mh >= 1 << 32 {
            return Err(Error::format(format!("invalid image size: {mw}x{mh}")));
        }

        let mut e: Option<Box<EncoderBuffer>> = None;
        if let Some(pool) = &self.buffer_pool {
            e = pool.get();
        }
        let mut e = e.unwrap_or_default();
        let result = self.encode_with(&mut e, w, m);
        if let Some(pool) = &self.buffer_pool {
            // Go: defer enc.BufferPool.Put((*EncoderBuffer)(e))
            pool.put(e);
        }
        result
    }

    fn encode_with(
        &self,
        e: &mut EncoderBuffer,
        w: &mut dyn Write,
        m: &dyn Image,
    ) -> Result<(), Error> {
        if e.tmp.len() != 4 * 256 {
            e.tmp = vec![0; 4 * 256];
        }
        let mut st = EncState { e, enc: self, w, m };

        let mut pal: Option<Palette> = None;
        // cbP8 encoding needs PalettedImage's ColorIndexAt method.
        if m.as_paletted_image().is_some() {
            if let Model::Palette(p) = m.color_model() {
                pal = Some(p);
            }
        }
        if let Some(pal) = &pal {
            if pal.len() <= 2 {
                st.e.cb = CB_P1;
            } else if pal.len() <= 4 {
                st.e.cb = CB_P2;
            } else if pal.len() <= 16 {
                st.e.cb = CB_P4;
            } else {
                st.e.cb = CB_P8;
            }
        } else {
            match m.color_model() {
                Model::Gray => st.e.cb = CB_G8,
                Model::Gray16 => st.e.cb = CB_G16,
                Model::RGBA | Model::NRGBA | Model::Alpha => {
                    if opaque(m) {
                        st.e.cb = CB_TC8;
                    } else {
                        st.e.cb = CB_TCA8;
                    }
                }
                _ => {
                    if opaque(m) {
                        st.e.cb = CB_TC16;
                    } else {
                        st.e.cb = CB_TCA16;
                    }
                }
            }
        }

        // Go: _, e.err = io.WriteString(w, pngHeader)
        st.e.err = w_write(st.w, PNG_HEADER);
        st.write_ihdr();
        if let Some(pal) = &pal {
            st.write_plte_and_trns(pal);
        }
        st.write_idats();
        st.write_iend();
        match &st.e.err {
            Some(err) => Err(err.clone()),
            None => Ok(()),
        }
    }
}
