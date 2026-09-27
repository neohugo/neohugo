//! Port of Go 1.27.1 `image/image.go`: the `Image`, `RGBA64Image` and
//! `PalettedImage` interfaces and the concrete in-memory image types.
//!
//! Every concrete type has inherent methods named after the Go methods
//! (`at`, `bounds`, `rgba_at`, `pix_offset`, `set`, `set_rgba64`, `sub_image`,
//! `opaque`, ...) and implements the object-safe traits by delegation.
//!
//! `Pix` is an owned `Vec<u8>`; `SubImage` returns an owned image whose `pix`
//! holds a copy of Go's `Pix[i:]` (identical contents, length and stride),
//! see PORTING.md ("SubImage aliasing").

use std::any::Any;

use crate::color::{self, Color, Model, Palette};
use crate::geom::{Point, Rectangle, mul3_non_neg};

/// Image is a finite rectangular grid of [`Color`] values taken from a color
/// model.
///
/// Go: image/image.go:Image. Go type switches on concrete image types are
/// expressed with [`Image::as_any`] (see also `<dyn Image>::downcast_ref`);
/// interface assertions to `image.RGBA64Image`, `image.PalettedImage` and
/// `interface{ Opaque() bool }` with the `as_*`/`try_opaque` methods, whose
/// default implementations return `None` (the assertion fails).
pub trait Image: Any + Send + Sync {
    /// ColorModel returns the Image's color model.
    fn color_model(&self) -> Model;
    /// Bounds returns the domain for which At can return non-zero color.
    fn bounds(&self) -> Rectangle;
    /// At returns the color of the pixel at (x, y).
    fn at(&self, x: i64, y: i64) -> Color;

    /// Upcast for Go-style type switches (`img.(type)`).
    fn as_any(&self) -> &dyn Any;
    /// Owned upcast (for moving a concrete image out of a `Box<dyn Image>`).
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    /// Go: `img.(image.RGBA64Image)`.
    fn as_rgba64_image(&self) -> Option<&dyn RGBA64Image> {
        None
    }
    /// Go: `img.(image.PalettedImage)`.
    fn as_paletted_image(&self) -> Option<&dyn PalettedImage> {
        None
    }
    /// Go: `img.(interface{ Opaque() bool })` — `Some(img.Opaque())` when the
    /// type has an `Opaque` method, `None` otherwise.
    fn try_opaque(&self) -> Option<bool> {
        None
    }
}

impl dyn Image {
    /// Go: `img.(*T)` for a concrete type T.
    pub fn downcast_ref<T: Image>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Go: `_, ok := img.(*T)`.
    pub fn is<T: Image>(&self) -> bool {
        self.as_any().is::<T>()
    }

    /// Moves the concrete image out of the box when it has type T.
    pub fn downcast<T: Image>(self: Box<Self>) -> Result<Box<T>, Box<dyn Image>> {
        if self.as_any().is::<T>() {
            Ok(self.into_any().downcast::<T>().expect("type checked above"))
        } else {
            Err(self)
        }
    }
}

/// RGBA64Image is an [`Image`] whose pixels can be converted directly to a
/// color.RGBA64.
///
/// Go: image/image.go:RGBA64Image
pub trait RGBA64Image: Image {
    /// RGBA64At returns the RGBA64 color of the pixel at (x, y).
    fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64;
}

/// PalettedImage is an image whose colors may come from a limited palette.
///
/// Go: image/image.go:PalettedImage
pub trait PalettedImage: Image {
    /// ColorIndexAt returns the palette index of the pixel at (x, y).
    fn color_index_at(&self, x: i64, y: i64) -> u8;
}

/// pixelBufferLength returns the length of the []uint8 typed Pix slice field
/// for the NewXxx functions. It panics (as Go does) if at least one of the
/// dimensions is negative or if the computation would overflow.
///
/// Go: image/image.go:pixelBufferLength
pub(crate) fn pixel_buffer_length(
    bytes_per_pixel: i64,
    r: Rectangle,
    image_type_name: &str,
) -> usize {
    let total_length = mul3_non_neg(bytes_per_pixel, r.dx(), r.dy());
    if total_length < 0 {
        panic!(
            "image: New{} Rectangle has huge or negative dimensions",
            image_type_name
        );
    }
    total_length as usize
}

/// Go's `Pix[i:]` for a sub-image: an owned copy of the tail of the buffer.
#[inline]
pub(crate) fn tail(pix: &[u8], i: i64) -> Vec<u8> {
    pix[i as usize..].to_vec()
}

/// Go's `Pix[i:]` consuming the parent (no copy when i == 0).
#[inline]
pub(crate) fn into_tail(mut pix: Vec<u8>, i: i64) -> Vec<u8> {
    let i = i as usize;
    if i > pix.len() {
        panic!("slice bounds out of range [{}:{}]", i, pix.len());
    }
    if i > 0 {
        pix.drain(..i);
    }
    pix
}

