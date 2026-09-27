//! Differential test: colour conversions over full (or exhaustive
//! per-channel) input spaces against the Go oracle (`go-image color`).

mod common;

use common::*;
use go_image::color::{self, Color, Model, palette};
use sha2::{Digest, Sha256};

/// Streams little-endian values into SHA-256 (the oracle's hasher).
struct Hasher {
    h: Sha256,
    buf: Vec<u8>,
}

impl Hasher {
    fn new() -> Hasher {
        Hasher {
            h: Sha256::new(),
            buf: Vec::with_capacity(1 << 16),
        }
    }
    fn flush_if_full(&mut self) {
        if self.buf.len() >= (1 << 16) - 16 {
            self.h.update(&self.buf);
            self.buf.clear();
        }
    }
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
        self.flush_if_full();
    }
    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.flush_if_full();
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.flush_if_full();
    }
    fn sum(mut self) -> String {
        self.h.update(&self.buf);
        self.h
            .finalize()
            .iter()
            .map(|x| format!("{:02x}", x))
            .collect()
    }
}

fn color_digest(h: &mut Hasher, c: Color) {
    match c {
        Color::RGBA(c) => {
            h.u8(0);
            h.u8(c.r);
            h.u8(c.g);
            h.u8(c.b);
            h.u8(c.a);
        }
        Color::RGBA64(c) => {
            h.u8(1);
            h.u16(c.r);
            h.u16(c.g);
            h.u16(c.b);
            h.u16(c.a);
        }
        Color::NRGBA(c) => {
            h.u8(2);
            h.u8(c.r);
            h.u8(c.g);
            h.u8(c.b);
            h.u8(c.a);
        }
        Color::NRGBA64(c) => {
            h.u8(3);
            h.u16(c.r);
            h.u16(c.g);
            h.u16(c.b);
            h.u16(c.a);
        }
        Color::Alpha(c) => {
            h.u8(4);
            h.u8(c.a);
        }
        Color::Alpha16(c) => {
            h.u8(5);
            h.u16(c.a);
        }
        Color::Gray(c) => {
            h.u8(6);
            h.u8(c.y);
        }
        Color::Gray16(c) => {
            h.u8(7);
            h.u16(c.y);
        }
        Color::YCbCr(c) => {
            h.u8(8);
            h.u8(c.y);
            h.u8(c.cb);
            h.u8(c.cr);
        }
        Color::NYCbCrA(c) => {
            h.u8(9);
            h.u8(c.ycbcr.y);
            h.u8(c.ycbcr.cb);
            h.u8(c.ycbcr.cr);
            h.u8(c.a);
        }
        Color::CMYK(c) => {
            h.u8(10);
            h.u8(c.c);
            h.u8(c.m);
            h.u8(c.y);
            h.u8(c.k);
        }
    }
}

fn rgba_digest(h: &mut Hasher, c: Color) {
    let (r, g, b, a) = c.rgba();
    h.u32(r);
    h.u32(g);
    h.u32(b);
    h.u32(a);
}

fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::RGBA(color::RGBA { r, g, b, a })
}

fn nrgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::NRGBA(color::NRGBA { r, g, b, a })
}

fn models() -> [Model; 11] {
    [
        Model::RGBA,
        Model::RGBA64,
        Model::NRGBA,
        Model::NRGBA64,
        Model::Alpha,
        Model::Alpha16,
        Model::Gray,
        Model::Gray16,
        Model::YCbCr,
        Model::NYCbCrA,
        Model::CMYK,
    ]
}

fn section_models_8bit() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let models = models();
    for (mi, m) in models.iter().enumerate() {
        let mut h = Hasher::new();
        for c in 0u32..256 {
            for a in 0u32..256 {
                let (c8, a8) = (c as u8, a as u8);
                color_digest(&mut h, m.convert(rgba8(c8, 255 - c8, c8 ^ 0x5a, a8)));
                color_digest(&mut h, m.convert(nrgba8(c8, 255 - c8, c8 ^ 0x5a, a8)));
            }
        }
        out.push((format!("model{}.8bit", mi), h.sum()));
    }
    out
}

fn section_0() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for i in 0u32..1 << 24 {
        let (y, cb, cr) = color::rgb_to_ycbcr((i >> 16) as u8, (i >> 8) as u8, i as u8);
        h.u8(y);
        h.u8(cb);
        h.u8(cr);
    }
    out.push(("RGBToYCbCr".to_string(), h.sum()));
    out
}

fn section_1() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for i in 0u32..1 << 24 {
        let (r, g, b) = color::ycbcr_to_rgb((i >> 16) as u8, (i >> 8) as u8, i as u8);
        h.u8(r);
        h.u8(g);
        h.u8(b);
    }
    out.push(("YCbCrToRGB".to_string(), h.sum()));
    out
}

fn section_2() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for i in 0u32..1 << 24 {
        rgba_digest(
            &mut h,
            Color::YCbCr(color::YCbCr {
                y: (i >> 16) as u8,
                cb: (i >> 8) as u8,
                cr: i as u8,
            }),
        );
    }
    out.push(("YCbCr.RGBA".to_string(), h.sum()));
    out
}

fn section_3() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for i in 0u32..1 << 24 {
        let (c, m, y, k) = color::rgb_to_cmyk((i >> 16) as u8, (i >> 8) as u8, i as u8);
        h.u8(c);
        h.u8(m);
        h.u8(y);
        h.u8(k);
    }
    out.push(("RGBToCMYK".to_string(), h.sum()));
    out
}

