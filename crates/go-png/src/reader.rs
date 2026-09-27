//! Port of go1.27.1 `image/png/reader.go`: the PNG decoder.
//!
//! The PNG specification is at <https://www.w3.org/TR/PNG/>.

use std::io::Read;

use go_flate::zlib;
use go_image::color::{self, Color, Model, Palette};
use go_image::{
    Config, Gray, Gray16, Image, NRGBA, NRGBA64, Paletted, RGBA, RGBA64, Rectangle, rect,
};

use crate::bufio;
use crate::error::Error;
use crate::paeth::filter_paeth;

// Color type, as per the PNG spec.
pub(crate) const CT_GRAYSCALE: u8 = 0;
pub(crate) const CT_TRUE_COLOR: u8 = 2;
pub(crate) const CT_PALETTED: u8 = 3;
pub(crate) const CT_GRAYSCALE_ALPHA: u8 = 4;
pub(crate) const CT_TRUE_COLOR_ALPHA: u8 = 6;

// A cb is a combination of color type and bit depth.
pub(crate) const CB_INVALID: i32 = 0;
pub(crate) const CB_G1: i32 = 1;
pub(crate) const CB_G2: i32 = 2;
pub(crate) const CB_G4: i32 = 3;
pub(crate) const CB_G8: i32 = 4;
pub(crate) const CB_GA8: i32 = 5;
pub(crate) const CB_TC8: i32 = 6;
pub(crate) const CB_P1: i32 = 7;
pub(crate) const CB_P2: i32 = 8;
pub(crate) const CB_P4: i32 = 9;
pub(crate) const CB_P8: i32 = 10;
pub(crate) const CB_TCA8: i32 = 11;
pub(crate) const CB_G16: i32 = 12;
pub(crate) const CB_GA16: i32 = 13;
pub(crate) const CB_TC16: i32 = 14;
pub(crate) const CB_TCA16: i32 = 15;

// Go: image/png/reader.go:cbPaletted
fn cb_paletted(cb: i32) -> bool {
    CB_P1 <= cb && cb <= CB_P8
}

// Go: image/png/reader.go:cbTrueColor
fn cb_true_color(cb: i32) -> bool {
    cb == CB_TC8 || cb == CB_TC16
}

// Filter type, as per the PNG spec.
pub(crate) const FT_NONE: usize = 0;
pub(crate) const FT_SUB: usize = 1;
pub(crate) const FT_UP: usize = 2;
pub(crate) const FT_AVERAGE: usize = 3;
pub(crate) const FT_PAETH: usize = 4;
pub(crate) const N_FILTER: usize = 5;

// Interlace type.
const IT_NONE: i64 = 0;
const IT_ADAM7: i64 = 1;

// interlaceScan defines the placement and size of a pass for Adam7 interlacing.
// Go: image/png/reader.go:interlaceScan
struct InterlaceScan {
    x_factor: i64,
    y_factor: i64,
    x_offset: i64,
    y_offset: i64,
}

// interlacing defines Adam7 interlacing, with 7 passes of reduced images.
// See https://www.w3.org/TR/PNG/#8Interlace
// Go: image/png/reader.go:interlacing
const INTERLACING: [InterlaceScan; 7] = [
    InterlaceScan {
        x_factor: 8,
        y_factor: 8,
        x_offset: 0,
        y_offset: 0,
    },
    InterlaceScan {
        x_factor: 8,
        y_factor: 8,
        x_offset: 4,
        y_offset: 0,
    },
    InterlaceScan {
        x_factor: 4,
        y_factor: 8,
        x_offset: 0,
        y_offset: 4,
    },
    InterlaceScan {
        x_factor: 4,
        y_factor: 4,
        x_offset: 2,
        y_offset: 0,
    },
    InterlaceScan {
        x_factor: 2,
        y_factor: 4,
        x_offset: 0,
        y_offset: 2,
    },
    InterlaceScan {
        x_factor: 2,
        y_factor: 2,
        x_offset: 1,
        y_offset: 0,
    },
    InterlaceScan {
        x_factor: 1,
        y_factor: 2,
        x_offset: 0,
        y_offset: 1,
    },
];

// Decoding stage.
// The PNG specification says that the IHDR, PLTE (if present), tRNS (if
// present), IDAT and IEND chunks must appear in that order. There may be
// multiple IDAT chunks, and IDAT chunks must be sequential (i.e. they may not
// have any other chunks between them).
// https://www.w3.org/TR/PNG/#5ChunkOrdering
const DS_START: i32 = 0;
const DS_SEEN_IHDR: i32 = 1;
const DS_SEEN_PLTE: i32 = 2;
const DS_SEEN_TRNS: i32 = 3;
const DS_SEEN_IDAT: i32 = 4;
const DS_SEEN_IEND: i32 = 5;

/// The PNG file signature.
///
/// Go: image/png/reader.go:pngHeader
pub const PNG_HEADER: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

// Go: image/png/reader.go:chunkOrderError
fn chunk_order_error() -> Error {
    Error::format("chunk out of order")
}

/// Go `io.ReadFull` over a Rust reader: `Eof` when nothing was read,
/// `UnexpectedEof` when the read was short, the reader's error otherwise.
pub(crate) fn read_full(r: &mut dyn Read, buf: &mut [u8]) -> Result<usize, Error> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => {
                return Err(if n == 0 {
                    Error::Eof
                } else {
                    Error::UnexpectedEof
                });
            }
            Ok(m) => n += m,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(Error::from_io(e)),
        }
    }
    Ok(n)
}

/// Go's `d.palette`, a `color.Palette` slice. After a PLTE chunk it is a
/// length-`np` slice of a 256-entry backing array whose tail is opaque
/// black; Go re-slices it (tRNS, out-of-range pixel indices) up to that
/// capacity, so the backing array is kept.
#[derive(Default)]
struct GoPalette {
    /// The backing array (empty while `d.palette` is nil).
    backing: Vec<Color>,
    len: usize,
}

impl GoPalette {
    fn slice(&self) -> Palette {
        Palette(self.backing[..self.len].to_vec())
    }
}

/// The decoder state that the IDAT reader (Go's `(*decoder).Read`) shares
/// with the chunk parser: `d.r`, `d.crc`, `d.idatLength` and `d.tmp`.
struct ChunkIo<'a> {
    r: &'a mut dyn Read,
    crc: crc32fast::Hasher,
    idat_length: u32,
    tmp: [u8; 3 * 256],
}

