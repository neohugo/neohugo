//! Port of gift v1.2.1 `colors.go`: per-channel and per-pixel colour filters.
//!
//! arm64 fusion (from `go tool objdump`, see PORTING.md) is written out with
//! `mul_add` at each site: getFromLut's `u*N + 0.5`, Sepia's matrix, Sigmoid's
//! `(sig1-sig0)*x + sig0`, Contrast, Colorize, Grayscale/Threshold luma,
//! ColorspaceLinearToSRGB, convertHSLToRGB.

use std::sync::Arc;

use go_image::{Image, Rectangle, draw, rect};

use crate::gift::{DEFAULT_OPTIONS, Filter, Options};
use crate::gomath;
use crate::pixels::{ImageType, Pixel, PixelGetter, PixelSetter};
use crate::utils::{absf32, copyimage_filter, expf32, logf32, maxf32, minf32, parallelize, powf32};

/// Go: colors.go:prepareLut
pub(crate) fn prepare_lut(lut_size: i64, f: &dyn Fn(f32) -> f32) -> Vec<f32> {
    let mut lut = vec![0f32; lut_size as usize];
    let q = 1.0f32 / (lut_size - 1) as f32;
    for v in 0..lut_size {
        let u = v as f32 * q;
        lut[v as usize] = f(u);
    }
    lut
}

/// Go: colors.go:getFromLut (`u*float32(len(lut)-1) + 0.5` is fused).
#[inline]
pub(crate) fn get_from_lut(lut: &[f32], u: f32) -> f32 {
    let v = u.mul_add((lut.len() as i64 - 1) as f32, 0.5) as i64;
    lut[v as usize]
}

type ChanFn = Box<dyn Fn(f32) -> f32 + Send + Sync>;
type PixelFn = Box<dyn Fn(Pixel) -> Pixel + Send + Sync>;

/// Go: colors.go:colorchanFilter
pub struct ColorchanFilter {
    pub(crate) f: ChanFn,
    pub(crate) lut: bool,
}

impl Filter for ColorchanFilter {
    // Go: colors.go:(*colorchanFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: colors.go:(*colorchanFilter).Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, options: Option<&Options>) {
        let options = options.unwrap_or(&DEFAULT_OPTIONS);

        let srcb = src.bounds();
        let dstb = dst.bounds();
        let pix_getter = PixelGetter::new(src);
        let mut pix_setter = PixelSetter::new(dst);

        let mut use_lut = false;
        let mut lut: Vec<f32> = Vec::new();

        if self.lut {
            let it = pix_getter.it;
            let lut_size = if it == ImageType::NRGBA
                || it == ImageType::RGBA
                || it == ImageType::Gray
                || it == ImageType::YCbCr
            {
                0xff + 1
            } else {
                0xffff + 1
            };

            let num_calculations = srcb.dx() * srcb.dy() * 3;
            if num_calculations > lut_size * 2 {
                use_lut = true;
                lut = prepare_lut(lut_size, &self.f);
            }
        }

        parallelize(
            options.parallelization,
            srcb.min.y,
            srcb.max.y,
            |start, stop| {
                for y in start..stop {
                    for x in srcb.min.x..srcb.max.x {
                        let mut px = pix_getter.get_pixel(x, y);
                        if use_lut {
                            px.r = get_from_lut(&lut, px.r);
                            px.g = get_from_lut(&lut, px.g);
                            px.b = get_from_lut(&lut, px.b);
                        } else {
                            px.r = (self.f)(px.r);
                            px.g = (self.f)(px.g);
                            px.b = (self.f)(px.b);
                        }
                        pix_setter.set_pixel(
                            dstb.min.x + x - srcb.min.x,
                            dstb.min.y + y - srcb.min.y,
                            px,
                        );
                    }
                }
            },
        );
    }
}

fn colorchan(f: impl Fn(f32) -> f32 + Send + Sync + 'static, lut: bool) -> Arc<dyn Filter> {
    Arc::new(ColorchanFilter {
        f: Box::new(f),
        lut,
    })
}

/// Invert creates a filter that negates the colors of an image.
///
/// Go: colors.go:Invert
pub fn invert() -> Arc<dyn Filter> {
    colorchan(|x| 1.0 - x, false)
}

