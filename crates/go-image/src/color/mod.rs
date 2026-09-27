//! Port of Go 1.27.1 `image/color` (color.go and ycbcr.go).
//!
//! Go's `color.Color` interface is the [`Color`] enum over the concrete colour
//! types of this package; `color.Model` is the [`Model`] enum (the standard
//! models, `color.Palette`, `*image.Uniform` used as a model, and
//! `color.ModelFunc`). All arithmetic is exact `uint32`/`int32` integer math as
//! in Go (wrapping where Go wraps).

pub mod palette;
mod ycbcr;

pub use ycbcr::{
    CMYK, NYCbCrA, YCbCr, cmyk_model, cmyk_to_rgb, n_ycbcr_a_model, rgb_to_cmyk, rgb_to_ycbcr,
    ycbcr_model, ycbcr_to_rgb,
};

use std::ops::{Deref, DerefMut};

/// RGBA represents a traditional 32-bit alpha-premultiplied color, having 8
/// bits for each of red, green, blue and alpha.
///
/// Go: image/color/color.go:RGBA
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGBA {
    // Go: image/color/color.go:RGBA.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let mut r = self.r as u32;
        r |= r << 8;
        let mut g = self.g as u32;
        g |= g << 8;
        let mut b = self.b as u32;
        b |= b << 8;
        let mut a = self.a as u32;
        a |= a << 8;
        (r, g, b, a)
    }
}

/// RGBA64 represents a 64-bit alpha-premultiplied color, having 16 bits for
/// each of red, green, blue and alpha.
///
/// Go: image/color/color.go:RGBA64
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RGBA64 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

impl RGBA64 {
    // Go: image/color/color.go:RGBA64.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        (self.r as u32, self.g as u32, self.b as u32, self.a as u32)
    }
}

/// NRGBA represents a non-alpha-premultiplied 32-bit color.
///
/// Go: image/color/color.go:NRGBA
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NRGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl NRGBA {
    // Go: image/color/color.go:NRGBA.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let mut r = self.r as u32;
        r |= r << 8;
        r *= self.a as u32;
        r /= 0xff;
        let mut g = self.g as u32;
        g |= g << 8;
        g *= self.a as u32;
        g /= 0xff;
        let mut b = self.b as u32;
        b |= b << 8;
        b *= self.a as u32;
        b /= 0xff;
        let mut a = self.a as u32;
        a |= a << 8;
        (r, g, b, a)
    }
}

/// NRGBA64 represents a non-alpha-premultiplied 64-bit color, having 16 bits
/// for each of red, green, blue and alpha.
///
/// Go: image/color/color.go:NRGBA64
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NRGBA64 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

impl NRGBA64 {
    // Go: image/color/color.go:NRGBA64.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let mut r = self.r as u32;
        r *= self.a as u32;
        r /= 0xffff;
        let mut g = self.g as u32;
        g *= self.a as u32;
        g /= 0xffff;
        let mut b = self.b as u32;
        b *= self.a as u32;
        b /= 0xffff;
        let a = self.a as u32;
        (r, g, b, a)
    }
}

/// Alpha represents an 8-bit alpha color.
///
/// Go: image/color/color.go:Alpha
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Alpha {
    pub a: u8,
}

impl Alpha {
    // Go: image/color/color.go:Alpha.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let mut a = self.a as u32;
        a |= a << 8;
        (a, a, a, a)
    }
}

/// Alpha16 represents a 16-bit alpha color.
///
/// Go: image/color/color.go:Alpha16
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Alpha16 {
    pub a: u16,
}

impl Alpha16 {
    // Go: image/color/color.go:Alpha16.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let a = self.a as u32;
        (a, a, a, a)
    }
}

/// Gray represents an 8-bit grayscale color.
///
/// Go: image/color/color.go:Gray
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Gray {
    pub y: u8,
}

impl Gray {
    // Go: image/color/color.go:Gray.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let mut y = self.y as u32;
        y |= y << 8;
        (y, y, y, 0xffff)
    }
}

/// Gray16 represents a 16-bit grayscale color.
///
/// Go: image/color/color.go:Gray16
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Gray16 {
    pub y: u16,
}

impl Gray16 {
    // Go: image/color/color.go:Gray16.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        let y = self.y as u32;
        (y, y, y, 0xffff)
    }
}