impl ChunkIo<'_> {
    // Go: image/png/reader.go:(*decoder).verifyChecksum
    fn verify_checksum(&mut self) -> Result<(), Error> {
        read_full(self.r, &mut self.tmp[..4])?;
        let want = u32::from_be_bytes([self.tmp[0], self.tmp[1], self.tmp[2], self.tmp[3]]);
        if want != self.crc.clone().finalize() {
            return Err(Error::format("invalid checksum"));
        }
        Ok(())
    }

    // Read presents one or more IDAT chunks as one continuous stream (minus the
    // intermediate chunk headers and footers). If the PNG data looked like:
    //
    //	... len0 IDAT xxx crc0 len1 IDAT yy crc1 len2 IEND crc2
    //
    // then this reader presents xxxyy. For well-formed PNG data, the decoder state
    // immediately before the first Read call is that d.r is positioned between the
    // first IDAT and xxx, and the decoder state immediately after the last Read
    // call is that d.r is positioned between yy and crc1.
    //
    // Go: image/png/reader.go:(*decoder).Read. `Ok(0)` for a non-empty `p`
    // is Go's `(0, io.EOF)`.
    fn idat_read(&mut self, p: &mut [u8]) -> Result<usize, Error> {
        if p.is_empty() {
            return Ok(0);
        }
        while self.idat_length == 0 {
            // We have exhausted an IDAT chunk. Verify the checksum of that chunk.
            self.verify_checksum()?;
            // Read the length and chunk type of the next chunk, and check that
            // it is an IDAT chunk.
            read_full(self.r, &mut self.tmp[..8])?;
            self.idat_length =
                u32::from_be_bytes([self.tmp[0], self.tmp[1], self.tmp[2], self.tmp[3]]);
            if &self.tmp[4..8] != b"IDAT" {
                return Err(Error::format("not enough pixel data"));
            }
            self.crc = crc32fast::Hasher::new();
            self.crc.update(&self.tmp[4..8]);
        }
        // Go: if int(d.idatLength) < 0 — never true for a 64-bit int.
        let m = p.len().min(self.idat_length as usize);
        let n = loop {
            match self.r.read(&mut p[..m]) {
                Ok(n) => break n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(Error::from_io(e)),
            }
        };
        self.crc.update(&p[..n]);
        self.idat_length -= n as u32;
        Ok(n)
    }
}

/// Go's `*decoder` used as the `io.Reader` handed to `zlib.NewReader`.
struct IdatReader<'b, 'a> {
    io: &'b mut ChunkIo<'a>,
}

impl Read for IdatReader<'_, '_> {
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        match self.io.idat_read(p) {
            Ok(n) => Ok(n),
            Err(Error::Eof) => Ok(0),
            Err(e) => Err(e.into_io()),
        }
    }
}

type ZReader<'b, 'a> = zlib::Reader<bufio::Reader<IdatReader<'b, 'a>>>;

/// Go `io.ReadFull(r, buf)` over the zlib reader (Go's `io.ReadAtLeast`
/// semantics, including dropping an error that comes with the last bytes).
fn read_full_zlib(r: &mut ZReader<'_, '_>, buf: &mut [u8]) -> (usize, Option<Error>) {
    let min = buf.len();
    let mut n = 0;
    let mut err: Option<Error> = None;
    while n < min && err.is_none() {
        let (nn, e) = r.read(&mut buf[n..]);
        err = e.map(Error::from_flate);
        n += nn;
    }
    if n >= min {
        err = None;
    } else if n > 0 && matches!(err, Some(Error::Eof)) {
        err = Some(Error::UnexpectedEof);
    }
    (n, err)
}

/// The concrete image types `readImagePass` can produce (Go returns them as
/// `image.Image` and type-switches on them in `mergePassInto`).
enum PassImage {
    Gray(Gray),
    RGBA(RGBA),
    Paletted(Paletted),
    NRGBA(NRGBA),
    Gray16(Gray16),
    RGBA64(RGBA64),
    NRGBA64(NRGBA64),
}

impl PassImage {
    fn bounds(&self) -> Rectangle {
        match self {
            PassImage::Gray(m) => m.rect,
            PassImage::RGBA(m) => m.rect,
            PassImage::Paletted(m) => m.rect,
            PassImage::NRGBA(m) => m.rect,
            PassImage::Gray16(m) => m.rect,
            PassImage::RGBA64(m) => m.rect,
            PassImage::NRGBA64(m) => m.rect,
        }
    }

    fn into_boxed(self) -> Box<dyn Image> {
        match self {
            PassImage::Gray(m) => Box::new(m),
            PassImage::RGBA(m) => Box::new(m),
            PassImage::Paletted(m) => Box::new(m),
            PassImage::NRGBA(m) => Box::new(m),
            PassImage::Gray16(m) => Box::new(m),
            PassImage::RGBA64(m) => Box::new(m),
            PassImage::NRGBA64(m) => Box::new(m),
        }
    }
}

// Go: image/png/reader.go:decoder
struct Decoder<'a> {
    io: ChunkIo<'a>,
    img: Option<PassImage>,
    width: i64,
    height: i64,
    depth: i64,
    palette: GoPalette,
    cb: i32,
    stage: i32,
    interlace: i64,

    // useTransparent and transparent are used for grayscale and truecolor
    // transparency, as opposed to palette transparency.
    use_transparent: bool,
    transparent: [u8; 6],
}