/// ColorspaceSRGBToLinear creates a filter that converts the colors of an
/// image from sRGB to linear RGB.
///
/// Go: colors.go:ColorspaceSRGBToLinear
pub fn colorspace_srgb_to_linear() -> Arc<dyn Filter> {
    colorchan(
        |x| {
            if x <= 0.04045 {
                return x / 12.92;
            }
            gomath::pow(((x + 0.055) / 1.055) as f64, 2.4) as f32
        },
        true,
    )
}

/// ColorspaceLinearToSRGB creates a filter that converts the colors of an
/// image from linear RGB to sRGB.
///
/// Go: colors.go:ColorspaceLinearToSRGB
pub fn colorspace_linear_to_srgb() -> Arc<dyn Filter> {
    // 1/2.4 as an untyped constant rounded to float64.
    const INV_24: f64 = f64::from_bits(0x3fdaaaaaaaaaaaab);
    colorchan(
        |x| {
            if x <= 0.0031308 {
                return x * 12.92;
            }
            // 1.055*math.Pow(...) - 0.055: FNMSUBD.
            gomath::pow(x as f64, INV_24).mul_add(1.055, -0.055) as f32
        },
        true,
    )
}

/// Gamma creates a filter that performs a gamma correction on an image.
///
/// Go: colors.go:Gamma
pub fn gamma(gamma: f32) -> Arc<dyn Filter> {
    let e = 1.0 / maxf32(gamma, 1.0e-5);
    colorchan(move |x| powf32(x, e), true)
}

/// Go: colors.go:sigmoid
#[inline]
fn sigmoid(a: f32, b: f32, x: f32) -> f32 {
    1.0 / (1.0 + expf32(b * (a - x)))
}

/// Sigmoid creates a filter that changes the contrast of an image using a
/// sigmoidal function and returns the adjusted image.
///
/// Go: colors.go:Sigmoid
pub fn sigmoid_filter(midpoint: f32, factor: f32) -> Arc<dyn Filter> {
    let a = minf32(maxf32(midpoint, 0.0), 1.0);
    let b = absf32(factor);
    let sig0 = sigmoid(a, b, 0.0);
    let sig1 = sigmoid(a, b, 1.0);
    let e = 1.0e-5f32;

    colorchan(
        move |x| {
            if factor == 0.0 {
                x
            } else if factor > 0.0 {
                let sig = sigmoid(a, b, x);
                (sig - sig0) / (sig1 - sig0)
            } else {
                let arg = minf32(maxf32((sig1 - sig0).mul_add(x, sig0), e), 1.0 - e);
                a - logf32(1.0 / arg - 1.0) / b
            }
        },
        true,
    )
}

/// Contrast creates a filter that changes the contrast of an image.
///
/// Go: colors.go:Contrast
pub fn contrast(percentage: f32) -> Arc<dyn Filter> {
    if percentage == 0.0 {
        return copyimage_filter();
    }

    let p = 1.0 + minf32(maxf32(percentage, -100.0), 100.0) / 100.0;

    colorchan(
        move |x| {
            if 0.0 <= p && p <= 1.0 {
                (x - 0.5).mul_add(p, 0.5)
            } else if 1.0 < p && p < 2.0 {
                (x - 0.5).mul_add(1.0 / (2.0 - p), 0.5)
            } else {
                if x < 0.5 {
                    return 0.0;
                }
                1.0
            }
        },
        false,
    )
}

/// Brightness creates a filter that changes the brightness of an image.
///
/// Go: colors.go:Brightness
pub fn brightness(percentage: f32) -> Arc<dyn Filter> {
    if percentage == 0.0 {
        return copyimage_filter();
    }

    let shift = minf32(maxf32(percentage, -100.0), 100.0) / 100.0;

    colorchan(move |x| x + shift, false)
}

/// Go: colors.go:colorFilter
pub struct ColorFilter {
    pub(crate) f: PixelFn,
}

impl Filter for ColorFilter {
    // Go: colors.go:(*colorFilter).Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }

