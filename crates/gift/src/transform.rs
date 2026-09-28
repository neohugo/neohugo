//! Port of gift v1.2.1 `transform.go`: 90-degree transforms and flips,
//! Rotate (with nearest, linear and cubic interpolation), Crop, CropToSize
//! and anchors.
//!
//! arm64 fusion (from `go tool objdump`, see PORTING.md): `float32(w)/2 - 0.5`
//! is `FNMSUBS` (w*0.5 - 0.5), `rotatePoint` fuses one product per
//! coordinate (which one differs between its inlined copies), and the
//! interpolation accumulations are FMADDS like resizeLine's.

use std::sync::Arc;

use go_image::color::Color;
use go_image::{Image, Point, Rectangle, draw, pt, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::pixels::{Pixel, PixelGetter, PixelSetter, pixel_from_color};
use crate::utils::{add_sub, add_sub_1, floorf32, maxf32, minf32, parallelize, sincosf32};

/// Go: transform.go:transformType
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformType {
    Rotate90 = 0,
    Rotate180 = 1,
    Rotate270 = 2,
    FlipHorizontal = 3,
    FlipVertical = 4,
    Transpose = 5,
    Transverse = 6,
}

/// Go: transform.go:transformFilter
#[derive(Clone, Copy, Debug)]
pub struct TransformFilter {
    pub tt: TransformType,
}

impl Filter for TransformFilter {
    // Go: transform.go:(*transformFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        if self.tt == TransformType::Rotate90
            || self.tt == TransformType::Rotate270
            || self.tt == TransformType::Transpose
            || self.tt == TransformType::Transverse
        {
            rect(0, 0, src_bounds.dy(), src_bounds.dx())
        } else {
            rect(0, 0, src_bounds.dx(), src_bounds.dy())
        }
    }

    // Go: transform.go:(*transformFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                for srcy in start..stop {
                    for srcx in srcb.min.x..srcb.max.x {
                        let (dstx, dsty) = match self.tt {
                            TransformType::Rotate90 => (
                                add_sub(dstb.min.x, srcy, srcb.min.y),
                                add_sub_1(dstb.min.y, srcb.max.x, srcx),
                            ),
                            TransformType::Rotate180 => (
                                add_sub_1(dstb.min.x, srcb.max.x, srcx),
                                add_sub_1(dstb.min.y, srcb.max.y, srcy),
                            ),
                            TransformType::Rotate270 => (
                                add_sub_1(dstb.min.x, srcb.max.y, srcy),
                                add_sub(dstb.min.y, srcx, srcb.min.x),
                            ),
                            TransformType::FlipHorizontal => (
                                add_sub_1(dstb.min.x, srcb.max.x, srcx),
                                add_sub(dstb.min.y, srcy, srcb.min.y),
                            ),
                            TransformType::FlipVertical => (
                                add_sub(dstb.min.x, srcx, srcb.min.x),
                                add_sub_1(dstb.min.y, srcb.max.y, srcy),
                            ),
                            TransformType::Transpose => (
                                add_sub(dstb.min.x, srcy, srcb.min.y),
                                add_sub(dstb.min.y, srcx, srcb.min.x),
                            ),
                            TransformType::Transverse => (
                                add_sub_1(dstb.min.y, srcb.max.y, srcy),
                                add_sub_1(dstb.min.x, srcb.max.x, srcx),
                            ),
                        };
                        pix_setter.set_pixel(dstx, dsty, pix_getter.get_pixel(srcx, srcy));
                    }
                }
            },
        );
    }
}

fn transform(tt: TransformType) -> Arc<dyn Filter> {
    Arc::new(TransformFilter { tt })
}

/// Rotate90 creates a filter that rotates an image 90 degrees
/// counter-clockwise.
///
/// Go: transform.go:Rotate90
pub fn rotate90() -> Arc<dyn Filter> {
    transform(TransformType::Rotate90)
}

/// Go: transform.go:Rotate180
pub fn rotate180() -> Arc<dyn Filter> {
    transform(TransformType::Rotate180)
}

/// Go: transform.go:Rotate270
pub fn rotate270() -> Arc<dyn Filter> {
    transform(TransformType::Rotate270)
}

/// Go: transform.go:FlipHorizontal
pub fn flip_horizontal() -> Arc<dyn Filter> {
    transform(TransformType::FlipHorizontal)
}

/// Go: transform.go:FlipVertical
pub fn flip_vertical() -> Arc<dyn Filter> {
    transform(TransformType::FlipVertical)
}

