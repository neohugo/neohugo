//! Port of Go 1.27.1 `image/ycbcr.go`.

use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::color::{self, Color, Model};
use crate::geom::{Point, Rectangle, add2_non_neg, mul3_non_neg};
use crate::image::{impl_image_traits, into_tail, tail};

/// YCbCrSubsampleRatio is the chroma subsample ratio used in a YCbCr image.
///
/// Go: image/ycbcr.go:YCbCrSubsampleRatio (an `int`; the discriminants are the
/// Go constant values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum YCbCrSubsampleRatio {
    #[default]
    Ratio444 = 0,
    Ratio422 = 1,
    Ratio420 = 2,
    Ratio440 = 3,
    Ratio411 = 4,
    Ratio410 = 5,
}

impl fmt::Display for YCbCrSubsampleRatio {
    // Go: image/ycbcr.go:YCbCrSubsampleRatio.String
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            YCbCrSubsampleRatio::Ratio444 => "YCbCrSubsampleRatio444",
            YCbCrSubsampleRatio::Ratio422 => "YCbCrSubsampleRatio422",
            YCbCrSubsampleRatio::Ratio420 => "YCbCrSubsampleRatio420",
            YCbCrSubsampleRatio::Ratio440 => "YCbCrSubsampleRatio440",
            YCbCrSubsampleRatio::Ratio411 => "YCbCrSubsampleRatio411",
            YCbCrSubsampleRatio::Ratio410 => "YCbCrSubsampleRatio410",
        })
    }
}

/// YCbCr is an in-memory image of Y'CbCr colors. There is one Y sample per
/// pixel, but each Cb and Cr sample can span one or more pixels.
///
/// Go: image/ycbcr.go:YCbCr. Go allocates Y, Cb and Cr as sub-slices of one
/// buffer; the port uses three vectors (no observable difference).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YCbCr {
    pub y: Vec<u8>,
    pub cb: Vec<u8>,
    pub cr: Vec<u8>,
    pub y_stride: i64,
    pub c_stride: i64,
    pub subsample_ratio: YCbCrSubsampleRatio,
    pub rect: Rectangle,
}

impl YCbCr {
    // Go: image/ycbcr.go:YCbCr.ColorModel
    pub fn color_model(&self) -> Model {
        Model::YCbCr
    }

    // Go: image/ycbcr.go:YCbCr.Bounds
    pub fn bounds(&self) -> Rectangle {
        self.rect
    }

