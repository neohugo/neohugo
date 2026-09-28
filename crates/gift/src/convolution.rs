//! Port of gift v1.2.1 `convolution.go`: Convolution, GaussianBlur,
//! UnsharpMask, Mean and Sobel.
//!
//! arm64 fusion (from `go tool objdump`, see PORTING.md): the weighted sums in
//! convolutionFilter.Draw and convolveLine are FMADDS (convolveLine's alpha
//! from the unrounded `c.a*w.weight`), unsharp's `orig + dif` is
//! `orig + (orig-blurred)*amount` fused, and Sobel's `h*h + v*v` fuses one
//! square (the vertical one for red, the horizontal one for green and blue).

use std::sync::Arc;

use go_image::{Image, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::gomath;
use crate::pixels::{Pixel, PixelGetter, PixelSetter};
use crate::utils::{absf32, add_sub, copyimage, create_temp_image, gen_disk, parallelize, sqrtf32};

/// Go: convolution.go:uweight
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UWeight {
    pub u: i64,
    pub weight: f32,
}

/// Go: convolution.go:uvweight
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UVWeight {
    pub u: i64,
    pub v: i64,
    pub weight: f32,
}

/// Go: convolution.go:prepareConvolutionWeights
pub(crate) fn prepare_convolution_weights(kernel: &[f32], normalize: bool) -> (i64, Vec<UVWeight>) {
    let mut size = (kernel.len() as f64).sqrt() as i64;
    if size % 2 == 0 {
        size -= 1;
    }
    if size < 1 {
        return (0, Vec::new());
    }
    let center = size / 2;

    let mut weights = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let k = j * size + i;
            let mut w = 0f32;
            if k < kernel.len() as i64 {
                w = kernel[k as usize];
            }
            if w != 0.0 {
                weights.push(UVWeight {
                    u: i - center,
                    v: j - center,
                    weight: w,
                });
            }
        }
    }

    if !normalize {
        return (size, weights);
    }

    let (mut sum, mut sumpositive) = (0f32, 0f32);
    for w in &weights {
        sum += w.weight;
        if w.weight > 0.0 {
            sumpositive += w.weight;
        }
    }

    let div;
    if sum != 0.0 {
        div = sum;
    } else if sumpositive != 0.0 {
        div = sumpositive;
    } else {
        return (size, weights);
    }

    for w in weights.iter_mut() {
        w.weight /= div;
    }

    (size, weights)
}

/// Go: convolution.go:convolutionFilter
#[derive(Clone, Debug)]
pub struct ConvolutionFilter {
    pub kernel: Vec<f32>,
    pub normalize: bool,
    pub alpha: bool,
    pub abs: bool,
    pub delta: f32,
}

impl Filter for ConvolutionFilter {
    // Go: convolution.go:(*convolutionFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: convolution.go:(*convolutionFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        let (ksize, weights) = prepare_convolution_weights(&self.kernel, self.normalize);
        let kcenter = ksize / 2;

        if ksize < 1 {
            copyimage(dst, src, Some(options));
            return;
        }

        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                // Init temporary rows.
                let starty = start;
                let mut rows: Vec<Vec<Pixel>> = Vec::with_capacity(ksize as usize);
                for i in 0..ksize {
                    let mut rowy = add_sub(starty, i, kcenter);
                    if rowy < srcb.min.y {
                        rowy = srcb.min.y;
                    } else if rowy > srcb.max.y - 1 {
                        rowy = srcb.max.y - 1;
                    }
                    let mut row = vec![Pixel::default(); srcb.dx() as usize];
                    pix_getter.get_pixel_row(rowy, &mut row);
                    rows.push(row);
                }