/// Go `copy(dst, src)`.
#[inline]
fn go_copy(dst: &mut [u8], src: &[u8]) -> usize {
    let n = dst.len().min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

impl<'a> Decoder<'a> {
    fn new(r: &'a mut dyn Read) -> Decoder<'a> {
        Decoder {
            io: ChunkIo {
                r,
                crc: crc32fast::Hasher::new(),
                idat_length: 0,
                tmp: [0; 3 * 256],
            },
            img: None,
            width: 0,
            height: 0,
            depth: 0,
            palette: GoPalette::default(),
            cb: CB_INVALID,
            stage: DS_START,
            interlace: 0,
            use_transparent: false,
            transparent: [0; 6],
        }
    }

    // Go: image/png/reader.go:(*decoder).parseIHDR
    fn parse_ihdr(&mut self, length: u32) -> Result<(), Error> {
        if length != 13 {
            return Err(Error::format("bad IHDR length"));
        }
        read_full(self.io.r, &mut self.io.tmp[..13])?;
        self.io.crc.update(&self.io.tmp[..13]);
        let tmp = &self.io.tmp;
        if tmp[10] != 0 {
            return Err(Error::unsupported("compression method"));
        }
        if tmp[11] != 0 {
            return Err(Error::unsupported("filter method"));
        }
        if tmp[12] as i64 != IT_NONE && tmp[12] as i64 != IT_ADAM7 {
            return Err(Error::format("invalid interlace method"));
        }
        self.interlace = tmp[12] as i64;

        let w = u32::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3]]) as i32;
        let h = u32::from_be_bytes([tmp[4], tmp[5], tmp[6], tmp[7]]) as i32;
        if w <= 0 || h <= 0 {
            return Err(Error::format("non-positive dimension"));
        }
        let n_pixels64 = w as i64 * h as i64;
        let n_pixels = n_pixels64;
        // Go: if nPixels64 != int64(nPixels) — never true for a 64-bit int.
        // There can be up to 8 bytes per pixel, for 16 bits per channel RGBA.
        if n_pixels != n_pixels.wrapping_mul(8) / 8 {
            return Err(Error::unsupported("dimension overflow"));
        }

        self.cb = CB_INVALID;
        self.depth = tmp[8] as i64;
        match self.depth {
            1 => match tmp[9] {
                CT_GRAYSCALE => self.cb = CB_G1,
                CT_PALETTED => self.cb = CB_P1,
                _ => {}
            },
            2 => match tmp[9] {
                CT_GRAYSCALE => self.cb = CB_G2,
                CT_PALETTED => self.cb = CB_P2,
                _ => {}
            },
            4 => match tmp[9] {
                CT_GRAYSCALE => self.cb = CB_G4,
                CT_PALETTED => self.cb = CB_P4,
                _ => {}
            },
            8 => match tmp[9] {
                CT_GRAYSCALE => self.cb = CB_G8,
                CT_TRUE_COLOR => self.cb = CB_TC8,
                CT_PALETTED => self.cb = CB_P8,
                CT_GRAYSCALE_ALPHA => self.cb = CB_GA8,
                CT_TRUE_COLOR_ALPHA => self.cb = CB_TCA8,
                _ => {}
            },
            16 => match tmp[9] {
                CT_GRAYSCALE => self.cb = CB_G16,
                CT_TRUE_COLOR => self.cb = CB_TC16,
                CT_GRAYSCALE_ALPHA => self.cb = CB_GA16,
                CT_TRUE_COLOR_ALPHA => self.cb = CB_TCA16,
                _ => {}
            },
            _ => {}
        }
        if self.cb == CB_INVALID {
            return Err(Error::unsupported(format!(
                "bit depth {}, color type {}",
                tmp[8], tmp[9]
            )));
        }
        self.width = w as i64;
        self.height = h as i64;
        self.io.verify_checksum()
    }

    // Go: image/png/reader.go:(*decoder).parsePLTE
    fn parse_plte(&mut self, length: u32) -> Result<(), Error> {
        let np = (length / 3) as i64; // The number of palette entries.
        if length % 3 != 0 || np <= 0 || np > 256 || np > 1i64 << (self.depth as u32 & 63) {
            return Err(Error::format("bad PLTE length"));
        }
        let np = np as usize;
        let n = read_full(self.io.r, &mut self.io.tmp[..3 * np])?;
        self.io.crc.update(&self.io.tmp[..n]);
        match self.cb {
            CB_P1 | CB_P2 | CB_P4 | CB_P8 => {
                let tmp = &self.io.tmp;
                let mut backing = Vec::with_capacity(256);
                for i in 0..np {
                    backing.push(Color::RGBA(color::RGBA {
                        r: tmp[3 * i],
                        g: tmp[3 * i + 1],
                        b: tmp[3 * i + 2],
                        a: 0xff,
                    }));
                }
                for _ in np..256 {
                    // Initialize the rest of the palette to opaque black. The spec (section
                    // 11.2.3) says that "any out-of-range pixel value found in the image data
                    // is an error", but some real-world PNG files have out-of-range pixel
                    // values. We fall back to opaque black, the same as libpng 1.5.13;
                    // ImageMagick 6.5.7 returns an error.
                    backing.push(Color::RGBA(color::RGBA {
                        r: 0x00,
                        g: 0x00,
                        b: 0x00,
                        a: 0xff,
                    }));
                }
                self.palette = GoPalette { backing, len: np };
            }
            CB_TC8 | CB_TCA8 | CB_TC16 | CB_TCA16 => {
                // As per the PNG spec, a PLTE chunk is optional (and for practical purposes,
                // ignorable) for the ctTrueColor and ctTrueColorAlpha color types (section 4.1.2).
            }
            _ => return Err(Error::format("PLTE, color type mismatch")),
        }
        self.io.verify_checksum()
    }

    // Go: image/png/reader.go:(*decoder).parsetRNS
    fn parse_trns(&mut self, length: u32) -> Result<(), Error> {
        match self.cb {
            CB_G1 | CB_G2 | CB_G4 | CB_G8 | CB_G16 => {
                if length != 2 {
                    return Err(Error::format("bad tRNS length"));
                }
                let n = read_full(self.io.r, &mut self.io.tmp[..length as usize])?;
                self.io.crc.update(&self.io.tmp[..n]);

                go_copy(&mut self.transparent, &self.io.tmp[..length as usize]);
                match self.cb {
                    CB_G1 => self.transparent[1] = self.transparent[1].wrapping_mul(0xff),
                    CB_G2 => self.transparent[1] = self.transparent[1].wrapping_mul(0x55),
                    CB_G4 => self.transparent[1] = self.transparent[1].wrapping_mul(0x11),
                    _ => {}
                }
                self.use_transparent = true;
            }
            CB_TC8 | CB_TC16 => {
                if length != 6 {
                    return Err(Error::format("bad tRNS length"));
                }
                let n = read_full(self.io.r, &mut self.io.tmp[..length as usize])?;
                self.io.crc.update(&self.io.tmp[..n]);

                go_copy(&mut self.transparent, &self.io.tmp[..length as usize]);
                self.use_transparent = true;
            }
            CB_P1 | CB_P2 | CB_P4 | CB_P8 => {
                if length > 256 {
                    return Err(Error::format("bad tRNS length"));
                }
                let n = read_full(self.io.r, &mut self.io.tmp[..length as usize])?;
                self.io.crc.update(&self.io.tmp[..n]);

                if self.palette.len < n {
                    // d.palette = d.palette[:n] (within the 256-entry backing array)
                    self.palette.len = n;
                }
                for i in 0..n {
                    let rgba = match self.palette.backing[i] {
                        Color::RGBA(c) => c,
                        // Go: d.palette[i].(color.RGBA) panics; every entry is a
                        // color.RGBA here because tRNS is accepted only once.
                        _ => unreachable!("palette entry is not color.RGBA"),
                    };
                    self.palette.backing[i] = Color::NRGBA(color::NRGBA {
                        r: rgba.r,
                        g: rgba.g,
                        b: rgba.b,
                        a: self.io.tmp[i],
                    });
                }
            }
            _ => return Err(Error::format("tRNS, color type mismatch")),
        }
        self.io.verify_checksum()
    }

    // decode decodes the IDAT data into an image.
    // Go: image/png/reader.go:(*decoder).decode
    fn decode(&mut self) -> Result<PassImage, Error> {
        let hdr = PassParams {
            width: self.width,
            height: self.height,
            depth: self.depth,
            cb: self.cb,
            interlace: self.interlace,
            use_transparent: self.use_transparent,
            transparent: self.transparent,
        };
        let palette = &self.palette;
        let mut r: ZReader<'_, '_> =
            zlib::new_reader(bufio::Reader::new(IdatReader { io: &mut self.io }))
                .map_err(Error::from_flate)?;
        // Go: defer r.Close() — its result is ignored and it reads nothing.
        let mut img: PassImage;
        if hdr.interlace == IT_NONE {
            img = read_image_pass(&hdr, palette, Some(&mut r), 0, false)?
                .expect("a non-interlaced image has a non-empty pass");
        } else {
            // IT_ADAM7 (parseIHDR accepts nothing else).
            // Allocate a blank image of the full size.
            img = read_image_pass(&hdr, palette, None, 0, true)?
                .expect("allocateOnly always returns an image");
            for pass in 0..7 {
                let image_pass = read_image_pass(&hdr, palette, Some(&mut r), pass, false)?;
                if let Some(image_pass) = image_pass {
                    merge_pass_into(&mut img, &image_pass, pass);
                }
            }
        }

        // Check for EOF, to verify the zlib checksum.
        let mut n = 0;
        let mut err: Option<Error> = None;
        let mut i = 0;
        let mut one = [0u8; 1];
        while n == 0 && err.is_none() {
            if i == 100 {
                return Err(Error::NoProgress);
            }
            let (nn, e) = r.read(&mut one);
            n = nn;
            err = e.map(Error::from_flate);
            i += 1;
        }
        let _ = r.close();
        drop(r);
        if let Some(e) = err
            && !e.is_eof()
        {
            return Err(Error::format(e.to_string()));
        }
        if n != 0 || self.io.idat_length != 0 {
            return Err(Error::format("too much pixel data"));
        }

        Ok(img)
    }

    // Go: image/png/reader.go:(*decoder).parseIDAT
    fn parse_idat(&mut self, length: u32) -> Result<(), Error> {
        self.io.idat_length = length;
        self.img = Some(self.decode()?);
        self.io.verify_checksum()
    }

    // Go: image/png/reader.go:(*decoder).parseIEND
    fn parse_iend(&mut self, length: u32) -> Result<(), Error> {
        if length != 0 {
            return Err(Error::format("bad IEND length"));
        }
        self.io.verify_checksum()
    }

    // Go: image/png/reader.go:(*decoder).parseChunk
    fn parse_chunk(&mut self, config_only: bool) -> Result<(), Error> {
        // Read the length and chunk type.
        read_full(self.io.r, &mut self.io.tmp[..8])?;
        let tmp = &self.io.tmp;
        let length = u32::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3]]);
        let name = [tmp[4], tmp[5], tmp[6], tmp[7]];
        self.io.crc = crc32fast::Hasher::new();
        self.io.crc.update(&name);

        // Read the chunk data.
        match &name {
            b"IHDR" => {
                if self.stage != DS_START {
                    return Err(chunk_order_error());
                }
                self.stage = DS_SEEN_IHDR;
                return self.parse_ihdr(length);
            }
            b"PLTE" => {
                if self.stage != DS_SEEN_IHDR {
                    return Err(chunk_order_error());
                }
                self.stage = DS_SEEN_PLTE;
                return self.parse_plte(length);
            }
            b"tRNS" => {
                if cb_paletted(self.cb) {
                    if self.stage != DS_SEEN_PLTE {
                        return Err(chunk_order_error());
                    }
                } else if cb_true_color(self.cb) {
                    if self.stage != DS_SEEN_IHDR && self.stage != DS_SEEN_PLTE {
                        return Err(chunk_order_error());
                    }
                } else if self.stage != DS_SEEN_IHDR {
                    return Err(chunk_order_error());
                }
                self.stage = DS_SEEN_TRNS;
                return self.parse_trns(length);
            }
            b"IDAT" => {
                if self.stage < DS_SEEN_IHDR
                    || self.stage > DS_SEEN_IDAT
                    || (self.stage == DS_SEEN_IHDR && cb_paletted(self.cb))
                {
                    return Err(chunk_order_error());
                } else if self.stage == DS_SEEN_IDAT {
                    // Ignore trailing zero-length or garbage IDAT chunks.
                    //
                    // This does not affect valid PNG images that contain multiple IDAT
                    // chunks, since the first call to parseIDAT below will consume all
                    // consecutive IDAT chunks required for decoding the image.
                    // (Go: break — fall through to the ignore code below.)
                } else {
                    self.stage = DS_SEEN_IDAT;
                    if config_only {
                        return Ok(());
                    }
                    return self.parse_idat(length);
                }
            }
            b"IEND" => {
                if self.stage != DS_SEEN_IDAT {
                    return Err(chunk_order_error());
                }
                self.stage = DS_SEEN_IEND;
                return self.parse_iend(length);
            }
            _ => {}
        }
        if length > 0x7fffffff {
            return Err(Error::format(format!("Bad chunk length: {length}")));
        }
        // Ignore this chunk (of a known length).
        let mut ignored = [0u8; 4096];
        let mut length = length;
        while length > 0 {
            let m = ignored.len().min(length as usize);
            let n = read_full(self.io.r, &mut ignored[..m])?;
            self.io.crc.update(&ignored[..n]);
            length -= n as u32;
        }
        self.io.verify_checksum()
    }

    // Go: image/png/reader.go:(*decoder).checkHeader
    fn check_header(&mut self) -> Result<(), Error> {
        read_full(self.io.r, &mut self.io.tmp[..PNG_HEADER.len()])?;
        if &self.io.tmp[..PNG_HEADER.len()] != PNG_HEADER {
            return Err(Error::format("not a PNG file"));
        }
        Ok(())
    }
}