    // Go: image/ycbcr.go:YCbCr.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::YCbCr(self.ycbcr_at(x, y))
    }

    // Go: image/ycbcr.go:YCbCr.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.ycbcr_at(x, y).rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/ycbcr.go:YCbCr.YCbCrAt
    pub fn ycbcr_at(&self, x: i64, y: i64) -> color::YCbCr {
        if !(Point { x, y }.in_(self.rect)) {
            return color::YCbCr::default();
        }
        let yi = self.y_offset(x, y) as usize;
        let ci = self.c_offset(x, y) as usize;
        color::YCbCr {
            y: self.y[yi],
            cb: self.cb[ci],
            cr: self.cr[ci],
        }
    }

    /// YOffset returns the index of the first element of Y that corresponds to
    /// the pixel at (x, y).
    ///
    /// Go: image/ycbcr.go:YCbCr.YOffset
    #[inline]
    pub fn y_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.rect.min.y)
            .wrapping_mul(self.y_stride)
            .wrapping_add(x.wrapping_sub(self.rect.min.x))
    }

    /// COffset returns the index of the first element of Cb or Cr that
    /// corresponds to the pixel at (x, y).
    ///
    /// Go: image/ycbcr.go:YCbCr.COffset
    #[inline]
    pub fn c_offset(&self, x: i64, y: i64) -> i64 {
        let r = self.rect;
        match self.subsample_ratio {
            YCbCrSubsampleRatio::Ratio422 => (y - r.min.y) * self.c_stride + (x / 2 - r.min.x / 2),
            YCbCrSubsampleRatio::Ratio420 => {
                (y / 2 - r.min.y / 2) * self.c_stride + (x / 2 - r.min.x / 2)
            }
            YCbCrSubsampleRatio::Ratio440 => (y / 2 - r.min.y / 2) * self.c_stride + (x - r.min.x),
            YCbCrSubsampleRatio::Ratio411 => (y - r.min.y) * self.c_stride + (x / 4 - r.min.x / 4),
            YCbCrSubsampleRatio::Ratio410 => {
                (y / 2 - r.min.y / 2) * self.c_stride + (x / 4 - r.min.x / 4)
            }
            // Default to 4:4:4 subsampling.
            YCbCrSubsampleRatio::Ratio444 => (y - r.min.y) * self.c_stride + (x - r.min.x),
        }
    }

    /// SubImage returns an image representing the portion of the image p
    /// visible through r (owned copy of Go's `Y[yi:]`, `Cb[ci:]`, `Cr[ci:]`).
    ///
    /// Go: image/ycbcr.go:YCbCr.SubImage
    pub fn sub_image(&self, r: Rectangle) -> YCbCr {
        let r = r.intersect(self.rect);
        // If r1 and r2 are Rectangles, r1.Intersect(r2) is not guaranteed to be inside
        // either r1 or r2 if the intersection is empty. Without explicitly checking for
        // this, the Pix[i:] expression below can panic.
        if r.empty() {
            return YCbCr {
                subsample_ratio: self.subsample_ratio,
                ..Default::default()
            };
        }
        let yi = self.y_offset(r.min.x, r.min.y);
        let ci = self.c_offset(r.min.x, r.min.y);
        YCbCr {
            y: tail(&self.y, yi),
            cb: tail(&self.cb, ci),
            cr: tail(&self.cr, ci),
            subsample_ratio: self.subsample_ratio,
            y_stride: self.y_stride,
            c_stride: self.c_stride,
            rect: r,
        }
    }

    /// Consuming variant of [`Self::sub_image`] (no copy when r.Min == Rect.Min).
    pub fn into_sub_image(self, r: Rectangle) -> YCbCr {
        let r = r.intersect(self.rect);
        if r.empty() {
            return YCbCr {
                subsample_ratio: self.subsample_ratio,
                ..Default::default()
            };
        }
        let yi = self.y_offset(r.min.x, r.min.y);
        let ci = self.c_offset(r.min.x, r.min.y);
        YCbCr {
            y: into_tail(self.y, yi),
            cb: into_tail(self.cb, ci),
            cr: into_tail(self.cr, ci),
            subsample_ratio: self.subsample_ratio,
            y_stride: self.y_stride,
            c_stride: self.c_stride,
            rect: r,
        }
    }

    // Go: image/ycbcr.go:YCbCr.Opaque
    pub fn opaque(&self) -> bool {
        true
    }

    /// NewYCbCr returns a new YCbCr image with the given bounds and subsample
    /// ratio.
    ///
    /// Go: image/ycbcr.go:NewYCbCr
    pub fn new(r: Rectangle, subsample_ratio: YCbCrSubsampleRatio) -> YCbCr {
        let (w, h, cw, ch) = ycbcr_size(r, subsample_ratio);

        // totalLength should be the same as i2, below, for a valid Rectangle r.
        let total_length = add2_non_neg(mul3_non_neg(1, w, h), mul3_non_neg(2, cw, ch));
        if total_length < 0 {
            panic!("image: NewYCbCr Rectangle has huge or negative dimensions");
        }

        let i0 = w * h;
        let c = cw * ch;
        YCbCr {
            y: vec![0; i0 as usize],
            cb: vec![0; c as usize],
            cr: vec![0; c as usize],
            subsample_ratio,
            y_stride: w,
            c_stride: cw,
            rect: r,
        }
    }
}
impl_image_traits!(YCbCr);

// Go: image/ycbcr.go:yCbCrSize
fn ycbcr_size(r: Rectangle, subsample_ratio: YCbCrSubsampleRatio) -> (i64, i64, i64, i64) {
    let (w, h) = (r.dx(), r.dy());
    let (cw, ch);
    match subsample_ratio {
        YCbCrSubsampleRatio::Ratio422 => {
            cw = (r.max.x + 1) / 2 - r.min.x / 2;
            ch = h;
        }
        YCbCrSubsampleRatio::Ratio420 => {
            cw = (r.max.x + 1) / 2 - r.min.x / 2;
            ch = (r.max.y + 1) / 2 - r.min.y / 2;
        }
        YCbCrSubsampleRatio::Ratio440 => {
            cw = w;
            ch = (r.max.y + 1) / 2 - r.min.y / 2;
        }
        YCbCrSubsampleRatio::Ratio411 => {
            cw = (r.max.x + 3) / 4 - r.min.x / 4;
            ch = h;
        }
        YCbCrSubsampleRatio::Ratio410 => {
            cw = (r.max.x + 3) / 4 - r.min.x / 4;
            ch = (r.max.y + 1) / 2 - r.min.y / 2;
        }
        YCbCrSubsampleRatio::Ratio444 => {
            // Default to 4:4:4 subsampling.
            cw = w;
            ch = h;
        }
    }
    (w, h, cw, ch)
}

/// NYCbCrA is an in-memory image of non-alpha-premultiplied Y'CbCr-with-alpha
/// colors. A and AStride are analogous to the Y and YStride fields of the
/// embedded YCbCr.
///
/// Go: image/ycbcr.go:NYCbCrA. The embedded YCbCr is the `ycbcr` field and is
/// also reachable through `Deref` (Go's field/method promotion).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NYCbCrA {
    pub ycbcr: YCbCr,
    pub a: Vec<u8>,
    pub a_stride: i64,
}

impl Deref for NYCbCrA {
    type Target = YCbCr;
    fn deref(&self) -> &YCbCr {
        &self.ycbcr
    }
}

impl DerefMut for NYCbCrA {
    fn deref_mut(&mut self) -> &mut YCbCr {
        &mut self.ycbcr
    }
}