/// Go: transform.go:Transpose
pub fn transpose() -> Arc<dyn Filter> {
    transform(TransformType::Transpose)
}

/// Go: transform.go:Transverse
pub fn transverse() -> Arc<dyn Filter> {
    transform(TransformType::Transverse)
}

/// Interpolation is an interpolation algorithm used for image transformation.
///
/// Go: transform.go:Interpolation (an int; values other than 1 and 2 behave
/// like NearestNeighborInterpolation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interpolation(pub i64);

/// Go: transform.go:NearestNeighborInterpolation
pub const NEAREST_NEIGHBOR_INTERPOLATION: Interpolation = Interpolation(0);
/// Go: transform.go:LinearInterpolation
pub const LINEAR_INTERPOLATION: Interpolation = Interpolation(1);
/// Go: transform.go:CubicInterpolation
pub const CUBIC_INTERPOLATION: Interpolation = Interpolation(2);

/// Go: transform.go:calcRotatedSize. `rotatePoint` is inlined four times;
/// each copy fuses a different product (from the objdump).
pub(crate) fn calc_rotated_size(w: i64, h: i64, angle: f32) -> (i64, i64) {
    if w <= 0 || h <= 0 {
        return (0, 0);
    }

    // float32(w)/2 - 0.5: FNMSUBS (w*0.5 - 0.5).
    let xoff = (w as f32).mul_add(0.5, -0.5);
    let yoff = (h as f32).mul_add(0.5, -0.5);

    let (asin, acos) = sincosf32(angle);
    let x0 = 0.0 - xoff;
    let y0 = 0.0 - yoff;
    let x1 = (w - 1) as f32 - xoff;
    let y1 = (h - 1) as f32 - yoff;
    // rotatePoint(x, y): newx = x*acos - y*asin; newy = x*asin + y*acos.
    let (px1, py1) = ((-y0).mul_add(asin, x0 * acos), x0.mul_add(asin, y0 * acos));
    let (px2, py2) = ((-y0).mul_add(asin, x1 * acos), x1.mul_add(asin, y0 * acos));
    let (px3, py3) = ((-y1).mul_add(asin, x1 * acos), y1.mul_add(acos, x1 * asin));
    let (px4, py4) = ((-y1).mul_add(asin, x0 * acos), y1.mul_add(acos, x0 * asin));

    let minx = minf32(px1, minf32(px2, minf32(px3, px4)));
    let maxx = maxf32(px1, maxf32(px2, maxf32(px3, px4)));
    let miny = minf32(py1, minf32(py2, minf32(py3, py4)));
    let maxy = maxf32(py1, maxf32(py2, maxf32(py3, py4)));

    let mut neww = maxx - minx + 1.0;
    if neww - floorf32(neww) > 0.01 {
        neww += 2.0;
    }
    let mut newh = maxy - miny + 1.0;
    if newh - floorf32(newh) > 0.01 {
        newh += 2.0;
    }
    (neww as i64, newh as i64)
}

/// Go: transform.go:rotateFilter
#[derive(Clone, Copy, Debug)]
pub struct RotateFilter {
    pub angle: f32,
    pub bgcolor: Color,
    pub interpolation: Interpolation,
}

impl Filter for RotateFilter {
    // Go: transform.go:(*rotateFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let (w, h) = calc_rotated_size(src_bounds.dx(), src_bounds.dy(), self.angle);
        rect(0, 0, w, h)
    }

    // Go: transform.go:(*rotateFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        let (w, h) = calc_rotated_size(srcb.dx(), srcb.dy(), self.angle);
        if w <= 0 || h <= 0 {
            return;
        }

        let srcxoff = (srcb.dx() as f32).mul_add(0.5, -0.5);
        let srcyoff = (srcb.dy() as f32).mul_add(0.5, -0.5);
        let dstxoff = (w as f32).mul_add(0.5, -0.5);
        let dstyoff = (h as f32).mul_add(0.5, -0.5);

        let bgpx = pixel_from_color(self.bgcolor);
        let (asin, acos) = sincosf32(self.angle);

        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        parallelize(options.parallelization, 0, h, |start, stop| {
            for y in start..stop {
                for x in 0..w {
                    let xx = x as f32 - dstxoff;
                    let yy = y as f32 - dstyoff;
                    // rotatePoint inlined: newx fuses y*asin, newy fuses y*acos.
                    let xf = (-yy).mul_add(asin, xx * acos);
                    let yf = yy.mul_add(acos, xx * asin);
                    let xf = srcb.min.x as f32 + xf + srcxoff;
                    let yf = srcb.min.y as f32 + yf + srcyoff;

                    let px = match self.interpolation {
                        CUBIC_INTERPOLATION => interpolate_cubic(xf, yf, srcb, &pix_getter, bgpx),
                        LINEAR_INTERPOLATION => interpolate_linear(xf, yf, srcb, &pix_getter, bgpx),
                        _ => interpolate_nearest(xf, yf, srcb, &pix_getter, bgpx),
                    };

                    pix_setter.set_pixel(
                        dstb.min.x.wrapping_add(x),
                        dstb.min.y.wrapping_add(y),
                        px,
                    );
                }
            }
        });
    }
}

