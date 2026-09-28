//! Shared helpers for the differential tests: a call-for-call port of the
//! oracle's generator (tools/go-oracle/gift/gen.go), the spec parsers, the
//! raw image dump reader (dump.go) and digests identical to the oracle's.

#![allow(dead_code, clippy::upper_case_acronyms)]

use std::io::Read;
use std::sync::Arc;

use gift::{Filter, Options, Resampling};
use go_image::color::{self, Color, Palette};
use go_image::{
    Alpha, Alpha16, CMYK, Gray, Gray16, Image, NRGBA, NRGBA64, NYCbCrA, Paletted, RGBA, RGBA64,
    Rectangle, YCbCr, YCbCrSubsampleRatio, draw, pt, rect,
};
use sha2::{Digest, Sha256};

pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads a (possibly gzip-compressed, by extension) text fixture.
pub fn read_text(path: &std::path::Path) -> String {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("reading {}: {}", path.display(), e));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut s = String::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_string(&mut s)
            .unwrap();
        s
    } else {
        String::from_utf8(raw).unwrap()
    }
}

pub fn read_tsv(path: &std::path::Path) -> Vec<Vec<String>> {
    read_text(path)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(|f| f.to_string()).collect())
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// splitmix64, as in the oracle.
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(s: u64) -> Rng {
        Rng { s }
    }
    pub fn next(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn intn(&mut self, n: i64) -> i64 {
        (self.next() % n as u64) as i64
    }
    pub fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}

// Go: gen.go:genVal
fn gen_val(r: &mut Rng, vmode: i64) -> u8 {
    if vmode == 0 {
        return r.byte();
    }
    match r.intn(8) {
        0 => 0,
        1 => 1,
        2 => 127,
        3 => 128,
        4 => 254,
        5 => 255,
        _ => r.byte(),
    }
}

// Go: gen.go:genAlpha8
fn gen_alpha8(r: &mut Rng, amode: i64) -> u8 {
    match amode {
        1 => 0xff,
        2 => match r.intn(3) {
            0 => 0,
            1 => 0xff,
            _ => r.byte(),
        },
        _ => r.byte(),
    }
}

// Go: gen.go:genAlpha16
fn gen_alpha16(r: &mut Rng, amode: i64) -> u16 {
    match amode {
        1 => 0xffff,
        2 => match r.intn(3) {
            0 => 0,
            1 => 0xffff,
            _ => {
                let hi = r.byte();
                let lo = r.byte();
                (hi as u16) << 8 | lo as u16
            }
        },
        _ => {
            let hi = r.byte();
            let lo = r.byte();
            (hi as u16) << 8 | lo as u16
        }
    }
}

// Go: gen.go:genVal16
fn gen_val16(r: &mut Rng, vmode: i64) -> u16 {
    let hi = gen_val(r, vmode);
    let lo = gen_val(r, vmode);
    (hi as u16) << 8 | lo as u16
}

// Go: gen.go:genColor
pub fn gen_color(r: &mut Rng, vmode: i64, amode: i64, valid: bool) -> Color {
    let k = r.intn(6);
    gen_color_k(r, k, vmode, amode, valid)
}

// Go: gen.go:genColorK
pub fn gen_color_k(r: &mut Rng, k: i64, vmode: i64, amode: i64, valid: bool) -> Color {
    match k {
        0 => {
            let mut c = color::RGBA {
                r: gen_val(r, vmode),
                g: gen_val(r, vmode),
                b: gen_val(r, vmode),
                a: gen_alpha8(r, amode),
            };
            if valid {
                c.r = c.r.min(c.a);
                c.g = c.g.min(c.a);
                c.b = c.b.min(c.a);
            }
            Color::RGBA(c)
        }
        1 => Color::NRGBA(color::NRGBA {
            r: gen_val(r, vmode),
            g: gen_val(r, vmode),
            b: gen_val(r, vmode),
            a: gen_alpha8(r, amode),
        }),
        2 => Color::Gray(color::Gray {
            y: gen_val(r, vmode),
        }),
        3 => {
            let mut c = color::RGBA64 {
                r: gen_val16(r, vmode),
                g: gen_val16(r, vmode),
                b: gen_val16(r, vmode),
                a: gen_alpha16(r, amode),
            };
            if valid {
                c.r = c.r.min(c.a);
                c.g = c.g.min(c.a);
                c.b = c.b.min(c.a);
            }
            Color::RGBA64(c)
        }
        4 => Color::NRGBA64(color::NRGBA64 {
            r: gen_val16(r, vmode),
            g: gen_val16(r, vmode),
            b: gen_val16(r, vmode),
            a: gen_alpha16(r, amode),
        }),
        _ => Color::Alpha(color::Alpha {
            a: gen_alpha8(r, amode),
        }),
    }
}

// Go: gen.go:genPalette
fn gen_palette(r: &mut Rng, vmode: i64, amode: i64, valid: bool) -> Palette {
    let n = match r.intn(3) {
        0 => 1 + r.intn(2),
        1 => 1 + r.intn(16),
        _ => 1 + r.intn(256),
    };
    let mut p = Vec::with_capacity(n as usize);
    for _ in 0..n {
        p.push(gen_color(r, vmode, amode, valid));
    }
    Palette(p)
}