/// Color can convert itself to alpha-premultiplied 16-bits per channel RGBA.
///
/// Go: image/color/color.go:Color (interface). The Rust port is a closed enum
/// over the concrete colour types of the Go standard library; a Go type
/// assertion `c.(RGBA)` becomes a `match`/`if let Color::RGBA(..)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    RGBA(RGBA),
    RGBA64(RGBA64),
    NRGBA(NRGBA),
    NRGBA64(NRGBA64),
    Alpha(Alpha),
    Alpha16(Alpha16),
    Gray(Gray),
    Gray16(Gray16),
    YCbCr(YCbCr),
    NYCbCrA(NYCbCrA),
    CMYK(CMYK),
}

impl Color {
    /// RGBA returns the alpha-premultiplied red, green, blue and alpha values
    /// for the color, each within [0, 0xffff].
    ///
    /// Go: image/color/color.go:Color.RGBA (dynamic dispatch)
    #[inline]
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        match self {
            Color::RGBA(c) => c.rgba(),
            Color::RGBA64(c) => c.rgba(),
            Color::NRGBA(c) => c.rgba(),
            Color::NRGBA64(c) => c.rgba(),
            Color::Alpha(c) => c.rgba(),
            Color::Alpha16(c) => c.rgba(),
            Color::Gray(c) => c.rgba(),
            Color::Gray16(c) => c.rgba(),
            Color::YCbCr(c) => c.rgba(),
            Color::NYCbCrA(c) => c.rgba(),
            Color::CMYK(c) => c.rgba(),
        }
    }
}

macro_rules! impl_from_color {
    ($($t:ident),*) => {
        $(impl From<$t> for Color {
            fn from(c: $t) -> Color {
                Color::$t(c)
            }
        })*
    };
}
impl_from_color!(
    RGBA, RGBA64, NRGBA, NRGBA64, Alpha, Alpha16, Gray, Gray16, YCbCr, NYCbCrA, CMYK
);

/// Model can convert any [`Color`] to one from its own color model.
///
/// Go: image/color/color.go:Model (interface). The standard model values
/// (`color.RGBAModel`, ...) are the unit variants; `color.Palette` used as a
/// model is [`Model::Palette`]; an `*image.Uniform` used as a model is
/// [`Model::Uniform`]; `color.ModelFunc(f)` is [`Model::Func`].
#[derive(Clone, Debug)]
pub enum Model {
    RGBA,
    RGBA64,
    NRGBA,
    NRGBA64,
    Alpha,
    Alpha16,
    Gray,
    Gray16,
    YCbCr,
    NYCbCrA,
    CMYK,
    Palette(Palette),
    Uniform(Color),
    Func(fn(Color) -> Color),
}