    // Go: colors.go:(*colorFilter).Draw
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
                for y in start..stop {
                    for x in srcb.min.x..srcb.max.x {
                        let px = pix_getter.get_pixel(x, y);
                        pix_setter.set_pixel(
                            dstb.min.x + x - srcb.min.x,
                            dstb.min.y + y - srcb.min.y,
                            (self.f)(px),
                        );
                    }
                }
            },
        );
    }
}

fn color_filter(f: impl Fn(Pixel) -> Pixel + Send + Sync + 'static) -> Arc<dyn Filter> {
    Arc::new(ColorFilter { f: Box::new(f) })
}

const K299: f32 = f32::from_bits(0x3e991687);
const K587: f32 = f32::from_bits(0x3f1645a2);
const K114: f32 = f32::from_bits(0x3de978d5);

/// `0.299*px.r + 0.587*px.g + 0.114*px.b` as compiled in Grayscale and
/// Threshold: the red product is rounded, green and blue are fused.
#[inline]
fn luma(px: Pixel) -> f32 {
    px.b.mul_add(K114, px.g.mul_add(K587, K299 * px.r))
}

/// Grayscale creates a filter that produces a grayscale version of an image.
///
/// Go: colors.go:Grayscale
pub fn grayscale() -> Arc<dyn Filter> {
    color_filter(|px| {
        let y = luma(px);
        Pixel::new(y, y, y, px.a)
    })
}

/// Sepia creates a filter that produces a sepia-toned version of an image.
///
/// Go: colors.go:Sepia
pub fn sepia(percentage: f32) -> Arc<dyn Filter> {
    let adjust_amount = minf32(maxf32(percentage, 0.0), 100.0) / 100.0;
    let rr = (-adjust_amount).mul_add(0.607, 1.0);
    let rg = 0.769 * adjust_amount;
    let rb = 0.189 * adjust_amount;
    let gr = 0.349 * adjust_amount;
    let gg = (-adjust_amount).mul_add(0.314, 1.0);
    let gb = 0.168 * adjust_amount;
    let br = 0.272 * adjust_amount;
    let bg = 0.534 * adjust_amount;
    let bb = (-adjust_amount).mul_add(0.869, 1.0);
    color_filter(move |px| {
        // px.r*rr + px.g*rg + px.b*rb: the green product is rounded.
        let r = px.b.mul_add(rb, px.r.mul_add(rr, px.g * rg));
        let g = px.b.mul_add(gb, px.r.mul_add(gr, px.g * gg));
        let b = px.b.mul_add(bb, px.r.mul_add(br, px.g * bg));
        Pixel::new(r, g, b, px.a)
    })
}

/// Go: colors.go:convertHSLToRGB
pub(crate) fn convert_hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }

    const ONE_SIXTH: f32 = f32::from_bits(0x3e2aaaab);
    const TWO_THIRDS: f32 = f32::from_bits(0x3f2aaaab);
    const ONE_THIRD: f32 = f32::from_bits(0x3eaaaaab);

    let hue_to_rgb = |p: f32, q: f32, t: f32| -> f32 {
        let mut t = t;
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < ONE_SIXTH {
            // p + (q-p)*6*t
            return ((q - p) * 6.0).mul_add(t, p);
        }
        if t < 0.5 {
            return q;
        }
        if t < TWO_THIRDS {
            // p + (q-p)*(2/3.0-t)*6
            return ((q - p) * (TWO_THIRDS - t)).mul_add(6.0, p);
        }
        p
    };

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        // l + s - l*s
        (-l).mul_add(s, l + s)
    };
    let p = (l + l) - q;

    let r = hue_to_rgb(p, q, h + ONE_THIRD);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - ONE_THIRD);

    (r, g, b)
}

/// Go: colors.go:convertRGBToHSL
pub(crate) fn convert_rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = maxf32(r, maxf32(g, b));
    let min = minf32(r, minf32(g, b));

    let l = (max + min) / 2.0;

    if max == min {
        return (0.0, 0.0, l);
    }

    let mut h;
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    if r == max {
        h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
    } else if g == max {
        h = (b - r) / d + 2.0;
    } else {
        h = (r - g) / d + 4.0;
    }
    h /= 6.0;

    (h, s, l)
}

/// Go: colors.go:normalizeHue
pub(crate) fn normalize_hue(hue: f32) -> f32 {
    let mut hue = hue - (hue as i64) as f32;
    if hue < 0.0 {
        hue += 1.0;
    }
    hue
}