                for y in start..stop {
                    // Calculate dst row.
                    for x in srcb.min.x..srcb.max.x {
                        let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
                        for w in &weights {
                            let mut wx = x.wrapping_add(w.u);
                            if wx < srcb.min.x {
                                wx = srcb.min.x;
                            } else if wx > srcb.max.x - 1 {
                                wx = srcb.max.x - 1;
                            }
                            let rowsx = wx - srcb.min.x;
                            let rowsy = kcenter + w.v;

                            let px = rows[rowsy as usize][rowsx as usize];
                            r = px.r.mul_add(w.weight, r);
                            g = px.g.mul_add(w.weight, g);
                            b = px.b.mul_add(w.weight, b);
                            if self.alpha {
                                a = px.a.mul_add(w.weight, a);
                            }
                        }
                        if self.abs {
                            r = absf32(r);
                            g = absf32(g);
                            b = absf32(b);
                            if self.alpha {
                                a = absf32(a);
                            }
                        }
                        if self.delta != 0.0 {
                            r += self.delta;
                            g += self.delta;
                            b += self.delta;
                            if self.alpha {
                                a += self.delta;
                            }
                        }
                        if !self.alpha {
                            a = rows[kcenter as usize][(x - srcb.min.x) as usize].a;
                        }
                        pix_setter.set_pixel(
                            add_sub(dstb.min.x, x, srcb.min.x),
                            add_sub(dstb.min.y, y, srcb.min.y),
                            Pixel::new(r, g, b, a),
                        );
                    }

                    // Rotate temporary rows.
                    if y < stop - 1 {
                        let mut tmprow = rows.remove(0);
                        let mut nextrowy = y.wrapping_add(ksize / 2).wrapping_add(1);
                        if nextrowy > srcb.max.y - 1 {
                            nextrowy = srcb.max.y - 1;
                        }
                        pix_getter.get_pixel_row(nextrowy, &mut tmprow);
                        rows.push(tmprow);
                    }
                }
            },
        );
    }
}

/// Convolution creates a filter that applies a square convolution kernel to
/// an image.
///
/// Go: convolution.go:Convolution
pub fn convolution(
    kernel: Vec<f32>,
    normalize: bool,
    alpha: bool,
    abs: bool,
    delta: f32,
) -> Arc<dyn Filter> {
    Arc::new(ConvolutionFilter {
        kernel,
        normalize,
        alpha,
        abs,
        delta,
    })
}

/// Go: convolution.go:prepareConvolutionWeights1d
pub(crate) fn prepare_convolution_weights_1d(kernel: &[f32]) -> (i64, Vec<UWeight>) {
    let mut size = kernel.len() as i64;
    if size % 2 == 0 {
        size -= 1;
    }
    if size < 1 {
        return (0, Vec::new());
    }
    let center = size / 2;
    let mut weights = Vec::new();
    for i in 0..size {
        let mut w = 0f32;
        if i < kernel.len() as i64 {
            w = kernel[i as usize];
        }
        if w != 0.0 {
            weights.push(UWeight {
                u: i - center,
                weight: w,
            });
        }
    }
    (size, weights)
}

/// Go: convolution.go:convolveLine
pub(crate) fn convolve_line(dst_buf: &mut [Pixel], src_buf: &[Pixel], weights: &[UWeight]) {
    let max = src_buf.len() as i64 - 1;
    if max < 0 {
        return;
    }
    for dstu in 0..src_buf.len() as i64 {
        let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
        for w in weights {
            let mut k = dstu + w.u;
            if k < 0 {
                k = 0;
            } else if k > max {
                k = max;
            }
            let c = src_buf[k as usize];
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
        dst_buf[dstu as usize] = Pixel::new(r, g, b, a);
    }
}

/// Go: convolution.go:convolve1dv
pub(crate) fn convolve_1dv(
    dst: &mut dyn draw::Image,
    src: &dyn Image,
    kernel: &[f32],
    options: &Options,
) {
    let srcb = src.bounds();
    let dstb = dst.bounds();
    if srcb.dx() <= 0 || srcb.dy() <= 0 {
        return;
    }
    if kernel.is_empty() {
        copyimage(dst, src, Some(options));
        return;
    }
    let (_, weights) = prepare_convolution_weights_1d(kernel);
    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);
    parallelize(
        options.parallelization,
        srcb.min.x,
        srcb.max.x,
        |start, stop| {
            let mut src_buf = vec![Pixel::default(); srcb.dy() as usize];
            let mut dst_buf = vec![Pixel::default(); srcb.dy() as usize];
            for x in start..stop {
                pix_getter.get_pixel_column(x, &mut src_buf);
                convolve_line(&mut dst_buf, &src_buf, &weights);
                pix_setter.set_pixel_column(add_sub(dstb.min.x, x, srcb.min.x), &dst_buf);
            }
        },
    );
}

