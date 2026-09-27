//! Port of Go 1.27.1 `image/draw/draw.go`.
//!
//! All fast paths of `DrawMask` are ported with the same dispatch order as
//! the Go type switches, followed by the `FALLBACK1.17` (RGBA64Image) and
//! `FALLBACK1.0` (Image) generic paths, and `drawPaletted` with optional
//! Floyd-Steinberg error diffusion. Unsigned arithmetic wraps exactly as Go's
//! `uint32`/`uint16` arithmetic does.
//!
//! Deviation: Go's `processBackward` (and the `dst == src` checks in the
//! RGBA mask paths) handle a source that *is* the destination. In safe Rust
//! `dst: &mut dyn Image` and `src: &dyn Image` can never alias, so those
//! checks are always false; drawing an image onto itself must be done by
//! cloning the source first (which yields the same pixels Go produces).

use std::any::Any;

use crate::color::{self, Color, Palette};
use crate::geom::{Point, Rectangle};
use crate::image::{Alpha, CMYK, Gray, NRGBA, NRGBA64, Paletted, RGBA};
use crate::imageutil;
use crate::names::Uniform;
use crate::ycbcr::YCbCr;

// m is the maximum color value returned by image.Color.RGBA.
const M: u32 = (1 << 16) - 1;

/// Image is an image.Image with a Set method to change a single pixel.
///
/// Go: image/draw/draw.go:Image
pub trait Image: crate::image::Image {
    fn set(&mut self, x: i64, y: i64, c: Color);
    /// Mutable upcast for Go-style type switches on the destination.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Go: `dst.(draw.RGBA64Image)`.
    fn as_draw_rgba64_image(&mut self) -> Option<&mut dyn RGBA64Image> {
        None
    }
}

impl dyn Image {
    /// Go: `dst.(*T)` on a draw.Image.
    pub fn downcast_mut<T: Image>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

/// RGBA64Image extends both the [`Image`] and [`crate::RGBA64Image`]
/// interfaces with a SetRGBA64 method to change a single pixel.
///
/// Go: image/draw/draw.go:RGBA64Image
pub trait RGBA64Image: Image + crate::image::RGBA64Image {
    fn set_rgba64(&mut self, x: i64, y: i64, c: color::RGBA64);
}

/// Quantizer produces a palette for an image.
///
/// Go: image/draw/draw.go:Quantizer
pub trait Quantizer {
    /// Quantize appends up to cap(p) - len(p) colors to p and returns the
    /// updated palette suitable for converting m to a paletted image.
    fn quantize(&self, p: Palette, m: &dyn crate::image::Image) -> Palette;
}

/// Op is a Porter-Duff compositing operator.
///
/// Go: image/draw/draw.go:Op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// Over specifies ``(src in mask) over dst''.
    Over = 0,
    /// Src specifies ``src in mask''.
    Src = 1,
}

/// Drawer contains the [`Drawer::draw`] method.
///
/// Go: image/draw/draw.go:Drawer
pub trait Drawer {
    /// Draw aligns r.Min in dst with sp in src and then replaces the
    /// rectangle r in dst with the result of drawing src on dst.
    fn draw(&self, dst: &mut dyn Image, r: Rectangle, src: &dyn crate::image::Image, sp: Point);
}

impl Drawer for Op {
    /// Draw implements the [`Drawer`] interface by calling the Draw function
    /// with this [`Op`].
    ///
    /// Go: image/draw/draw.go:Op.Draw
    fn draw(&self, dst: &mut dyn Image, r: Rectangle, src: &dyn crate::image::Image, sp: Point) {
        draw_mask(dst, r, src, sp, None, Point::default(), *self);
    }
}

/// FloydSteinberg is a [`Drawer`] that is the [`Op::Src`] Op with
/// Floyd-Steinberg error diffusion.
///
/// Go: image/draw/draw.go:FloydSteinberg
pub const FLOYD_STEINBERG: FloydSteinberg = FloydSteinberg;

/// Go: image/draw/draw.go:floydSteinberg
#[derive(Clone, Copy, Debug, Default)]
pub struct FloydSteinberg;

impl Drawer for FloydSteinberg {
    // Go: image/draw/draw.go:floydSteinberg.Draw
    fn draw(
        &self,
        dst: &mut dyn Image,
        mut r: Rectangle,
        src: &dyn crate::image::Image,
        mut sp: Point,
    ) {
        clip(dst.bounds(), &mut r, src, &mut sp, None, None);
        if r.empty() {
            return;
        }
        draw_paletted(dst, r, src, sp, true);
    }
}

/// clip clips r against each image's bounds (after translating into the
/// destination image's coordinate space) and shifts the points sp and mp by
/// the same amount as the change in r.Min.
///
/// Go: image/draw/draw.go:clip (takes dst.Bounds() instead of dst).
fn clip(
    dst_bounds: Rectangle,
    r: &mut Rectangle,
    src: &dyn crate::image::Image,
    sp: &mut Point,
    mask: Option<&dyn crate::image::Image>,
    mp: Option<&mut Point>,
) {
    let orig = r.min;
    *r = r.intersect(dst_bounds);
    *r = r.intersect(src.bounds().add(orig.sub(*sp)));
    if let Some(mask) = mask {
        let mpv = mp.as_deref().copied().unwrap_or_default();
        *r = r.intersect(mask.bounds().add(orig.sub(mpv)));
    }
    let dx = r.min.x.wrapping_sub(orig.x);
    let dy = r.min.y.wrapping_sub(orig.y);
    if dx == 0 && dy == 0 {
        return;
    }
    sp.x = sp.x.wrapping_add(dx);
    sp.y = sp.y.wrapping_add(dy);
    if let Some(mp) = mp {
        mp.x = mp.x.wrapping_add(dx);
        mp.y = mp.y.wrapping_add(dy);
    }
}

// Go's `dst == src` interface comparison. In safe Rust a `&mut` destination
// and a `&` source can never be the same object, so this is always false; it
// is kept (as an address comparison) to mirror the Go control flow.
fn same_image(dst: *const (), src: *const ()) -> bool {
    std::ptr::eq(dst, src)
}

// Go: image/draw/draw.go:processBackward
fn process_backward(
    dst: *const (),
    r: Rectangle,
    src: &dyn crate::image::Image,
    sp: Point,
) -> bool {
    same_image(dst, src as *const dyn crate::image::Image as *const ())
        && r.overlaps(r.add(sp.sub(r.min)))
        && (sp.y < r.min.y || (sp.y == r.min.y && sp.x < r.min.x))
}

/// Draw calls [`draw_mask`] with a nil mask.
///
/// Go: image/draw/draw.go:Draw
pub fn draw(dst: &mut dyn Image, r: Rectangle, src: &dyn crate::image::Image, sp: Point, op: Op) {
    draw_mask(dst, r, src, sp, None, Point::default(), op);
}