/// Go: gen.go:imgSpec
#[derive(Clone, Debug)]
pub struct ImgSpec {
    pub typ: String,
    pub rect: Rectangle,
    pub seed: u64,
    pub vmode: i64,
    pub amode: i64,
    pub valid: bool,
    pub blank: bool,
}

// Go: gen.go:parseImgSpec
pub fn parse_img_spec(s: &str) -> ImgSpec {
    let f: Vec<&str> = s.split(':').collect();
    assert_eq!(f.len(), 7, "bad img spec {}", s);
    let r: Vec<i64> = f[1].split(',').map(|v| v.parse().unwrap()).collect();
    ImgSpec {
        typ: f[0].to_string(),
        rect: rect(r[0], r[1], r[2], r[3]),
        seed: f[2].parse().unwrap(),
        vmode: f[3].parse().unwrap(),
        amode: f[4].parse().unwrap(),
        valid: f[5] == "1",
        blank: f[6] == "1",
    }
}

fn ycbcr_ratio(typ: &str) -> YCbCrSubsampleRatio {
    match &typ[typ.len() - 3..] {
        "444" => YCbCrSubsampleRatio::Ratio444,
        "422" => YCbCrSubsampleRatio::Ratio422,
        "420" => YCbCrSubsampleRatio::Ratio420,
        "440" => YCbCrSubsampleRatio::Ratio440,
        "411" => YCbCrSubsampleRatio::Ratio411,
        "410" => YCbCrSubsampleRatio::Ratio410,
        _ => panic!("{}", typ),
    }
}

// Go: gen.go:fill8
fn fill8(r: &mut Rng, pix: &mut [u8], vmode: i64) {
    for p in pix.iter_mut() {
        *p = gen_val(r, vmode);
    }
}

// Go: gen.go:fillRGBA8
fn fill_rgba8(r: &mut Rng, pix: &mut [u8], s: &ImgSpec, premul: bool) {
    let mut i = 0;
    while i + 3 < pix.len() {
        pix[i] = gen_val(r, s.vmode);
        pix[i + 1] = gen_val(r, s.vmode);
        pix[i + 2] = gen_val(r, s.vmode);
        let a = gen_alpha8(r, s.amode);
        pix[i + 3] = a;
        if premul && s.valid {
            pix[i] = pix[i].min(a);
            pix[i + 1] = pix[i + 1].min(a);
            pix[i + 2] = pix[i + 2].min(a);
        }
        i += 4;
    }
}

// Go: gen.go:fillRGBA16
fn fill_rgba16(r: &mut Rng, pix: &mut [u8], s: &ImgSpec, premul: bool) {
    let mut i = 0;
    while i + 7 < pix.len() {
        let mut c = [0u16; 3];
        for k in c.iter_mut() {
            *k = gen_val16(r, s.vmode);
        }
        let a = gen_alpha16(r, s.amode);
        if premul && s.valid {
            for k in c.iter_mut() {
                *k = (*k).min(a);
            }
        }
        for k in 0..3 {
            pix[i + 2 * k] = (c[k] >> 8) as u8;
            pix[i + 2 * k + 1] = c[k] as u8;
        }
        pix[i + 6] = (a >> 8) as u8;
        pix[i + 7] = a as u8;
        i += 8;
    }
}

/// A generated image: settable (draw.Image) or read-only (YCbCr, NYCbCrA).
pub enum TestImage {
    Draw(Box<dyn draw::Image>),
    Plain(Box<dyn Image>),
}

impl TestImage {
    pub fn image(&self) -> &dyn Image {
        match self {
            TestImage::Draw(m) => &**m,
            TestImage::Plain(m) => &**m,
        }
    }
    pub fn draw_image(&mut self) -> &mut dyn draw::Image {
        match self {
            TestImage::Draw(m) => &mut **m,
            TestImage::Plain(_) => panic!("not a draw.Image"),
        }
    }
}