/// Go: convolution.go:convolve1dh
pub(crate) fn convolve_1dh(
    dst: &mut dyn draw::Image,
    src: &dyn Image,
    kernel: &[f32],
    options: &Options,
) {
    let srcb = src.bounds();
    let dstb = dst.bounds();
    if srcb.dx() <= 0 || srcb.dy() <= 0 {
        return;
    }
    if kernel.is_empty() {
        copyimage(dst, src, Some(options));
        return;
    }
    let (_, weights) = prepare_convolution_weights_1d(kernel);
    let pix_getter = PixelGetter::new(src);
    let mut pix_setter = PixelSetter::new(dst);
    parallelize(
        options.parallelization,
        srcb.min.y,
        srcb.max.y,
        |start, stop| {
            let mut src_buf = vec![Pixel::default(); srcb.dx() as usize];
            let mut dst_buf = vec![Pixel::default(); srcb.dx() as usize];
            for y in start..stop {
                pix_getter.get_pixel_row(y, &mut src_buf);
                convolve_line(&mut dst_buf, &src_buf, &weights);
                pix_setter.set_pixel_row(add_sub(dstb.min.y, y, srcb.min.y), &dst_buf);
            }
        },
    );
}

/// Go: convolution.go:gaussianBlurKernel
pub(crate) fn gaussian_blur_kernel(x: f32, sigma: f32) -> f32 {
    // math.Sqrt(2*math.Pi), folded by the compiler.
    const SQRT_2PI: f64 = f64::from_bits(0x40040d931ff62705);
    (gomath::exp(-((x * x) as f64) / ((2.0 * sigma * sigma) as f64)) / (sigma as f64 * SQRT_2PI))
        as f32
}

/// Go: convolution.go:gausssianBlurFilter
#[derive(Clone, Copy, Debug)]
pub struct GaussianBlurFilter {
    pub sigma: f32,
}

impl Filter for GaussianBlurFilter {
    // Go: convolution.go:(*gausssianBlurFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: convolution.go:(*gausssianBlurFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        if self.sigma <= 0.0 {
            copyimage(dst, src, Some(options));
            return;
        }

        let radius = ((self.sigma * 3.0) as f64).ceil() as i64;
        let size = 2 * radius + 1;
        let center = radius;
        let mut kernel = vec![0f32; size as usize];

        kernel[center as usize] = gaussian_blur_kernel(0.0, self.sigma);
        let mut sum = kernel[center as usize];

        for i in 1..=radius {
            let f = gaussian_blur_kernel(i as f32, self.sigma);
            kernel[(center - i) as usize] = f;
            kernel[(center + i) as usize] = f;
            sum += 2.0 * f;
        }

        for k in kernel.iter_mut() {
            *k /= sum;
        }

        let mut tmp = create_temp_image(srcb);
        convolve_1dh(&mut tmp, src, &kernel, options);
        convolve_1dv(dst, &tmp, &kernel, options);
    }
}

/// GaussianBlur creates a filter that applies a gaussian blur to an image.
///
/// Go: convolution.go:GaussianBlur
pub fn gaussian_blur(sigma: f32) -> Arc<dyn Filter> {
    Arc::new(GaussianBlurFilter { sigma })
}

/// Go: convolution.go:unsharpMaskFilter
#[derive(Clone, Copy, Debug)]
pub struct UnsharpMaskFilter {
    pub sigma: f32,
    pub amount: f32,
    pub threshold: f32,
}

/// Go: convolution.go:unsharp. The returned `orig + dif` is compiled as the
/// fused `orig + (orig-blurred)*amount`; the comparison uses the rounded dif.
#[inline]
fn unsharp(orig: f32, blurred: f32, amount: f32, threshold: f32) -> f32 {
    let d = orig - blurred;
    let dif = d * amount;
    if absf32(dif) > absf32(threshold) {
        return d.mul_add(amount, orig);
    }
    orig
}

impl Filter for UnsharpMaskFilter {
    // Go: convolution.go:(*unsharpMaskFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: convolution.go:(*unsharpMaskFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        let mut blurred = create_temp_image(srcb);
        let blur = GaussianBlurFilter { sigma: self.sigma };
        blur.draw(&mut blurred, src, Some(options));

        let pix_getter_orig = PixelGetter::new(src);
        let pix_getter_blur = PixelGetter::new(&blurred);
        let mut pixel_setter = PixelSetter::new(dst);

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                for y in start..stop {
                    for x in srcb.min.x..srcb.max.x {
                        let px_orig = pix_getter_orig.get_pixel(x, y);
                        let px_blur = pix_getter_blur.get_pixel(x, y);

                        let r = unsharp(px_orig.r, px_blur.r, self.amount, self.threshold);
                        let g = unsharp(px_orig.g, px_blur.g, self.amount, self.threshold);
                        let b = unsharp(px_orig.b, px_blur.b, self.amount, self.threshold);
                        let a = unsharp(px_orig.a, px_blur.a, self.amount, self.threshold);

                        pixel_setter.set_pixel(
                            add_sub(dstb.min.x, x, srcb.min.x),
                            add_sub(dstb.min.y, y, srcb.min.y),
                            Pixel::new(r, g, b, a),
                        );
                    }
                }
            },
        );
    }
}

