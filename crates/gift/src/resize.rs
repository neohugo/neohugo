//! Port of gift v1.2.1 `resize.go`: resampling filters, weight preparation,
//! separable resizing and the Resize / ResizeToFit / ResizeToFill filters.
//!
//! arm64 fusion (from `go tool objdump`, see PORTING.md): the resample
//! `center` is `FNMSUBS` (`(i+0.5)*delta - 0.5` rounded once), the
//! `resizeLine` accumulations are `FMADDS` (alpha from the unrounded
//! `c.a*w.weight`), and `bcspline` has the fused sites written out below.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use go_image::{Image, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::gomath;
use crate::pixels::{Pixel, PixelGetter, PixelSetter};
use crate::transform::{Anchor, crop_to_size};
use crate::utils::{add_sub, copyimage, create_temp_image, maxint, minint, parallelize};

/// Resampling is an interpolation algorithm used for image resizing.
///
/// Go: resize.go:Resampling. `Any` is a supertrait so a `&dyn Resampling`
/// can be downcast (e.g. to [`Resamp`] for its name).
pub trait Resampling: Any + Send + Sync {
    fn support(&self) -> f32;
    fn kernel(&self, x: f32) -> f32;
}

/// Go: resize.go:bcspline, as compiled for arm64 (a standalone function; the
/// kernels call it, it is not inlined into them).
pub fn bcspline(x: f32, b: f32, c: f32) -> f32 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x < 1.0 {
        // ((12-9*b-6*c)*x*x*x + (-18+12*b+6*c)*x*x + (6 - 2*b)) / 6
        let k3 = (-c).mul_add(6.0, (-b).mul_add(9.0, 12.0));
        let t3 = k3 * x * x;
        let k2 = c.mul_add(6.0, b.mul_add(12.0, -18.0));
        let t2 = k2 * x * x;
        let s = t3.mul_add(x, t2);
        let k0 = 6.0 - (b + b);
        return (k0 + s) / 6.0;
    }
    if x < 2.0 {
        // ((-b-6*c)*x*x*x + (6*b+30*c)*x*x + (-12*b-48*c)*x + (8*b + 24*c)) / 6
        let k3 = (-c).mul_add(6.0, -b);
        let t3 = k3 * x * x;
        let k2 = b.mul_add(6.0, 30.0 * c);
        let t2 = k2 * x * x;
        let s1 = t3.mul_add(x, t2);
        let k1 = (-c).mul_add(48.0, -12.0 * b);
        let s2 = k1.mul_add(x, s1);
        let k0 = b.mul_add(8.0, 24.0 * c);
        return (s2 + k0) / 6.0;
    }
    0.0
}

/// Go: resize.go:sinc
pub fn sinc(x: f32) -> f32 {
    if x == 0.0 {
        return 1.0;
    }
    let px = std::f64::consts::PI * x as f64;
    (gomath::sin(px) / px) as f32
}

/// Go: resize.go:resamp (and neohugo resources/images/resampling.go:resamp,
/// which has the same shape).
#[derive(Clone, Copy)]
pub struct Resamp {
    pub name: &'static str,
    pub support: f32,
    pub kernel: fn(f32) -> f32,
}

impl fmt::Display for Resamp {
    // Go: resize.go:resamp.String
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Debug for Resamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resamp")
            .field("name", &self.name)
            .field("support", &self.support)
            .finish()
    }
}

impl Resampling for Resamp {
    // Go: resize.go:resamp.Support
    fn support(&self) -> f32 {
        self.support
    }

    // Go: resize.go:resamp.Kernel
    fn kernel(&self, x: f32) -> f32 {
        (self.kernel)(x)
    }
}

// Go: resize.go:init (the kernel closures).
fn nearest_neighbor_kernel(_x: f32) -> f32 {
    0.0
}

fn box_kernel(x: f32) -> f32 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x <= 0.5 {
        return 1.0;
    }
    0.0
}

fn linear_kernel(x: f32) -> f32 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x < 1.0 {
        return 1.0 - x;
    }
    0.0
}

fn cubic_kernel(x: f32) -> f32 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x < 2.0 {
        return bcspline(x, 0.0, 0.5);
    }
    0.0
}

fn lanczos_kernel(x: f32) -> f32 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x < 3.0 {
        return sinc(x) * sinc(x / 3.0);
    }
    0.0
}