// Go: gen.go:genImage
pub fn gen_image(s: &ImgSpec) -> TestImage {
    let r = &mut Rng::new(s.seed);
    let d = |m: Box<dyn draw::Image>| TestImage::Draw(m);
    match s.typ.as_str() {
        "nrgba" => {
            let mut m = NRGBA::new(s.rect);
            if !s.blank {
                fill_rgba8(r, &mut m.pix, s, false);
            }
            d(Box::new(m))
        }
        "rgba" => {
            let mut m = RGBA::new(s.rect);
            if !s.blank {
                fill_rgba8(r, &mut m.pix, s, true);
            }
            d(Box::new(m))
        }
        "nrgba64" => {
            let mut m = NRGBA64::new(s.rect);
            if !s.blank {
                fill_rgba16(r, &mut m.pix, s, false);
            }
            d(Box::new(m))
        }
        "rgba64" => {
            let mut m = RGBA64::new(s.rect);
            if !s.blank {
                fill_rgba16(r, &mut m.pix, s, true);
            }
            d(Box::new(m))
        }
        "gray" => {
            let mut m = Gray::new(s.rect);
            if !s.blank {
                fill8(r, &mut m.pix, s.vmode);
            }
            d(Box::new(m))
        }
        "gray16" => {
            let mut m = Gray16::new(s.rect);
            if !s.blank {
                fill8(r, &mut m.pix, s.vmode);
            }
            d(Box::new(m))
        }
        "cmyk" => {
            let mut m = CMYK::new(s.rect);
            if !s.blank {
                fill8(r, &mut m.pix, s.vmode);
            }
            d(Box::new(m))
        }
        "alpha" => {
            let mut m = Alpha::new(s.rect);
            if !s.blank {
                for p in m.pix.iter_mut() {
                    *p = gen_alpha8(r, s.amode);
                }
            }
            d(Box::new(m))
        }
        "alpha16" => {
            let mut m = Alpha16::new(s.rect);
            if !s.blank {
                let mut i = 0;
                while i + 1 < m.pix.len() {
                    let a = gen_alpha16(r, s.amode);
                    m.pix[i] = (a >> 8) as u8;
                    m.pix[i + 1] = a as u8;
                    i += 2;
                }
            }
            d(Box::new(m))
        }
        "paletted" => {
            let p = gen_palette(r, s.vmode, s.amode, s.valid);
            let n = p.len() as i64;
            let mut m = Paletted::new(s.rect, p);
            if !s.blank {
                for px in m.pix.iter_mut() {
                    *px = r.intn(n) as u8;
                }
            }
            d(Box::new(m))
        }
        "gray16all" => {
            // Every 16-bit value in order (Go: gen.go "gray16all").
            let mut m = Gray16::new(s.rect);
            let mut i = 0;
            while i + 1 < m.pix.len() {
                let v = (i / 2) & 0xffff;
                m.pix[i] = (v >> 8) as u8;
                m.pix[i + 1] = v as u8;
                i += 2;
            }
            d(Box::new(m))
        }
        "ycbcr444" | "ycbcr422" | "ycbcr420" | "ycbcr440" | "ycbcr411" | "ycbcr410" => {
            let mut m = YCbCr::new(s.rect, ycbcr_ratio(&s.typ));
            if !s.blank {
                fill8(r, &mut m.y, s.vmode);
                fill8(r, &mut m.cb, s.vmode);
                fill8(r, &mut m.cr, s.vmode);
            }
            TestImage::Plain(Box::new(m))
        }
        "nycbcra444" | "nycbcra420" | "nycbcra422" | "nycbcra440" | "nycbcra411" | "nycbcra410" => {
            let mut m = NYCbCrA::new(s.rect, ycbcr_ratio(&s.typ));
            if !s.blank {
                fill8(r, &mut m.ycbcr.y, s.vmode);
                fill8(r, &mut m.ycbcr.cb, s.vmode);
                fill8(r, &mut m.ycbcr.cr, s.vmode);
                for p in m.a.iter_mut() {
                    *p = gen_alpha8(r, s.amode);
                }
            }
            TestImage::Plain(Box::new(m))
        }
        t => panic!("unknown image type {}", t),
    }
}

/// Go: gen.go:digest — SHA-256 of the pixel buffer and the bounds.
pub fn digest(img: &dyn Image) -> String {
    let mut h = Sha256::new();
    if let Some(m) = img.downcast_ref::<NRGBA>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<NRGBA64>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<RGBA>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<RGBA64>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<Gray>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<Gray16>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<CMYK>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<Alpha>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<Alpha16>() {
        h.update(&m.pix);
    } else if let Some(m) = img.downcast_ref::<Paletted>() {
        h.update(&m.pix);
    } else {
        panic!("digest: unsupported image type");
    }
    let b = img.bounds();
    h.update(format!("|{},{},{},{}", b.min.x, b.min.y, b.max.x, b.max.y).as_bytes());
    hex(&h.finalize())
}

/// Go: gen.go:pf — "f<hex bits>" float32.
pub fn pf(s: &str) -> f32 {
    assert!(s.starts_with('f'), "bad float {}", s);
    f32::from_bits(u32::from_str_radix(&s[1..], 16).unwrap())
}

pub fn pi(s: &str) -> i64 {
    s.parse().unwrap_or_else(|_| panic!("bad int {}", s))
}

/// Go: gen.go:parseColor
pub fn parse_color(s: &str) -> Color {
    let f: Vec<&str> = s.split('/').collect();
    let u = |i: usize| -> i64 { pi(f[i]) };
    match f[0] {
        "rgba" => Color::RGBA(color::RGBA {
            r: u(1) as u8,
            g: u(2) as u8,
            b: u(3) as u8,
            a: u(4) as u8,
        }),
        "nrgba" => Color::NRGBA(color::NRGBA {
            r: u(1) as u8,
            g: u(2) as u8,
            b: u(3) as u8,
            a: u(4) as u8,
        }),
        "gray" => Color::Gray(color::Gray { y: u(1) as u8 }),
        "rgba64" => Color::RGBA64(color::RGBA64 {
            r: u(1) as u16,
            g: u(2) as u16,
            b: u(3) as u16,
            a: u(4) as u16,
        }),
        "nrgba64" => Color::NRGBA64(color::NRGBA64 {
            r: u(1) as u16,
            g: u(2) as u16,
            b: u(3) as u16,
            a: u(4) as u16,
        }),
        "alpha" => Color::Alpha(color::Alpha { a: u(1) as u8 }),
        _ => panic!("bad color {}", s),
    }
}