/// UnsharpMask creates a filter that sharpens an image.
///
/// Go: convolution.go:UnsharpMask
pub fn unsharp_mask(sigma: f32, amount: f32, threshold: f32) -> Arc<dyn Filter> {
    Arc::new(UnsharpMaskFilter {
        sigma,
        amount,
        threshold,
    })
}

/// Go: convolution.go:meanFilter
#[derive(Clone, Copy, Debug)]
pub struct MeanFilter {
    pub ksize: i64,
    pub disk: bool,
}

impl Filter for MeanFilter {
    // Go: convolution.go:(*meanFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: convolution.go:(*meanFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        let mut ksize = self.ksize;
        if ksize % 2 == 0 {
            ksize -= 1;
        }

        if ksize <= 1 {
            copyimage(dst, src, Some(options));
            return;
        }

        if self.disk {
            let disk_kernel = gen_disk(self.ksize);
            let f = ConvolutionFilter {
                kernel: disk_kernel,
                normalize: true,
                alpha: true,
                abs: false,
                delta: 0.0,
            };
            f.draw(dst, src, Some(options));
        } else {
            let kernel = vec![1f32; (ksize * ksize) as usize];
            let f = ConvolutionFilter {
                kernel,
                normalize: true,
                alpha: true,
                abs: false,
                delta: 0.0,
            };
            f.draw(dst, src, Some(options));
        }
    }
}

/// Mean creates a local mean image filter.
///
/// Go: convolution.go:Mean
pub fn mean(ksize: i64, disk: bool) -> Arc<dyn Filter> {
    Arc::new(MeanFilter { ksize, disk })
}

/// Go: convolution.go:hvConvolutionFilter
#[derive(Clone, Debug)]
pub struct HvConvolutionFilter {
    pub hkernel: Vec<f32>,
    pub vkernel: Vec<f32>,
}

impl Filter for HvConvolutionFilter {
    // Go: convolution.go:(*hvConvolutionFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: convolution.go:(*hvConvolutionFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();

        if srcb.dx() <= 0 || srcb.dy() <= 0 {
            return;
        }

        let mut tmph = create_temp_image(srcb);
        ConvolutionFilter {
            kernel: self.hkernel.clone(),
            normalize: false,
            alpha: false,
            abs: true,
            delta: 0.0,
        }
        .draw(&mut tmph, src, Some(options));
        let pix_getter_h = PixelGetter::new(&tmph);

        let mut tmpv = create_temp_image(srcb);
        ConvolutionFilter {
            kernel: self.vkernel.clone(),
            normalize: false,
            alpha: false,
            abs: true,
            delta: 0.0,
        }
        .draw(&mut tmpv, src, Some(options));
        let pix_getter_v = PixelGetter::new(&tmpv);

        let mut pix_setter = PixelSetter::new(dst);

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                for y in start..stop {
                    for x in srcb.min.x..srcb.max.x {
                        let pxh = pix_getter_h.get_pixel(x, y);
                        let pxv = pix_getter_v.get_pixel(x, y);
                        let r = sqrtf32(pxv.r.mul_add(pxv.r, pxh.r * pxh.r));
                        let g = sqrtf32(pxh.g.mul_add(pxh.g, pxv.g * pxv.g));
                        let b = sqrtf32(pxh.b.mul_add(pxh.b, pxv.b * pxv.b));
                        pix_setter.set_pixel(
                            add_sub(dstb.min.x, x, srcb.min.x),
                            add_sub(dstb.min.y, y, srcb.min.y),
                            Pixel::new(r, g, b, pxh.a),
                        );
                    }
                }
            },
        );
    }
}

/// Sobel creates a filter that applies a sobel operator to an image.
///
/// Go: convolution.go:Sobel
pub fn sobel() -> Arc<dyn Filter> {
    Arc::new(HvConvolutionFilter {
        hkernel: vec![-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0],
        vkernel: vec![-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0],
    })
}