/// NearestNeighborResampling is a nearest neighbor resampling filter.
pub static NEAREST_NEIGHBOR_RESAMPLING: Resamp = Resamp {
    name: "NearestNeighborResampling",
    support: 0.0,
    kernel: nearest_neighbor_kernel,
};

/// BoxResampling is a box resampling filter (average of surrounding pixels).
pub static BOX_RESAMPLING: Resamp = Resamp {
    name: "BoxResampling",
    support: 0.5,
    kernel: box_kernel,
};

/// LinearResampling is a bilinear resampling filter.
pub static LINEAR_RESAMPLING: Resamp = Resamp {
    name: "LinearResampling",
    support: 1.0,
    kernel: linear_kernel,
};

/// CubicResampling is a bicubic resampling filter (Catmull-Rom).
pub static CUBIC_RESAMPLING: Resamp = Resamp {
    name: "CubicResampling",
    support: 2.0,
    kernel: cubic_kernel,
};

/// LanczosResampling is a Lanczos resampling filter (3 lobes).
pub static LANCZOS_RESAMPLING: Resamp = Resamp {
    name: "LanczosResampling",
    support: 3.0,
    kernel: lanczos_kernel,
};

/// Go: resize.go:resampWeight
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResampWeight {
    pub index: i64,
    pub weight: f32,
}

/// Go: resize.go:prepareResampWeights
pub(crate) fn prepare_resamp_weights(
    dst_size: i64,
    src_size: i64,
    resampling: &dyn Resampling,
) -> Vec<Vec<ResampWeight>> {
    let delta = src_size as f32 / dst_size as f32;
    let mut scale = delta;
    if scale < 1.0 {
        scale = 1.0;
    }
    let radius = ((scale * resampling.support()) as f64).ceil() as f32;

    let mut result = Vec::with_capacity(dst_size.max(0) as usize);

    for i in 0..dst_size {
        // (float32(i)+0.5)*delta - 0.5: FNMSUBS, one rounding.
        let center = (i as f32 + 0.5).mul_add(delta, -0.5);

        let mut left = ((center - radius) as f64).ceil() as i64;
        if left < 0 {
            left = 0;
        }
        let mut right = ((center + radius) as f64).floor() as i64;
        if right > src_size - 1 {
            right = src_size - 1;
        }

        let mut tmp = Vec::new();
        let mut sum: f32 = 0.0;
        let mut j = left;
        while j <= right {
            let weight = resampling.kernel((j as f32 - center) / scale);
            if weight != 0.0 {
                tmp.push(ResampWeight { index: j, weight });
                sum += weight;
            }
            j += 1;
        }

        for w in tmp.iter_mut() {
            w.weight /= sum;
        }

        result.push(tmp);
    }

    result
}

/// Go: resize.go:resizeLine
pub(crate) fn resize_line(dst: &mut [Pixel], src: &[Pixel], weights: &[Vec<ResampWeight>]) {
    for i in 0..dst.len() {
        let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
        for w in &weights[i] {
            let c = src[w.index as usize];
            let wa = c.a * w.weight;
            r = c.r.mul_add(wa, r);
            g = c.g.mul_add(wa, g);
            b = c.b.mul_add(wa, b);
            a = c.a.mul_add(w.weight, a);
        }
        if a != 0.0 {
            r /= a;
            g /= a;
            b /= a;
        }
        dst[i] = Pixel::new(r, g, b, a);
    }
}

/// Go: resize.go:resizeHorizontal
pub(crate) fn resize_horizontal(
    dst: &mut dyn draw::Image,
    src: &dyn Image,
    w: i64,
    resampling: &dyn Resampling,
    options: &Options,
) {
    let srcb = src.bounds();
    let dstb = dst.bounds();

    let weights = prepare_resamp_weights(w, srcb.dx(), resampling);

    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);

    parallelize(
        options.parallelization,
        srcb.min.y,
        srcb.max.y,
        |start, stop| {
            let mut src_buf = vec![Pixel::default(); srcb.dx() as usize];
            let mut dst_buf = vec![Pixel::default(); w as usize];
            for srcy in start..stop {
                pix_getter.get_pixel_row(srcy, &mut src_buf);
                resize_line(&mut dst_buf, &src_buf, &weights);
                pix_setter.set_pixel_row(add_sub(dstb.min.y, srcy, srcb.min.y), &dst_buf);
            }
        },
    );
}