/// The oracle's resampling names (gift's five and neohugo's extra ones).
pub fn resampling(name: &str) -> &'static dyn Resampling {
    match name {
        "cubic" => &gift::CUBIC_RESAMPLING,
        _ => gift::hugo_resampling::image_filter(name)
            .unwrap_or_else(|| panic!("no resampling {}", name)),
    }
}

/// Go: gen.go:colorFuncs
pub fn color_func(i: i64) -> Arc<dyn Filter> {
    match i {
        0 => gift::color_func(|r0, g0, _b0, a0| (1.0 - r0, g0 + 0.1, 0.0, a0)),
        1 => gift::color_func(|r0, g0, b0, a0| (b0, r0, g0, a0 / 2.0)),
        2 => gift::color_func(|r0, g0, b0, a0| (r0 + g0, g0 - b0, -b0, a0 + 0.25)),
        3 => {
            let tab = setter_table();
            gift::color_func(move |r0, _g0, _b0, _a0| {
                let k = ((r0 as f64) * 65535.0).round() as i64;
                (
                    tab[(k & 0xffff) as usize],
                    tab[((k * 7 + 1) & 0xffff) as usize],
                    tab[((k * 13 + 2) & 0xffff) as usize],
                    tab[((k * 29 + 3) & 0xffff) as usize],
                )
            })
        }
        _ => panic!("colorfunc {}", i),
    }
}

/// Go: gen.go:parseFilter
pub fn parse_filter(s: &str, dumps: &dyn Fn(&str) -> Arc<dyn Image>) -> Arc<dyn Filter> {
    let open = s.find('(').unwrap_or_else(|| panic!("bad filter {}", s));
    assert!(s.ends_with(')'), "bad filter {}", s);
    let name = &s[..open];
    let inner = &s[open + 1..s.len() - 1];
    let a: Vec<&str> = if inner.is_empty() {
        Vec::new()
    } else {
        inner.split(',').collect()
    };
    let b = |i: usize| a[i] == "1";
    match name {
        "resize" => gift::resize(pi(a[0]), pi(a[1]), resampling(a[2])),
        "resizetofit" => gift::resize_to_fit(pi(a[0]), pi(a[1]), resampling(a[2])),
        "resizetofill" => {
            gift::resize_to_fill(pi(a[0]), pi(a[1]), resampling(a[2]), gift::Anchor(pi(a[3])))
        }
        "crop" => gift::crop(rect(pi(a[0]), pi(a[1]), pi(a[2]), pi(a[3]))),
        "croptosize" => gift::crop_to_size(pi(a[0]), pi(a[1]), gift::Anchor(pi(a[2]))),
        "rotate90" => gift::rotate90(),
        "rotate180" => gift::rotate180(),
        "rotate270" => gift::rotate270(),
        "fliph" => gift::flip_horizontal(),
        "flipv" => gift::flip_vertical(),
        "transpose" => gift::transpose(),
        "transverse" => gift::transverse(),
        "rotate" => gift::rotate(pf(a[0]), parse_color(a[1]), gift::Interpolation(pi(a[2]))),
        "invert" => gift::invert(),
        "srgbtolinear" => gift::colorspace_srgb_to_linear(),
        "lineartosrgb" => gift::colorspace_linear_to_srgb(),
        "gamma" => gift::gamma(pf(a[0])),
        "sigmoid" => gift::sigmoid_filter(pf(a[0]), pf(a[1])),
        "contrast" => gift::contrast(pf(a[0])),
        "brightness" => gift::brightness(pf(a[0])),
        "grayscale" => gift::grayscale(),
        "sepia" => gift::sepia(pf(a[0])),
        "hue" => gift::hue(pf(a[0])),
        "saturation" => gift::saturation(pf(a[0])),
        "colorize" => gift::colorize(pf(a[0]), pf(a[1]), pf(a[2])),
        "colorbalance" => gift::color_balance(pf(a[0]), pf(a[1]), pf(a[2])),
        "threshold" => gift::threshold(pf(a[0])),
        "colorfunc" => color_func(pi(a[0])),
        "convolution" => {
            let k: Vec<f32> = if a[0].is_empty() {
                Vec::new()
            } else {
                a[0].split('/').map(pf).collect()
            };
            gift::convolution(k, b(1), b(2), b(3), pf(a[4]))
        }
        "gaussianblur" => gift::gaussian_blur(pf(a[0])),
        "unsharpmask" => gift::unsharp_mask(pf(a[0]), pf(a[1]), pf(a[2])),
        "mean" => gift::mean(pi(a[0]), b(1)),
        "sobel" => gift::sobel(),
        "median" => gift::median(pi(a[0]), b(1)),
        "minimum" => gift::minimum(pi(a[0]), b(1)),
        "maximum" => gift::maximum(pi(a[0]), b(1)),
        "pixelate" => gift::pixelate(pi(a[0])),
        "copy" => Arc::new(CopyFilter),
        "overlay" => Arc::new(OverlayFilter {
            src: dumps(a[0]),
            x: pi(a[1]),
            y: pi(a[2]),
        }),
        _ => panic!("unknown filter {}", name),
    }
}