/// Implements [`Image`] and [`RGBA64Image`] (and optionally the draw traits)
/// for a concrete type by delegating to its inherent methods.
macro_rules! impl_image_traits {
    ($t:ident) => {
        impl $crate::image::Image for $t {
            fn color_model(&self) -> $crate::color::Model {
                $t::color_model(self)
            }
            fn bounds(&self) -> $crate::geom::Rectangle {
                $t::bounds(self)
            }
            fn at(&self, x: i64, y: i64) -> $crate::color::Color {
                $t::at(self, x, y)
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn ::std::any::Any> {
                self
            }
            fn as_rgba64_image(&self) -> Option<&dyn $crate::image::RGBA64Image> {
                Some(self)
            }
            fn try_opaque(&self) -> Option<bool> {
                Some($t::opaque(self))
            }
        }
        impl $crate::image::RGBA64Image for $t {
            fn rgba64_at(&self, x: i64, y: i64) -> $crate::color::RGBA64 {
                $t::rgba64_at(self, x, y)
            }
        }
    };
    ($t:ident, draw) => {
        impl_image_traits!($t);
        impl_image_traits!(@draw $t);
    };
    ($t:ident, draw, paletted) => {
        impl $crate::image::Image for $t {
            fn color_model(&self) -> $crate::color::Model {
                $t::color_model(self)
            }
            fn bounds(&self) -> $crate::geom::Rectangle {
                $t::bounds(self)
            }
            fn at(&self, x: i64, y: i64) -> $crate::color::Color {
                $t::at(self, x, y)
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn ::std::any::Any> {
                self
            }
            fn as_rgba64_image(&self) -> Option<&dyn $crate::image::RGBA64Image> {
                Some(self)
            }
            fn as_paletted_image(&self) -> Option<&dyn $crate::image::PalettedImage> {
                Some(self)
            }
            fn try_opaque(&self) -> Option<bool> {
                Some($t::opaque(self))
            }
        }
        impl $crate::image::RGBA64Image for $t {
            fn rgba64_at(&self, x: i64, y: i64) -> $crate::color::RGBA64 {
                $t::rgba64_at(self, x, y)
            }
        }
        impl_image_traits!(@draw $t);
    };
    (@draw $t:ident) => {
        impl $crate::draw::Image for $t {
            fn set(&mut self, x: i64, y: i64, c: $crate::color::Color) {
                $t::set(self, x, y, c)
            }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }
            fn as_draw_rgba64_image(&mut self) -> Option<&mut dyn $crate::draw::RGBA64Image> {
                Some(self)
            }
        }
        impl $crate::draw::RGBA64Image for $t {
            fn set_rgba64(&mut self, x: i64, y: i64, c: $crate::color::RGBA64) {
                $t::set_rgba64(self, x, y, c)
            }
        }
    };
}
pub(crate) use impl_image_traits;

/// Generates `sub_image`, `into_sub_image` and `with_sub_image_mut` for the
/// single-plane image types (all fields except `pix`/`rect` are copied).
macro_rules! impl_sub_image {
    ($t:ident) => {
        impl $t {
            /// SubImage returns an image representing the portion of the image p
            /// visible through r. Go shares pixels with the original image; the
            /// port returns an owned copy of Go's `Pix[i:]` (see PORTING.md).
            pub fn sub_image(&self, r: Rectangle) -> $t {
                let r = r.intersect(self.rect);
                // If r1 and r2 are Rectangles, r1.Intersect(r2) is not guaranteed to be inside
                // either r1 or r2 if the intersection is empty. Without explicitly checking for
                // this, the Pix[i:] expression below can panic.
                if r.empty() {
                    return $t::default();
                }
                let i = self.pix_offset(r.min.x, r.min.y);
                $t {
                    pix: tail(&self.pix, i),
                    stride: self.stride,
                    rect: r,
                }
            }

            /// Like [`Self::sub_image`] but consumes `self`, avoiding the copy when
            /// r.Min equals the image's Min.
            pub fn into_sub_image(self, r: Rectangle) -> $t {
                let r = r.intersect(self.rect);
                if r.empty() {
                    return $t::default();
                }
                let i = self.pix_offset(r.min.x, r.min.y);
                $t {
                    pix: into_tail(self.pix, i),
                    stride: self.stride,
                    rect: r,
                }
            }

            /// Emulates Go's aliasing SubImage for one mutable view: runs `f`
            /// on the sub-image whose `pix` is exactly Go's `Pix[i:]`, then
            /// writes the (possibly modified) tail back into `self`.
            pub fn with_sub_image_mut<R>(
                &mut self,
                r: Rectangle,
                f: impl FnOnce(&mut $t) -> R,
            ) -> R {
                let r = r.intersect(self.rect);
                if r.empty() {
                    let mut empty = $t::default();
                    return f(&mut empty);
                }
                let i = self.pix_offset(r.min.x, r.min.y) as usize;
                let tail = self.pix.split_off(i);
                let mut sub = $t {
                    pix: tail,
                    stride: self.stride,
                    rect: r,
                };
                let ret = f(&mut sub);
                self.pix.append(&mut sub.pix);
                ret
            }
        }
    };
}

// ---------------------------------------------------------------------------
// RGBA
// ---------------------------------------------------------------------------

/// RGBA is an in-memory image whose At method returns [`color::RGBA`] values.
///
/// Go: image/image.go:RGBA
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RGBA {
    /// Pix holds the image's pixels, in R, G, B, A order. The pixel at
    /// (x, y) starts at Pix[(y-Rect.Min.Y)*Stride + (x-Rect.Min.X)*4].
    pub pix: Vec<u8>,
    /// Stride is the Pix stride (in bytes) between vertically adjacent pixels.
    pub stride: i64,
    /// Rect is the image's bounds.
    pub rect: Rectangle,
}

impl RGBA {
    /// NewRGBA returns a new [`RGBA`] image with the given bounds.
    ///
    /// Go: image/image.go:NewRGBA
    pub fn new(r: Rectangle) -> RGBA {
        RGBA {
            pix: vec![0; pixel_buffer_length(4, r, "RGBA")],
            stride: 4 * r.dx(),
            rect: r,
        }
    }

    // Go: image/image.go:RGBA.ColorModel
    pub fn color_model(&self) -> Model {
        Model::RGBA
    }

    // Go: image/image.go:RGBA.Bounds
    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:RGBA.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::RGBA(self.rgba_at(x, y))
    }

    // Go: image/image.go:RGBA.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        if !(Point { x, y }.in_(self.rect)) {
            return color::RGBA64::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 4];
        let r = s[0] as u16;
        let g = s[1] as u16;
        let b = s[2] as u16;
        let a = s[3] as u16;
        color::RGBA64 {
            r: (r << 8) | r,
            g: (g << 8) | g,
            b: (b << 8) | b,
            a: (a << 8) | a,
        }
    }

    // Go: image/image.go:RGBA.RGBAAt
    pub fn rgba_at(&self, x: i64, y: i64) -> color::RGBA {
        if !(Point { x, y }.in_(self.rect)) {
            return color::RGBA::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 4];
        color::RGBA {
            r: s[0],
            g: s[1],
            b: s[2],
            a: s[3],
        }
    }

    /// PixOffset returns the index of the first element of Pix that
    /// corresponds to the pixel at (x, y).
    ///
    /// Go: image/image.go:RGBA.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(4))
    }

    // Go: image/image.go:RGBA.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::rgba_model(c) {
            Color::RGBA(c1) => c1,
            _ => unreachable!(),
        };
        let s = &mut self.pix[i..i + 4];
        s[0] = c1.r;
        s[1] = c1.g;
        s[2] = c1.b;
        s[3] = c1.a;
    }

    // Go: image/image.go:RGBA.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = (c.r >> 8) as u8;
        s[1] = (c.g >> 8) as u8;
        s[2] = (c.b >> 8) as u8;
        s[3] = (c.a >> 8) as u8;
    }

    // Go: image/image.go:RGBA.SetRGBA
    pub fn set_rgba(&mut self, x: i64, y: i64, c: color::RGBA) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = c.r;
        s[1] = c.g;
        s[2] = c.b;
        s[3] = c.a;
    }

    /// Opaque scans the entire image and reports whether it is fully opaque.
    ///
    /// Go: image/image.go:RGBA.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (3i64, self.rect.dx() * 4);
        for _y in self.rect.min.y..self.rect.max.y {
            let mut i = i0;
            while i < i1 {
                if self.pix[i as usize] != 0xff {
                    return false;
                }
                i += 4;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(RGBA);
impl_image_traits!(RGBA, draw);

// ---------------------------------------------------------------------------
// RGBA64
// ---------------------------------------------------------------------------

/// RGBA64 is an in-memory image whose At method returns [`color::RGBA64`]
/// values.
///
/// Go: image/image.go:RGBA64
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RGBA64 {
    /// Pix holds the image's pixels, in R, G, B, A order and big-endian format.
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl RGBA64 {
    // Go: image/image.go:NewRGBA64
    pub fn new(r: Rectangle) -> RGBA64 {
        RGBA64 {
            pix: vec![0; pixel_buffer_length(8, r, "RGBA64")],
            stride: 8 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::RGBA64
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:RGBA64.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::RGBA64(self.rgba64_at(x, y))
    }

    // Go: image/image.go:RGBA64.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        if !(Point { x, y }.in_(self.rect)) {
            return color::RGBA64::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 8];
        color::RGBA64 {
            r: (s[0] as u16) << 8 | s[1] as u16,
            g: (s[2] as u16) << 8 | s[3] as u16,
            b: (s[4] as u16) << 8 | s[5] as u16,
            a: (s[6] as u16) << 8 | s[7] as u16,
        }
    }

    // Go: image/image.go:RGBA64.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(8))
    }

    // Go: image/image.go:RGBA64.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::rgba64_model(c) {
            Color::RGBA64(c1) => c1,
            _ => unreachable!(),
        };
        let s = &mut self.pix[i..i + 8];
        s[0] = (c1.r >> 8) as u8;
        s[1] = c1.r as u8;
        s[2] = (c1.g >> 8) as u8;
        s[3] = c1.g as u8;
        s[4] = (c1.b >> 8) as u8;
        s[5] = c1.b as u8;
        s[6] = (c1.a >> 8) as u8;
        s[7] = c1.a as u8;
    }

    // Go: image/image.go:RGBA64.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 8];
        s[0] = (c.r >> 8) as u8;
        s[1] = c.r as u8;
        s[2] = (c.g >> 8) as u8;
        s[3] = c.g as u8;
        s[4] = (c.b >> 8) as u8;
        s[5] = c.b as u8;
        s[6] = (c.a >> 8) as u8;
        s[7] = c.a as u8;
    }

    // Go: image/image.go:RGBA64.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (6i64, self.rect.dx() * 8);
        for _y in self.rect.min.y..self.rect.max.y {
            let mut i = i0;
            while i < i1 {
                if self.pix[i as usize] != 0xff || self.pix[i as usize + 1] != 0xff {
                    return false;
                }
                i += 8;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(RGBA64);
impl_image_traits!(RGBA64, draw);

// ---------------------------------------------------------------------------
// NRGBA
// ---------------------------------------------------------------------------

/// NRGBA is an in-memory image whose At method returns [`color::NRGBA`] values.
///
/// Go: image/image.go:NRGBA
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NRGBA {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl NRGBA {
    // Go: image/image.go:NewNRGBA
    pub fn new(r: Rectangle) -> NRGBA {
        NRGBA {
            pix: vec![0; pixel_buffer_length(4, r, "NRGBA")],
            stride: 4 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::NRGBA
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:NRGBA.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::NRGBA(self.nrgba_at(x, y))
    }

    // Go: image/image.go:NRGBA.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.nrgba_at(x, y).rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/image.go:NRGBA.NRGBAAt
    pub fn nrgba_at(&self, x: i64, y: i64) -> color::NRGBA {
        if !(Point { x, y }.in_(self.rect)) {
            return color::NRGBA::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 4];
        color::NRGBA {
            r: s[0],
            g: s[1],
            b: s[2],
            a: s[3],
        }
    }

    // Go: image/image.go:NRGBA.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(4))
    }

    // Go: image/image.go:NRGBA.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::nrgba_model(c) {
            Color::NRGBA(c1) => c1,
            _ => unreachable!(),
        };
        let s = &mut self.pix[i..i + 4];
        s[0] = c1.r;
        s[1] = c1.g;
        s[2] = c1.b;
        s[3] = c1.a;
    }

    // Go: image/image.go:NRGBA.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let (mut r, mut g, mut b, a) = (c.r as u32, c.g as u32, c.b as u32, c.a as u32);
        if (a != 0) && (a != 0xffff) {
            r = r.wrapping_mul(0xffff) / a;
            g = g.wrapping_mul(0xffff) / a;
            b = b.wrapping_mul(0xffff) / a;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = (r >> 8) as u8;
        s[1] = (g >> 8) as u8;
        s[2] = (b >> 8) as u8;
        s[3] = (a >> 8) as u8;
    }

    // Go: image/image.go:NRGBA.SetNRGBA
    pub fn set_nrgba(&mut self, x: i64, y: i64, c: color::NRGBA) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = c.r;
        s[1] = c.g;
        s[2] = c.b;
        s[3] = c.a;
    }

    // Go: image/image.go:NRGBA.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (3i64, self.rect.dx() * 4);
        for _y in self.rect.min.y..self.rect.max.y {
            let mut i = i0;
            while i < i1 {
                if self.pix[i as usize] != 0xff {
                    return false;
                }
                i += 4;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(NRGBA);
impl_image_traits!(NRGBA, draw);

// ---------------------------------------------------------------------------
// NRGBA64
// ---------------------------------------------------------------------------

/// NRGBA64 is an in-memory image whose At method returns [`color::NRGBA64`]
/// values.
///
/// Go: image/image.go:NRGBA64
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NRGBA64 {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl NRGBA64 {
    // Go: image/image.go:NewNRGBA64
    pub fn new(r: Rectangle) -> NRGBA64 {
        NRGBA64 {
            pix: vec![0; pixel_buffer_length(8, r, "NRGBA64")],
            stride: 8 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::NRGBA64
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:NRGBA64.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::NRGBA64(self.nrgba64_at(x, y))
    }

    // Go: image/image.go:NRGBA64.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.nrgba64_at(x, y).rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/image.go:NRGBA64.NRGBA64At
    pub fn nrgba64_at(&self, x: i64, y: i64) -> color::NRGBA64 {
        if !(Point { x, y }.in_(self.rect)) {
            return color::NRGBA64::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 8];
        color::NRGBA64 {
            r: (s[0] as u16) << 8 | s[1] as u16,
            g: (s[2] as u16) << 8 | s[3] as u16,
            b: (s[4] as u16) << 8 | s[5] as u16,
            a: (s[6] as u16) << 8 | s[7] as u16,
        }
    }

    // Go: image/image.go:NRGBA64.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(8))
    }

    // Go: image/image.go:NRGBA64.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::nrgba64_model(c) {
            Color::NRGBA64(c1) => c1,
            _ => unreachable!(),
        };
        let s = &mut self.pix[i..i + 8];
        s[0] = (c1.r >> 8) as u8;
        s[1] = c1.r as u8;
        s[2] = (c1.g >> 8) as u8;
        s[3] = c1.g as u8;
        s[4] = (c1.b >> 8) as u8;
        s[5] = c1.b as u8;
        s[6] = (c1.a >> 8) as u8;
        s[7] = c1.a as u8;
    }

    // Go: image/image.go:NRGBA64.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let (mut r, mut g, mut b, a) = (c.r as u32, c.g as u32, c.b as u32, c.a as u32);
        if (a != 0) && (a != 0xffff) {
            r = r.wrapping_mul(0xffff) / a;
            g = g.wrapping_mul(0xffff) / a;
            b = b.wrapping_mul(0xffff) / a;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 8];
        s[0] = (r >> 8) as u8;
        s[1] = r as u8;
        s[2] = (g >> 8) as u8;
        s[3] = g as u8;
        s[4] = (b >> 8) as u8;
        s[5] = b as u8;
        s[6] = (a >> 8) as u8;
        s[7] = a as u8;
    }

    // Go: image/image.go:NRGBA64.SetNRGBA64
    pub fn set_nrgba64(&mut self, x: i64, y: i64, c: color::NRGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 8];
        s[0] = (c.r >> 8) as u8;
        s[1] = c.r as u8;
        s[2] = (c.g >> 8) as u8;
        s[3] = c.g as u8;
        s[4] = (c.b >> 8) as u8;
        s[5] = c.b as u8;
        s[6] = (c.a >> 8) as u8;
        s[7] = c.a as u8;
    }

    // Go: image/image.go:NRGBA64.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (6i64, self.rect.dx() * 8);
        for _y in self.rect.min.y..self.rect.max.y {
            let mut i = i0;
            while i < i1 {
                if self.pix[i as usize] != 0xff || self.pix[i as usize + 1] != 0xff {
                    return false;
                }
                i += 8;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(NRGBA64);
impl_image_traits!(NRGBA64, draw);

// ---------------------------------------------------------------------------
// Alpha
// ---------------------------------------------------------------------------

/// Alpha is an in-memory image whose At method returns [`color::Alpha`] values.
///
/// Go: image/image.go:Alpha
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Alpha {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl Alpha {
    // Go: image/image.go:NewAlpha
    pub fn new(r: Rectangle) -> Alpha {
        Alpha {
            pix: vec![0; pixel_buffer_length(1, r, "Alpha")],
            stride: r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::Alpha
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:Alpha.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::Alpha(self.alpha_at(x, y))
    }

    // Go: image/image.go:Alpha.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let mut a = self.alpha_at(x, y).a as u16;
        a |= a << 8;
        color::RGBA64 {
            r: a,
            g: a,
            b: a,
            a,
        }
    }

    // Go: image/image.go:Alpha.AlphaAt
    pub fn alpha_at(&self, x: i64, y: i64) -> color::Alpha {
        if !(Point { x, y }.in_(self.rect)) {
            return color::Alpha::default();
        }
        let i = self.pix_offset(x, y) as usize;
        color::Alpha { a: self.pix[i] }
    }

    // Go: image/image.go:Alpha.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x))
    }

    // Go: image/image.go:Alpha.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = match color::alpha_model(c) {
            Color::Alpha(c1) => c1.a,
            _ => unreachable!(),
        };
    }

    // Go: image/image.go:Alpha.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = (c.a >> 8) as u8;
    }

    // Go: image/image.go:Alpha.SetAlpha
    pub fn set_alpha(&mut self, x: i64, y: i64, c: color::Alpha) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = c.a;
    }

    // Go: image/image.go:Alpha.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (0i64, self.rect.dx());
        for _y in self.rect.min.y..self.rect.max.y {
            for i in i0..i1 {
                if self.pix[i as usize] != 0xff {
                    return false;
                }
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(Alpha);
impl_image_traits!(Alpha, draw);

// ---------------------------------------------------------------------------
// Alpha16
// ---------------------------------------------------------------------------

/// Alpha16 is an in-memory image whose At method returns [`color::Alpha16`]
/// values.
///
/// Go: image/image.go:Alpha16
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Alpha16 {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl Alpha16 {
    // Go: image/image.go:NewAlpha16
    pub fn new(r: Rectangle) -> Alpha16 {
        Alpha16 {
            pix: vec![0; pixel_buffer_length(2, r, "Alpha16")],
            stride: 2 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::Alpha16
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:Alpha16.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::Alpha16(self.alpha16_at(x, y))
    }

    // Go: image/image.go:Alpha16.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let a = self.alpha16_at(x, y).a;
        color::RGBA64 {
            r: a,
            g: a,
            b: a,
            a,
        }
    }

    // Go: image/image.go:Alpha16.Alpha16At
    pub fn alpha16_at(&self, x: i64, y: i64) -> color::Alpha16 {
        if !(Point { x, y }.in_(self.rect)) {
            return color::Alpha16::default();
        }
        let i = self.pix_offset(x, y) as usize;
        color::Alpha16 {
            a: (self.pix[i] as u16) << 8 | self.pix[i + 1] as u16,
        }
    }

    // Go: image/image.go:Alpha16.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(2))
    }

    // Go: image/image.go:Alpha16.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::alpha16_model(c) {
            Color::Alpha16(c1) => c1,
            _ => unreachable!(),
        };
        self.pix[i] = (c1.a >> 8) as u8;
        self.pix[i + 1] = c1.a as u8;
    }

    // Go: image/image.go:Alpha16.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = (c.a >> 8) as u8;
        self.pix[i + 1] = c.a as u8;
    }

    // Go: image/image.go:Alpha16.SetAlpha16
    pub fn set_alpha16(&mut self, x: i64, y: i64, c: color::Alpha16) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = (c.a >> 8) as u8;
        self.pix[i + 1] = c.a as u8;
    }

    // Go: image/image.go:Alpha16.Opaque
    pub fn opaque(&self) -> bool {
        if self.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (0i64, self.rect.dx() * 2);
        for _y in self.rect.min.y..self.rect.max.y {
            let mut i = i0;
            while i < i1 {
                if self.pix[i as usize] != 0xff || self.pix[i as usize + 1] != 0xff {
                    return false;
                }
                i += 2;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        true
    }
}
impl_sub_image!(Alpha16);
impl_image_traits!(Alpha16, draw);

// ---------------------------------------------------------------------------
// Gray
// ---------------------------------------------------------------------------

/// Gray is an in-memory image whose At method returns [`color::Gray`] values.
///
/// Go: image/image.go:Gray
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gray {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl Gray {
    // Go: image/image.go:NewGray
    pub fn new(r: Rectangle) -> Gray {
        Gray {
            pix: vec![0; pixel_buffer_length(1, r, "Gray")],
            stride: r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::Gray
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:Gray.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::Gray(self.gray_at(x, y))
    }

    // Go: image/image.go:Gray.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let mut gray = self.gray_at(x, y).y as u16;
        gray |= gray << 8;
        color::RGBA64 {
            r: gray,
            g: gray,
            b: gray,
            a: 0xffff,
        }
    }

    // Go: image/image.go:Gray.GrayAt
    pub fn gray_at(&self, x: i64, y: i64) -> color::Gray {
        if !(Point { x, y }.in_(self.rect)) {
            return color::Gray::default();
        }
        let i = self.pix_offset(x, y) as usize;
        color::Gray { y: self.pix[i] }
    }

    // Go: image/image.go:Gray.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x))
    }

    // Go: image/image.go:Gray.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = match color::gray_model(c) {
            Color::Gray(c1) => c1.y,
            _ => unreachable!(),
        };
    }

    // Go: image/image.go:Gray.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        // This formula is the same as in color.grayModel.
        let gray = (19595 * c.r as u32 + 38470 * c.g as u32 + 7471 * c.b as u32 + (1 << 15)) >> 24;
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = gray as u8;
    }

    // Go: image/image.go:Gray.SetGray
    pub fn set_gray(&mut self, x: i64, y: i64, c: color::Gray) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = c.y;
    }

    // Go: image/image.go:Gray.Opaque
    pub fn opaque(&self) -> bool {
        true
    }
}
impl_sub_image!(Gray);
impl_image_traits!(Gray, draw);