/// Go: resize.go:resizeVertical
pub(crate) fn resize_vertical(
    dst: &mut dyn draw::Image,
    src: &dyn Image,
    h: i64,
    resampling: &dyn Resampling,
    options: &Options,
) {
    let srcb = src.bounds();
    let dstb = dst.bounds();

    let weights = prepare_resamp_weights(h, srcb.dy(), resampling);

    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);

    parallelize(
        options.parallelization,
        srcb.min.x,
        srcb.max.x,
        |start, stop| {
            let mut src_buf = vec![Pixel::default(); srcb.dy() as usize];
            let mut dst_buf = vec![Pixel::default(); h as usize];
            for srcx in start..stop {
                pix_getter.get_pixel_column(srcx, &mut src_buf);
                resize_line(&mut dst_buf, &src_buf, &weights);
                pix_setter.set_pixel_column(add_sub(dstb.min.x, srcx, srcb.min.x), &dst_buf);
            }
        },
    );
}

/// Go: resize.go:resizeNearest
pub(crate) fn resize_nearest(
    dst: &mut dyn draw::Image,
    src: &dyn Image,
    w: i64,
    h: i64,
    options: &Options,
) {
    let srcb = src.bounds();
    let dstb = dst.bounds();
    let dx = srcb.dx() as f64 / w as f64;
    let dy = srcb.dy() as f64 / h as f64;

    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);

    parallelize(
        options.parallelization,
        dstb.min.y,
        dstb.min.y.wrapping_add(h),
        |start, stop| {
            for dsty in start..stop {
                for dstx in dstb.min.x..dstb.min.x.wrapping_add(w) {
                    let fx = ((dstx.wrapping_sub(dstb.min.x) as f64 + 0.5) * dx).floor();
                    let fy = ((dsty.wrapping_sub(dstb.min.y) as f64 + 0.5) * dy).floor();
                    let srcx = srcb.min.x + fx as i64;
                    let srcy = srcb.min.y + fy as i64;
                    let px = pix_getter.get_pixel(srcx, srcy);
                    pix_setter.set_pixel(dstx, dsty, px);
                }
            }
        },
    );
}

/// Go: resize.go:resizeFilter
#[derive(Clone, Copy)]
pub struct ResizeFilter {
    pub width: i64,
    pub height: i64,
    pub resampling: &'static dyn Resampling,
}

impl Filter for ResizeFilter {
    // Go: resize.go:(*resizeFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let (w, h) = (self.width, self.height);
        let (srcw, srch) = (src_bounds.dx(), src_bounds.dy());

        if (w == 0 && h == 0) || w < 0 || h < 0 || srcw <= 0 || srch <= 0 {
            rect(0, 0, 0, 0)
        } else if w == 0 {
            let fw = h as f64 * srcw as f64 / srch as f64;
            let dstw = gomath::max(1.0, (fw + 0.5).floor()) as i64;
            rect(0, 0, dstw, h)
        } else if h == 0 {
            let fh = w as f64 * srch as f64 / srcw as f64;
            let dsth = gomath::max(1.0, (fh + 0.5).floor()) as i64;
            rect(0, 0, w, dsth)
        } else {
            rect(0, 0, w, h)
        }
    }

    // Go: resize.go:(*resizeFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let b = self.bounds(src.bounds());
        let (w, h) = (b.dx(), b.dy());

        if w <= 0 || h <= 0 {
            return;
        }

        if src.bounds().dx() == w && src.bounds().dy() == h {
            copyimage(dst, src, Some(options));
            return;
        }

        if self.resampling.support() <= 0.0 {
            resize_nearest(dst, src, w, h, options);
            return;
        }

        if src.bounds().dx() == w {
            resize_vertical(dst, src, h, self.resampling, options);
            return;
        }

        if src.bounds().dy() == h {
            resize_horizontal(dst, src, w, self.resampling, options);
            return;
        }

        let mut tmp = create_temp_image(rect(0, 0, w, src.bounds().dy()));
        resize_horizontal(&mut tmp, src, w, self.resampling, options);
        resize_vertical(dst, &tmp, h, self.resampling, options);
    }
}