fn section_4() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for k in [0u8, 1, 77, 128, 254, 255] {
        let mut h = Hasher::new();
        for i in 0u32..1 << 24 {
            let (r, g, b) = color::cmyk_to_rgb((i >> 16) as u8, (i >> 8) as u8, i as u8, k);
            h.u8(r);
            h.u8(g);
            h.u8(b);
            rgba_digest(
                &mut h,
                Color::CMYK(color::CMYK {
                    c: (i >> 16) as u8,
                    m: (i >> 8) as u8,
                    y: i as u8,
                    k,
                }),
            );
        }
        out.push((format!("CMYKToRGB.k{}", k), h.sum()));
    }
    out
}

fn section_5() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for a in [0u8, 1, 127, 128, 255] {
        let mut h = Hasher::new();
        let mut i = 0u32;
        while i < 1 << 24 {
            rgba_digest(
                &mut h,
                Color::NYCbCrA(color::NYCbCrA {
                    ycbcr: color::YCbCr {
                        y: (i >> 16) as u8,
                        cb: (i >> 8) as u8,
                        cr: i as u8,
                    },
                    a,
                }),
            );
            i += 7;
        }
        out.push((format!("NYCbCrA.RGBA.a{}", a), h.sum()));
    }
    out
}

fn section_6() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for c in 0u32..256 {
        for a in 0u32..256 {
            let (c8, a8) = (c as u8, a as u8);
            rgba_digest(&mut h, nrgba8(c8, 255 - c8, c8 ^ 0x5a, a8));
            rgba_digest(&mut h, rgba8(c8, 255 - c8, c8 ^ 0x5a, a8));
            rgba_digest(&mut h, Color::Alpha(color::Alpha { a: a8 }));
            rgba_digest(&mut h, Color::Gray(color::Gray { y: c8 }));
        }
    }
    out.push(("8bit.RGBA".to_string(), h.sum()));
    out
}

fn section_7() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let models = models();
    for (mi, m) in models.iter().enumerate() {
        let mut h = Hasher::new();
        let mut r = Rng::new(mi as u64 + 99);
        for _ in 0..200000 {
            let c = rand_color(&mut r);
            color_digest(&mut h, m.convert(c));
            rgba_digest(&mut h, c);
        }
        out.push((format!("model{}.random", mi), h.sum()));
    }
    out
}

fn section_8() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    for i in 0u32..1 << 24 {
        let c = rgba8((i >> 16) as u8, (i >> 8) as u8, i as u8, 0xff);
        color_digest(&mut h, Model::Gray.convert(c));
        color_digest(&mut h, Model::Gray16.convert(c));
        color_digest(&mut h, Model::YCbCr.convert(c));
    }
    out.push(("GrayYCbCrModels.cube".to_string(), h.sum()));
    out
}

fn section_9() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut h = Hasher::new();
    let mut r = Rng::new(7);
    for _ in 0..1000000 {
        let (cr, cg, cb, ca) = (r.u16(), r.u16(), r.u16(), r.u16());
        let c = color::NRGBA64 {
            r: cr,
            g: cg,
            b: cb,
            a: ca,
        };
        rgba_digest(&mut h, Color::NRGBA64(c));
        let c64 = Color::RGBA64(color::RGBA64 {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        });
        color_digest(&mut h, Model::NRGBA64.convert(c64));
        color_digest(&mut h, Model::NRGBA.convert(c64));
    }
    out.push(("NRGBA64.random".to_string(), h.sum()));
    out
}

fn section_10() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (pi, p) in [palette::plan9(), palette::web_safe()].iter().enumerate() {
        let mut h = Hasher::new();
        let mut r = Rng::new(pi as u64 + 5);
        for _ in 0..100000 {
            let c = rand_color(&mut r);
            h.u32(p.index(c) as u32);
            color_digest(&mut h, p.convert(c).unwrap());
        }
        out.push((format!("palette{}.index", pi), h.sum()));
    }
    let mut h = Hasher::new();
    let mut r = Rng::new(11);
    for _ in 0..20000 {
        let p = rand_palette(&mut r);
        let c = rand_color(&mut r);
        h.u32(p.index(c) as u32);
    }
    out.push(("randpalette.index".to_string(), h.sum()));
    out
}

fn compute() -> Vec<(String, String)> {
    type Section = fn() -> Vec<(String, String)>;
    let sections: Vec<Section> = vec![
        section_0,
        section_1,
        section_2,
        section_3,
        section_4,
        section_5,
        section_6,
        section_models_8bit,
        section_7,
        section_8,
        section_9,
        section_10,
    ];
    std::thread::scope(|s| {
        let handles: Vec<_> = sections.into_iter().map(|f| s.spawn(f)).collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

#[test]
fn color_conversions_full_space() {
    let want = read_tsv("color.tsv");
    let got = compute();
    assert_eq!(got.len(), want.len());
    let mut failures = 0;
    for ((gn, gh), w) in got.iter().zip(want.iter()) {
        assert_eq!(gn, &w[0]);
        if gh != &w[1] {
            failures += 1;
            eprintln!("MISMATCH {}: got {} want {}", gn, gh, w[1]);
        }
    }
    assert_eq!(failures, 0);
}