// ---------------------------------------------------------------------------
// Gray16
// ---------------------------------------------------------------------------

/// Gray16 is an in-memory image whose At method returns [`color::Gray16`]
/// values.
///
/// Go: image/image.go:Gray16
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gray16 {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl Gray16 {
    // Go: image/image.go:NewGray16
    pub fn new(r: Rectangle) -> Gray16 {
        Gray16 {
            pix: vec![0; pixel_buffer_length(2, r, "Gray16")],
            stride: 2 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::Gray16
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:Gray16.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::Gray16(self.gray16_at(x, y))
    }

    // Go: image/image.go:Gray16.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let gray = self.gray16_at(x, y).y;
        color::RGBA64 {
            r: gray,
            g: gray,
            b: gray,
            a: 0xffff,
        }
    }

    // Go: image/image.go:Gray16.Gray16At
    pub fn gray16_at(&self, x: i64, y: i64) -> color::Gray16 {
        if !(Point { x, y }.in_(self.rect)) {
            return color::Gray16::default();
        }
        let i = self.pix_offset(x, y) as usize;
        color::Gray16 {
            y: (self.pix[i] as u16) << 8 | self.pix[i + 1] as u16,
        }
    }

    // Go: image/image.go:Gray16.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(2))
    }

    // Go: image/image.go:Gray16.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::gray16_model(c) {
            Color::Gray16(c1) => c1,
            _ => unreachable!(),
        };
        self.pix[i] = (c1.y >> 8) as u8;
        self.pix[i + 1] = c1.y as u8;
    }

    // Go: image/image.go:Gray16.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        // This formula is the same as in color.gray16Model.
        let gray = (19595 * c.r as u32 + 38470 * c.g as u32 + 7471 * c.b as u32 + (1 << 15)) >> 16;
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = (gray >> 8) as u8;
        self.pix[i + 1] = gray as u8;
    }

    // Go: image/image.go:Gray16.SetGray16
    pub fn set_gray16(&mut self, x: i64, y: i64, c: color::Gray16) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = (c.y >> 8) as u8;
        self.pix[i + 1] = c.y as u8;
    }

    // Go: image/image.go:Gray16.Opaque
    pub fn opaque(&self) -> bool {
        true
    }
}
impl_sub_image!(Gray16);
impl_image_traits!(Gray16, draw);