/// Resize creates a filter that resizes an image to the specified width and
/// height using the specified resampling. If one of width or height is 0,
/// the image aspect ratio is preserved.
///
/// Go: resize.go:Resize
pub fn resize(width: i64, height: i64, resampling: &'static dyn Resampling) -> Arc<dyn Filter> {
    Arc::new(ResizeFilter {
        width,
        height,
        resampling,
    })
}

/// Go: resize.go:resizeToFitFilter
#[derive(Clone, Copy)]
pub struct ResizeToFitFilter {
    pub width: i64,
    pub height: i64,
    pub resampling: &'static dyn Resampling,
}

impl Filter for ResizeToFitFilter {
    // Go: resize.go:(*resizeToFitFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let (w, h) = (self.width, self.height);
        let (srcw, srch) = (src_bounds.dx(), src_bounds.dy());

        if w <= 0 || h <= 0 || srcw <= 0 || srch <= 0 {
            return rect(0, 0, 0, 0);
        }

        if srcw <= w && srch <= h {
            return rect(0, 0, srcw, srch);
        }

        let wratio = srcw as f64 / w as f64;
        let hratio = srch as f64 / h as f64;

        let (dstw, dsth);
        if wratio > hratio {
            dstw = w;
            dsth = minint((srch as f64 / wratio + 0.5) as i64, h);
        } else {
            dsth = h;
            dstw = minint((srcw as f64 / hratio + 0.5) as i64, w);
        }

        rect(0, 0, dstw, dsth)
    }

    // Go: resize.go:(*resizeToFitFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let b = self.bounds(src.bounds());
        ResizeFilter {
            width: b.dx(),
            height: b.dy(),
            resampling: self.resampling,
        }
        .draw(dst, src, options);
    }
}

/// ResizeToFit creates a filter that resizes an image to fit within the
/// specified dimensions while preserving the aspect ratio.
///
/// Go: resize.go:ResizeToFit
pub fn resize_to_fit(
    width: i64,
    height: i64,
    resampling: &'static dyn Resampling,
) -> Arc<dyn Filter> {
    Arc::new(ResizeToFitFilter {
        width,
        height,
        resampling,
    })
}

/// Go: resize.go:resizeToFillFilter
#[derive(Clone, Copy)]
pub struct ResizeToFillFilter {
    pub width: i64,
    pub height: i64,
    pub anchor: Anchor,
    pub resampling: &'static dyn Resampling,
}

impl Filter for ResizeToFillFilter {
    // Go: resize.go:(*resizeToFillFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        let (w, h) = (self.width, self.height);
        let (srcw, srch) = (src_bounds.dx(), src_bounds.dy());

        if w <= 0 || h <= 0 || srcw <= 0 || srch <= 0 {
            return rect(0, 0, 0, 0);
        }

        rect(0, 0, w, h)
    }

    // Go: resize.go:(*resizeToFillFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let b = self.bounds(src.bounds());
        let (w, h) = (b.dx(), b.dy());

        if w <= 0 || h <= 0 {
            return;
        }

        let (srcw, srch) = (src.bounds().dx(), src.bounds().dy());

        let wratio = srcw as f64 / w as f64;
        let hratio = srch as f64 / h as f64;

        let (tmpw, tmph);
        if wratio < hratio {
            tmpw = w;
            tmph = maxint((srch as f64 / wratio + 0.5) as i64, h);
        } else {
            tmph = h;
            tmpw = maxint((srcw as f64 / hratio + 0.5) as i64, w);
        }

        let mut tmp = create_temp_image(rect(0, 0, tmpw, tmph));
        ResizeFilter {
            width: tmpw,
            height: tmph,
            resampling: self.resampling,
        }
        .draw(&mut tmp, src, options);
        crop_to_size(w, h, self.anchor).draw(dst, &tmp, options);
    }
}

/// ResizeToFill creates a filter that resizes an image to the smallest
/// possible size that will cover the specified dimensions, then crops the
/// resized image to the specified dimensions using the specified anchor point.
///
/// Go: resize.go:ResizeToFill
pub fn resize_to_fill(
    width: i64,
    height: i64,
    resampling: &'static dyn Resampling,
    anchor: Anchor,
) -> Arc<dyn Filter> {
    Arc::new(ResizeToFillFilter {
        width,
        height,
        anchor,
        resampling,
    })
}