/// The header fields `readImagePass` reads from the decoder.
struct PassParams {
    width: i64,
    height: i64,
    depth: i64,
    cb: i32,
    interlace: i64,
    use_transparent: bool,
    transparent: [u8; 6],
}

// readImagePass reads a single image pass, sized according to the pass number.
// Go: image/png/reader.go:(*decoder).readImagePass
fn read_image_pass(
    d: &PassParams,
    d_palette: &GoPalette,
    r: Option<&mut ZReader<'_, '_>>,
    pass: usize,
    allocate_only: bool,
) -> Result<Option<PassImage>, Error> {
    let bits_per_pixel: i64;
    let mut pix_offset: usize = 0;
    let (mut width, mut height) = (d.width, d.height);
    if d.interlace == IT_ADAM7 && !allocate_only {
        let p = &INTERLACING[pass];
        // Add the multiplication factor and subtract one, effectively rounding up.
        width = (width - p.x_offset + p.x_factor - 1) / p.x_factor;
        height = (height - p.y_offset + p.y_factor - 1) / p.y_factor;
        // A PNG image can't have zero width or height, but for an interlaced
        // image, an individual pass might have zero width or height. If so, we
        // shouldn't even read a per-row filter type byte, so return early.
        if width == 0 || height == 0 {
            return Ok(None);
        }
    }
    let r0 = rect(0, 0, width, height);
    let mut img = match d.cb {
        CB_G1 | CB_G2 | CB_G4 | CB_G8 => {
            bits_per_pixel = d.depth;
            if d.use_transparent {
                PassImage::NRGBA(NRGBA::new(r0))
            } else {
                PassImage::Gray(Gray::new(r0))
            }
        }
        CB_GA8 => {
            bits_per_pixel = 16;
            PassImage::NRGBA(NRGBA::new(r0))
        }
        CB_TC8 => {
            bits_per_pixel = 24;
            if d.use_transparent {
                PassImage::NRGBA(NRGBA::new(r0))
            } else {
                PassImage::RGBA(RGBA::new(r0))
            }
        }
        CB_P1 | CB_P2 | CB_P4 | CB_P8 => {
            bits_per_pixel = d.depth;
            PassImage::Paletted(Paletted::new(r0, d_palette.slice()))
        }
        CB_TCA8 => {
            bits_per_pixel = 32;
            PassImage::NRGBA(NRGBA::new(r0))
        }
        CB_G16 => {
            bits_per_pixel = 16;
            if d.use_transparent {
                PassImage::NRGBA64(NRGBA64::new(r0))
            } else {
                PassImage::Gray16(Gray16::new(r0))
            }
        }
        CB_GA16 => {
            bits_per_pixel = 32;
            PassImage::NRGBA64(NRGBA64::new(r0))
        }
        CB_TC16 => {
            bits_per_pixel = 48;
            if d.use_transparent {
                PassImage::NRGBA64(NRGBA64::new(r0))
            } else {
                PassImage::RGBA64(RGBA64::new(r0))
            }
        }
        CB_TCA16 => {
            bits_per_pixel = 64;
            PassImage::NRGBA64(NRGBA64::new(r0))
        }
        _ => unreachable!("parseIHDR rejects invalid color type/bit depth combinations"),
    };
    if allocate_only {
        return Ok(Some(img));
    }
    let r = r.expect("a reader is passed unless allocateOnly");
    let bytes_per_pixel = ((bits_per_pixel + 7) / 8) as usize;

    // The +1 is for the per-row filter type, which is at cr[0].
    let row_size = 1 + (bits_per_pixel * width + 7) / 8;
    // Go: if rowSize != int64(int(rowSize)) — never true for a 64-bit int.
    let row_size = row_size as usize;
    // cr and pr are the bytes for the current and previous row.
    let mut cr = vec![0u8; row_size];
    let mut pr = vec![0u8; row_size];

    for y in 0..height {
        // Read the decompressed bytes.
        let (_, err) = read_full_zlib(r, &mut cr);
        if let Some(err) = err {
            if matches!(err, Error::Eof | Error::UnexpectedEof) {
                return Err(Error::format("not enough pixel data"));
            }
            return Err(err);
        }

        // Apply the filter.
        let filter_type = cr[0];
        let cdat = &mut cr[1..];
        let pdat = &pr[1..];
        match filter_type as usize {
            FT_NONE => {
                // No-op.
            }
            FT_SUB => {
                for i in bytes_per_pixel..cdat.len() {
                    cdat[i] = cdat[i].wrapping_add(cdat[i - bytes_per_pixel]);
                }
            }
            FT_UP => {
                for (i, &p) in pdat.iter().enumerate() {
                    cdat[i] = cdat[i].wrapping_add(p);
                }
            }
            FT_AVERAGE => {
                // The first column has no column to the left of it, so it is a
                // special case. We know that the first column exists because we
                // check above that width != 0, and so len(cdat) != 0.
                for i in 0..bytes_per_pixel {
                    cdat[i] = cdat[i].wrapping_add(pdat[i] / 2);
                }
                for i in bytes_per_pixel..cdat.len() {
                    cdat[i] = cdat[i].wrapping_add(
                        ((cdat[i - bytes_per_pixel] as i64 + pdat[i] as i64) / 2) as u8,
                    );
                }
            }
            FT_PAETH => {
                filter_paeth(cdat, pdat, bytes_per_pixel);
            }
            _ => return Err(Error::format("bad filter type")),
        }

        // Convert from bytes to colors.
        let cdat = &cr[1..];
        let w = width as usize;
        match d.cb {
            CB_G1 => {
                if d.use_transparent {
                    let PassImage::NRGBA(nrgba) = &mut img else {
                        unreachable!()
                    };
                    let ty = d.transparent[1];
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 8];
                        let mut x2 = 0;
                        while x2 < 8 && x + x2 < w {
                            let ycol = (b >> 7).wrapping_mul(0xff);
                            let mut acol = 0xffu8;
                            if ycol == ty {
                                acol = 0x00;
                            }
                            nrgba.set_nrgba(
                                (x + x2) as i64,
                                y,
                                color::NRGBA {
                                    r: ycol,
                                    g: ycol,
                                    b: ycol,
                                    a: acol,
                                },
                            );
                            b <<= 1;
                            x2 += 1;
                        }
                        x += 8;
                    }
                } else {
                    let PassImage::Gray(gray) = &mut img else {
                        unreachable!()
                    };
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 8];
                        let mut x2 = 0;
                        while x2 < 8 && x + x2 < w {
                            gray.set_gray(
                                (x + x2) as i64,
                                y,
                                color::Gray {
                                    y: (b >> 7).wrapping_mul(0xff),
                                },
                            );
                            b <<= 1;
                            x2 += 1;
                        }
                        x += 8;
                    }
                }
            }
            CB_G2 => {
                if d.use_transparent {
                    let PassImage::NRGBA(nrgba) = &mut img else {
                        unreachable!()
                    };
                    let ty = d.transparent[1];
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 4];
                        let mut x2 = 0;
                        while x2 < 4 && x + x2 < w {
                            let ycol = (b >> 6).wrapping_mul(0x55);
                            let mut acol = 0xffu8;
                            if ycol == ty {
                                acol = 0x00;
                            }
                            nrgba.set_nrgba(
                                (x + x2) as i64,
                                y,
                                color::NRGBA {
                                    r: ycol,
                                    g: ycol,
                                    b: ycol,
                                    a: acol,
                                },
                            );
                            b <<= 2;
                            x2 += 1;
                        }
                        x += 4;
                    }
                } else {
                    let PassImage::Gray(gray) = &mut img else {
                        unreachable!()
                    };
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 4];
                        let mut x2 = 0;
                        while x2 < 4 && x + x2 < w {
                            gray.set_gray(
                                (x + x2) as i64,
                                y,
                                color::Gray {
                                    y: (b >> 6).wrapping_mul(0x55),
                                },
                            );
                            b <<= 2;
                            x2 += 1;
                        }
                        x += 4;
                    }
                }
            }
            CB_G4 => {
                if d.use_transparent {
                    let PassImage::NRGBA(nrgba) = &mut img else {
                        unreachable!()
                    };
                    let ty = d.transparent[1];
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 2];
                        let mut x2 = 0;
                        while x2 < 2 && x + x2 < w {
                            let ycol = (b >> 4).wrapping_mul(0x11);
                            let mut acol = 0xffu8;
                            if ycol == ty {
                                acol = 0x00;
                            }
                            nrgba.set_nrgba(
                                (x + x2) as i64,
                                y,
                                color::NRGBA {
                                    r: ycol,
                                    g: ycol,
                                    b: ycol,
                                    a: acol,
                                },
                            );
                            b <<= 4;
                            x2 += 1;
                        }
                        x += 2;
                    }
                } else {
                    let PassImage::Gray(gray) = &mut img else {
                        unreachable!()
                    };
                    let mut x = 0;
                    while x < w {
                        let mut b = cdat[x / 2];
                        let mut x2 = 0;
                        while x2 < 2 && x + x2 < w {
                            gray.set_gray(
                                (x + x2) as i64,
                                y,
                                color::Gray {
                                    y: (b >> 4).wrapping_mul(0x11),
                                },
                            );
                            b <<= 4;
                            x2 += 1;
                        }
                        x += 2;
                    }
                }
            }
            CB_G8 => {
                if d.use_transparent {
                    let PassImage::NRGBA(nrgba) = &mut img else {
                        unreachable!()
                    };
                    let ty = d.transparent[1];
                    for x in 0..w {
                        let ycol = cdat[x];
                        let mut acol = 0xffu8;
                        if ycol == ty {
                            acol = 0x00;
                        }
                        nrgba.set_nrgba(
                            x as i64,
                            y,
                            color::NRGBA {
                                r: ycol,
                                g: ycol,
                                b: ycol,
                                a: acol,
                            },
                        );
                    }
                } else {
                    let PassImage::Gray(gray) = &mut img else {
                        unreachable!()
                    };
                    go_copy(&mut gray.pix[pix_offset..], cdat);
                    pix_offset += gray.stride as usize;
                }
            }
            CB_GA8 => {
                let PassImage::NRGBA(nrgba) = &mut img else {
                    unreachable!()
                };
                for x in 0..w {
                    let ycol = cdat[2 * x];
                    nrgba.set_nrgba(
                        x as i64,
                        y,
                        color::NRGBA {
                            r: ycol,
                            g: ycol,
                            b: ycol,
                            a: cdat[2 * x + 1],
                        },
                    );
                }
            }
            CB_TC8 => {
                if d.use_transparent {
                    let PassImage::NRGBA(nrgba) = &mut img else {
                        unreachable!()
                    };
                    let (pix, mut i, mut j) = (&mut nrgba.pix, pix_offset, 0);
                    let (tr, tg, tb) = (d.transparent[1], d.transparent[3], d.transparent[5]);
                    for _ in 0..w {
                        let r = cdat[j];
                        let g = cdat[j + 1];
                        let b = cdat[j + 2];
                        let mut a = 0xffu8;
                        if r == tr && g == tg && b == tb {
                            a = 0x00;
                        }
                        pix[i] = r;
                        pix[i + 1] = g;
                        pix[i + 2] = b;
                        pix[i + 3] = a;
                        i += 4;
                        j += 3;
                    }
                    pix_offset += nrgba.stride as usize;
                } else {
                    let PassImage::RGBA(rgba) = &mut img else {
                        unreachable!()
                    };
                    let (pix, mut i, mut j) = (&mut rgba.pix, pix_offset, 0);
                    for _ in 0..w {
                        pix[i] = cdat[j];
                        pix[i + 1] = cdat[j + 1];
                        pix[i + 2] = cdat[j + 2];
                        pix[i + 3] = 0xff;
                        i += 4;
                        j += 3;
                    }
                    pix_offset += rgba.stride as usize;
                }
            }
            CB_P1 | CB_P2 | CB_P4 => {
                let PassImage::Paletted(paletted) = &mut img else {
                    unreachable!()
                };
                // Go has one loop per depth; they differ only in these constants.
                let (per_byte, shift) = match d.cb {
                    CB_P1 => (8usize, 1u32),
                    CB_P2 => (4, 2),
                    _ => (2, 4),
                };
                let mut x = 0;
                while x < w {
                    let mut b = cdat[x / per_byte];
                    let mut x2 = 0;
                    while x2 < per_byte && x + x2 < w {
                        let idx = b >> (8 - shift);
                        if paletted.palette.len() <= idx as usize {
                            // paletted.Palette = paletted.Palette[:int(idx)+1]
                            extend_palette(&mut paletted.palette, d_palette, idx as usize + 1);
                        }
                        paletted.set_color_index((x + x2) as i64, y, idx);
                        b <<= shift;
                        x2 += 1;
                    }
                    x += per_byte;
                }
            }
            CB_P8 => {
                let PassImage::Paletted(paletted) = &mut img else {
                    unreachable!()
                };
                if paletted.palette.len() != 256 {
                    for &c in &cdat[..w] {
                        if paletted.palette.len() <= c as usize {
                            extend_palette(&mut paletted.palette, d_palette, c as usize + 1);
                        }
                    }
                }
                go_copy(&mut paletted.pix[pix_offset..], cdat);
                pix_offset += paletted.stride as usize;
            }
            CB_TCA8 => {
                let PassImage::NRGBA(nrgba) = &mut img else {
                    unreachable!()
                };
                go_copy(&mut nrgba.pix[pix_offset..], cdat);
                pix_offset += nrgba.stride as usize;
            }
            CB_G16 => {
                if d.use_transparent {
                    let PassImage::NRGBA64(nrgba64) = &mut img else {
                        unreachable!()
                    };
                    let ty = (d.transparent[0] as u16) << 8 | d.transparent[1] as u16;
                    for x in 0..w {
                        let ycol = (cdat[2 * x] as u16) << 8 | cdat[2 * x + 1] as u16;
                        let mut acol = 0xffffu16;
                        if ycol == ty {
                            acol = 0x0000;
                        }
                        nrgba64.set_nrgba64(
                            x as i64,
                            y,
                            color::NRGBA64 {
                                r: ycol,
                                g: ycol,
                                b: ycol,
                                a: acol,
                            },
                        );
                    }
                } else {
                    let PassImage::Gray16(gray16) = &mut img else {
                        unreachable!()
                    };
                    for x in 0..w {
                        let ycol = (cdat[2 * x] as u16) << 8 | cdat[2 * x + 1] as u16;
                        gray16.set_gray16(x as i64, y, color::Gray16 { y: ycol });
                    }
                }
            }
            CB_GA16 => {
                let PassImage::NRGBA64(nrgba64) = &mut img else {
                    unreachable!()
                };
                for x in 0..w {
                    let ycol = (cdat[4 * x] as u16) << 8 | cdat[4 * x + 1] as u16;
                    let acol = (cdat[4 * x + 2] as u16) << 8 | cdat[4 * x + 3] as u16;
                    nrgba64.set_nrgba64(
                        x as i64,
                        y,
                        color::NRGBA64 {
                            r: ycol,
                            g: ycol,
                            b: ycol,
                            a: acol,
                        },
                    );
                }
            }
            CB_TC16 => {
                if d.use_transparent {
                    let PassImage::NRGBA64(nrgba64) = &mut img else {
                        unreachable!()
                    };
                    let tr = (d.transparent[0] as u16) << 8 | d.transparent[1] as u16;
                    let tg = (d.transparent[2] as u16) << 8 | d.transparent[3] as u16;
                    let tb = (d.transparent[4] as u16) << 8 | d.transparent[5] as u16;
                    for x in 0..w {
                        let rcol = (cdat[6 * x] as u16) << 8 | cdat[6 * x + 1] as u16;
                        let gcol = (cdat[6 * x + 2] as u16) << 8 | cdat[6 * x + 3] as u16;
                        let bcol = (cdat[6 * x + 4] as u16) << 8 | cdat[6 * x + 5] as u16;
                        let mut acol = 0xffffu16;
                        if rcol == tr && gcol == tg && bcol == tb {
                            acol = 0x0000;
                        }
                        nrgba64.set_nrgba64(
                            x as i64,
                            y,
                            color::NRGBA64 {
                                r: rcol,
                                g: gcol,
                                b: bcol,
                                a: acol,
                            },
                        );
                    }
                } else {
                    let PassImage::RGBA64(rgba64) = &mut img else {
                        unreachable!()
                    };
                    for x in 0..w {
                        let rcol = (cdat[6 * x] as u16) << 8 | cdat[6 * x + 1] as u16;
                        let gcol = (cdat[6 * x + 2] as u16) << 8 | cdat[6 * x + 3] as u16;
                        let bcol = (cdat[6 * x + 4] as u16) << 8 | cdat[6 * x + 5] as u16;
                        rgba64.set_rgba64(
                            x as i64,
                            y,
                            color::RGBA64 {
                                r: rcol,
                                g: gcol,
                                b: bcol,
                                a: 0xffff,
                            },
                        );
                    }
                }
            }
            CB_TCA16 => {
                let PassImage::NRGBA64(nrgba64) = &mut img else {
                    unreachable!()
                };
                for x in 0..w {
                    let rcol = (cdat[8 * x] as u16) << 8 | cdat[8 * x + 1] as u16;
                    let gcol = (cdat[8 * x + 2] as u16) << 8 | cdat[8 * x + 3] as u16;
                    let bcol = (cdat[8 * x + 4] as u16) << 8 | cdat[8 * x + 5] as u16;
                    let acol = (cdat[8 * x + 6] as u16) << 8 | cdat[8 * x + 7] as u16;
                    nrgba64.set_nrgba64(
                        x as i64,
                        y,
                        color::NRGBA64 {
                            r: rcol,
                            g: gcol,
                            b: bcol,
                            a: acol,
                        },
                    );
                }
            }
            _ => {}
        }

        // The current row for y is the previous row for y+1.
        std::mem::swap(&mut pr, &mut cr);
    }

    Ok(Some(img))
}