impl NYCbCrA {
    // Go: image/ycbcr.go:NYCbCrA.ColorModel
    pub fn color_model(&self) -> Model {
        Model::NYCbCrA
    }

    // Go: promoted YCbCr.Bounds
    pub fn bounds(&self) -> Rectangle {
        self.ycbcr.rect
    }

    // Go: image/ycbcr.go:NYCbCrA.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        Color::NYCbCrA(self.nycbcra_at(x, y))
    }

    // Go: image/ycbcr.go:NYCbCrA.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.nycbcra_at(x, y).rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    // Go: image/ycbcr.go:NYCbCrA.NYCbCrAAt
    pub fn nycbcra_at(&self, x: i64, y: i64) -> color::NYCbCrA {
        if !(Point { x, y }.in_(self.ycbcr.rect)) {
            return color::NYCbCrA::default();
        }
        let yi = self.y_offset(x, y) as usize;
        let ci = self.c_offset(x, y) as usize;
        let ai = self.a_offset(x, y) as usize;
        color::NYCbCrA {
            ycbcr: color::YCbCr {
                y: self.ycbcr.y[yi],
                cb: self.ycbcr.cb[ci],
                cr: self.ycbcr.cr[ci],
            },
            a: self.a[ai],
        }
    }

    /// AOffset returns the index of the first element of A that corresponds to
    /// the pixel at (x, y).
    ///
    /// Go: image/ycbcr.go:NYCbCrA.AOffset
    #[inline]
    pub fn a_offset(&self, x: i64, y: i64) -> i64 {
        y.wrapping_sub(self.ycbcr.rect.min.y)
            .wrapping_mul(self.a_stride)
            .wrapping_add(x.wrapping_sub(self.ycbcr.rect.min.x))
    }

    /// Go: image/ycbcr.go:NYCbCrA.SubImage (owned copy, see PORTING.md).
    pub fn sub_image(&self, r: Rectangle) -> NYCbCrA {
        let r = r.intersect(self.ycbcr.rect);
        // If r1 and r2 are Rectangles, r1.Intersect(r2) is not guaranteed to be inside
        // either r1 or r2 if the intersection is empty. Without explicitly checking for
        // this, the Pix[i:] expression below can panic.
        if r.empty() {
            return NYCbCrA {
                ycbcr: YCbCr {
                    subsample_ratio: self.ycbcr.subsample_ratio,
                    ..Default::default()
                },
                ..Default::default()
            };
        }
        let yi = self.y_offset(r.min.x, r.min.y);
        let ci = self.c_offset(r.min.x, r.min.y);
        let ai = self.a_offset(r.min.x, r.min.y);
        NYCbCrA {
            ycbcr: YCbCr {
                y: tail(&self.ycbcr.y, yi),
                cb: tail(&self.ycbcr.cb, ci),
                cr: tail(&self.ycbcr.cr, ci),
                subsample_ratio: self.ycbcr.subsample_ratio,
                y_stride: self.ycbcr.y_stride,
                c_stride: self.ycbcr.c_stride,
                rect: r,
            },
            a: tail(&self.a, ai),
            a_stride: self.a_stride,
        }
    }

    /// Opaque scans the entire image and reports whether it is fully opaque.
    ///
    /// Go: image/ycbcr.go:NYCbCrA.Opaque
    pub fn opaque(&self) -> bool {
        if self.ycbcr.rect.empty() {
            return true;
        }
        let (mut i0, mut i1) = (0i64, self.ycbcr.rect.dx());
        for _y in self.ycbcr.rect.min.y..self.ycbcr.rect.max.y {
            for &a in &self.a[i0 as usize..i1 as usize] {
                if a != 0xff {
                    return false;
                }
            }
            i0 += self.a_stride;
            i1 += self.a_stride;
        }
        true
    }

    /// NewNYCbCrA returns a new [`NYCbCrA`] image with the given bounds and
    /// subsample ratio.
    ///
    /// Go: image/ycbcr.go:NewNYCbCrA
    pub fn new(r: Rectangle, subsample_ratio: YCbCrSubsampleRatio) -> NYCbCrA {
        let (w, h, cw, ch) = ycbcr_size(r, subsample_ratio);

        // totalLength should be the same as i3, below, for a valid Rectangle r.
        let total_length = add2_non_neg(mul3_non_neg(2, w, h), mul3_non_neg(2, cw, ch));
        if total_length < 0 {
            panic!("image: NewNYCbCrA Rectangle has huge or negative dimension");
        }

        let i0 = w * h;
        let c = cw * ch;
        NYCbCrA {
            ycbcr: YCbCr {
                y: vec![0; i0 as usize],
                cb: vec![0; c as usize],
                cr: vec![0; c as usize],
                subsample_ratio,
                y_stride: w,
                c_stride: cw,
                rect: r,
            },
            a: vec![0; i0 as usize],
            a_stride: w,
        }
    }
}
impl_image_traits!(NYCbCrA);
