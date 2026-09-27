//! Port of Go 1.27.1 `image/internal/imageutil/impl.go` (generated code).

use crate::geom::{Point, Rectangle};
use crate::image::RGBA;
use crate::ycbcr::{YCbCr, YCbCrSubsampleRatio};

// Inline version of image/color/ycbcr.go's func YCbCrToRGB, writing the four
// RGBA bytes. The bit twiddling is equivalent to clamping (v >> 16) to
// [0, 0xff].
#[inline(always)]
fn ycbcr_px(rgba: &mut [u8], y: u8, cb: u8, cr: u8) {
    let yy1 = (y as i32) * 0x10101;
    let cb1 = (cb as i32) - 128;
    let cr1 = (cr as i32) - 128;

    let mut r = yy1 + 91881 * cr1;
    if (r as u32) & 0xff000000 == 0 {
        r >>= 16;
    } else {
        r = !(r >> 31);
    }

    let mut g = yy1 - 22554 * cb1 - 46802 * cr1;
    if (g as u32) & 0xff000000 == 0 {
        g >>= 16;
    } else {
        g = !(g >> 31);
    }

    let mut b = yy1 + 116130 * cb1;
    if (b as u32) & 0xff000000 == 0 {
        b >>= 16;
    } else {
        b = !(b >> 31);
    }

    rgba[0] = r as u8;
    rgba[1] = g as u8;
    rgba[2] = b as u8;
    rgba[3] = 255;
}

/// DrawYCbCr draws the YCbCr source image on the RGBA destination image with
/// r.Min in dst aligned with sp in src. It reports whether the draw was
/// successful. If it returns false, no dst pixels were changed.
///
/// This function assumes that r is entirely within dst's bounds and the
/// translation of r from dst coordinate space to src coordinate space is
/// entirely within src's bounds.
///
/// Go: image/internal/imageutil/impl.go:DrawYCbCr
pub(crate) fn draw_ycbcr(dst: &mut RGBA, r: Rectangle, src: &YCbCr, sp: Point) -> bool {
    let x0 = (r.min.x - dst.rect.min.x) * 4;
    let x1 = (r.max.x - dst.rect.min.x) * 4;
    let y0 = r.min.y - dst.rect.min.y;
    let y1 = r.max.y - dst.rect.min.y;
    let stride = dst.stride;
    match src.subsample_ratio {
        YCbCrSubsampleRatio::Ratio444 => {
            let (mut y, mut sy) = (y0, sp.y);
            while y != y1 {
                let dpix = &mut dst.pix[(y * stride) as usize..];
                let yi = (sy - src.rect.min.y) * src.y_stride + (sp.x - src.rect.min.x);
                let ci = (sy - src.rect.min.y) * src.c_stride + (sp.x - src.rect.min.x);
                // for x := x0; x != x1; x, yi, ci = x+4, yi+1, ci+1
                let n = ((x1 - x0) / 4) as usize;
                let (yi, ci) = (yi as usize, ci as usize);
                let d = &mut dpix[x0 as usize..x0 as usize + 4 * n];
                let ys = &src.y[yi..yi + n];
                let cbs = &src.cb[ci..ci + n];
                let crs = &src.cr[ci..ci + n];
                let (d, _) = d.as_chunks_mut::<4>();
                for (((px, &yy), &cb), &cr) in d.iter_mut().zip(ys).zip(cbs).zip(crs) {
                    ycbcr_px(px, yy, cb, cr);
                }
                y += 1;
                sy += 1;
            }
        }

        YCbCrSubsampleRatio::Ratio422 => {
            let (mut y, mut sy) = (y0, sp.y);
            while y != y1 {
                let dpix = &mut dst.pix[(y * stride) as usize..];
                let mut yi = (sy - src.rect.min.y) * src.y_stride + (sp.x - src.rect.min.x);
                let ci_base = (sy - src.rect.min.y) * src.c_stride - src.rect.min.x / 2;
                let (mut x, mut sx) = (x0, sp.x);
                while x != x1 {
                    let ci = ci_base + sx / 2;
                    ycbcr_px(
                        &mut dpix[x as usize..x as usize + 4],
                        src.y[yi as usize],
                        src.cb[ci as usize],
                        src.cr[ci as usize],
                    );
                    x += 4;
                    sx += 1;
                    yi += 1;
                }
                y += 1;
                sy += 1;
            }
        }

        YCbCrSubsampleRatio::Ratio420 => {
            let (mut y, mut sy) = (y0, sp.y);
            while y != y1 {
                let dpix = &mut dst.pix[(y * stride) as usize..];
                let yi = (sy - src.rect.min.y) * src.y_stride + (sp.x - src.rect.min.x);
                let ci_base = (sy / 2 - src.rect.min.y / 2) * src.c_stride - src.rect.min.x / 2;
                // for x, sx := x0, sp.X; x != x1; x, sx, yi = x+4, sx+1, yi+1
                let n = ((x1 - x0) / 4) as usize;
                let d = &mut dpix[x0 as usize..x0 as usize + 4 * n];
                let ys = &src.y[yi as usize..yi as usize + n];
                let (d, _) = d.as_chunks_mut::<4>();
                for (sx, (px, &yy)) in (sp.x..).zip(d.iter_mut().zip(ys)) {
                    let ci = (ci_base + sx / 2) as usize;
                    ycbcr_px(px, yy, src.cb[ci], src.cr[ci]);
                }
                y += 1;
                sy += 1;
            }
        }

        YCbCrSubsampleRatio::Ratio440 => {
            let (mut y, mut sy) = (y0, sp.y);
            while y != y1 {
                let dpix = &mut dst.pix[(y * stride) as usize..];
                let mut yi = (sy - src.rect.min.y) * src.y_stride + (sp.x - src.rect.min.x);
                let mut ci = (sy / 2 - src.rect.min.y / 2) * src.c_stride + (sp.x - src.rect.min.x);
                let mut x = x0;
                while x != x1 {
                    ycbcr_px(
                        &mut dpix[x as usize..x as usize + 4],
                        src.y[yi as usize],
                        src.cb[ci as usize],
                        src.cr[ci as usize],
                    );
                    x += 4;
                    yi += 1;
                    ci += 1;
                }
                y += 1;
                sy += 1;
            }
        }

        _ => return false,
    }
    true
}
