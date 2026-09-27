//! Shared helpers for the differential tests: a call-for-call port of the
//! oracle's (tools/go-oracle/go-image/main.go) splitmix64 generator and
//! random image/colour builders, plus digest formatting identical to the
//! oracle's `imgDigest`.

#![allow(dead_code, clippy::upper_case_acronyms)]

use go_image::color::{self, Color, Palette, palette};
use go_image::{
    Alpha, Alpha16, CMYK, Gray, Gray16, Image, NRGBA, NRGBA64, NYCbCrA, Paletted, RGBA, RGBA64,
    Rectangle, YCbCr, YCbCrSubsampleRatio, pt, rect,
};
use sha2::{Digest, Sha256};

pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn read_tsv(name: &str) -> Vec<Vec<String>> {
    let s = std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("reading fixture {}: {}", name, e));
    parse_tsv(&s)
}

pub fn parse_tsv(s: &str) -> Vec<Vec<String>> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(|f| f.to_string()).collect())
        .collect()
}

pub fn sha(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn sha16(b: &[u8]) -> String {
    sha(b)[..16].to_string()
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
    pub fn u16(&mut self) -> u16 {
        self.next() as u16
    }
}

pub fn rect_str(r: Rectangle) -> String {
    format!("{},{},{},{}", r.min.x, r.min.y, r.max.x, r.max.y)
}

/// Port of the oracle's imgDigest.
pub fn img_digest(m: &dyn Image) -> String {
    if let Some(m) = m.downcast_ref::<Gray>() {
        return format!(
            "Gray {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<RGBA>() {
        return format!(
            "RGBA {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<CMYK>() {
        return format!(
            "CMYK {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<YCbCr>() {
        return format!(
            "YCbCr {} {} {} {} {} {} {} {} {} {}",
            rect_str(m.rect),
            m.subsample_ratio as i32,
            m.y_stride,
            m.c_stride,
            m.y.len(),
            m.cb.len(),
            m.cr.len(),
            sha(&m.y),
            sha(&m.cb),
            sha(&m.cr)
        );
    }
    "other".to_string()
}

pub fn model_name(m: &color::Model) -> &'static str {
    match m {
        color::Model::Gray => "Gray",
        color::Model::YCbCr => "YCbCr",
        color::Model::RGBA => "RGBA",
        color::Model::CMYK => "CMYK",
        _ => "other",
    }
}

pub const RATIOS: [YCbCrSubsampleRatio; 6] = [
    YCbCrSubsampleRatio::Ratio444,
    YCbCrSubsampleRatio::Ratio422,
    YCbCrSubsampleRatio::Ratio420,
    YCbCrSubsampleRatio::Ratio440,
    YCbCrSubsampleRatio::Ratio411,
    YCbCrSubsampleRatio::Ratio410,
];

/// Port of the oracle's fill.
pub fn fill(r: &mut Rng, b: &mut [u8], stride: i64) {
    let mode = r.intn(2);
    if mode == 0 {
        for x in b.iter_mut() {
            *x = r.byte();
        }
        return;
    }
    let k1 = r.intn(7);
    let k2 = r.intn(7);
    let c = r.intn(256);
    let noise = r.intn(8);
    let stride = if stride <= 0 { 1 } else { stride };
    for (i, x) in b.iter_mut().enumerate() {
        let i = i as i64;
        let mut v = (i % stride) * k1 + (i / stride) * k2 + c;
        if noise > 0 {
            v += r.intn(noise);
        }
        *x = v as u8;
    }
}

pub fn rand_color(r: &mut Rng) -> Color {
    match r.intn(11) {
        0 => Color::RGBA(color::RGBA {
            r: r.byte(),
            g: r.byte(),
            b: r.byte(),
            a: r.byte(),
        }),
        1 => Color::NRGBA(color::NRGBA {
            r: r.byte(),
            g: r.byte(),
            b: r.byte(),
            a: r.byte(),
        }),
        2 => Color::RGBA64(color::RGBA64 {
            r: r.u16(),
            g: r.u16(),
            b: r.u16(),
            a: r.u16(),
        }),
        3 => Color::NRGBA64(color::NRGBA64 {
            r: r.u16(),
            g: r.u16(),
            b: r.u16(),
            a: r.u16(),
        }),
        4 => Color::Gray(color::Gray { y: r.byte() }),
        5 => Color::Gray16(color::Gray16 { y: r.u16() }),
        6 => Color::Alpha(color::Alpha { a: r.byte() }),
        7 => Color::Alpha16(color::Alpha16 { a: r.u16() }),
        8 => Color::YCbCr(color::YCbCr {
            y: r.byte(),
            cb: r.byte(),
            cr: r.byte(),
        }),
        9 => {
            let (y, cb, cr) = (r.byte(), r.byte(), r.byte());
            Color::NYCbCrA(color::NYCbCrA {
                ycbcr: color::YCbCr { y, cb, cr },
                a: r.byte(),
            })
        }
        _ => Color::CMYK(color::CMYK {
            c: r.byte(),
            m: r.byte(),
            y: r.byte(),
            k: r.byte(),
        }),
    }
}

pub fn rand_palette(r: &mut Rng) -> Palette {
    match r.intn(3) {
        0 => palette::plan9(),
        1 => palette::web_safe(),
        _ => {
            let n = 1 + r.intn(20);
            let mut p = Vec::new();
            for _ in 0..n {
                p.push(rand_color(r));
            }
            Palette(p)
        }
    }
}

pub const NUM_KINDS: i64 = 17;

/// A generated image (the oracle returns image.Image / draw.Image).
pub enum Gen {
    RGBA(RGBA),
    NRGBA(NRGBA),
    Gray(Gray),
    YCbCr(YCbCr),
    CMYK(CMYK),
    RGBA64(RGBA64),
    NRGBA64(NRGBA64),
    Paletted(Paletted),
    Gray16(Gray16),
    Alpha(Alpha),
    NYCbCrA(NYCbCrA),
    Alpha16(Alpha16),
}

impl Gen {
    pub fn as_image(&self) -> &dyn Image {
        match self {
            Gen::RGBA(m) => m,
            Gen::NRGBA(m) => m,
            Gen::Gray(m) => m,
            Gen::YCbCr(m) => m,
            Gen::CMYK(m) => m,
            Gen::RGBA64(m) => m,
            Gen::NRGBA64(m) => m,
            Gen::Paletted(m) => m,
            Gen::Gray16(m) => m,
            Gen::Alpha(m) => m,
            Gen::NYCbCrA(m) => m,
            Gen::Alpha16(m) => m,
        }
    }

    pub fn as_draw_image(&mut self) -> &mut dyn go_image::draw::Image {
        match self {
            Gen::RGBA(m) => m,
            Gen::NRGBA(m) => m,
            Gen::Gray(m) => m,
            Gen::CMYK(m) => m,
            Gen::RGBA64(m) => m,
            Gen::NRGBA64(m) => m,
            Gen::Paletted(m) => m,
            Gen::Gray16(m) => m,
            Gen::Alpha(m) => m,
            Gen::Alpha16(m) => m,
            Gen::YCbCr(_) | Gen::NYCbCrA(_) => panic!("not a draw.Image"),
        }
    }

    pub fn go_type(&self) -> &'static str {
        match self {
            Gen::RGBA(_) => "*image.RGBA",
            Gen::NRGBA(_) => "*image.NRGBA",
            Gen::Gray(_) => "*image.Gray",
            Gen::YCbCr(_) => "*image.YCbCr",
            Gen::CMYK(_) => "*image.CMYK",
            Gen::RGBA64(_) => "*image.RGBA64",
            Gen::NRGBA64(_) => "*image.NRGBA64",
            Gen::Paletted(_) => "*image.Paletted",
            Gen::Gray16(_) => "*image.Gray16",
            Gen::Alpha(_) => "*image.Alpha",
            Gen::NYCbCrA(_) => "*image.NYCbCrA",
            Gen::Alpha16(_) => "*image.Alpha16",
        }
    }

    pub fn pix(&self) -> &[u8] {
        match self {
            Gen::RGBA(m) => &m.pix,
            Gen::NRGBA(m) => &m.pix,
            Gen::Gray(m) => &m.pix,
            Gen::CMYK(m) => &m.pix,
            Gen::RGBA64(m) => &m.pix,
            Gen::NRGBA64(m) => &m.pix,
            Gen::Paletted(m) => &m.pix,
            Gen::Gray16(m) => &m.pix,
            Gen::Alpha(m) => &m.pix,
            Gen::Alpha16(m) => &m.pix,
            Gen::YCbCr(_) | Gen::NYCbCrA(_) => panic!("pixOf"),
        }
    }
}

/// Port of the oracle's genImage.
pub fn gen_image(r: &mut Rng, kind: i64, rect_: Rectangle) -> Gen {
    let mut rect_ = rect_;
    match kind {
        0 => {
            let mut m = RGBA::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::RGBA(m)
        }
        1 => {
            let mut m = NRGBA::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::NRGBA(m)
        }
        2 => {
            let mut m = Gray::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::Gray(m)
        }
        3..=8 => {
            rect_ = rect_.add(pt(16, 16));
            let mut m = YCbCr::new(rect_, RATIOS[(kind - 3) as usize]);
            let (ys, cs) = (m.y_stride, m.c_stride);
            fill(r, &mut m.y, ys);
            fill(r, &mut m.cb, cs);
            fill(r, &mut m.cr, cs);
            Gen::YCbCr(m)
        }
        9 => {
            let mut m = CMYK::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::CMYK(m)
        }
        10 => {
            let mut m = RGBA64::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::RGBA64(m)
        }
        11 => {
            let mut m = NRGBA64::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::NRGBA64(m)
        }
        12 => {
            let p = rand_palette(r);
            let n = p.len() as i64;
            let mut m = Paletted::new(rect_, p);
            for x in m.pix.iter_mut() {
                *x = r.intn(n) as u8;
            }
            Gen::Paletted(m)
        }
        13 => {
            let mut m = Gray16::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::Gray16(m)
        }
        14 => {
            let mut m = Alpha::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::Alpha(m)
        }
        15 => {
            rect_ = rect_.add(pt(16, 16));
            let ratio = RATIOS[r.intn(6) as usize];
            let mut m = NYCbCrA::new(rect_, ratio);
            let (ys, cs, as_) = (m.y_stride, m.c_stride, m.a_stride);
            fill(r, &mut m.ycbcr.y, ys);
            fill(r, &mut m.ycbcr.cb, cs);
            fill(r, &mut m.ycbcr.cr, cs);
            fill(r, &mut m.a, as_);
            Gen::NYCbCrA(m)
        }
        _ => {
            let mut m = Alpha16::new(rect_);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Gen::Alpha16(m)
        }
    }
}

pub fn rand_rect(r: &mut Rng, max_size: i64, max_off: i64) -> Rectangle {
    let w = 1 + r.intn(max_size);
    let h = 1 + r.intn(max_size);
    let x0 = r.intn(2 * max_off + 1) - max_off;
    let y0 = r.intn(2 * max_off + 1) - max_off;
    rect(x0, y0, x0 + w, y0 + h)
}