/// Go: transform.go:interpolateCubic
fn interpolate_cubic(
    xf: f32,
    yf: f32,
    bounds: Rectangle,
    pix_getter: &PixelGetter<'_>,
    bgpx: Pixel,
) -> Pixel {
    let mut pxs = [Pixel::default(); 16];
    let mut cfs = [0f32; 16];
    let mut px = Pixel::default();

    let (x0, y0) = (floorf32(xf) as i64, floorf32(yf) as i64);
    if !pt(x0, y0).in_(rect(
        bounds.min.x.wrapping_sub(1),
        bounds.min.y.wrapping_sub(1),
        bounds.max.x,
        bounds.max.y,
    )) {
        return bgpx;
    }
    let (xq, yq) = (xf - x0 as f32, yf - y0 as f32);

    for i in 0..4 {
        for j in 0..4 {
            let p = pt(x0.wrapping_add(j - 1), y0.wrapping_add(i - 1));
            if p.in_(bounds) {
                pxs[(i * 4 + j) as usize] = pix_getter.get_pixel(p.x, p.y);
            } else {
                pxs[(i * 4 + j) as usize] = bgpx;
            }
        }
    }

    const K04: f32 = 1.0 / 4.0;
    const K12: f32 = f32::from_bits(0x3daaaaab); // 1 / 12.0
    const K36: f32 = f32::from_bits(0x3ce38e39); // 1 / 36.0

    cfs[0] = K36 * xq * yq * (xq - 1.0) * (xq - 2.0) * (yq - 1.0) * (yq - 2.0);
    cfs[1] = -K12 * yq * (xq - 1.0) * (xq - 2.0) * (xq + 1.0) * (yq - 1.0) * (yq - 2.0);
    cfs[2] = K12 * xq * yq * (xq + 1.0) * (xq - 2.0) * (yq - 1.0) * (yq - 2.0);
    cfs[3] = -K36 * xq * yq * (xq - 1.0) * (xq + 1.0) * (yq - 1.0) * (yq - 2.0);
    cfs[4] = -K12 * xq * (xq - 1.0) * (xq - 2.0) * (yq - 1.0) * (yq - 2.0) * (yq + 1.0);
    cfs[5] = K04 * (xq - 1.0) * (xq - 2.0) * (xq + 1.0) * (yq - 1.0) * (yq - 2.0) * (yq + 1.0);
    cfs[6] = -K04 * xq * (xq + 1.0) * (xq - 2.0) * (yq - 1.0) * (yq - 2.0) * (yq + 1.0);
    cfs[7] = K12 * xq * (xq - 1.0) * (xq + 1.0) * (yq - 1.0) * (yq - 2.0) * (yq + 1.0);
    cfs[8] = K12 * xq * yq * (xq - 1.0) * (xq - 2.0) * (yq + 1.0) * (yq - 2.0);
    cfs[9] = -K04 * yq * (xq - 1.0) * (xq - 2.0) * (xq + 1.0) * (yq + 1.0) * (yq - 2.0);
    cfs[10] = K04 * xq * yq * (xq + 1.0) * (xq - 2.0) * (yq + 1.0) * (yq - 2.0);
    cfs[11] = -K12 * xq * yq * (xq - 1.0) * (xq + 1.0) * (yq + 1.0) * (yq - 2.0);
    cfs[12] = -K36 * xq * yq * (xq - 1.0) * (xq - 2.0) * (yq - 1.0) * (yq + 1.0);
    cfs[13] = K12 * yq * (xq - 1.0) * (xq - 2.0) * (xq + 1.0) * (yq - 1.0) * (yq + 1.0);
    cfs[14] = -K12 * xq * yq * (xq + 1.0) * (xq - 2.0) * (yq - 1.0) * (yq + 1.0);
    cfs[15] = K36 * xq * yq * (xq - 1.0) * (xq + 1.0) * (yq - 1.0) * (yq + 1.0);

    for i in 0..pxs.len() {
        let wa = pxs[i].a * cfs[i];
        px.r = pxs[i].r.mul_add(wa, px.r);
        px.g = pxs[i].g.mul_add(wa, px.g);
        px.b = pxs[i].b.mul_add(wa, px.b);
        px.a = pxs[i].a.mul_add(cfs[i], px.a);
    }

    if px.a != 0.0 {
        px.r /= px.a;
        px.g /= px.a;
        px.b /= px.a;
    }

    px
}