// ---------------------------------------------------------------------------
// CMYK
// ---------------------------------------------------------------------------

/// CMYK is an in-memory image whose At method returns [`color::CMYK`] values.
///
/// Go: image/image.go:CMYK
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CMYK {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
}

impl CMYK {
    // Go: image/image.go:NewCMYK
    pub fn new(r: Rectangle) -> CMYK {
        CMYK {
            pix: vec![0; pixel_buffer_length(4, r, "CMYK")],
            stride: 4 * r.dx(),
            rect: r,
        }
    }

    pub fn color_model(&self) -> Model {
        Model::CMYK
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/image.go:CMYK.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::CMYK(self.cmyk_at(x, y))
    }

    // Go: image/image.go:CMYK.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.cmyk_at(x, y).rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/image.go:CMYK.CMYKAt
    pub fn cmyk_at(&self, x: i64, y: i64) -> color::CMYK {
        if !(Point { x, y }.in_(self.rect)) {
            return color::CMYK::default();
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &self.pix[i..i + 4];
        color::CMYK {
            c: s[0],
            m: s[1],
            y: s[2],
            k: s[3],
        }
    }

    // Go: image/image.go:CMYK.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x).wrapping_mul(4))
    }

    // Go: image/image.go:CMYK.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let c1 = match color::cmyk_model(c) {
            Color::CMYK(c1) => c1,
            _ => unreachable!(),
        };
        let s = &mut self.pix[i..i + 4];
        s[0] = c1.c;
        s[1] = c1.m;
        s[2] = c1.y;
        s[3] = c1.k;
    }

    // Go: image/image.go:CMYK.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let (cc, mm, yy, kk) =
            color::rgb_to_cmyk((c.r >> 8) as u8, (c.g >> 8) as u8, (c.b >> 8) as u8);
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = cc;
        s[1] = mm;
        s[2] = yy;
        s[3] = kk;
    }

    // Go: image/image.go:CMYK.SetCMYK
    pub fn set_cmyk(&mut self, x: i64, y: i64, c: color::CMYK) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        let s = &mut self.pix[i..i + 4];
        s[0] = c.c;
        s[1] = c.m;
        s[2] = c.y;
        s[3] = c.k;
    }

    // Go: image/image.go:CMYK.Opaque
    pub fn opaque(&self) -> bool {
        true
    }
}
impl_sub_image!(CMYK);
impl_image_traits!(CMYK, draw);