/// Go `paletted.Palette = paletted.Palette[:n]`: re-slicing within the
/// decoder palette's 256-entry backing array (which the image's palette
/// shares in Go).
fn extend_palette(p: &mut Palette, d_palette: &GoPalette, n: usize) {
    let cur = p.len();
    p.extend_from_slice(&d_palette.backing[cur..n]);
}

// mergePassInto merges a single pass into a full sized image.
// Go: image/png/reader.go:(*decoder).mergePassInto. (Go also handles
// *image.Alpha and *image.Alpha16 destinations, which readImagePass never
// produces.)
fn merge_pass_into(dst: &mut PassImage, src: &PassImage, pass: usize) {
    let p = &INTERLACING[pass];
    let bounds = src.bounds();
    let (src_pix, dst_pix, stride, rect, bytes_per_pixel): (&[u8], &mut [u8], i64, Rectangle, i64) =
        match (dst, src) {
            (PassImage::Gray(target), PassImage::Gray(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 1)
            }
            (PassImage::Gray16(target), PassImage::Gray16(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 2)
            }
            (PassImage::NRGBA(target), PassImage::NRGBA(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 4)
            }
            (PassImage::NRGBA64(target), PassImage::NRGBA64(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 8)
            }
            (PassImage::Paletted(target), PassImage::Paletted(source)) => {
                if target.palette.len() < source.palette.len() {
                    // readImagePass can return a paletted image whose implicit palette
                    // length (one more than the maximum Pix value) is larger than the
                    // explicit palette length (what's in the PLTE chunk). Make the
                    // same adjustment here.
                    target.palette = source.palette.clone();
                }
                (&source.pix, &mut target.pix, target.stride, target.rect, 1)
            }
            (PassImage::RGBA(target), PassImage::RGBA(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 4)
            }
            (PassImage::RGBA64(target), PassImage::RGBA64(s)) => {
                (&s.pix, &mut target.pix, target.stride, target.rect, 8)
            }
            _ => unreachable!("all passes have the full image's type"),
        };
    let mut s = 0usize;
    let bpp = bytes_per_pixel as usize;
    for y in bounds.min.y..bounds.max.y {
        let d_base = (y * p.y_factor + p.y_offset - rect.min.y) * stride
            + (p.x_offset - rect.min.x) * bytes_per_pixel;
        for x in bounds.min.x..bounds.max.x {
            let d = (d_base + x * p.x_factor * bytes_per_pixel) as usize;
            go_copy(&mut dst_pix[d..], &src_pix[s..s + bpp]);
            s += bpp;
        }
    }
}