/// Go: transform.go:interpolateLinear
fn interpolate_linear(
    xf: f32,
    yf: f32,
    bounds: Rectangle,
    pix_getter: &PixelGetter<'_>,
    bgpx: Pixel,
) -> Pixel {
    let mut pxs = [Pixel::default(); 4];
    let mut cfs = [0f32; 4];
    let mut px = Pixel::default();

    let (x0, y0) = (floorf32(xf) as i64, floorf32(yf) as i64);
    if !pt(x0, y0).in_(rect(
        bounds.min.x.wrapping_sub(1),
        bounds.min.y.wrapping_sub(1),
        bounds.max.x,
        bounds.max.y,
    )) {
        return bgpx;
    }
    let (xq, yq) = (xf - x0 as f32, yf - y0 as f32);

    for i in 0..2 {
        for j in 0..2 {
            let p = pt(x0.wrapping_add(j), y0.wrapping_add(i));
            if p.in_(bounds) {
                pxs[(i * 2 + j) as usize] = pix_getter.get_pixel(p.x, p.y);
            } else {
                pxs[(i * 2 + j) as usize] = bgpx;
            }
        }
    }

    cfs[0] = (1.0 - xq) * (1.0 - yq);
    cfs[1] = xq * (1.0 - yq);
    cfs[2] = (1.0 - xq) * yq;
    cfs[3] = xq * yq;

    for i in 0..pxs.len() {
        let wa = pxs[i].a * cfs[i];
        px.r = pxs[i].r.mul_add(wa, px.r);
        px.g = pxs[i].g.mul_add(wa, px.g);
        px.b = pxs[i].b.mul_add(wa, px.b);
        px.a = pxs[i].a.mul_add(cfs[i], px.a);
    }

    if px.a != 0.0 {
        px.r /= px.a;
        px.g /= px.a;
        px.b /= px.a;
    }

    px
}

/// Go: transform.go:interpolateNearest
fn interpolate_nearest(
    xf: f32,
    yf: f32,
    bounds: Rectangle,
    pix_getter: &PixelGetter<'_>,
    bgpx: Pixel,
) -> Pixel {
    let (x0, y0) = (floorf32(xf + 0.5) as i64, floorf32(yf + 0.5) as i64);
    if pt(x0, y0).in_(bounds) {
        return pix_getter.get_pixel(x0, y0);
    }
    bgpx
}

/// Rotate creates a filter that rotates an image by the given angle
/// counter-clockwise. The angle parameter is the rotation angle in degrees.
///
/// Go: transform.go:Rotate
pub fn rotate(
    angle: f32,
    background_color: Color,
    interpolation: Interpolation,
) -> Arc<dyn Filter> {
    Arc::new(RotateFilter {
        angle,
        bgcolor: background_color,
        interpolation,
    })
}

/// Go: transform.go:cropFilter
#[derive(Clone, Copy, Debug)]
pub struct CropFilter {
    pub rect: Rectangle,
}

impl Filter for CropFilter {
    // Go: transform.go:(*cropFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let b = src_bounds.intersect(self.rect);
        b.sub(b.min)
    }

    // Go: transform.go:(*cropFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds().intersect(self.rect);
        let dstb = dst.bounds();
        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                for srcy in start..stop {
                    for srcx in srcb.min.x..srcb.max.x {
                        let dstx = add_sub(dstb.min.x, srcx, srcb.min.x);
                        let dsty = add_sub(dstb.min.y, srcy, srcb.min.y);
                        pix_setter.set_pixel(dstx, dsty, pix_getter.get_pixel(srcx, srcy));
                    }
                }
            },
        );
    }
}