// ---------------------------------------------------------------------------
// Paletted
// ---------------------------------------------------------------------------

/// Paletted is an in-memory image of uint8 indices into a given palette.
///
/// Go: image/image.go:Paletted
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paletted {
    pub pix: Vec<u8>,
    pub stride: i64,
    pub rect: Rectangle,
    /// Palette is the image's palette.
    pub palette: Palette,
}

impl Paletted {
    /// NewPaletted returns a new [`Paletted`] image with the given width,
    /// height and palette.
    ///
    /// Go: image/image.go:NewPaletted
    pub fn new(r: Rectangle, p: Palette) -> Paletted {
        Paletted {
            pix: vec![0; pixel_buffer_length(1, r, "Paletted")],
            stride: r.dx(),
            rect: r,
            palette: p,
        }
    }

    // Go: image/image.go:Paletted.ColorModel
    pub fn color_model(&self) -> Model {
        Model::Palette(self.palette.clone())
    }

    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    /// Go: image/image.go:Paletted.At.
    ///
    /// Deviation: Go returns a nil Color for an empty palette; the port
    /// returns the zero `color.RGBA64` (what `RGBA64At` returns in that case).
    pub fn at(&self, x: i64, y: i64) -> Color {
        if self.palette.len() == 0 {
            return Color::RGBA64(color::RGBA64::default());
        }
        if !(Point { x, y }.in_(self.rect)) {
            return self.palette[0];
        }
        let i = self.pix_offset(x, y) as usize;
        self.palette[self.pix[i] as usize]
    }