/// Decode reads a PNG image from r and returns it as an [`Image`].
/// The type of Image returned depends on the PNG contents: a
/// [`Gray`], [`RGBA`], [`NRGBA`], [`Paletted`], [`Gray16`], [`RGBA64`] or
/// [`NRGBA64`] (use `downcast_ref` / `downcast` to get the concrete type).
///
/// Go: image/png/reader.go:Decode
pub fn decode(r: &mut dyn Read) -> Result<Box<dyn Image>, Error> {
    let mut d = Decoder::new(r);
    if let Err(mut err) = d.check_header() {
        if err.is_eof() {
            err = Error::UnexpectedEof;
        }
        return Err(err);
    }
    while d.stage != DS_SEEN_IEND {
        if let Err(mut err) = d.parse_chunk(false) {
            if err.is_eof() {
                err = Error::UnexpectedEof;
            }
            return Err(err);
        }
    }
    Ok(d.img
        .take()
        .expect("IEND is accepted only after IDAT")
        .into_boxed())
}

/// DecodeConfig returns the color model and dimensions of a PNG image without
/// decoding the entire image.
///
/// Go: image/png/reader.go:DecodeConfig
pub fn decode_config(r: &mut dyn Read) -> Result<Config, Error> {
    let mut d = Decoder::new(r);
    if let Err(mut err) = d.check_header() {
        if err.is_eof() {
            err = Error::UnexpectedEof;
        }
        return Err(err);
    }

    loop {
        if let Err(mut err) = d.parse_chunk(true) {
            if err.is_eof() {
                err = Error::UnexpectedEof;
            }
            return Err(err);
        }

        if cb_paletted(d.cb) {
            if d.stage >= DS_SEEN_TRNS {
                break;
            }
        } else if d.stage >= DS_SEEN_IHDR {
            break;
        }
    }

    let cm = match d.cb {
        CB_G1 | CB_G2 | CB_G4 | CB_G8 => Model::Gray,
        CB_GA8 => Model::NRGBA,
        CB_TC8 => Model::RGBA,
        CB_P1 | CB_P2 | CB_P4 | CB_P8 => Model::Palette(d.palette.slice()),
        CB_TCA8 => Model::NRGBA,
        CB_G16 => Model::Gray16,
        CB_GA16 => Model::NRGBA64,
        CB_TC16 => Model::RGBA64,
        CB_TCA16 => Model::NRGBA64,
        _ => unreachable!("parseIHDR rejects invalid color type/bit depth combinations"),
    };
    Ok(Config {
        color_model: cm,
        width: d.width,
        height: d.height,
    })
}

fn decode_boxed(r: &mut dyn Read) -> Result<Box<dyn Image>, go_image::BoxError> {
    decode(r).map_err(|e| Box::new(e) as go_image::BoxError)
}

fn decode_config_boxed(r: &mut dyn Read) -> Result<Config, go_image::BoxError> {
    decode_config(r).map_err(|e| Box::new(e) as go_image::BoxError)
}

/// Registers the PNG format with [`go_image::decode`] (Go's package `init`:
/// `image.RegisterFormat("png", pngHeader, Decode, DecodeConfig)`).
/// Idempotent.
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        go_image::register_format("png", PNG_HEADER, decode_boxed, decode_config_boxed);
    });
}