pub fn parse_filters(s: &str, dumps: &dyn Fn(&str) -> Arc<dyn Image>) -> Vec<Arc<dyn Filter>> {
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split('|').map(|p| parse_filter(p, dumps)).collect()
}

/// Go: gen.go:copyFilter
pub struct CopyFilter;

impl Filter for CopyFilter {
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, _options: Option<&Options>) {
        gift::new(Vec::new()).draw(dst, src);
    }
    fn bounds(&self, b: Rectangle) -> Rectangle {
        rect(0, 0, b.dx(), b.dy())
    }
}

/// neohugo resources/images/overlay.go:overlayFilter with an in-memory
/// overlay image (Go: gen.go:overlayFilter).
pub struct OverlayFilter {
    pub src: Arc<dyn Image>,
    pub x: i64,
    pub y: i64,
}

impl Filter for OverlayFilter {
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, _options: Option<&Options>) {
        gift::new(Vec::new()).draw(dst, src);
        gift::new(Vec::new()).draw_at(dst, &*self.src, pt(self.x, self.y), gift::OVER_OPERATOR);
    }
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }
}

/// neohugo resources/images/image.go:(*ImageProcessor).doFilter (non-GIF):
/// the destination type follows the source type.
pub fn do_filter(src: &dyn Image, filters: Vec<Arc<dyn Filter>>) -> Box<dyn draw::Image> {
    let g = gift::new(filters);
    let bounds = g.bounds(src.bounds());
    let mut dst: Box<dyn draw::Image> = if src.is::<RGBA>() {
        Box::new(RGBA::new(bounds))
    } else if src.is::<NRGBA>() {
        Box::new(NRGBA::new(bounds))
    } else if src.is::<Gray>() {
        Box::new(Gray::new(bounds))
    } else {
        Box::new(NRGBA::new(bounds))
    };
    g.draw(&mut *dst, src);
    dst
}