    // Go: image/image.go:Paletted.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        if self.palette.len() == 0 {
            return color::RGBA64::default();
        }
        let c = if !(Point { x, y }.in_(self.rect)) {
            self.palette[0]
        } else {
            let i = self.pix_offset(x, y) as usize;
            self.palette[self.pix[i] as usize]
        };
        let (r, g, b, a) = c.rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/image.go:Paletted.PixOffset
    #[inline]
    pub fn pix_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x))
    }

    // Go: image/image.go:Paletted.Set
    pub fn set(&mut self, x: i64, y: i64, c: Color) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = self.palette.index(c) as u8;
    }

    // Go: image/image.go:Paletted.SetRGBA64
    pub fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = self.palette.index(Color::RGBA64(c)) as u8;
    }

    // Go: image/image.go:Paletted.ColorIndexAt
    pub fn color_index_at(&self, x: i64, y: i64) -> u8 {
        if !(Point { x, y }.in_(self.rect)) {
            return 0;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i]
    }

    // Go: image/image.go:Paletted.SetColorIndex
    pub fn set_color_index(&mut self, x: i64, y: i64, index: u8) {
        if !(Point { x, y }.in_(self.rect)) {
            return;
        }
        let i = self.pix_offset(x, y) as usize;
        self.pix[i] = index;
    }

    /// Go: image/image.go:Paletted.SubImage (owned copy, see PORTING.md).
    pub fn sub_image(&self, r: Rectangle) -> Paletted {
        let r = r.intersect(self.rect);
        // If r1 and r2 are Rectangles, r1.Intersect(r2) is not guaranteed to be inside
        // either r1 or r2 if the intersection is empty. Without explicitly checking for
        // this, the Pix[i:] expression below can panic.
        if r.empty() {
            return Paletted {
                palette: self.palette.clone(),
                ..Default::default()
            };
        }
        let i = self.pix_offset(r.min.x, r.min.y);
        Paletted {
            pix: tail(&self.pix, i),
            stride: self.stride,
            rect: self.rect.intersect(r),
            palette: self.palette.clone(),
        }
    }

    /// Consuming variant of [`Self::sub_image`].
    pub fn into_sub_image(self, r: Rectangle) -> Paletted {
        let r = r.intersect(self.rect);
        if r.empty() {
            return Paletted {
                palette: self.palette,
                ..Default::default()
            };
        }
        let i = self.pix_offset(r.min.x, r.min.y);
        let rect = self.rect.intersect(r);
        Paletted {
            pix: into_tail(self.pix, i),
            stride: self.stride,
            rect,
            palette: self.palette,
        }
    }

    /// Opaque scans the entire image and reports whether it is fully opaque.
    ///
    /// Go: image/image.go:Paletted.Opaque
    pub fn opaque(&self) -> bool {
        let mut present = [false; 256];
        let (mut i0, mut i1) = (0i64, self.rect.dx());
        for _y in self.rect.min.y..self.rect.max.y {
            for &c in &self.pix[i0 as usize..i1 as usize] {
                present[c as usize] = true;
            }
            i0 += self.stride;
            i1 += self.stride;
        }
        for (i, c) in self.palette.iter().enumerate() {
            // Go ranges over the whole palette and indexes present[i]; a
            // palette longer than 256 entries panics there.
            if !present[i] {
                continue;
            }
            let (_, _, _, a) = c.rgba();
            if a != 0xffff {
                return false;
            }
        }
        true
    }
}
impl_image_traits!(Paletted, draw, paletted);

impl PalettedImage for Paletted {
    fn color_index_at(&self, x: i64, y: i64) -> u8 {
        Paletted::color_index_at(self, x, y)
    }
}