/// DrawMask aligns r.Min in dst with sp in src and mp in mask and then
/// replaces the rectangle r in dst with the result of a Porter-Duff
/// composition. A nil mask is treated as opaque.
///
/// Go: image/draw/draw.go:DrawMask
pub fn draw_mask(
    dst: &mut dyn Image,
    mut r: Rectangle,
    src: &dyn crate::image::Image,
    mut sp: Point,
    mask: Option<&dyn crate::image::Image>,
    mut mp: Point,
    op: Op,
) {
    clip(dst.bounds(), &mut r, src, &mut sp, mask, Some(&mut mp));
    if r.empty() {
        return;
    }
    let dst_ptr = dst as *const dyn Image as *const ();

    // Fast paths for special cases. If none of them apply, then we fall back
    // to general but slower implementations.
    //
    // For NRGBA and NRGBA64 image types, the code paths aren't just faster.
    // They also avoid the information loss that would otherwise occur from
    // converting non-alpha-premultiplied color to and from alpha-premultiplied
    // color. See TestDrawSrcNonpremultiplied.
    if let Some(dst0) = dst.as_any_mut().downcast_mut::<RGBA>() {
        let src_any = src.as_any();
        if op == Op::Over {
            if mask.is_none() {
                if let Some(src0) = src_any.downcast_ref::<Uniform>() {
                    let (sr, sg, sb, sa) = src0.rgba();
                    if sa == 0xffff {
                        draw_fill_src(dst0, r, sr, sg, sb, sa);
                    } else {
                        draw_fill_over(dst0, r, sr, sg, sb, sa);
                    }
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<RGBA>() {
                    draw_copy_over(dst0, r, src0, sp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<NRGBA>() {
                    draw_nrgba_over(dst0, r, src0, sp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<YCbCr>() {
                    // An image.YCbCr is always fully opaque, and so if the
                    // mask is nil (i.e. fully opaque) then the op is
                    // effectively always Src. Similarly for image.Gray and
                    // image.CMYK.
                    if imageutil::draw_ycbcr(dst0, r, src0, sp) {
                        return;
                    }
                } else if let Some(src0) = src_any.downcast_ref::<Gray>() {
                    draw_gray(dst0, r, src0, sp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<CMYK>() {
                    draw_cmyk(dst0, r, src0, sp);
                    return;
                }
            } else if let Some(mask0) = mask.and_then(|m| m.as_any().downcast_ref::<Alpha>()) {
                if let Some(src0) = src_any.downcast_ref::<Uniform>() {
                    draw_glyph_over(dst0, r, src0, mask0, mp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<RGBA>() {
                    draw_rgba_mask_over(dst0, r, src0, sp, mask0, mp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<Gray>() {
                    draw_gray_mask_over(dst0, r, src0, sp, mask0, mp);
                    return;
                // Case order matters. The next case (image.RGBA64Image) is an
                // interface type that the concrete types above also implement.
                } else if let Some(src0) = src.as_rgba64_image() {
                    draw_rgba64_image_mask_over(dst0, r, src0, sp, mask0, mp);
                    return;
                }
            }
        } else {
            if mask.is_none() {
                if let Some(src0) = src_any.downcast_ref::<Uniform>() {
                    let (sr, sg, sb, sa) = src0.rgba();
                    draw_fill_src(dst0, r, sr, sg, sb, sa);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<RGBA>() {
                    let d0 = dst0.pix_offset(r.min.x, r.min.y);
                    let s0 = src0.pix_offset(sp.x, sp.y);
                    let dst_stride = dst0.stride;
                    draw_copy_src(
                        &mut dst0.pix[d0 as usize..],
                        dst_stride,
                        r,
                        &src0.pix[s0 as usize..],
                        src0.stride,
                        sp,
                        4 * r.dx(),
                    );
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<NRGBA>() {
                    draw_nrgba_src(dst0, r, src0, sp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<YCbCr>() {
                    if imageutil::draw_ycbcr(dst0, r, src0, sp) {
                        return;
                    }
                } else if let Some(src0) = src_any.downcast_ref::<Gray>() {
                    draw_gray(dst0, r, src0, sp);
                    return;
                } else if let Some(src0) = src_any.downcast_ref::<CMYK>() {
                    draw_cmyk(dst0, r, src0, sp);
                    return;
                }
            }
        }
        draw_rgba(dst0, dst_ptr, r, src, sp, mask, mp, op);
        return;
    } else if dst.as_any().is::<Paletted>() {
        if op == Op::Src && mask.is_none() {
            if let Some(src0) = src.as_any().downcast_ref::<Uniform>() {
                let dst0 = dst.as_any_mut().downcast_mut::<Paletted>().unwrap();
                let color_index = dst0.palette.index(src0.c) as u8;
                let mut i0 = dst0.pix_offset(r.min.x, r.min.y) as usize;
                let mut i1 = i0 + r.dx() as usize;
                for i in i0..i1 {
                    dst0.pix[i] = color_index;
                }
                let first_row_start = i0;
                let stride = dst0.stride as usize;
                for _y in r.min.y + 1..r.max.y {
                    i0 += stride;
                    i1 += stride;
                    dst0.pix
                        .copy_within(first_row_start..first_row_start + (i1 - i0), i0);
                }
                return;
            } else if !process_backward(dst_ptr, r, src, sp) {
                draw_paletted(dst, r, src, sp, false);
                return;
            }
        }
    } else if let Some(dst0) = dst.as_any_mut().downcast_mut::<NRGBA>() {
        if op == Op::Src && mask.is_none() {
            if let Some(src0) = src.as_any().downcast_ref::<NRGBA>() {
                let d0 = dst0.pix_offset(r.min.x, r.min.y);
                let s0 = src0.pix_offset(sp.x, sp.y);
                let dst_stride = dst0.stride;
                draw_copy_src(
                    &mut dst0.pix[d0 as usize..],
                    dst_stride,
                    r,
                    &src0.pix[s0 as usize..],
                    src0.stride,
                    sp,
                    4 * r.dx(),
                );
                return;
            }
        }
    } else if let Some(dst0) = dst.as_any_mut().downcast_mut::<NRGBA64>() {
        if op == Op::Src && mask.is_none() {
            if let Some(src0) = src.as_any().downcast_ref::<NRGBA64>() {
                let d0 = dst0.pix_offset(r.min.x, r.min.y);
                let s0 = src0.pix_offset(sp.x, sp.y);
                let dst_stride = dst0.stride;
                draw_copy_src(
                    &mut dst0.pix[d0 as usize..],
                    dst_stride,
                    r,
                    &src0.pix[s0 as usize..],
                    src0.stride,
                    sp,
                    8 * r.dx(),
                );
                return;
            }
        }
    }

    let (mut x0, mut x1, mut dx) = (r.min.x, r.max.x, 1i64);
    let (mut y0, mut y1, mut dy) = (r.min.y, r.max.y, 1i64);
    if process_backward(dst_ptr, r, src, sp) {
        (x0, x1, dx) = (x1 - 1, x0 - 1, -1);
        (y0, y1, dy) = (y1 - 1, y0 - 1, -1);
    }

    // FALLBACK1.17
    //
    // Try the draw.RGBA64Image and image.RGBA64Image interfaces, part of the
    // standard library since Go 1.17. These are like the draw.Image and
    // image.Image interfaces but they can avoid allocations from converting
    // concrete color types to the color.Color interface type.

    if let Some(dst0) = dst.as_draw_rgba64_image() {
        if let Some(src0) = src.as_rgba64_image() {
            if mask.is_none() {
                let mut sy = sp.y + y0 - r.min.y;
                let mut _my = mp.y + y0 - r.min.y;
                let mut y = y0;
                while y != y1 {
                    let mut sx = sp.x + x0 - r.min.x;
                    let mut _mx = mp.x + x0 - r.min.x;
                    let mut x = x0;
                    while x != x1 {
                        if op == Op::Src {
                            dst0.set_rgba64(x, y, src0.rgba64_at(sx, sy));
                        } else {
                            let srgba = src0.rgba64_at(sx, sy);
                            let a = M.wrapping_sub(srgba.a as u32);
                            let drgba = dst0.rgba64_at(x, y);
                            dst0.set_rgba64(
                                x,
                                y,
                                color::RGBA64 {
                                    r: ((((drgba.r as u32).wrapping_mul(a)) / M) as u16)
                                        .wrapping_add(srgba.r),
                                    g: ((((drgba.g as u32).wrapping_mul(a)) / M) as u16)
                                        .wrapping_add(srgba.g),
                                    b: ((((drgba.b as u32).wrapping_mul(a)) / M) as u16)
                                        .wrapping_add(srgba.b),
                                    a: ((((drgba.a as u32).wrapping_mul(a)) / M) as u16)
                                        .wrapping_add(srgba.a),
                                },
                            );
                        }
                        x += dx;
                        sx += dx;
                        _mx += dx;
                    }
                    y += dy;
                    sy += dy;
                    _my += dy;
                }
                return;
            } else if let Some(mask0) = mask.and_then(|m| m.as_rgba64_image()) {
                let mut sy = sp.y + y0 - r.min.y;
                let mut my = mp.y + y0 - r.min.y;
                let mut y = y0;
                while y != y1 {
                    let mut sx = sp.x + x0 - r.min.x;
                    let mut mx = mp.x + x0 - r.min.x;
                    let mut x = x0;
                    while x != x1 {
                        let ma = mask0.rgba64_at(mx, my).a as u32;
                        if ma == 0 {
                            if op == Op::Over {
                                // No-op.
                            } else {
                                dst0.set_rgba64(x, y, color::RGBA64::default());
                            }
                        } else if ma == M && op == Op::Src {
                            dst0.set_rgba64(x, y, src0.rgba64_at(sx, sy));
                        } else {
                            let srgba = src0.rgba64_at(sx, sy);
                            if op == Op::Over {
                                let drgba = dst0.rgba64_at(x, y);
                                let a = M.wrapping_sub((srgba.a as u32).wrapping_mul(ma) / M);
                                dst0.set_rgba64(
                                    x,
                                    y,
                                    color::RGBA64 {
                                        r: ((drgba.r as u32)
                                            .wrapping_mul(a)
                                            .wrapping_add((srgba.r as u32).wrapping_mul(ma))
                                            / M) as u16,
                                        g: ((drgba.g as u32)
                                            .wrapping_mul(a)
                                            .wrapping_add((srgba.g as u32).wrapping_mul(ma))
                                            / M) as u16,
                                        b: ((drgba.b as u32)
                                            .wrapping_mul(a)
                                            .wrapping_add((srgba.b as u32).wrapping_mul(ma))
                                            / M) as u16,
                                        a: ((drgba.a as u32)
                                            .wrapping_mul(a)
                                            .wrapping_add((srgba.a as u32).wrapping_mul(ma))
                                            / M) as u16,
                                    },
                                );
                            } else {
                                dst0.set_rgba64(
                                    x,
                                    y,
                                    color::RGBA64 {
                                        r: ((srgba.r as u32).wrapping_mul(ma) / M) as u16,
                                        g: ((srgba.g as u32).wrapping_mul(ma) / M) as u16,
                                        b: ((srgba.b as u32).wrapping_mul(ma) / M) as u16,
                                        a: ((srgba.a as u32).wrapping_mul(ma) / M) as u16,
                                    },
                                );
                            }
                        }
                        x += dx;
                        sx += dx;
                        mx += dx;
                    }
                    y += dy;
                    sy += dy;
                    my += dy;
                }
                return;
            }
        }
    }

    // FALLBACK1.0
    //
    // If none of the faster code paths above apply, use the draw.Image and
    // image.Image interfaces, part of the standard library since Go 1.0.

    let mut out = color::RGBA64::default();
    let mut sy = sp.y + y0 - r.min.y;
    let mut my = mp.y + y0 - r.min.y;
    let mut y = y0;
    while y != y1 {
        let mut sx = sp.x + x0 - r.min.x;
        let mut mx = mp.x + x0 - r.min.x;
        let mut x = x0;
        while x != x1 {
            let mut ma = M;
            if let Some(mask) = mask {
                (_, _, _, ma) = mask.at(mx, my).rgba();
            }
            if ma == 0 {
                if op == Op::Over {
                    // No-op.
                } else {
                    dst.set(x, y, Color::Alpha16(color::TRANSPARENT));
                }
            } else if ma == M && op == Op::Src {
                dst.set(x, y, src.at(sx, sy));
            } else {
                let (sr, sg, sb, sa) = src.at(sx, sy).rgba();
                if op == Op::Over {
                    let (dr, dg, db, da) = dst.at(x, y).rgba();
                    let a = M.wrapping_sub(sa.wrapping_mul(ma) / M);
                    out.r = (dr.wrapping_mul(a).wrapping_add(sr.wrapping_mul(ma)) / M) as u16;
                    out.g = (dg.wrapping_mul(a).wrapping_add(sg.wrapping_mul(ma)) / M) as u16;
                    out.b = (db.wrapping_mul(a).wrapping_add(sb.wrapping_mul(ma)) / M) as u16;
                    out.a = (da.wrapping_mul(a).wrapping_add(sa.wrapping_mul(ma)) / M) as u16;
                } else {
                    out.r = (sr.wrapping_mul(ma) / M) as u16;
                    out.g = (sg.wrapping_mul(ma) / M) as u16;
                    out.b = (sb.wrapping_mul(ma) / M) as u16;
                    out.a = (sa.wrapping_mul(ma) / M) as u16;
                }
                // The third argument is &out instead of out in Go (a
                // *color.RGBA64, which converts identically).
                dst.set(x, y, Color::RGBA64(out));
            }
            x += dx;
            sx += dx;
            mx += dx;
        }
        y += dy;
        sy += dy;
        my += dy;
    }
}

// Go: image/draw/draw.go:drawFillOver
fn draw_fill_over(dst: &mut RGBA, r: Rectangle, sr: u32, sg: u32, sb: u32, sa: u32) {
    // The 0x101 is here for the same reason as in drawRGBA.
    let a = (M - sa).wrapping_mul(0x101);
    let mut i0 = dst.pix_offset(r.min.x, r.min.y);
    let mut i1 = i0 + r.dx() * 4;
    let mut y = r.min.y;
    while y != r.max.y {
        let mut i = i0;
        while i < i1 {
            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            d[0] = ((d[0] as u32).wrapping_mul(a) / M)
                .wrapping_add(sr)
                .wrapping_shr(8) as u8;
            d[1] = ((d[1] as u32).wrapping_mul(a) / M)
                .wrapping_add(sg)
                .wrapping_shr(8) as u8;
            d[2] = ((d[2] as u32).wrapping_mul(a) / M)
                .wrapping_add(sb)
                .wrapping_shr(8) as u8;
            d[3] = ((d[3] as u32).wrapping_mul(a) / M)
                .wrapping_add(sa)
                .wrapping_shr(8) as u8;
            i += 4;
        }
        i0 += dst.stride;
        i1 += dst.stride;
        y += 1;
    }
}

// Go: image/draw/draw.go:drawFillSrc
fn draw_fill_src(dst: &mut RGBA, r: Rectangle, sr: u32, sg: u32, sb: u32, sa: u32) {
    let sr8 = (sr >> 8) as u8;
    let sg8 = (sg >> 8) as u8;
    let sb8 = (sb >> 8) as u8;
    let sa8 = (sa >> 8) as u8;
    // The built-in copy function is faster than a straightforward for loop to fill the destination with
    // the color, but copy requires a slice source. We therefore use a for loop to fill the first row, and
    // then use the first row as the slice source for the remaining rows.
    let mut i0 = dst.pix_offset(r.min.x, r.min.y) as usize;
    let mut i1 = i0 + (r.dx() * 4) as usize;
    let mut i = i0;
    while i < i1 {
        dst.pix[i] = sr8;
        dst.pix[i + 1] = sg8;
        dst.pix[i + 2] = sb8;
        dst.pix[i + 3] = sa8;
        i += 4;
    }
    let first_row = i0;
    let stride = dst.stride as usize;
    for _y in r.min.y + 1..r.max.y {
        i0 += stride;
        i1 += stride;
        dst.pix.copy_within(first_row..first_row + (i1 - i0), i0);
    }
}

// Go: image/draw/draw.go:drawCopyOver
fn draw_copy_over(dst: &mut RGBA, r: Rectangle, src: &RGBA, sp: Point) {
    let (dx, mut dy) = (r.dx(), r.dy());
    let mut d0 = dst.pix_offset(r.min.x, r.min.y);
    let mut s0 = src.pix_offset(sp.x, sp.y);
    let (ddelta, sdelta);
    let (i0, i1, idelta);
    if r.min.y < sp.y || r.min.y == sp.y && r.min.x <= sp.x {
        ddelta = dst.stride;
        sdelta = src.stride;
        (i0, i1, idelta) = (0i64, dx * 4, 4i64);
    } else {
        // If the source start point is higher than the destination start point, or equal height but to the left,
        // then we compose the rows in right-to-left, bottom-up order instead of left-to-right, top-down.
        d0 += (dy - 1) * dst.stride;
        s0 += (dy - 1) * src.stride;
        ddelta = -dst.stride;
        sdelta = -src.stride;
        (i0, i1, idelta) = ((dx - 1) * 4, -4i64, -4i64);
    }
    while dy > 0 {
        let dpix = &mut dst.pix[d0 as usize..];
        let spix = &src.pix[s0 as usize..];
        let mut i = i0;
        while i != i1 {
            let iu = i as usize;
            let s = &spix[iu..iu + 4];
            let sr = (s[0] as u32) * 0x101;
            let sg = (s[1] as u32) * 0x101;
            let sb = (s[2] as u32) * 0x101;
            let sa = (s[3] as u32) * 0x101;

            // The 0x101 is here for the same reason as in drawRGBA.
            let a = (M - sa).wrapping_mul(0x101);

            let d = &mut dpix[iu..iu + 4];
            d[0] = ((d[0] as u32).wrapping_mul(a) / M)
                .wrapping_add(sr)
                .wrapping_shr(8) as u8;
            d[1] = ((d[1] as u32).wrapping_mul(a) / M)
                .wrapping_add(sg)
                .wrapping_shr(8) as u8;
            d[2] = ((d[2] as u32).wrapping_mul(a) / M)
                .wrapping_add(sb)
                .wrapping_shr(8) as u8;
            d[3] = ((d[3] as u32).wrapping_mul(a) / M)
                .wrapping_add(sa)
                .wrapping_shr(8) as u8;
            i += idelta;
        }
        d0 += ddelta;
        s0 += sdelta;
        dy -= 1;
    }
}

/// drawCopySrc copies bytes to dstPix from srcPix. These arguments roughly
/// correspond to the Pix fields of the image package's concrete image.Image
/// implementations, but are offset (dstPix is dst.Pix[dpOffset:] not
/// dst.Pix).
///
/// Go: image/draw/draw.go:drawCopySrc
fn draw_copy_src(
    dst_pix: &mut [u8],
    dst_stride: i64,
    r: Rectangle,
    src_pix: &[u8],
    src_stride: i64,
    sp: Point,
    bytes_per_row: i64,
) {
    let (mut d0, mut s0, mut ddelta, mut sdelta, mut dy) =
        (0i64, 0i64, dst_stride, src_stride, r.dy());
    if r.min.y > sp.y {
        // If the source start point is higher than the destination start
        // point, then we compose the rows in bottom-up order instead of
        // top-down. Unlike the drawCopyOver function, we don't have to check
        // the x coordinates because the built-in copy function can handle
        // overlapping slices.
        d0 = (dy - 1) * dst_stride;
        s0 = (dy - 1) * src_stride;
        ddelta = -dst_stride;
        sdelta = -src_stride;
    }
    let n = bytes_per_row as usize;
    while dy > 0 {
        let (d, s) = (d0 as usize, s0 as usize);
        dst_pix[d..d + n].copy_from_slice(&src_pix[s..s + n]);
        d0 += ddelta;
        s0 += sdelta;
        dy -= 1;
    }
}

// Go: image/draw/draw.go:drawNRGBAOver
fn draw_nrgba_over(dst: &mut RGBA, r: Rectangle, src: &NRGBA, sp: Point) {
    let i0 = (r.min.x - dst.rect.min.x) * 4;
    let i1 = (r.max.x - dst.rect.min.x) * 4;
    let si0 = (sp.x - src.rect.min.x) * 4;
    let y_max = r.max.y - dst.rect.min.y;

    let mut y = r.min.y - dst.rect.min.y;
    let mut sy = sp.y - src.rect.min.y;
    while y != y_max {
        let dpix = &mut dst.pix[(y * dst.stride) as usize..];
        let spix = &src.pix[(sy * src.stride) as usize..];

        let (mut i, mut si) = (i0, si0);
        while i < i1 {
            // Convert from non-premultiplied color to pre-multiplied color.
            let s = &spix[si as usize..si as usize + 4];
            let sa = (s[3] as u32) * 0x101;
            let sr = (s[0] as u32) * sa / 0xff;
            let sg = (s[1] as u32) * sa / 0xff;
            let sb = (s[2] as u32) * sa / 0xff;

            let d = &mut dpix[i as usize..i as usize + 4];
            let dr = d[0] as u32;
            let dg = d[1] as u32;
            let db = d[2] as u32;
            let da = d[3] as u32;

            // The 0x101 is here for the same reason as in drawRGBA.
            let a = (M - sa).wrapping_mul(0x101);

            d[0] = (dr.wrapping_mul(a) / M).wrapping_add(sr).wrapping_shr(8) as u8;
            d[1] = (dg.wrapping_mul(a) / M).wrapping_add(sg).wrapping_shr(8) as u8;
            d[2] = (db.wrapping_mul(a) / M).wrapping_add(sb).wrapping_shr(8) as u8;
            d[3] = (da.wrapping_mul(a) / M).wrapping_add(sa).wrapping_shr(8) as u8;
            i += 4;
            si += 4;
        }
        y += 1;
        sy += 1;
    }
}

// Go: image/draw/draw.go:drawNRGBASrc
fn draw_nrgba_src(dst: &mut RGBA, r: Rectangle, src: &NRGBA, sp: Point) {
    let i0 = (r.min.x - dst.rect.min.x) * 4;
    let i1 = (r.max.x - dst.rect.min.x) * 4;
    let si0 = (sp.x - src.rect.min.x) * 4;
    let y_max = r.max.y - dst.rect.min.y;

    let mut y = r.min.y - dst.rect.min.y;
    let mut sy = sp.y - src.rect.min.y;
    while y != y_max {
        let dpix = &mut dst.pix[(y * dst.stride) as usize..];
        let spix = &src.pix[(sy * src.stride) as usize..];

        let (mut i, mut si) = (i0, si0);
        while i < i1 {
            // Convert from non-premultiplied color to pre-multiplied color.
            let s = &spix[si as usize..si as usize + 4];
            let sa = (s[3] as u32) * 0x101;
            let sr = (s[0] as u32) * sa / 0xff;
            let sg = (s[1] as u32) * sa / 0xff;
            let sb = (s[2] as u32) * sa / 0xff;

            let d = &mut dpix[i as usize..i as usize + 4];
            d[0] = (sr >> 8) as u8;
            d[1] = (sg >> 8) as u8;
            d[2] = (sb >> 8) as u8;
            d[3] = (sa >> 8) as u8;
            i += 4;
            si += 4;
        }
        y += 1;
        sy += 1;
    }
}

// Go: image/draw/draw.go:drawGray
fn draw_gray(dst: &mut RGBA, r: Rectangle, src: &Gray, sp: Point) {
    let i0 = (r.min.x - dst.rect.min.x) * 4;
    let i1 = (r.max.x - dst.rect.min.x) * 4;
    let si0 = sp.x - src.rect.min.x;
    let y_max = r.max.y - dst.rect.min.y;

    let mut y = r.min.y - dst.rect.min.y;
    let mut sy = sp.y - src.rect.min.y;
    while y != y_max {
        let dpix = &mut dst.pix[(y * dst.stride) as usize..];
        let spix = &src.pix[(sy * src.stride) as usize..];

        let (mut i, mut si) = (i0, si0);
        while i < i1 {
            let p = spix[si as usize];
            let d = &mut dpix[i as usize..i as usize + 4];
            d[0] = p;
            d[1] = p;
            d[2] = p;
            d[3] = 255;
            i += 4;
            si += 1;
        }
        y += 1;
        sy += 1;
    }
}

// Go: image/draw/draw.go:drawCMYK
fn draw_cmyk(dst: &mut RGBA, r: Rectangle, src: &CMYK, sp: Point) {
    let i0 = (r.min.x - dst.rect.min.x) * 4;
    let i1 = (r.max.x - dst.rect.min.x) * 4;
    let si0 = (sp.x - src.rect.min.x) * 4;
    let y_max = r.max.y - dst.rect.min.y;

    let mut y = r.min.y - dst.rect.min.y;
    let mut sy = sp.y - src.rect.min.y;
    while y != y_max {
        let dpix = &mut dst.pix[(y * dst.stride) as usize..];
        let spix = &src.pix[(sy * src.stride) as usize..];

        let (mut i, mut si) = (i0, si0);
        while i < i1 {
            let s = &spix[si as usize..si as usize + 4];
            let d = &mut dpix[i as usize..i as usize + 4];
            (d[0], d[1], d[2]) = color::cmyk_to_rgb(s[0], s[1], s[2], s[3]);
            d[3] = 255;
            i += 4;
            si += 4;
        }
        y += 1;
        sy += 1;
    }
}

// Go: image/draw/draw.go:drawGlyphOver
fn draw_glyph_over(dst: &mut RGBA, r: Rectangle, src: &Uniform, mask: &Alpha, mp: Point) {
    let mut i0 = dst.pix_offset(r.min.x, r.min.y);
    let mut i1 = i0 + r.dx() * 4;
    let mut mi0 = mask.pix_offset(mp.x, mp.y);
    let (sr, sg, sb, sa) = src.rgba();
    let (mut y, mut _my) = (r.min.y, mp.y);
    while y != r.max.y {
        let (mut i, mut mi) = (i0, mi0);
        while i < i1 {
            let mut ma = mask.pix[mi as usize] as u32;
            if ma == 0 {
                i += 4;
                mi += 1;
                continue;
            }
            ma |= ma << 8;

            // The 0x101 is here for the same reason as in drawRGBA.
            let a = M.wrapping_sub(sa.wrapping_mul(ma) / M).wrapping_mul(0x101);

            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            d[0] = ((d[0] as u32)
                .wrapping_mul(a)
                .wrapping_add(sr.wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[1] = ((d[1] as u32)
                .wrapping_mul(a)
                .wrapping_add(sg.wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[2] = ((d[2] as u32)
                .wrapping_mul(a)
                .wrapping_add(sb.wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[3] = ((d[3] as u32)
                .wrapping_mul(a)
                .wrapping_add(sa.wrapping_mul(ma))
                / M
                >> 8) as u8;
            i += 4;
            mi += 1;
        }
        i0 += dst.stride;
        i1 += dst.stride;
        mi0 += mask.stride;
        y += 1;
        _my += 1;
    }
}

// Go: image/draw/draw.go:drawGrayMaskOver
fn draw_gray_mask_over(
    dst: &mut RGBA,
    r: Rectangle,
    src: &Gray,
    sp: Point,
    mask: &Alpha,
    mp: Point,
) {
    let (mut x0, mut x1, mut dx) = (r.min.x, r.max.x, 1i64);
    let (mut y0, mut y1, mut dy) = (r.min.y, r.max.y, 1i64);
    if r.overlaps(r.add(sp.sub(r.min))) {
        if sp.y < r.min.y || sp.y == r.min.y && sp.x < r.min.x {
            (x0, x1, dx) = (x1 - 1, x0 - 1, -1);
            (y0, y1, dy) = (y1 - 1, y0 - 1, -1);
        }
    }

    let mut sy = sp.y + y0 - r.min.y;
    let mut my = mp.y + y0 - r.min.y;
    let sx0 = sp.x + x0 - r.min.x;
    let mx0 = mp.x + x0 - r.min.x;
    let sx1 = sx0 + (x1 - x0);
    let mut i0 = dst.pix_offset(x0, y0);
    let di = dx * 4;
    let mut y = y0;
    while y != y1 {
        let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
        while sx != sx1 {
            let mi = mask.pix_offset(mx, my);
            let mut ma = mask.pix[mi as usize] as u32;
            ma |= ma << 8;
            let si = src.pix_offset(sx, sy);
            let mut sy_ = src.pix[si as usize] as u32;
            sy_ |= sy_ << 8;
            let sa = 0xffffu32;

            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            let dr = d[0] as u32;
            let dg = d[1] as u32;
            let db = d[2] as u32;
            let da = d[3] as u32;

            // dr, dg, db and da are all 8-bit color at the moment, ranging in [0,255].
            // We work in 16-bit color, and so would normally do:
            // dr |= dr << 8
            // and similarly for dg, db and da, but instead we multiply a
            // (which is a 16-bit color, ranging in [0,65535]) by 0x101.
            // This yields the same result, but is fewer arithmetic operations.
            let a = M.wrapping_sub(sa.wrapping_mul(ma) / M).wrapping_mul(0x101);

            d[0] = (dr.wrapping_mul(a).wrapping_add(sy_.wrapping_mul(ma)) / M >> 8) as u8;
            d[1] = (dg.wrapping_mul(a).wrapping_add(sy_.wrapping_mul(ma)) / M >> 8) as u8;
            d[2] = (db.wrapping_mul(a).wrapping_add(sy_.wrapping_mul(ma)) / M >> 8) as u8;
            d[3] = (da.wrapping_mul(a).wrapping_add(sa.wrapping_mul(ma)) / M >> 8) as u8;
            i += di;
            sx += dx;
            mx += dx;
        }
        i0 += dy * dst.stride;
        y += dy;
        sy += dy;
        my += dy;
    }
}

// Go: image/draw/draw.go:drawRGBAMaskOver
fn draw_rgba_mask_over(
    dst: &mut RGBA,
    r: Rectangle,
    src: &RGBA,
    sp: Point,
    mask: &Alpha,
    mp: Point,
) {
    let (mut x0, mut x1, mut dx) = (r.min.x, r.max.x, 1i64);
    let (mut y0, mut y1, mut dy) = (r.min.y, r.max.y, 1i64);
    if same_image(
        dst as *const RGBA as *const (),
        src as *const RGBA as *const (),
    ) && r.overlaps(r.add(sp.sub(r.min)))
    {
        if sp.y < r.min.y || sp.y == r.min.y && sp.x < r.min.x {
            (x0, x1, dx) = (x1 - 1, x0 - 1, -1);
            (y0, y1, dy) = (y1 - 1, y0 - 1, -1);
        }
    }

    let mut sy = sp.y + y0 - r.min.y;
    let mut my = mp.y + y0 - r.min.y;
    let sx0 = sp.x + x0 - r.min.x;
    let mx0 = mp.x + x0 - r.min.x;
    let sx1 = sx0 + (x1 - x0);
    let mut i0 = dst.pix_offset(x0, y0);
    let di = dx * 4;
    let mut y = y0;
    while y != y1 {
        let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
        while sx != sx1 {
            let mi = mask.pix_offset(mx, my);
            let mut ma = mask.pix[mi as usize] as u32;
            ma |= ma << 8;
            let si = src.pix_offset(sx, sy) as usize;
            let mut sr = src.pix[si] as u32;
            let mut sg = src.pix[si + 1] as u32;
            let mut sb = src.pix[si + 2] as u32;
            let mut sa = src.pix[si + 3] as u32;
            sr |= sr << 8;
            sg |= sg << 8;
            sb |= sb << 8;
            sa |= sa << 8;
            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            let dr = d[0] as u32;
            let dg = d[1] as u32;
            let db = d[2] as u32;
            let da = d[3] as u32;

            // See drawGrayMaskOver for the 0x101 trick.
            let a = M.wrapping_sub(sa.wrapping_mul(ma) / M).wrapping_mul(0x101);

            d[0] = (dr.wrapping_mul(a).wrapping_add(sr.wrapping_mul(ma)) / M >> 8) as u8;
            d[1] = (dg.wrapping_mul(a).wrapping_add(sg.wrapping_mul(ma)) / M >> 8) as u8;
            d[2] = (db.wrapping_mul(a).wrapping_add(sb.wrapping_mul(ma)) / M >> 8) as u8;
            d[3] = (da.wrapping_mul(a).wrapping_add(sa.wrapping_mul(ma)) / M >> 8) as u8;
            i += di;
            sx += dx;
            mx += dx;
        }
        i0 += dy * dst.stride;
        y += dy;
        sy += dy;
        my += dy;
    }
}

// Go: image/draw/draw.go:drawRGBA64ImageMaskOver
fn draw_rgba64_image_mask_over(
    dst: &mut RGBA,
    r: Rectangle,
    src: &dyn crate::image::RGBA64Image,
    sp: Point,
    mask: &Alpha,
    mp: Point,
) {
    let (mut x0, mut x1, mut dx) = (r.min.x, r.max.x, 1i64);
    let (mut y0, mut y1, mut dy) = (r.min.y, r.max.y, 1i64);
    if same_image(
        dst as *const RGBA as *const (),
        src as *const dyn crate::image::RGBA64Image as *const (),
    ) && r.overlaps(r.add(sp.sub(r.min)))
    {
        if sp.y < r.min.y || sp.y == r.min.y && sp.x < r.min.x {
            (x0, x1, dx) = (x1 - 1, x0 - 1, -1);
            (y0, y1, dy) = (y1 - 1, y0 - 1, -1);
        }
    }

    let mut sy = sp.y + y0 - r.min.y;
    let mut my = mp.y + y0 - r.min.y;
    let sx0 = sp.x + x0 - r.min.x;
    let mx0 = mp.x + x0 - r.min.x;
    let sx1 = sx0 + (x1 - x0);
    let mut i0 = dst.pix_offset(x0, y0);
    let di = dx * 4;
    let mut y = y0;
    while y != y1 {
        let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
        while sx != sx1 {
            let mi = mask.pix_offset(mx, my);
            let mut ma = mask.pix[mi as usize] as u32;
            ma |= ma << 8;
            let srgba = src.rgba64_at(sx, sy);
            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            let dr = d[0] as u32;
            let dg = d[1] as u32;
            let db = d[2] as u32;
            let da = d[3] as u32;

            // See drawGrayMaskOver for the 0x101 trick.
            let a = M
                .wrapping_sub((srgba.a as u32).wrapping_mul(ma) / M)
                .wrapping_mul(0x101);

            d[0] = (dr
                .wrapping_mul(a)
                .wrapping_add((srgba.r as u32).wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[1] = (dg
                .wrapping_mul(a)
                .wrapping_add((srgba.g as u32).wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[2] = (db
                .wrapping_mul(a)
                .wrapping_add((srgba.b as u32).wrapping_mul(ma))
                / M
                >> 8) as u8;
            d[3] = (da
                .wrapping_mul(a)
                .wrapping_add((srgba.a as u32).wrapping_mul(ma))
                / M
                >> 8) as u8;
            i += di;
            sx += dx;
            mx += dx;
        }
        i0 += dy * dst.stride;
        y += dy;
        sy += dy;
        my += dy;
    }
}

// Go: image/draw/draw.go:drawRGBA
fn draw_rgba(
    dst: &mut RGBA,
    dst_ptr: *const (),
    r: Rectangle,
    src: &dyn crate::image::Image,
    sp: Point,
    mask: Option<&dyn crate::image::Image>,
    mp: Point,
    op: Op,
) {
    let (mut x0, mut x1, mut dx) = (r.min.x, r.max.x, 1i64);
    let (mut y0, mut y1, mut dy) = (r.min.y, r.max.y, 1i64);
    if same_image(dst_ptr, src as *const dyn crate::image::Image as *const ())
        && r.overlaps(r.add(sp.sub(r.min)))
    {
        if sp.y < r.min.y || sp.y == r.min.y && sp.x < r.min.x {
            (x0, x1, dx) = (x1 - 1, x0 - 1, -1);
            (y0, y1, dy) = (y1 - 1, y0 - 1, -1);
        }
    }

    let mut sy = sp.y + y0 - r.min.y;
    let mut my = mp.y + y0 - r.min.y;
    let sx0 = sp.x + x0 - r.min.x;
    let mx0 = mp.x + x0 - r.min.x;
    let sx1 = sx0 + (x1 - x0);
    let mut i0 = dst.pix_offset(x0, y0);
    let di = dx * 4;

    // Try the image.RGBA64Image interface, part of the standard library since
    // Go 1.17.
    //
    // This optimization is similar to how FALLBACK1.17 optimizes FALLBACK1.0
    // in DrawMask, except here the concrete type of dst is known to be
    // *image.RGBA.
    if let Some(src0) = src.as_rgba64_image() {
        if mask.is_none() {
            if op == Op::Over {
                let mut y = y0;
                while y != y1 {
                    let (mut i, mut sx) = (i0, sx0);
                    while sx != sx1 {
                        let srgba = src0.rgba64_at(sx, sy);
                        let iu = i as usize;
                        let d = &mut dst.pix[iu..iu + 4];
                        let dr = d[0] as u32;
                        let dg = d[1] as u32;
                        let db = d[2] as u32;
                        let da = d[3] as u32;
                        let a = M.wrapping_sub(srgba.a as u32).wrapping_mul(0x101);
                        d[0] = (dr.wrapping_mul(a) / M)
                            .wrapping_add(srgba.r as u32)
                            .wrapping_shr(8) as u8;
                        d[1] = (dg.wrapping_mul(a) / M)
                            .wrapping_add(srgba.g as u32)
                            .wrapping_shr(8) as u8;
                        d[2] = (db.wrapping_mul(a) / M)
                            .wrapping_add(srgba.b as u32)
                            .wrapping_shr(8) as u8;
                        d[3] = (da.wrapping_mul(a) / M)
                            .wrapping_add(srgba.a as u32)
                            .wrapping_shr(8) as u8;
                        i += di;
                        sx += dx;
                    }
                    i0 += dy * dst.stride;
                    y += dy;
                    sy += dy;
                    my += dy;
                }
            } else {
                let mut y = y0;
                while y != y1 {
                    let (mut i, mut sx) = (i0, sx0);
                    while sx != sx1 {
                        let srgba = src0.rgba64_at(sx, sy);
                        let iu = i as usize;
                        let d = &mut dst.pix[iu..iu + 4];
                        d[0] = (srgba.r >> 8) as u8;
                        d[1] = (srgba.g >> 8) as u8;
                        d[2] = (srgba.b >> 8) as u8;
                        d[3] = (srgba.a >> 8) as u8;
                        i += di;
                        sx += dx;
                    }
                    i0 += dy * dst.stride;
                    y += dy;
                    sy += dy;
                    my += dy;
                }
            }
            let _ = my;
            return;
        } else if let Some(mask0) = mask.and_then(|m| m.as_rgba64_image()) {
            if op == Op::Over {
                let mut y = y0;
                while y != y1 {
                    let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
                    while sx != sx1 {
                        let ma = mask0.rgba64_at(mx, my).a as u32;
                        let srgba = src0.rgba64_at(sx, sy);
                        let iu = i as usize;
                        let d = &mut dst.pix[iu..iu + 4];
                        let dr = d[0] as u32;
                        let dg = d[1] as u32;
                        let db = d[2] as u32;
                        let da = d[3] as u32;
                        let a = M
                            .wrapping_sub((srgba.a as u32).wrapping_mul(ma) / M)
                            .wrapping_mul(0x101);
                        d[0] = (dr
                            .wrapping_mul(a)
                            .wrapping_add((srgba.r as u32).wrapping_mul(ma))
                            / M
                            >> 8) as u8;
                        d[1] = (dg
                            .wrapping_mul(a)
                            .wrapping_add((srgba.g as u32).wrapping_mul(ma))
                            / M
                            >> 8) as u8;
                        d[2] = (db
                            .wrapping_mul(a)
                            .wrapping_add((srgba.b as u32).wrapping_mul(ma))
                            / M
                            >> 8) as u8;
                        d[3] = (da
                            .wrapping_mul(a)
                            .wrapping_add((srgba.a as u32).wrapping_mul(ma))
                            / M
                            >> 8) as u8;
                        i += di;
                        sx += dx;
                        mx += dx;
                    }
                    i0 += dy * dst.stride;
                    y += dy;
                    sy += dy;
                    my += dy;
                }
            } else {
                let mut y = y0;
                while y != y1 {
                    let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
                    while sx != sx1 {
                        let ma = mask0.rgba64_at(mx, my).a as u32;
                        let srgba = src0.rgba64_at(sx, sy);
                        let iu = i as usize;
                        let d = &mut dst.pix[iu..iu + 4];
                        d[0] = ((srgba.r as u32).wrapping_mul(ma) / M >> 8) as u8;
                        d[1] = ((srgba.g as u32).wrapping_mul(ma) / M >> 8) as u8;
                        d[2] = ((srgba.b as u32).wrapping_mul(ma) / M >> 8) as u8;
                        d[3] = ((srgba.a as u32).wrapping_mul(ma) / M >> 8) as u8;
                        i += di;
                        sx += dx;
                        mx += dx;
                    }
                    i0 += dy * dst.stride;
                    y += dy;
                    sy += dy;
                    my += dy;
                }
            }
            return;
        }
    }

    // Use the image.Image interface, part of the standard library since Go
    // 1.0.
    //
    // This is similar to FALLBACK1.0 in DrawMask, except here the concrete
    // type of dst is known to be *image.RGBA.
    let mut y = y0;
    while y != y1 {
        let (mut i, mut sx, mut mx) = (i0, sx0, mx0);
        while sx != sx1 {
            let mut ma = M;
            if let Some(mask) = mask {
                (_, _, _, ma) = mask.at(mx, my).rgba();
            }
            let (sr, sg, sb, sa) = src.at(sx, sy).rgba();
            let iu = i as usize;
            let d = &mut dst.pix[iu..iu + 4];
            if op == Op::Over {
                let dr = d[0] as u32;
                let dg = d[1] as u32;
                let db = d[2] as u32;
                let da = d[3] as u32;

                // See drawGrayMaskOver for the 0x101 trick.
                let a = M.wrapping_sub(sa.wrapping_mul(ma) / M).wrapping_mul(0x101);

                d[0] = (dr.wrapping_mul(a).wrapping_add(sr.wrapping_mul(ma)) / M >> 8) as u8;
                d[1] = (dg.wrapping_mul(a).wrapping_add(sg.wrapping_mul(ma)) / M >> 8) as u8;
                d[2] = (db.wrapping_mul(a).wrapping_add(sb.wrapping_mul(ma)) / M >> 8) as u8;
                d[3] = (da.wrapping_mul(a).wrapping_add(sa.wrapping_mul(ma)) / M >> 8) as u8;
            } else {
                d[0] = (sr.wrapping_mul(ma) / M >> 8) as u8;
                d[1] = (sg.wrapping_mul(ma) / M >> 8) as u8;
                d[2] = (sb.wrapping_mul(ma) / M >> 8) as u8;
                d[3] = (sa.wrapping_mul(ma) / M >> 8) as u8;
            }
            i += di;
            sx += dx;
            mx += dx;
        }
        i0 += dy * dst.stride;
        y += dy;
        sy += dy;
        my += dy;
    }
}

/// clamp clamps i to the interval [0, 0xffff].
///
/// Go: image/draw/draw.go:clamp
#[inline]
fn clamp(i: i32) -> i32 {
    if i < 0 {
        return 0;
    }
    if i > 0xffff {
        return 0xffff;
    }
    i
}

/// sqDiff returns the squared-difference of x and y, shifted by 2 so that
/// adding four of those won't overflow a uint32.
///
/// Go: image/draw/draw.go:sqDiff (wrapping uint32 arithmetic).
#[inline]
fn sq_diff(x: i32, y: i32) -> u32 {
    let d = x.wrapping_sub(y) as u32;
    d.wrapping_mul(d) >> 2
}

// Go: image/draw/draw.go:drawPaletted
fn draw_paletted(
    dst: &mut dyn Image,
    r: Rectangle,
    src: &dyn crate::image::Image,
    sp: Point,
    floyd_steinberg: bool,
) {
    // TODO(nigeltao): handle the case where the dst and src overlap.
    // Does it even make sense to try and do Floyd-Steinberg whilst
    // walking the image backward (right-to-left bottom-to-top)?

    // If dst is an *image.Paletted, we have a fast path for dst.Set and
    // dst.At. The dst.Set equivalent is a batch version of the algorithm
    // used by color.Palette's Index method in image/color/color.go, plus
    // optional Floyd-Steinberg error diffusion.
    let mut palette: Option<Vec<[i32; 4]>> = None;
    let (mut pix_off, mut stride) = (0usize, 0i64);
    if let Some(p) = dst.as_any().downcast_ref::<Paletted>() {
        let mut pal = vec![[0i32; 4]; p.palette.len()];
        for (i, col) in p.palette.iter().enumerate() {
            let (r, g, b, a) = col.rgba();
            pal[i][0] = r as i32;
            pal[i][1] = g as i32;
            pal[i][2] = b as i32;
            pal[i][3] = a as i32;
        }
        palette = Some(pal);
        (pix_off, stride) = (p.pix_offset(r.min.x, r.min.y) as usize, p.stride);
    }

    // quantErrorCurr and quantErrorNext are the Floyd-Steinberg quantization
    // errors that have been propagated to the pixels in the current and next
    // rows. The +2 simplifies calculation near the edges.
    let (mut quant_error_curr, mut quant_error_next): (Vec<[i32; 4]>, Vec<[i32; 4]>) =
        (Vec::new(), Vec::new());
    if floyd_steinberg {
        quant_error_curr = vec![[0i32; 4]; (r.dx() + 2) as usize];
        quant_error_next = vec![[0i32; 4]; (r.dx() + 2) as usize];
    }
    // Fast paths for special cases to avoid excessive use of the color.Color
    // interface which escapes to the heap but need to be discovered for
    // each pixel on r. See also https://golang.org/issues/15759.
    let src_any = src.as_any();
    let px_rgba: Box<dyn Fn(i64, i64) -> (u32, u32, u32, u32) + '_> =
        if let Some(src0) = src_any.downcast_ref::<RGBA>() {
            Box::new(move |x, y| src0.rgba_at(x, y).rgba())
        } else if let Some(src0) = src_any.downcast_ref::<NRGBA>() {
            Box::new(move |x, y| src0.nrgba_at(x, y).rgba())
        } else if let Some(src0) = src_any.downcast_ref::<YCbCr>() {
            Box::new(move |x, y| src0.ycbcr_at(x, y).rgba())
        } else {
            Box::new(move |x, y| src.at(x, y).rgba())
        };

    // For the Paletted fast path, borrow the destination's Pix once.
    let mut pal_dst: Option<&mut Paletted> = None;
    let mut other_dst: Option<&mut dyn Image> = None;
    if palette.is_some() {
        pal_dst = dst.as_any_mut().downcast_mut::<Paletted>();
    } else {
        other_dst = Some(dst);
    }

    // Loop over each source pixel.
    let mut out = color::RGBA64 {
        a: 0xffff,
        ..Default::default()
    };
    let (rdx, rdy) = (r.dx(), r.dy());
    let mut y = 0i64;
    while y != rdy {
        let mut x = 0i64;
        while x != rdx {
            // er, eg and eb are the pixel's R,G,B values plus the
            // optional Floyd-Steinberg error.
            let (sr, sg, sb, sa) = px_rgba(sp.x + x, sp.y + y);
            let (mut er, mut eg, mut eb, mut ea) = (sr as i32, sg as i32, sb as i32, sa as i32);
            let xu = x as usize;
            if floyd_steinberg {
                er = clamp(er.wrapping_add(quant_error_curr[xu + 1][0] / 16));
                eg = clamp(eg.wrapping_add(quant_error_curr[xu + 1][1] / 16));
                eb = clamp(eb.wrapping_add(quant_error_curr[xu + 1][2] / 16));
                ea = clamp(ea.wrapping_add(quant_error_curr[xu + 1][3] / 16));
            }

            if let Some(palette) = &palette {
                // Find the closest palette color in Euclidean R,G,B,A space:
                // the one that minimizes sum-squared-difference.
                // TODO(nigeltao): consider smarter algorithms.
                let (mut best_index, mut best_sum) = (0usize, u32::MAX);
                for (index, p) in palette.iter().enumerate() {
                    let sum = sq_diff(er, p[0])
                        .wrapping_add(sq_diff(eg, p[1]))
                        .wrapping_add(sq_diff(eb, p[2]))
                        .wrapping_add(sq_diff(ea, p[3]));
                    if sum < best_sum {
                        (best_index, best_sum) = (index, sum);
                        if sum == 0 {
                            break;
                        }
                    }
                }
                let pd = pal_dst.as_deref_mut().unwrap();
                pd.pix[pix_off + (y * stride + x) as usize] = best_index as u8;

                if !floyd_steinberg {
                    x += 1;
                    continue;
                }
                er = er.wrapping_sub(palette[best_index][0]);
                eg = eg.wrapping_sub(palette[best_index][1]);
                eb = eb.wrapping_sub(palette[best_index][2]);
                ea = ea.wrapping_sub(palette[best_index][3]);
            } else {
                out.r = er as u16;
                out.g = eg as u16;
                out.b = eb as u16;
                out.a = ea as u16;
                // The third argument is &out instead of out in Go.
                let od = other_dst.as_deref_mut().unwrap();
                od.set(r.min.x + x, r.min.y + y, Color::RGBA64(out));

                if !floyd_steinberg {
                    x += 1;
                    continue;
                }
                let (sr, sg, sb, sa) = od.at(r.min.x + x, r.min.y + y).rgba();
                er = er.wrapping_sub(sr as i32);
                eg = eg.wrapping_sub(sg as i32);
                eb = eb.wrapping_sub(sb as i32);
                ea = ea.wrapping_sub(sa as i32);
            }

            // Propagate the Floyd-Steinberg quantization error.
            let qn = &mut quant_error_next;
            qn[xu][0] = qn[xu][0].wrapping_add(er.wrapping_mul(3));
            qn[xu][1] = qn[xu][1].wrapping_add(eg.wrapping_mul(3));
            qn[xu][2] = qn[xu][2].wrapping_add(eb.wrapping_mul(3));
            qn[xu][3] = qn[xu][3].wrapping_add(ea.wrapping_mul(3));
            qn[xu + 1][0] = qn[xu + 1][0].wrapping_add(er.wrapping_mul(5));
            qn[xu + 1][1] = qn[xu + 1][1].wrapping_add(eg.wrapping_mul(5));
            qn[xu + 1][2] = qn[xu + 1][2].wrapping_add(eb.wrapping_mul(5));
            qn[xu + 1][3] = qn[xu + 1][3].wrapping_add(ea.wrapping_mul(5));
            qn[xu + 2][0] = qn[xu + 2][0].wrapping_add(er);
            qn[xu + 2][1] = qn[xu + 2][1].wrapping_add(eg);
            qn[xu + 2][2] = qn[xu + 2][2].wrapping_add(eb);
            qn[xu + 2][3] = qn[xu + 2][3].wrapping_add(ea);
            let qc = &mut quant_error_curr;
            qc[xu + 2][0] = qc[xu + 2][0].wrapping_add(er.wrapping_mul(7));
            qc[xu + 2][1] = qc[xu + 2][1].wrapping_add(eg.wrapping_mul(7));
            qc[xu + 2][2] = qc[xu + 2][2].wrapping_add(eb.wrapping_mul(7));
            qc[xu + 2][3] = qc[xu + 2][3].wrapping_add(ea.wrapping_mul(7));
            x += 1;
        }

        // Recycle the quantization error buffers.
        if floyd_steinberg {
            std::mem::swap(&mut quant_error_curr, &mut quant_error_next);
            for e in quant_error_next.iter_mut() {
                *e = [0; 4];
            }
        }
        y += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: image/draw/draw_test.go:TestSqDiff
    #[test]
    fn test_sq_diff() {
        let orig = |x: i32, y: i32| -> u32 {
            let d = if x > y {
                x.wrapping_sub(y) as u32
            } else {
                y.wrapping_sub(x) as u32
            };
            d.wrapping_mul(d) >> 2
        };
        let cases: [i32; 15] = [
            0,
            1,
            2,
            0x0fffd,
            0x0fffe,
            0x0ffff,
            0x10000,
            0x10001,
            0x10002,
            0x7ffffffd,
            0x7ffffffe,
            0x7fffffff,
            -0x7ffffffd,
            -0x7ffffffe,
            i32::MIN,
        ];
        for &x in &cases {
            for &y in &cases {
                assert_eq!(sq_diff(x, y), orig(x, y), "sqDiff({:#x}, {:#x})", x, y);
            }
        }
        let mut s = 1u64;
        for _ in 0..100000 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let (x, y) = (s as i32, (s >> 32) as i32);
            assert_eq!(sq_diff(x, y), orig(x, y));
        }
    }

    // Go: image/draw/clip_test.go:TestClip
    #[test]
    fn test_clip() {
        use crate::geom::{pt, rect};
        struct ClipTest {
            desc: &'static str,
            r: Rectangle,
            dr: Rectangle,
            sr: Rectangle,
            mr: Rectangle,
            sp: Point,
            mp: Point,
            nil_mask: bool,
            r0: Rectangle,
            sp0: Point,
            mp0: Point,
        }
        let z = Rectangle::default();
        let o = Point::default();
        let clip_tests = [
            // The following tests all have a nil mask.
            ClipTest {
                desc: "basic",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 100, 100),
                sr: rect(0, 0, 100, 100),
                mr: z,
                sp: o,
                mp: o,
                nil_mask: true,
                r0: rect(0, 0, 100, 100),
                sp0: o,
                mp0: o,
            },
            ClipTest {
                desc: "clip dr",
                r: rect(0, 0, 100, 100),
                dr: rect(40, 40, 60, 60),
                sr: rect(0, 0, 100, 100),
                mr: z,
                sp: o,
                mp: o,
                nil_mask: true,
                r0: rect(40, 40, 60, 60),
                sp0: pt(40, 40),
                mp0: o,
            },
            ClipTest {
                desc: "clip sr",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 100, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: o,
                mp: o,
                nil_mask: true,
                r0: rect(20, 20, 80, 80),
                sp0: pt(20, 20),
                mp0: o,
            },
            ClipTest {
                desc: "clip dr and sr",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 50, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: o,
                mp: o,
                nil_mask: true,
                r0: rect(20, 20, 50, 80),
                sp0: pt(20, 20),
                mp0: o,
            },
            ClipTest {
                desc: "clip dr and sr, sp outside sr (top-left)",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 50, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: pt(15, 8),
                mp: o,
                nil_mask: true,
                r0: rect(5, 12, 50, 72),
                sp0: pt(20, 20),
                mp0: o,
            },
            ClipTest {
                desc: "clip dr and sr, sp outside sr (middle-left)",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 50, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: pt(15, 66),
                mp: o,
                nil_mask: true,
                r0: rect(5, 0, 50, 14),
                sp0: pt(20, 66),
                mp0: o,
            },
            ClipTest {
                desc: "clip dr and sr, sp outside sr (bottom-left)",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 50, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: pt(15, 91),
                mp: o,
                nil_mask: true,
                r0: z,
                sp0: pt(15, 91),
                mp0: o,
            },
            ClipTest {
                desc: "clip dr and sr, sp inside sr",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 50, 100),
                sr: rect(20, 20, 80, 80),
                mr: z,
                sp: pt(44, 33),
                mp: o,
                nil_mask: true,
                r0: rect(0, 0, 36, 47),
                sp0: pt(44, 33),
                mp0: o,
            },
            // The following tests all have a non-nil mask.
            ClipTest {
                desc: "basic mask",
                r: rect(0, 0, 80, 80),
                dr: rect(20, 0, 100, 80),
                sr: rect(0, 0, 50, 49),
                mr: rect(0, 0, 46, 47),
                sp: o,
                mp: o,
                nil_mask: false,
                r0: rect(20, 0, 46, 47),
                sp0: pt(20, 0),
                mp0: pt(20, 0),
            },
            ClipTest {
                desc: "clip sr and mr",
                r: rect(0, 0, 100, 100),
                dr: rect(0, 0, 100, 100),
                sr: rect(23, 23, 55, 86),
                mr: rect(44, 44, 87, 58),
                sp: pt(10, 10),
                mp: pt(11, 11),
                nil_mask: false,
                r0: rect(33, 33, 45, 47),
                sp0: pt(43, 43),
                mp0: pt(44, 44),
            },
        ];
        let dst0 = RGBA::new(rect(0, 0, 100, 100));
        let src0 = RGBA::new(rect(0, 0, 100, 100));
        let mask0 = RGBA::new(rect(0, 0, 100, 100));
        for c in &clip_tests {
            let dst = dst0.sub_image(c.dr);
            let src = src0.sub_image(c.sr);
            let (mut r, mut sp, mut mp) = (c.r, c.sp, c.mp);
            if c.nil_mask {
                clip(dst.bounds(), &mut r, &src, &mut sp, None, None);
            } else {
                let mask = mask0.sub_image(c.mr);
                clip(
                    dst.bounds(),
                    &mut r,
                    &src,
                    &mut sp,
                    Some(&mask),
                    Some(&mut mp),
                );
            }

            // Check that the actual results equal the expected results.
            assert!(
                c.r0.eq(r),
                "{}: clip rectangle want {} got {}",
                c.desc,
                c.r0,
                r
            );
            assert!(c.sp0.eq(sp), "{}: sp want {} got {}", c.desc, c.sp0, sp);
            if !c.nil_mask {
                assert!(c.mp0.eq(mp), "{}: mp want {} got {}", c.desc, c.mp0, mp);
            }

            // Check that the clipped rectangle is contained by the dst / src /
            // mask rectangles, in their respective coordinate spaces.
            assert!(
                r.in_(c.dr),
                "{}: c.dr {} does not contain r {}",
                c.desc,
                c.dr,
                r
            );
            // sr is r translated into src's coordinate space.
            let sr = r.add(c.sp.sub(c.dr.min));
            assert!(
                sr.in_(c.sr),
                "{}: c.sr {} does not contain sr {}",
                c.desc,
                c.sr,
                sr
            );
            if !c.nil_mask {
                // mr is r translated into mask's coordinate space.
                let mr = r.add(c.mp.sub(c.dr.min));
                assert!(
                    mr.in_(c.mr),
                    "{}: c.mr {} does not contain mr {}",
                    c.desc,
                    c.mr,
                    mr
                );
            }
        }
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(-1), 0);
        assert_eq!(clamp(0x10000), 0xffff);
        assert_eq!(clamp(1234), 1234);
    }
}