/// Go: dump.go:loadDump — a gzip-compressed raw image dump.
pub fn load_dump(path: &std::path::Path) -> Box<dyn Image> {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("reading {}: {}", path.display(), e));
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(&raw[..])
        .read_to_end(&mut data)
        .unwrap();
    let mut rd = &data[..];
    let mut take = |n: usize| -> Vec<u8> {
        let (a, b) = rd.split_at(n);
        rd = b;
        a.to_vec()
    };
    assert_eq!(take(8), b"GIFTIMG1");
    let u8_ = |t: &mut dyn FnMut(usize) -> Vec<u8>| t(1)[0];
    let i64_ = |t: &mut dyn FnMut(usize) -> Vec<u8>| i64::from_le_bytes(t(8).try_into().unwrap());
    let buf = |t: &mut dyn FnMut(usize) -> Vec<u8>| {
        let n = u64::from_le_bytes(t(8).try_into().unwrap()) as usize;
        t(n)
    };
    let kind = u8_(&mut take);
    let r = rect(
        i64_(&mut take),
        i64_(&mut take),
        i64_(&mut take),
        i64_(&mut take),
    );
    match kind {
        1 => {
            let stride = i64_(&mut take);
            Box::new(NRGBA {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        2 => {
            let stride = i64_(&mut take);
            Box::new(NRGBA64 {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        3 => {
            let stride = i64_(&mut take);
            Box::new(RGBA {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        4 => {
            let stride = i64_(&mut take);
            Box::new(RGBA64 {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        5 => {
            let stride = i64_(&mut take);
            Box::new(Gray {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        6 => {
            let stride = i64_(&mut take);
            Box::new(Gray16 {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        7 => {
            let ratio = match u8_(&mut take) {
                0 => YCbCrSubsampleRatio::Ratio444,
                1 => YCbCrSubsampleRatio::Ratio422,
                2 => YCbCrSubsampleRatio::Ratio420,
                3 => YCbCrSubsampleRatio::Ratio440,
                4 => YCbCrSubsampleRatio::Ratio411,
                5 => YCbCrSubsampleRatio::Ratio410,
                v => panic!("ratio {}", v),
            };
            let y_stride = i64_(&mut take);
            let c_stride = i64_(&mut take);
            let y = buf(&mut take);
            let cb = buf(&mut take);
            let cr = buf(&mut take);
            Box::new(YCbCr {
                y,
                cb,
                cr,
                y_stride,
                c_stride,
                subsample_ratio: ratio,
                rect: r,
            })
        }
        8 => {
            let stride = i64_(&mut take);
            let pix = buf(&mut take);
            let n = u32::from_le_bytes(take(4).try_into().unwrap());
            let mut pal = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let c = match u8_(&mut take) {
                    1 => {
                        let v = take(4);
                        Color::RGBA(color::RGBA {
                            r: v[0],
                            g: v[1],
                            b: v[2],
                            a: v[3],
                        })
                    }
                    2 => {
                        let v = take(4);
                        Color::NRGBA(color::NRGBA {
                            r: v[0],
                            g: v[1],
                            b: v[2],
                            a: v[3],
                        })
                    }
                    3 => Color::Gray(color::Gray { y: take(1)[0] }),
                    4 => {
                        let v = take(8);
                        let w = |i: usize| (v[i] as u16) << 8 | v[i + 1] as u16;
                        Color::RGBA64(color::RGBA64 {
                            r: w(0),
                            g: w(2),
                            b: w(4),
                            a: w(6),
                        })
                    }
                    5 => {
                        let v = take(8);
                        let w = |i: usize| (v[i] as u16) << 8 | v[i + 1] as u16;
                        Color::NRGBA64(color::NRGBA64 {
                            r: w(0),
                            g: w(2),
                            b: w(4),
                            a: w(6),
                        })
                    }
                    6 => Color::Alpha(color::Alpha { a: take(1)[0] }),
                    k => panic!("palette colour kind {}", k),
                };
                pal.push(c);
            }
            Box::new(Paletted {
                pix,
                stride,
                rect: r,
                palette: Palette(pal),
            })
        }
        9 => {
            let stride = i64_(&mut take);
            Box::new(CMYK {
                pix: buf(&mut take),
                stride,
                rect: r,
            })
        }
        k => panic!("dump kind {}", k),
    }
}

/// Go: dump.go:runRealOp
pub fn run_real_op(
    src: &dyn Image,
    kind: &str,
    arg: &str,
    dumps: &dyn Fn(&str) -> Arc<dyn Image>,
) -> Box<dyn draw::Image> {
    match kind {
        "dofilter" => do_filter(src, parse_filters(arg, dumps)),
        "overlay" => {
            let f: Vec<&str> = arg.split(',').collect();
            let b = src.bounds();
            let wm = do_filter(
                &*dumps(f[0]),
                vec![gift::resize(b.dx(), b.dy(), &gift::BOX_RESAMPLING)],
            );
            do_filter(
                src,
                vec![Arc::new(OverlayFilter {
                    src: Arc::from(boxed_draw_to_image(wm)),
                    x: pi(f[1]),
                    y: pi(f[2]),
                })],
            )
        }
        "overlaya" => {
            let f: Vec<&str> = arg.split(',').collect();
            let (w, h) = (pi(f[1]), pi(f[2]));
            let a = do_filter(src, vec![gift::resize(w, h, &gift::BOX_RESAMPLING)]);
            let wm = do_filter(
                &*dumps(f[0]),
                vec![gift::resize(w, h, &gift::BOX_RESAMPLING)],
            );
            do_filter(
                &*boxed_draw_to_image(a),
                vec![Arc::new(OverlayFilter {
                    src: Arc::from(boxed_draw_to_image(wm)),
                    x: 0,
                    y: 0,
                })],
            )
        }
        _ => panic!("bad op {}", kind),
    }
}

/// Upcasts a boxed draw.Image to a boxed image.Image.
pub fn boxed_draw_to_image(m: Box<dyn draw::Image>) -> Box<dyn Image> {
    m
}

/// Go: extra.go:setterTable — 65536 float32 values around the setters'
/// rounding boundaries, out-of-range and special values.
pub fn setter_table() -> Vec<f32> {
    let mut tab = vec![0f32; 65536];
    for (i, slot) in tab.iter_mut().enumerate() {
        let t = i % 1024;
        let kind = (i / 1024) % 8;
        let v: f64 = match kind {
            0 => ((t % 256) as f64 + 0.5) / 255.0,
            1 => (((t * 61) % 65535) as f64 + 0.5) / 65535.0,
            2 => t as f64 / 1023.0,
            3 => ((t % 256) as f64 + 0.5) / 255.0 * 0.5,
            4 => -(t as f64) / 4096.0,
            5 => 1.0 + t as f64 / 512.0,
            6 => ((t % 256) as f64 + 0.5) / 65535.0 * 257.0,
            _ => 0.0,
        };
        let bits = if kind == 7 {
            [
                0x7fc00000u32,
                0x7f800000,
                0xff800000,
                0,
                0x80000000,
                0x3f800000,
                0x00000001,
                0x7f7fffff,
            ][t % 8]
        } else {
            let dlt = ((i / 8192) % 9) as i32 - 4;
            ((v as f32).to_bits() as i32).wrapping_add(dlt) as u32
        };
        *slot = f32::from_bits(bits);
    }
    tab
}

// ---------------------------------------------------------------------------
// Red-team spec grammar (Go: redteam.go). The generators live only in Go;
// the Rust tests replay their lines.

/// Go: redteam.go:wrapImg — hides the concrete type (generic getter, no
/// Opaque method).
pub struct WrapImg(pub Box<dyn Image>);

impl Image for WrapImg {
    fn color_model(&self) -> color::Model {
        self.0.color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.at(x, y)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

/// Go: redteam.go:wrapDraw — hides the concrete type of a settable image
/// (generic getter and setter).
pub struct WrapDraw(pub Box<dyn draw::Image>);

impl Image for WrapDraw {
    fn color_model(&self) -> color::Model {
        self.0.color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.at(x, y)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl draw::Image for WrapDraw {
    fn set(&mut self, x: i64, y: i64, c: Color) {
        self.0.set(x, y, c)
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Go: redteam.go:parseSlashRect
fn parse_slash_rect(s: &str) -> Rectangle {
    let f: Vec<i64> = s.split('/').map(pi).collect();
    assert_eq!(f.len(), 4, "bad rect {}", s);
    rect(f[0], f[1], f[2], f[3])
}

/// Go: `parent.(subImager).SubImage(r)` for every sub-imageable type.
fn sub_image_of(img: TestImage, r: Rectangle) -> TestImage {
    macro_rules! try_draw {
        ($m:expr, $($t:ty),*) => {
            $(if let Some(m) = $m.downcast_ref::<$t>() {
                return TestImage::Draw(Box::new(m.sub_image(r)));
            })*
        };
    }
    match img {
        TestImage::Draw(m) => {
            let m: &dyn Image = &*m;
            try_draw!(
                m, NRGBA, RGBA, NRGBA64, RGBA64, Gray, Gray16, CMYK, Alpha, Alpha16, Paletted
            );
            panic!("no SubImage");
        }
        TestImage::Plain(m) => {
            if let Some(m) = m.downcast_ref::<YCbCr>() {
                return TestImage::Plain(Box::new(m.sub_image(r)));
            }
            if let Some(m) = m.downcast_ref::<NYCbCrA>() {
                return TestImage::Plain(Box::new(m.sub_image(r)));
            }
            panic!("no SubImage");
        }
    }
}

/// Go: redteam.go:genImageX
pub fn gen_image_x(s: &ImgSpec) -> TestImage {
    let mut typ = s.typ.as_str();
    let wrap = typ.starts_with("w.");
    if wrap {
        typ = &typ[2..];
    }
    let img = if let Some(at) = typ.find('@') {
        let mut ps = s.clone();
        ps.typ = typ[..at].to_string();
        ps.rect = parse_slash_rect(&typ[at + 1..]);
        sub_image_of(gen_image_base(&ps), s.rect)
    } else {
        let mut ps = s.clone();
        ps.typ = typ.to_string();
        gen_image_base(&ps)
    };
    if wrap {
        return match img {
            TestImage::Draw(m) => TestImage::Draw(Box::new(WrapDraw(m))),
            TestImage::Plain(m) => TestImage::Plain(Box::new(WrapImg(m))),
        };
    }
    img
}

// Go: redteam.go:genImageBase
fn gen_image_base(s: &ImgSpec) -> TestImage {
    let r = &mut Rng::new(s.seed);
    // (gen_image also handles every NYCbCrA ratio, Go's genImageBase does.)
    match s.typ.as_str() {
        "palettedx" | "palettedoor" => {
            let p = gen_palette2(r, s.vmode, s.amode, s.valid);
            let n = p.len() as i64;
            let mut m = Paletted::new(s.rect, p);
            if !s.blank {
                for px in m.pix.iter_mut() {
                    *px = if s.typ == "palettedoor" {
                        r.byte()
                    } else {
                        r.intn(n) as u8
                    };
                }
            }
            TestImage::Draw(Box::new(m))
        }
        "palettedempty" => TestImage::Draw(Box::new(Paletted::new(s.rect, Palette(Vec::new())))),
        "rectimg" => TestImage::Plain(Box::new(s.rect)),
        "grayseq" => {
            let mut m = Gray::new(s.rect);
            for (i, p) in m.pix.iter_mut().enumerate() {
                *p = i as u8;
            }
            TestImage::Draw(Box::new(m))
        }
        _ => gen_image(s),
    }
}

// Go: redteam.go:genColor2
pub fn gen_color2(r: &mut Rng, vmode: i64, amode: i64, valid: bool) -> Color {
    let k = r.intn(11);
    match k {
        6 => Color::Gray16(color::Gray16 {
            y: gen_val16(r, vmode),
        }),
        7 => Color::Alpha16(color::Alpha16 {
            a: gen_alpha16(r, amode),
        }),
        8 => Color::CMYK(color::CMYK {
            c: gen_val(r, vmode),
            m: gen_val(r, vmode),
            y: gen_val(r, vmode),
            k: gen_val(r, vmode),
        }),
        9 => Color::YCbCr(color::YCbCr {
            y: gen_val(r, vmode),
            cb: gen_val(r, vmode),
            cr: gen_val(r, vmode),
        }),
        10 => {
            let y = gen_val(r, vmode);
            let cb = gen_val(r, vmode);
            let cr = gen_val(r, vmode);
            Color::NYCbCrA(color::NYCbCrA {
                ycbcr: color::YCbCr { y, cb, cr },
                a: gen_alpha8(r, amode),
            })
        }
        _ => gen_color_k(r, k, vmode, amode, valid),
    }
}

// Go: redteam.go:genPalette2
fn gen_palette2(r: &mut Rng, vmode: i64, amode: i64, valid: bool) -> Palette {
    let n = match r.intn(4) {
        0 => 1 + r.intn(2),
        1 => 1 + r.intn(16),
        2 => 1 + r.intn(256),
        _ => 257 + r.intn(44),
    };
    let mut p = Vec::with_capacity(n as usize);
    for _ in 0..n {
        p.push(gen_color2(r, vmode, amode, valid));
    }
    Palette(p)
}

/// Go: redteam.go:parseColor2
pub fn parse_color2(s: &str) -> Color {
    let f: Vec<&str> = s.split('/').collect();
    let u = |i: usize| -> i64 { pi(f[i]) };
    match f[0] {
        "gray16" => Color::Gray16(color::Gray16 { y: u(1) as u16 }),
        "alpha16" => Color::Alpha16(color::Alpha16 { a: u(1) as u16 }),
        "cmyk" => Color::CMYK(color::CMYK {
            c: u(1) as u8,
            m: u(2) as u8,
            y: u(3) as u8,
            k: u(4) as u8,
        }),
        "ycbcr" => Color::YCbCr(color::YCbCr {
            y: u(1) as u8,
            cb: u(2) as u8,
            cr: u(3) as u8,
        }),
        "nycbcra" => Color::NYCbCrA(color::NYCbCrA {
            ycbcr: color::YCbCr {
                y: u(1) as u8,
                cb: u(2) as u8,
                cr: u(3) as u8,
            },
            a: u(4) as u8,
        }),
        _ => parse_color(s),
    }
}

/// Go: redteam.go:witnessTab — float32 pixels (r, g, b, a bits) whose fused
/// and unfused setter roundings differ (RGBA f32u8(r*fa), RGBA64
/// f32u16(r*fa)).
const WITNESS_TAB: [[u32; 4]; 5] = [
    [0x3f000000, 0x3e800000, 0x3f400000, 0x3f800000],
    [0x3d2026c5, 0x3d2026c5, 0x3d2026c5, 0x3d4d68a1],
    [0x3d20086b, 0x3d20086b, 0x00000000, 0x3d4d8f96],
    [0x391fc3d7, 0x391fc3d7, 0x391fc3d7, 0x3d4d1ab7],
    [0x391f873d, 0x3d2026c5, 0x391f873d, 0x3d4d68a1],
];

/// Go: redteam.go:witnessFunc (no float arithmetic but the index).
fn witness_filter() -> Arc<dyn Filter> {
    gift::color_func(|r0, _g0, _b0, _a0| {
        let k = ((r0 as f64) * 255.0).round() as usize;
        let w = WITNESS_TAB[k % WITNESS_TAB.len()];
        (
            f32::from_bits(w[0]),
            f32::from_bits(w[1]),
            f32::from_bits(w[2]),
            f32::from_bits(w[3]),
        )
    })
}

/// Go: redteam.go:parseFilterX
pub fn parse_filter_x(s: &str) -> Arc<dyn Filter> {
    if s == "witness()" {
        return witness_filter();
    }
    if let Some(inner) = s.strip_prefix("rotate(") {
        let a: Vec<&str> = inner[..inner.len() - 1].split(',').collect();
        return gift::rotate(pf(a[0]), parse_color2(a[1]), gift::Interpolation(pi(a[2])));
    }
    if let Some(inner) = s.strip_prefix("overlayg(") {
        let a: Vec<&str> = inner[..inner.len() - 1].split(',').collect();
        let spec = ImgSpec {
            typ: a[0].to_string(),
            rect: rect(pi(a[1]), pi(a[2]), pi(a[3]), pi(a[4])),
            seed: a[5].parse().unwrap(),
            vmode: pi(a[6]),
            amode: 2,
            valid: true,
            blank: false,
        };
        let src: Arc<dyn Image> = match gen_image_x(&spec) {
            TestImage::Draw(m) => Arc::from(boxed_draw_to_image(m)),
            TestImage::Plain(m) => Arc::from(m),
        };
        return Arc::new(OverlayFilter {
            src,
            x: pi(a[7]),
            y: pi(a[8]),
        });
    }
    parse_filter(s, &|n: &str| -> Arc<dyn Image> { panic!("no dump {}", n) })
}

pub fn parse_filters_x(s: &str) -> Vec<Arc<dyn Filter>> {
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split('|').map(parse_filter_x).collect()
}

/// Go: redteam.go:digestX (wrappers are unwrapped).
pub fn digest_x(img: &dyn Image) -> String {
    if let Some(w) = img.downcast_ref::<WrapDraw>() {
        return digest(&*w.0);
    }
    digest(img)
}

/// Go: redteam.go:runSynthX — the digest of one synth-format line.
pub fn run_synth_x(row: &[String]) -> String {
    let src = gen_image_x(&parse_img_spec(&row[2]));
    let g = gift::new(parse_filters_x(&row[5]));
    let mut dst = gen_image_x(&parse_img_spec(&row[3]));
    if row[1] == "drawat" {
        let f: Vec<&str> = row[4].split(',').collect();
        let op = if pi(f[2]) == 1 {
            gift::OVER_OPERATOR
        } else {
            gift::COPY_OPERATOR
        };
        g.draw_at(dst.draw_image(), src.image(), pt(pi(f[0]), pi(f[1])), op);
    } else {
        g.draw(dst.draw_image(), src.image());
    }
    digest_x(dst.image())
}

/// The result of one red-team line: the digest, `panic`, or for `bounds`
/// lines the rectangle `x0,y0,x1,y1` (Go: redteam.go:rtBounds).
pub fn run_redteam_line(row: &[String]) -> String {
    let run = || {
        if row[1] == "bounds" {
            let r: Vec<i64> = row[2].split(',').map(pi).collect();
            let b = gift::new(parse_filters_x(&row[5])).bounds(rect(r[0], r[1], r[2], r[3]));
            return format!("{},{},{},{}", b.min.x, b.min.y, b.max.x, b.max.y);
        }
        run_synth_x(row)
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(d) => d,
        Err(_) => "panic".to_string(),
    }
}