/// Hue creates a filter that rotates the hue of an image.
///
/// Go: colors.go:Hue
pub fn hue(shift: f32) -> Arc<dyn Filter> {
    let p = normalize_hue(shift / 360.0);
    if p == 0.0 {
        return copyimage_filter();
    }

    color_filter(move |px| {
        let (h, s, l) = convert_rgb_to_hsl(px.r, px.g, px.b);
        let h = normalize_hue(h + p);
        let (r, g, b) = convert_hsl_to_rgb(h, s, l);
        Pixel::new(r, g, b, px.a)
    })
}

/// Saturation creates a filter that changes the saturation of an image.
///
/// Go: colors.go:Saturation
pub fn saturation(percentage: f32) -> Arc<dyn Filter> {
    let p = 1.0 + minf32(maxf32(percentage, -100.0), 500.0) / 100.0;
    if p == 1.0 {
        return copyimage_filter();
    }

    color_filter(move |px| {
        let (h, mut s, l) = convert_rgb_to_hsl(px.r, px.g, px.b);
        s *= p;
        if s > 1.0 {
            s = 1.0;
        }
        let (r, g, b) = convert_hsl_to_rgb(h, s, l);
        Pixel::new(r, g, b, px.a)
    })
}

/// Colorize creates a filter that produces a colorized version of an image.
///
/// Go: colors.go:Colorize
pub fn colorize(hue: f32, saturation: f32, percentage: f32) -> Arc<dyn Filter> {
    let h = normalize_hue(hue / 360.0);
    let s = minf32(maxf32(saturation, 0.0), 100.0) / 100.0;
    let p = minf32(maxf32(percentage, 0.0), 100.0) / 100.0;
    if p == 0.0 {
        return copyimage_filter();
    }

    color_filter(move |mut px| {
        let (_, _, l) = convert_rgb_to_hsl(px.r, px.g, px.b);
        let (r, g, b) = convert_hsl_to_rgb(h, s, l);
        px.r = (r - px.r).mul_add(p, px.r);
        px.g = (g - px.g).mul_add(p, px.g);
        px.b = (b - px.b).mul_add(p, px.b);
        px
    })
}

/// ColorBalance creates a filter that changes the color balance of an image.
///
/// Go: colors.go:ColorBalance
pub fn color_balance(
    percentage_red: f32,
    percentage_green: f32,
    percentage_blue: f32,
) -> Arc<dyn Filter> {
    let pr = 1.0 + minf32(maxf32(percentage_red, -100.0), 500.0) / 100.0;
    let pg = 1.0 + minf32(maxf32(percentage_green, -100.0), 500.0) / 100.0;
    let pb = 1.0 + minf32(maxf32(percentage_blue, -100.0), 500.0) / 100.0;

    color_filter(move |mut px| {
        px.r *= pr;
        px.g *= pg;
        px.b *= pb;
        px
    })
}

/// Threshold creates a filter that applies black/white thresholding to the
/// image.
///
/// Go: colors.go:Threshold
pub fn threshold(percentage: f32) -> Arc<dyn Filter> {
    let p = minf32(maxf32(percentage, 0.0), 100.0) / 100.0;
    color_filter(move |px| {
        let y = luma(px);
        if y > p {
            return Pixel::new(1.0, 1.0, 1.0, px.a);
        }
        Pixel::new(0.0, 0.0, 0.0, px.a)
    })
}

/// ColorFunc creates a filter that changes the colors of an image using
/// custom function. The function takes red, green, blue and alpha channels of
/// a pixel as float32 values in range (0, 1) and returns the modified channel
/// values. (Float expressions inside `f` are compiled by rustc, which never
/// fuses; a caller porting Go callbacks must add `mul_add` where Go fuses.)
///
/// Go: colors.go:ColorFunc
pub fn color_func(
    f: impl Fn(f32, f32, f32, f32) -> (f32, f32, f32, f32) + Send + Sync + 'static,
) -> Arc<dyn Filter> {
    color_filter(move |px| {
        let (r, g, b, a) = f(px.r, px.g, px.b, px.a);
        Pixel::new(r, g, b, a)
    })
}
