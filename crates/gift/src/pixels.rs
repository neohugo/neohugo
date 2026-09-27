//! Port of gift v1.2.1 `pixels.go`: float32 pixel getters and setters for the
//! standard image types, including the premultiplied RGBA conversions.
//!
//! Go keeps one getter/setter struct with a pointer per concrete type and a
//! type tag; the port keeps the tag (`it`) and an enum of references. Floats
//! follow the arm64 build exactly: `f32u8(v*255)` and `f32u16(v*65535)` are
//! inlined into `setPixel`, where the compiler fuses `v*k + 0.5` (FMADDS).

use go_image::color::{self, Color};
use go_image::{
    Gray, Gray16, Image, NRGBA, NRGBA64, Paletted, RGBA, RGBA64, Rectangle, YCbCr,
    YCbCrSubsampleRatio, draw, pt,
};

/// Go: pixels.go:pixel
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixel {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Pixel {
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Pixel {
        Pixel { r, g, b, a }
    }
}

/// Go: pixels.go:imageType
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageType {
    Generic,
    NRGBA,
    NRGBA64,
    RGBA,
    RGBA64,
    YCbCr,
    Gray,
    Gray16,
    Paletted,
}

// qf8 = 1.0 / 0xff, qf16 = 1.0 / 0xffff, epal = qf16 * qf16 / 2 (untyped
// constants rounded once to float32).
pub(crate) const QF8: f32 = f32::from_bits(0x3b808081);
pub(crate) const QF16: f32 = f32::from_bits(0x37800080);
pub(crate) const EPAL: f32 = f32::from_bits(0x2f000100);
// inv = 1.0 / (255 * 1e5) in getPixel's YCbCr case.
const YCC_INV: f32 = f32::from_bits(0x33286e1a);
const YCC_MAX: i32 = 25_500_000;

/// Go: pixels.go:pixelFromColor
pub fn pixel_from_color(c: Color) -> Pixel {
    let (r16, g16, b16, a16) = c.rgba();
    match a16 {
        0 => Pixel::new(0.0, 0.0, 0.0, 0.0),
        0xffff => {
            let r = r16 as f32 * QF16;
            let g = g16 as f32 * QF16;
            let b = b16 as f32 * QF16;
            Pixel::new(r, g, b, 1.0)
        }
        _ => {
            let q = 1.0f32 / a16 as f32;
            let r = r16 as f32 * q;
            let g = g16 as f32 * q;
            let b = b16 as f32 * q;
            let a = a16 as f32 * QF16;
            Pixel::new(r, g, b, a)
        }
    }
}

/// Go: pixels.go:convertPalette
pub fn convert_palette(p: &[Color]) -> Vec<Pixel> {
    let mut pal = vec![Pixel::default(); p.len()];
    for i in 0..p.len() {
        pal[i] = pixel_from_color(p[i]);
    }
    pal
}

/// Go: pixels.go:getPaletteIndex. `dcur += d * d` is fused (FMADDS).
pub fn get_palette_index(pal: &[Pixel], px: Pixel) -> i64 {
    let mut k: i64 = 0;
    let mut dmin: f32 = 4.0;
    for (i, palpx) in pal.iter().enumerate() {
        let mut d = px.r - palpx.r;
        let mut dcur = d * d;
        d = px.g - palpx.g;
        dcur = d.mul_add(d, dcur);
        d = px.b - palpx.b;
        dcur = d.mul_add(d, dcur);
        d = px.a - palpx.a;
        dcur = d.mul_add(d, dcur);
        if dcur < EPAL {
            return i as i64;
        }
        if dcur < dmin {
            dmin = dcur;
            k = i as i64;
        }
    }
    k
}

/// Go: pixels.go:f32u8 (every Go call site passes a product and is fused,
/// see [`f32u8_mul`]; kept for the port of TestF32u8).
#[inline]
#[allow(dead_code)]
pub fn f32u8(val: f32) -> u8 {
    let x = (val + 0.5) as i64;
    if x > 0xff {
        return 0xff;
    }
    if x > 0 {
        return x as u8;
    }
    0
}

/// Go: pixels.go:f32u16 (see [`f32u8`]).
#[inline]
#[allow(dead_code)]
pub fn f32u16(val: f32) -> u16 {
    let x = (val + 0.5) as i64;
    if x > 0xffff {
        return 0xffff;
    }
    if x > 0 {
        return x as u16;
    }
    0
}

/// `f32u8(v * k)` as compiled when inlined into setPixel: `v*k + 0.5` is one
/// fused multiply-add (FMADDS), then the int64 truncation and clamping.
#[inline]
fn f32u8_mul(v: f32, k: f32) -> u8 {
    let x = v.mul_add(k, 0.5) as i64;
    if x > 0xff {
        return 0xff;
    }
    if x > 0 {
        return x as u8;
    }
    0
}

/// `f32u16(v * k)` as compiled when inlined into setPixel (fused, see
/// [`f32u8_mul`]).
#[inline]
fn f32u16_mul(v: f32, k: f32) -> u16 {
    let x = v.mul_add(k, 0.5) as i64;
    if x > 0xffff {
        return 0xffff;
    }
    if x > 0 {
        return x as u16;
    }
    0
}

/// Go: pixels.go:clampi32 (returns 0, not min, when val <= min).
#[inline]
pub fn clampi32(val: i32, min: i32, max: i32) -> i32 {
    if val > max {
        return max;
    }
    if val > min {
        return val;
    }
    0
}

#[inline]
fn be16(pix: &[u8], i: usize) -> u16 {
    (pix[i] as u16) << 8 | pix[i + 1] as u16
}

// --- getters on concrete types (pixels.go:getPixel cases) ---

#[inline]
fn get_nrgba(m: &NRGBA, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let r = m.pix[i] as f32 * QF8;
    let g = m.pix[i + 1] as f32 * QF8;
    let b = m.pix[i + 2] as f32 * QF8;
    let a = m.pix[i + 3] as f32 * QF8;
    Pixel::new(r, g, b, a)
}

#[inline]
fn get_nrgba64(m: &NRGBA64, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let r = be16(&m.pix, i) as f32 * QF16;
    let g = be16(&m.pix, i + 2) as f32 * QF16;
    let b = be16(&m.pix, i + 4) as f32 * QF16;
    let a = be16(&m.pix, i + 6) as f32 * QF16;
    Pixel::new(r, g, b, a)
}

#[inline]
fn get_rgba(m: &RGBA, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let a8 = m.pix[i + 3];
    match a8 {
        0xff => {
            let r = m.pix[i] as f32 * QF8;
            let g = m.pix[i + 1] as f32 * QF8;
            let b = m.pix[i + 2] as f32 * QF8;
            Pixel::new(r, g, b, 1.0)
        }
        0 => Pixel::new(0.0, 0.0, 0.0, 0.0),
        _ => {
            let q = 1.0f32 / a8 as f32;
            let r = m.pix[i] as f32 * q;
            let g = m.pix[i + 1] as f32 * q;
            let b = m.pix[i + 2] as f32 * q;
            let a = a8 as f32 * QF8;
            Pixel::new(r, g, b, a)
        }
    }
}

#[inline]
fn get_rgba64(m: &RGBA64, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let a16 = be16(&m.pix, i + 6);
    match a16 {
        0xffff => {
            let r = be16(&m.pix, i) as f32 * QF16;
            let g = be16(&m.pix, i + 2) as f32 * QF16;
            let b = be16(&m.pix, i + 4) as f32 * QF16;
            Pixel::new(r, g, b, 1.0)
        }
        0 => Pixel::new(0.0, 0.0, 0.0, 0.0),
        _ => {
            let q = 1.0f32 / a16 as f32;
            let r = be16(&m.pix, i) as f32 * q;
            let g = be16(&m.pix, i + 2) as f32 * q;
            let b = be16(&m.pix, i + 4) as f32 * q;
            let a = a16 as f32 * QF16;
            Pixel::new(r, g, b, a)
        }
    }
}

#[inline]
fn get_gray(m: &Gray, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let v = m.pix[i] as f32 * QF8;
    Pixel::new(v, v, v, 1.0)
}

#[inline]
fn get_gray16(m: &Gray16, x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let v = be16(&m.pix, i) as f32 * QF16;
    Pixel::new(v, v, v, 1.0)
}

#[inline]
fn get_ycbcr(m: &YCbCr, x: i64, y: i64) -> Pixel {
    let r = m.rect;
    let iy = (y - r.min.y) * m.y_stride + (x - r.min.x);

    let ic = match m.subsample_ratio {
        YCbCrSubsampleRatio::Ratio444 => (y - r.min.y) * m.c_stride + (x - r.min.x),
        YCbCrSubsampleRatio::Ratio422 => (y - r.min.y) * m.c_stride + (x / 2 - r.min.x / 2),
        YCbCrSubsampleRatio::Ratio420 => (y / 2 - r.min.y / 2) * m.c_stride + (x / 2 - r.min.x / 2),
        YCbCrSubsampleRatio::Ratio440 => (y / 2 - r.min.y / 2) * m.c_stride + (x - r.min.x),
        _ => m.c_offset(x, y),
    };

    let y1 = m.y[iy as usize] as i32 * 100_000;
    let cb1 = m.cb[ic as usize] as i32 - 128;
    let cr1 = m.cr[ic as usize] as i32 - 128;

    let r1 = y1 + 140200 * cr1;
    let g1 = y1 - 34414 * cb1 - 71414 * cr1;
    let b1 = y1 + 177200 * cb1;

    let r = clampi32(r1, 0, YCC_MAX) as f32 * YCC_INV;
    let g = clampi32(g1, 0, YCC_MAX) as f32 * YCC_INV;
    let b = clampi32(b1, 0, YCC_MAX) as f32 * YCC_INV;

    Pixel::new(r, g, b, 1.0)
}

#[inline]
fn get_paletted(m: &Paletted, palette: &[Pixel], x: i64, y: i64) -> Pixel {
    let i = m.pix_offset(x, y) as usize;
    let k = m.pix[i];
    palette[k as usize]
}

#[derive(Clone, Copy)]
pub(crate) enum GetterImg<'a> {
    Generic(&'a dyn Image),
    NRGBA(&'a NRGBA),
    NRGBA64(&'a NRGBA64),
    RGBA(&'a RGBA),
    RGBA64(&'a RGBA64),
    Gray(&'a Gray),
    Gray16(&'a Gray16),
    YCbCr(&'a YCbCr),
    Paletted(&'a Paletted),
}

impl GetterImg<'_> {
    #[inline]
    fn get_pixel(&self, palette: &[Pixel], x: i64, y: i64) -> Pixel {
        match *self {
            GetterImg::NRGBA(m) => get_nrgba(m, x, y),
            GetterImg::NRGBA64(m) => get_nrgba64(m, x, y),
            GetterImg::RGBA(m) => get_rgba(m, x, y),
            GetterImg::RGBA64(m) => get_rgba64(m, x, y),
            GetterImg::Gray(m) => get_gray(m, x, y),
            GetterImg::Gray16(m) => get_gray16(m, x, y),
            GetterImg::YCbCr(m) => get_ycbcr(m, x, y),
            GetterImg::Paletted(m) => get_paletted(m, palette, x, y),
            GetterImg::Generic(m) => pixel_from_color(m.at(x, y)),
        }
    }
}

/// Go: pixels.go:pixelGetter
pub(crate) struct PixelGetter<'a> {
    pub it: ImageType,
    pub bounds: Rectangle,
    pub(crate) img: GetterImg<'a>,
    palette: Vec<Pixel>,
}

impl<'a> PixelGetter<'a> {
    /// Go: pixels.go:newPixelGetter
    pub fn new(img: &'a dyn Image) -> PixelGetter<'a> {
        let bounds = img.bounds();
        let mk = |it, img| PixelGetter {
            it,
            bounds,
            img,
            palette: Vec::new(),
        };
        if let Some(m) = img.downcast_ref::<NRGBA>() {
            mk(ImageType::NRGBA, GetterImg::NRGBA(m))
        } else if let Some(m) = img.downcast_ref::<NRGBA64>() {
            mk(ImageType::NRGBA64, GetterImg::NRGBA64(m))
        } else if let Some(m) = img.downcast_ref::<RGBA>() {
            mk(ImageType::RGBA, GetterImg::RGBA(m))
        } else if let Some(m) = img.downcast_ref::<RGBA64>() {
            mk(ImageType::RGBA64, GetterImg::RGBA64(m))
        } else if let Some(m) = img.downcast_ref::<Gray>() {
            mk(ImageType::Gray, GetterImg::Gray(m))
        } else if let Some(m) = img.downcast_ref::<Gray16>() {
            mk(ImageType::Gray16, GetterImg::Gray16(m))
        } else if let Some(m) = img.downcast_ref::<YCbCr>() {
            mk(ImageType::YCbCr, GetterImg::YCbCr(m))
        } else if let Some(m) = img.downcast_ref::<Paletted>() {
            PixelGetter {
                it: ImageType::Paletted,
                bounds,
                img: GetterImg::Paletted(m),
                palette: convert_palette(&m.palette),
            }
        } else {
            mk(ImageType::Generic, GetterImg::Generic(img))
        }
    }

    /// Go: pixels.go:(*pixelGetter).getPixel
    #[inline]
    pub fn get_pixel(&self, x: i64, y: i64) -> Pixel {
        self.img.get_pixel(&self.palette, x, y)
    }

    /// Go: pixels.go:(*pixelGetter).getPixelRow
    pub fn get_pixel_row(&self, y: i64, buf: &mut Vec<Pixel>) {
        buf.clear();
        let mut x = self.bounds.min.x;
        while x != self.bounds.max.x {
            buf.push(self.get_pixel(x, y));
            x += 1;
        }
    }

    /// Go: pixels.go:(*pixelGetter).getPixelColumn
    pub fn get_pixel_column(&self, x: i64, buf: &mut Vec<Pixel>) {
        buf.clear();
        let mut y = self.bounds.min.y;
        while y != self.bounds.max.y {
            buf.push(self.get_pixel(x, y));
            y += 1;
        }
    }

    /// The converted palette (Go's `pixelGetter.palette`).
    pub fn palette(&self) -> &[Pixel] {
        &self.palette
    }
}

pub(crate) enum SetterImg<'a> {
    Generic(&'a mut dyn draw::Image),
    NRGBA(&'a mut NRGBA),
    NRGBA64(&'a mut NRGBA64),
    RGBA(&'a mut RGBA),
    RGBA64(&'a mut RGBA64),
    Gray(&'a mut Gray),
    Gray16(&'a mut Gray16),
    Paletted(&'a mut Paletted),
}

/// Go: pixels.go:pixelSetter
pub(crate) struct PixelSetter<'a> {
    #[allow(dead_code)]
    pub it: ImageType,
    pub bounds: Rectangle,
    pub(crate) img: SetterImg<'a>,
    palette: Vec<Pixel>,
}

impl<'a> PixelSetter<'a> {
    /// Go: pixels.go:newPixelSetter
    pub fn new(img: &'a mut dyn draw::Image) -> PixelSetter<'a> {
        let bounds = img.bounds();
        let mk = |it, img| PixelSetter {
            it,
            bounds,
            img,
            palette: Vec::new(),
        };
        let any = img.as_any();
        if any.is::<NRGBA>() {
            mk(
                ImageType::NRGBA,
                SetterImg::NRGBA(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<NRGBA64>() {
            mk(
                ImageType::NRGBA64,
                SetterImg::NRGBA64(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<RGBA>() {
            mk(
                ImageType::RGBA,
                SetterImg::RGBA(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<RGBA64>() {
            mk(
                ImageType::RGBA64,
                SetterImg::RGBA64(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<Gray>() {
            mk(
                ImageType::Gray,
                SetterImg::Gray(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<Gray16>() {
            mk(
                ImageType::Gray16,
                SetterImg::Gray16(img.as_any_mut().downcast_mut().unwrap()),
            )
        } else if any.is::<Paletted>() {
            let m: &'a mut Paletted = img.as_any_mut().downcast_mut().unwrap();
            let palette = convert_palette(&m.palette);
            PixelSetter {
                it: ImageType::Paletted,
                bounds,
                img: SetterImg::Paletted(m),
                palette,
            }
        } else {
            mk(ImageType::Generic, SetterImg::Generic(img))
        }
    }

    /// Go: pixels.go:(*pixelSetter).setPixel
    pub fn set_pixel(&mut self, x: i64, y: i64, px: Pixel) {
        if !pt(x, y).in_(self.bounds) {
            return;
        }
        match &mut self.img {
            SetterImg::NRGBA(m) => {
                let i = m.pix_offset(x, y) as usize;
                m.pix[i] = f32u8_mul(px.r, 255.0);
                m.pix[i + 1] = f32u8_mul(px.g, 255.0);
                m.pix[i + 2] = f32u8_mul(px.b, 255.0);
                m.pix[i + 3] = f32u8_mul(px.a, 255.0);
            }
            SetterImg::NRGBA64(m) => {
                let r16 = f32u16_mul(px.r, 65535.0);
                let g16 = f32u16_mul(px.g, 65535.0);
                let b16 = f32u16_mul(px.b, 65535.0);
                let a16 = f32u16_mul(px.a, 65535.0);
                let i = m.pix_offset(x, y) as usize;
                m.pix[i] = (r16 >> 8) as u8;
                m.pix[i + 1] = (r16 & 0xff) as u8;
                m.pix[i + 2] = (g16 >> 8) as u8;
                m.pix[i + 3] = (g16 & 0xff) as u8;
                m.pix[i + 4] = (b16 >> 8) as u8;
                m.pix[i + 5] = (b16 & 0xff) as u8;
                m.pix[i + 6] = (a16 >> 8) as u8;
                m.pix[i + 7] = (a16 & 0xff) as u8;
            }
            SetterImg::RGBA(m) => {
                // fa := px.a * 0xff is rounded for the colour channels, but
                // f32u8(fa) is compiled as the fused px.a*255 + 0.5.
                let fa = px.a * 255.0;
                let i = m.pix_offset(x, y) as usize;
                m.pix[i] = f32u8_mul(px.r, fa);
                m.pix[i + 1] = f32u8_mul(px.g, fa);
                m.pix[i + 2] = f32u8_mul(px.b, fa);
                m.pix[i + 3] = f32u8_mul(px.a, 255.0);
            }
            SetterImg::RGBA64(m) => {
                let fa = px.a * 65535.0;
                let r16 = f32u16_mul(px.r, fa);
                let g16 = f32u16_mul(px.g, fa);
                let b16 = f32u16_mul(px.b, fa);
                let a16 = f32u16_mul(px.a, 65535.0);
                let i = m.pix_offset(x, y) as usize;
                m.pix[i] = (r16 >> 8) as u8;
                m.pix[i + 1] = (r16 & 0xff) as u8;
                m.pix[i + 2] = (g16 >> 8) as u8;
                m.pix[i + 3] = (g16 & 0xff) as u8;
                m.pix[i + 4] = (b16 >> 8) as u8;
                m.pix[i + 5] = (b16 & 0xff) as u8;
                m.pix[i + 6] = (a16 >> 8) as u8;
                m.pix[i + 7] = (a16 & 0xff) as u8;
            }
            SetterImg::Gray(m) => {
                let i = m.pix_offset(x, y) as usize;
                m.pix[i] = f32u8_mul(gray_luma(px), 255.0);
            }
            SetterImg::Gray16(m) => {
                let i = m.pix_offset(x, y) as usize;
                let y16 = f32u16_mul(gray_luma(px), 65535.0);
                m.pix[i] = (y16 >> 8) as u8;
                m.pix[i + 1] = (y16 & 0xff) as u8;
            }
            SetterImg::Paletted(m) => {
                let px1 = Pixel::new(
                    minf32(maxf32(px.r, 0.0), 1.0),
                    minf32(maxf32(px.g, 0.0), 1.0),
                    minf32(maxf32(px.b, 0.0), 1.0),
                    minf32(maxf32(px.a, 0.0), 1.0),
                );
                let i = m.pix_offset(x, y) as usize;
                let k = get_palette_index(&self.palette, px1);
                m.pix[i] = k as u8;
            }
            SetterImg::Generic(m) => {
                let r16 = f32u16_mul(px.r, 65535.0);
                let g16 = f32u16_mul(px.g, 65535.0);
                let b16 = f32u16_mul(px.b, 65535.0);
                let a16 = f32u16_mul(px.a, 65535.0);
                m.set(
                    x,
                    y,
                    Color::NRGBA64(color::NRGBA64 {
                        r: r16,
                        g: g16,
                        b: b16,
                        a: a16,
                    }),
                );
            }
        }
    }

    /// Go: pixels.go:(*pixelSetter).setPixelRow
    pub fn set_pixel_row(&mut self, y: i64, buf: &[Pixel]) {
        let mut x = self.bounds.min.x;
        for px in buf {
            self.set_pixel(x, y, *px);
            x += 1;
        }
    }

    /// Go: pixels.go:(*pixelSetter).setPixelColumn
    pub fn set_pixel_column(&mut self, x: i64, buf: &[Pixel]) {
        let mut y = self.bounds.min.y;
        for px in buf {
            self.set_pixel(x, y, *px);
            y += 1;
        }
    }

    /// Reads a pixel of the destination image with the semantics of a
    /// `newPixelGetter(dst)` created before this setter (`palette` is that
    /// getter's converted palette). Used by `DrawAt` with `OverOperator`,
    /// where Go reads and writes the same image through a getter and a setter.
    pub fn get_pixel(&self, palette: &[Pixel], x: i64, y: i64) -> Pixel {
        let img = match &self.img {
            SetterImg::Generic(m) => GetterImg::Generic(&**m),
            SetterImg::NRGBA(m) => GetterImg::NRGBA(m),
            SetterImg::NRGBA64(m) => GetterImg::NRGBA64(m),
            SetterImg::RGBA(m) => GetterImg::RGBA(m),
            SetterImg::RGBA64(m) => GetterImg::RGBA64(m),
            SetterImg::Gray(m) => GetterImg::Gray(m),
            SetterImg::Gray16(m) => GetterImg::Gray16(m),
            SetterImg::Paletted(m) => GetterImg::Paletted(m),
        };
        img.get_pixel(palette, x, y)
    }
}

/// `(0.299*px.r + 0.587*px.g + 0.114*px.b) * px.a` as compiled in setPixel's
/// Gray/Gray16 cases: the red and blue products are fused, green is rounded.
/// (The trailing `* 0xff`/`* 0xffff` is fused into f32u8's `+ 0.5`.)
#[inline]
fn gray_luma(px: Pixel) -> f32 {
    const K299: f32 = f32::from_bits(0x3e991687);
    const K587: f32 = f32::from_bits(0x3f1645a2);
    const K114: f32 = f32::from_bits(0x3de978d5);
    let t = px.r.mul_add(K299, K587 * px.g);
    let t = px.b.mul_add(K114, t);
    t * px.a
}

/// Go: utils.go:minf32
#[inline]
pub fn minf32(x: f32, y: f32) -> f32 {
    if x < y {
        return x;
    }
    y
}

/// Go: utils.go:maxf32
#[inline]
pub fn maxf32(x: f32, y: f32) -> f32 {
    if x > y {
        return x;
    }
    y
}