impl PartialEq for Model {
    /// Go compares models by interface identity: the standard models are
    /// singletons, palettes compare element-wise here (Go would panic on
    /// comparing slices), funcs by address.
    fn eq(&self, other: &Model) -> bool {
        match (self, other) {
            (Model::Palette(a), Model::Palette(b)) => a == b,
            (Model::Uniform(a), Model::Uniform(b)) => a == b,
            (Model::Func(a), Model::Func(b)) => std::ptr::fn_addr_eq(*a, *b),
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
}

impl Model {
    /// Go: Model.Convert.
    ///
    /// Deviation: Go's `Palette.Convert` returns a nil `Color` for an empty
    /// palette; there is no nil [`Color`], so this panics in that case (any
    /// Go caller using the result would panic too). Use
    /// [`Palette::convert`] to observe the empty case.
    pub fn convert(&self, c: Color) -> Color {
        match self {
            Model::RGBA => rgba_model(c),
            Model::RGBA64 => rgba64_model(c),
            Model::NRGBA => nrgba_model(c),
            Model::NRGBA64 => nrgba64_model(c),
            Model::Alpha => alpha_model(c),
            Model::Alpha16 => alpha16_model(c),
            Model::Gray => gray_model(c),
            Model::Gray16 => gray16_model(c),
            Model::YCbCr => ycbcr_model(c),
            Model::NYCbCrA => n_ycbcr_a_model(c),
            Model::CMYK => cmyk_model(c),
            Model::Palette(p) => p
                .convert(c)
                .expect("color: Convert on an empty Palette returns a nil Color"),
            Model::Uniform(u) => *u,
            Model::Func(f) => f(c),
        }
    }
}

/// ModelFunc returns a [`Model`] that invokes f to implement the conversion.
///
/// Go: image/color/color.go:ModelFunc
pub fn model_func(f: fn(Color) -> Color) -> Model {
    Model::Func(f)
}

// Models for the standard color types.
// Go: image/color/color.go:RGBAModel etc.
pub const RGBA_MODEL: Model = Model::RGBA;
pub const RGBA64_MODEL: Model = Model::RGBA64;
pub const NRGBA_MODEL: Model = Model::NRGBA;
pub const NRGBA64_MODEL: Model = Model::NRGBA64;
pub const ALPHA_MODEL: Model = Model::Alpha;
pub const ALPHA16_MODEL: Model = Model::Alpha16;
pub const GRAY_MODEL: Model = Model::Gray;
pub const GRAY16_MODEL: Model = Model::Gray16;
pub const YCBCR_MODEL: Model = Model::YCbCr;
pub const NYCBCRA_MODEL: Model = Model::NYCbCrA;
pub const CMYK_MODEL: Model = Model::CMYK;

// Go: image/color/color.go:rgbaModel
pub fn rgba_model(c: Color) -> Color {
    if let Color::RGBA(_) = c {
        return c;
    }
    let (r, g, b, a) = c.rgba();
    Color::RGBA(RGBA {
        r: (r >> 8) as u8,
        g: (g >> 8) as u8,
        b: (b >> 8) as u8,
        a: (a >> 8) as u8,
    })
}

// Go: image/color/color.go:rgba64Model
pub fn rgba64_model(c: Color) -> Color {
    if let Color::RGBA64(_) = c {
        return c;
    }
    let (r, g, b, a) = c.rgba();
    Color::RGBA64(RGBA64 {
        r: r as u16,
        g: g as u16,
        b: b as u16,
        a: a as u16,
    })
}

// Go: image/color/color.go:nrgbaModel
pub fn nrgba_model(c: Color) -> Color {
    if let Color::NRGBA(_) = c {
        return c;
    }
    let (mut r, mut g, mut b, a) = c.rgba();
    if a == 0xffff {
        return Color::NRGBA(NRGBA {
            r: (r >> 8) as u8,
            g: (g >> 8) as u8,
            b: (b >> 8) as u8,
            a: 0xff,
        });
    }
    if a == 0 {
        return Color::NRGBA(NRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    // Since Color.RGBA returns an alpha-premultiplied color, we should have r <= a && g <= a && b <= a.
    r = r.wrapping_mul(0xffff) / a;
    g = g.wrapping_mul(0xffff) / a;
    b = b.wrapping_mul(0xffff) / a;
    Color::NRGBA(NRGBA {
        r: (r >> 8) as u8,
        g: (g >> 8) as u8,
        b: (b >> 8) as u8,
        a: (a >> 8) as u8,
    })
}

// Go: image/color/color.go:nrgba64Model
pub fn nrgba64_model(c: Color) -> Color {
    if let Color::NRGBA64(_) = c {
        return c;
    }
    let (mut r, mut g, mut b, a) = c.rgba();
    if a == 0xffff {
        return Color::NRGBA64(NRGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: 0xffff,
        });
    }
    if a == 0 {
        return Color::NRGBA64(NRGBA64 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    // Since Color.RGBA returns an alpha-premultiplied color, we should have r <= a && g <= a && b <= a.
    r = r.wrapping_mul(0xffff) / a;
    g = g.wrapping_mul(0xffff) / a;
    b = b.wrapping_mul(0xffff) / a;
    Color::NRGBA64(NRGBA64 {
        r: r as u16,
        g: g as u16,
        b: b as u16,
        a: a as u16,
    })
}

// Go: image/color/color.go:alphaModel
pub fn alpha_model(c: Color) -> Color {
    if let Color::Alpha(_) = c {
        return c;
    }
    let (_, _, _, a) = c.rgba();
    Color::Alpha(Alpha { a: (a >> 8) as u8 })
}

// Go: image/color/color.go:alpha16Model
pub fn alpha16_model(c: Color) -> Color {
    if let Color::Alpha16(_) = c {
        return c;
    }
    let (_, _, _, a) = c.rgba();
    Color::Alpha16(Alpha16 { a: a as u16 })
}

// Go: image/color/color.go:grayModel
pub fn gray_model(c: Color) -> Color {
    if let Color::Gray(_) = c {
        return c;
    }
    let (r, g, b, _) = c.rgba();

    // These coefficients (the fractions 0.299, 0.587 and 0.114) are the same
    // as those given by the JFIF specification and used by func RGBToYCbCr in
    // ycbcr.go.
    //
    // Note that 19595 + 38470 + 7471 equals 65536.
    //
    // The 24 is 16 + 8. The 16 is the same as used in RGBToYCbCr. The 8 is
    // because the return value is 8 bit color, not 16 bit color.
    let y = (19595u32
        .wrapping_mul(r)
        .wrapping_add(38470u32.wrapping_mul(g))
        .wrapping_add(7471u32.wrapping_mul(b))
        .wrapping_add(1 << 15))
        >> 24;

    Color::Gray(Gray { y: y as u8 })
}

// Go: image/color/color.go:gray16Model
pub fn gray16_model(c: Color) -> Color {
    if let Color::Gray16(_) = c {
        return c;
    }
    let (r, g, b, _) = c.rgba();

    // These coefficients (the fractions 0.299, 0.587 and 0.114) are the same
    // as those given by the JFIF specification and used by func RGBToYCbCr in
    // ycbcr.go.
    //
    // Note that 19595 + 38470 + 7471 equals 65536.
    let y = (19595u32
        .wrapping_mul(r)
        .wrapping_add(38470u32.wrapping_mul(g))
        .wrapping_add(7471u32.wrapping_mul(b))
        .wrapping_add(1 << 15))
        >> 16;

    Color::Gray16(Gray16 { y: y as u16 })
}

/// Palette is a palette of colors.
///
/// Go: image/color/color.go:Palette (`[]Color`). Derefs to `Vec<Color>`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Palette(pub Vec<Color>);

impl Deref for Palette {
    type Target = Vec<Color>;
    fn deref(&self) -> &Vec<Color> {
        &self.0
    }
}

impl DerefMut for Palette {
    fn deref_mut(&mut self) -> &mut Vec<Color> {
        &mut self.0
    }
}

impl From<Vec<Color>> for Palette {
    fn from(v: Vec<Color>) -> Palette {
        Palette(v)
    }
}

impl FromIterator<Color> for Palette {
    fn from_iter<I: IntoIterator<Item = Color>>(iter: I) -> Palette {
        Palette(iter.into_iter().collect())
    }
}

impl Palette {
    /// Convert returns the palette color closest to c in Euclidean R,G,B space.
    /// `None` is Go's nil (empty palette).
    ///
    /// Go: image/color/color.go:Palette.Convert
    pub fn convert(&self, c: Color) -> Option<Color> {
        if self.0.is_empty() {
            return None;
        }
        Some(self.0[self.index(c)])
    }

    /// Index returns the index of the palette color closest to c in Euclidean
    /// R,G,B,A space.
    ///
    /// Go: image/color/color.go:Palette.Index
    pub fn index(&self, c: Color) -> usize {
        // A batch version of this computation is in image/draw/draw.go.

        let (cr, cg, cb, ca) = c.rgba();
        let (mut ret, mut best_sum) = (0usize, u32::MAX);
        for (i, v) in self.0.iter().enumerate() {
            let (vr, vg, vb, va) = v.rgba();
            let sum = sq_diff(cr, vr)
                .wrapping_add(sq_diff(cg, vg))
                .wrapping_add(sq_diff(cb, vb))
                .wrapping_add(sq_diff(ca, va));
            if sum < best_sum {
                if sum == 0 {
                    return i;
                }
                (ret, best_sum) = (i, sum);
            }
        }
        ret
    }
}

/// sqDiff returns the squared-difference of x and y, shifted by 2 so that
/// adding four of those won't overflow a uint32.
///
/// Go: image/color/color.go:sqDiff (wrapping uint32 arithmetic).
#[inline]
pub(crate) fn sq_diff(x: u32, y: u32) -> u32 {
    let d = x.wrapping_sub(y);
    d.wrapping_mul(d) >> 2
}

// Standard colors.
// Go: image/color/color.go:Black, White, Transparent, Opaque
pub const BLACK: Gray16 = Gray16 { y: 0 };
pub const WHITE: Gray16 = Gray16 { y: 0xffff };
pub const TRANSPARENT: Alpha16 = Alpha16 { a: 0 };
pub const OPAQUE: Alpha16 = Alpha16 { a: 0xffff };

#[cfg(test)]
mod tests {
    use super::*;

    // Go: image/color/color_test.go:TestSqDiff
    #[test]
    #[allow(clippy::manual_abs_diff)] // Go's canonical reference implementation
    fn test_sq_diff() {
        let orig = |x: u32, y: u32| -> u32 {
            let d = if x > y { x - y } else { y - x };
            d.wrapping_mul(d) >> 2
        };
        let cases: [u32; 12] = [
            0, 1, 2, 0x0fffd, 0x0fffe, 0x0ffff, 0x10000, 0x10001, 0x10002, 0xfffffffd, 0xfffffffe,
            0xffffffff,
        ];
        for &x in &cases {
            for &y in &cases {
                assert_eq!(sq_diff(x, y), orig(x, y), "sqDiff({:#x}, {:#x})", x, y);
            }
        }
        let mut s = 7u64;
        for _ in 0..100000 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let (x, y) = (s as u32, (s >> 32) as u32);
            assert_eq!(sq_diff(x, y), orig(x, y));
        }
    }
}