/// Crop creates a filter that crops the specified rectangular region from an
/// image.
///
/// Go: transform.go:Crop
pub fn crop(r: Rectangle) -> Arc<dyn Filter> {
    Arc::new(CropFilter { rect: r })
}

/// Anchor is the anchor point for image cropping.
///
/// Go: transform.go:Anchor (an int; unknown values behave like CenterAnchor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Anchor(pub i64);

/// Go: transform.go:CenterAnchor
pub const CENTER_ANCHOR: Anchor = Anchor(0);
/// Go: transform.go:TopLeftAnchor
pub const TOP_LEFT_ANCHOR: Anchor = Anchor(1);
/// Go: transform.go:TopAnchor
pub const TOP_ANCHOR: Anchor = Anchor(2);
/// Go: transform.go:TopRightAnchor
pub const TOP_RIGHT_ANCHOR: Anchor = Anchor(3);
/// Go: transform.go:LeftAnchor
pub const LEFT_ANCHOR: Anchor = Anchor(4);
/// Go: transform.go:RightAnchor
pub const RIGHT_ANCHOR: Anchor = Anchor(5);
/// Go: transform.go:BottomLeftAnchor
pub const BOTTOM_LEFT_ANCHOR: Anchor = Anchor(6);
/// Go: transform.go:BottomAnchor
pub const BOTTOM_ANCHOR: Anchor = Anchor(7);
/// Go: transform.go:BottomRightAnchor
pub const BOTTOM_RIGHT_ANCHOR: Anchor = Anchor(8);

/// Go: transform.go:anchorPt. Go's `int` arithmetic wraps (CropToSize with a
/// huge width or height, or bounds whose Dx overflows), so every operation
/// is `wrapping_*` (a plain `-` panics in builds with overflow checks).
pub(crate) fn anchor_pt(b: Rectangle, w: i64, h: i64, anchor: Anchor) -> Point {
    // b.Min.X + (b.Dx()-w)/2 (b.Dx() itself wraps in go-image).
    let mid_x = || b.min.x.wrapping_add(b.dx().wrapping_sub(w) / 2);
    let mid_y = || b.min.y.wrapping_add(b.dy().wrapping_sub(h) / 2);
    let (x, y) = match anchor {
        TOP_LEFT_ANCHOR => (b.min.x, b.min.y),
        TOP_ANCHOR => (mid_x(), b.min.y),
        TOP_RIGHT_ANCHOR => (b.max.x.wrapping_sub(w), b.min.y),
        LEFT_ANCHOR => (b.min.x, mid_y()),
        RIGHT_ANCHOR => (b.max.x.wrapping_sub(w), mid_y()),
        BOTTOM_LEFT_ANCHOR => (b.min.x, b.max.y.wrapping_sub(h)),
        BOTTOM_ANCHOR => (mid_x(), b.max.y.wrapping_sub(h)),
        BOTTOM_RIGHT_ANCHOR => (b.max.x.wrapping_sub(w), b.max.y.wrapping_sub(h)),
        _ => (mid_x(), mid_y()),
    };
    pt(x, y)
}

/// Go: transform.go:cropToSizeFilter
#[derive(Clone, Copy, Debug)]
pub struct CropToSizeFilter {
    pub w: i64,
    pub h: i64,
    pub anchor: Anchor,
}

impl Filter for CropToSizeFilter {
    // Go: transform.go:(*cropToSizeFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        if self.w <= 0 || self.h <= 0 {
            return rect(0, 0, 0, 0);
        }
        let p = anchor_pt(src_bounds, self.w, self.h, self.anchor);
        let r = rect(0, 0, self.w, self.h).add(p);
        let b = src_bounds.intersect(r);
        b.sub(b.min)
    }

    // Go: transform.go:(*cropToSizeFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        if self.w <= 0 || self.h <= 0 {
            return;
        }
        let p = anchor_pt(src.bounds(), self.w, self.h, self.anchor);
        let r = rect(0, 0, self.w, self.h).add(p);
        let b = src.bounds().intersect(r);
        CropFilter { rect: b }.draw(dst, src, options);
    }
}

/// CropToSize creates a filter that crops an image to the specified size
/// using the specified anchor point.
///
/// Go: transform.go:CropToSize
pub fn crop_to_size(width: i64, height: i64, anchor: Anchor) -> Arc<dyn Filter> {
    Arc::new(CropToSizeFilter {
        w: width,
        h: height,
        anchor,
    })
}
