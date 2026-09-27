//! Ports of go1.27.1 `image/geom_test.go`, `image/image_test.go`,
//! `image/ycbcr_test.go`, `image/color/ycbcr_test.go` and
//! `image/color/color_test.go` (TestPalette).

#![allow(clippy::type_complexity)]

use go_image::color::{self, Color, Model, Palette, palette};
use go_image::draw::{Image as DrawImage, RGBA64Image as DrawRGBA64Image};
use go_image::{
    Alpha, Alpha16, CMYK, Gray, Gray16, Image, NRGBA, NRGBA64, NYCbCrA, Paletted, Point, RGBA,
    RGBA64, Rectangle, Uniform, YCbCr, YCbCrSubsampleRatio, pt, rect,
};

// Go: TestRectangle
#[test]
fn rectangle() {
    fn in_(f: Rectangle, g: Rectangle) -> bool {
        if !f.in_(g) {
            return false;
        }
        for y in f.min.y..f.max.y {
            for x in f.min.x..f.max.x {
                if !pt(x, y).in_(g) {
                    return false;
                }
            }
        }
        true
    }
    let rects = [
        rect(0, 0, 10, 10),
        rect(10, 0, 20, 10),
        rect(1, 2, 3, 4),
        rect(4, 6, 10, 10),
        rect(2, 3, 12, 5),
        rect(-1, -2, 0, 0),
        rect(-1, -2, 4, 6),
        rect(-10, -20, 30, 40),
        rect(8, 8, 8, 8),
        rect(88, 88, 88, 88),
        rect(6, 5, 4, 3),
    ];
    for r in rects {
        for s in rects {
            assert_eq!(r.eq(s), in_(r, s) && in_(s, r), "Eq r={} s={}", r, s);
        }
    }
    for r in rects {
        for s in rects {
            let a = r.intersect(s);
            assert!(in_(a, r) && in_(a, s));
            assert_ne!(a == Rectangle::default(), r.overlaps(s));
            let mut larger = [a, a, a, a];
            larger[0].min.x -= 1;
            larger[1].min.y -= 1;
            larger[2].max.x += 1;
            larger[3].max.y += 1;
            for b in larger {
                if b.empty() {
                    continue;
                }
                assert!(!(in_(b, r) && in_(b, s)), "intersection could be larger");
            }
        }
    }
    for r in rects {
        for s in rects {
            let a = r.union(s);
            assert!(in_(r, a) && in_(s, a));
            if a.empty() {
                continue;
            }
            let mut smaller = [a, a, a, a];
            smaller[0].min.x += 1;
            smaller[1].min.y += 1;
            smaller[2].max.x -= 1;
            smaller[3].max.y -= 1;
            for b in smaller {
                assert!(!(in_(r, b) && in_(s, b)), "union could be smaller");
            }
        }
    }
    assert_eq!(rect(3, 4, 6, 5).to_string(), "(3,4)-(6,5)");
    assert_eq!(pt(-3, 4).to_string(), "(-3,4)");
    assert_eq!(pt(7, -3).mod_(rect(0, 0, 5, 5)), pt(2, 2));
    assert_eq!(rect(0, 0, 10, 4).inset(3), rect(3, 2, 7, 2));
}

fn cmp(cm: &Model, c0: Color, c1: Color) -> bool {
    cm.convert(c0).rgba() == cm.convert(c1).rgba()
}

const TRANSPARENT: Color = Color::Alpha16(color::TRANSPARENT);
const OPAQUE: Color = Color::Alpha16(color::OPAQUE);

macro_rules! test_image {
    ($m:expr) => {{
        let mut m = $m;
        assert!(rect(0, 0, 10, 10).eq(m.bounds()));
        assert!(cmp(&m.color_model(), TRANSPARENT, m.at(6, 3)));
        m.set(6, 3, OPAQUE);
        assert!(cmp(&m.color_model(), OPAQUE, m.at(6, 3)));
        assert!(m.sub_image(rect(6, 3, 7, 4)).opaque());
        let mut m = m.sub_image(rect(3, 2, 9, 8));
        assert!(rect(3, 2, 9, 8).eq(m.bounds()));
        assert!(cmp(&m.color_model(), OPAQUE, m.at(6, 3)));
        assert!(cmp(&m.color_model(), TRANSPARENT, m.at(3, 3)));
        m.set(3, 3, OPAQUE);
        assert!(cmp(&m.color_model(), OPAQUE, m.at(3, 3)));
        // Taking an empty sub-image starting at a corner does not panic.
        m.sub_image(rect(0, 0, 0, 0));
        m.sub_image(rect(10, 0, 10, 0));
        m.sub_image(rect(0, 10, 0, 10));
        m.sub_image(rect(10, 10, 10, 10));
    }};
}

// Go: TestImage
#[test]
fn image() {
    let r = rect(0, 0, 10, 10);
    test_image!(RGBA::new(r));
    test_image!(RGBA64::new(r));
    test_image!(NRGBA::new(r));
    test_image!(NRGBA64::new(r));
    test_image!(Alpha::new(r));
    test_image!(Alpha16::new(r));
    test_image!(Gray::new(r));
    test_image!(Gray16::new(r));
    test_image!(Paletted::new(r, Palette(vec![TRANSPARENT, OPAQUE])));
}

// Go: TestNewXxxBadRectangle
#[test]
fn new_xxx_bad_rectangle() {
    let fs: Vec<(&str, fn(Rectangle))> = vec![
        ("RGBA", |r| {
            RGBA::new(r);
        }),
        ("RGBA64", |r| {
            RGBA64::new(r);
        }),
        ("NRGBA", |r| {
            NRGBA::new(r);
        }),
        ("NRGBA64", |r| {
            NRGBA64::new(r);
        }),
        ("Alpha", |r| {
            Alpha::new(r);
        }),
        ("Alpha16", |r| {
            Alpha16::new(r);
        }),
        ("Gray", |r| {
            Gray::new(r);
        }),
        ("Gray16", |r| {
            Gray16::new(r);
        }),
        ("CMYK", |r| {
            CMYK::new(r);
        }),
        ("Paletted", |r| {
            Paletted::new(r, palette::plan9());
        }),
        ("YCbCr", |r| {
            YCbCr::new(r, YCbCrSubsampleRatio::Ratio422);
        }),
        ("NYCbCrA", |r| {
            NYCbCrA::new(r, YCbCrSubsampleRatio::Ratio444);
        }),
    ];
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for (name, f) in fs {
        for neg_dx in [false, true] {
            for neg_dy in [false, true] {
                let mut r = Rectangle {
                    min: pt(15, 28),
                    max: pt(16, 29),
                };
                if neg_dx {
                    r.max.x = 14;
                }
                if neg_dy {
                    r.max.y = 27;
                }
                let got = std::panic::catch_unwind(|| f(r)).is_ok();
                assert_eq!(got, !neg_dx && !neg_dy, "New{}", name);
            }
        }
        let got = std::panic::catch_unwind(|| {
            f(Rectangle {
                min: pt(0, 0),
                max: pt(i64::MAX, i64::MAX),
            })
        })
        .is_ok();
        assert!(!got, "New{}: overflow", name);
    }
    std::panic::set_hook(prev);
}

// Go: Test16BitsPerColorChannel
#[test]
fn bits16_per_color_channel() {
    for cm in [Model::RGBA64, Model::NRGBA64, Model::Alpha16, Model::Gray16] {
        let c = cm.convert(Color::RGBA64(color::RGBA64 {
            r: 0x1234,
            g: 0x1234,
            b: 0x1234,
            a: 0x1234,
        }));
        assert_eq!(c.rgba().0, 0x1234);
    }
    let c = Color::NRGBA64(color::NRGBA64 {
        r: 0xffff,
        g: 0xffff,
        b: 0xffff,
        a: 0x1357,
    });
    let r = rect(0, 0, 10, 10);
    let mut m1 = RGBA64::new(r);
    m1.set(1, 2, c);
    assert_eq!(m1.at(1, 2).rgba().0, 0x1357);
    let mut m2 = NRGBA64::new(r);
    m2.set(1, 2, c);
    assert_eq!(m2.at(1, 2).rgba().0, 0x1357);
    let mut m3 = Alpha16::new(r);
    m3.set(1, 2, c);
    assert_eq!(m3.at(1, 2).rgba().0, 0x1357);
    let mut m4 = Gray16::new(r);
    m4.set(1, 2, c);
    assert_eq!(m4.at(1, 2).rgba().0, 0x1357);
}

// Go: TestRGBA64Image
#[test]
fn rgba64_image() {
    let r = rect(0, 0, 3, 2);
    let c = color::RGBA64 {
        r: 0x7FFF,
        g: 0x3FFF,
        b: 0x0000,
        a: 0x7FFF,
    };
    let mut settable: Vec<Box<dyn DrawRGBA64Image>> = vec![
        Box::new(Alpha::new(r)),
        Box::new(Alpha16::new(r)),
        Box::new(CMYK::new(r)),
        Box::new(Gray::new(r)),
        Box::new(Gray16::new(r)),
        Box::new(NRGBA::new(r)),
        Box::new(NRGBA64::new(r)),
        Box::new(Paletted::new(r, palette::plan9())),
        Box::new(RGBA::new(r)),
        Box::new(RGBA64::new(r)),
    ];
    let mut others: Vec<Box<dyn Image>> = Vec::new();
    let mut ny = NYCbCrA::new(r, YCbCrSubsampleRatio::Ratio444);
    ny.ycbcr.y.fill(0x77);
    ny.ycbcr.cb.fill(0x88);
    ny.ycbcr.cr.fill(0x99);
    ny.a.fill(0xAA);
    others.push(Box::new(ny));
    others.push(Box::new(Uniform::new(Color::RGBA64(c))));
    let mut y = YCbCr::new(r, YCbCrSubsampleRatio::Ratio444);
    y.y.fill(0x77);
    y.cb.fill(0x88);
    y.cr.fill(0x99);
    others.push(Box::new(y));
    others.push(Box::new(r));
    for m in settable.iter_mut() {
        m.set_rgba64(1, 1, c);
        let got = m.rgba64_at(1, 1);
        let (wr, wg, wb, wa) = m.at(1, 1).rgba();
        assert_eq!(
            (got.r as u32, got.g as u32, got.b as u32, got.a as u32),
            (wr, wg, wb, wa)
        );
    }
    for m in others.iter() {
        let got = m.as_rgba64_image().unwrap().rgba64_at(1, 1);
        let (wr, wg, wb, wa) = m.at(1, 1).rgba();
        assert_eq!(
            (got.r as u32, got.g as u32, got.b as u32, got.a as u32),
            (wr, wg, wb, wa)
        );
    }
}

// Go: TestYCbCr
#[test]
fn ycbcr() {
    let rects = [
        rect(0, 0, 16, 16),
        rect(1, 0, 16, 16),
        rect(0, 1, 16, 16),
        rect(1, 1, 16, 16),
        rect(1, 1, 15, 16),
        rect(1, 1, 16, 15),
        rect(1, 1, 15, 15),
        rect(2, 3, 14, 15),
        rect(7, 0, 7, 16),
        rect(0, 8, 16, 8),
        rect(0, 0, 10, 11),
        rect(5, 6, 16, 16),
        rect(7, 7, 8, 8),
        rect(7, 8, 8, 9),
        rect(8, 7, 9, 8),
        rect(8, 8, 9, 9),
        rect(7, 7, 17, 17),
        rect(8, 8, 17, 17),
        rect(9, 9, 17, 17),
        rect(10, 10, 17, 17),
    ];
    let ratios = [
        YCbCrSubsampleRatio::Ratio444,
        YCbCrSubsampleRatio::Ratio422,
        YCbCrSubsampleRatio::Ratio420,
        YCbCrSubsampleRatio::Ratio440,
        YCbCrSubsampleRatio::Ratio411,
        YCbCrSubsampleRatio::Ratio410,
    ];
    let deltas = [pt(0, 0), pt(1000, 1001), pt(5001, -400), pt(-701, -801)];
    for r in rects {
        for ratio in ratios {
            for delta in deltas {
                test_ycbcr(r, ratio, delta);
            }
        }
    }
}

fn test_ycbcr(r: Rectangle, ratio: YCbCrSubsampleRatio, delta: Point) {
    let r1 = r.add(delta);
    let mut m = YCbCr::new(r1, ratio);
    assert!(m.y.len() <= 100 * 100);
    for y in r1.min.y..r1.max.y {
        for x in r1.min.x..r1.max.x {
            let yi = m.y_offset(x, y) as usize;
            let ci = m.c_offset(x, y) as usize;
            m.y[yi] = (16 * y + x) as u8;
            m.cb[ci] = (y + 16 * x) as u8;
            m.cr[ci] = (y + 16 * x) as u8;
        }
    }
    for y0 in delta.y + 3..delta.y + 7 {
        for y1 in delta.y + 8..delta.y + 13 {
            for x0 in delta.x + 3..delta.x + 7 {
                for x1 in delta.x + 8..delta.x + 13 {
                    let sub = m.sub_image(rect(x0, y0, x1, y1));
                    for y in sub.rect.min.y..sub.rect.max.y {
                        for x in sub.rect.min.x..sub.rect.max.x {
                            assert_eq!(
                                m.ycbcr_at(x, y),
                                sub.ycbcr_at(x, y),
                                "r={} ratio={} delta={} x={} y={}",
                                r,
                                ratio,
                                delta,
                                x,
                                y
                            );
                        }
                    }
                }
            }
        }
    }
}

// Go: color TestYCbCrRoundtrip, TestYCbCrToRGBConsistency, TestYCbCrGray,
// TestNYCbCrAAlpha, TestNYCbCrAYCbCr, TestCMYKRoundtrip,
// TestCMYKToRGBConsistency, TestCMYKGray.
#[test]
fn color_consistency() {
    let delta = |x: u8, y: u8| x.abs_diff(y);
    for r in (0..256).step_by(7) {
        for g in (0..256).step_by(5) {
            for b in (0..256).step_by(3) {
                let (r0, g0, b0) = (r as u8, g as u8, b as u8);
                let (y, cb, cr) = color::rgb_to_ycbcr(r0, g0, b0);
                let (r1, g1, b1) = color::ycbcr_to_rgb(y, cb, cr);
                assert!(delta(r0, r1) <= 2 && delta(g0, g1) <= 2 && delta(b0, b1) <= 2);
                let (c, m, yy, k) = color::rgb_to_cmyk(r0, g0, b0);
                let (r1, g1, b1) = color::cmyk_to_rgb(c, m, yy, k);
                assert!(delta(r0, r1) <= 1 && delta(g0, g1) <= 1 && delta(b0, b1) <= 1);
            }
        }
    }
    for y in (0..256).step_by(7) {
        for cb in (0..256).step_by(5) {
            for cr in (0..256).step_by(3) {
                let x = color::YCbCr {
                    y: y as u8,
                    cb: cb as u8,
                    cr: cr as u8,
                };
                let (r0, g0, b0, _) = x.rgba();
                assert_eq!(
                    ((r0 >> 8) as u8, (g0 >> 8) as u8, (b0 >> 8) as u8),
                    color::ycbcr_to_rgb(x.y, x.cb, x.cr)
                );
            }
        }
    }
    for i in 0..256u32 {
        let i8 = i as u8;
        assert_eq!(
            color::YCbCr {
                y: i8,
                cb: 0x80,
                cr: 0x80
            }
            .rgba(),
            color::Gray { y: i8 }.rgba()
        );
        assert_eq!(
            color::NYCbCrA {
                ycbcr: color::YCbCr {
                    y: 0xff,
                    cb: 0x80,
                    cr: 0x80
                },
                a: i8
            }
            .rgba(),
            color::Alpha { a: i8 }.rgba()
        );
        assert_eq!(
            color::NYCbCrA {
                ycbcr: color::YCbCr {
                    y: i8,
                    cb: 0x40,
                    cr: 0xc0
                },
                a: 0xff
            }
            .rgba(),
            color::YCbCr {
                y: i8,
                cb: 0x40,
                cr: 0xc0
            }
            .rgba()
        );
        assert_eq!(
            color::CMYK {
                c: 0,
                m: 0,
                y: 0,
                k: 255 - i8
            }
            .rgba(),
            color::Gray { y: i8 }.rgba()
        );
    }
    for c in (0..256).step_by(7) {
        for m in (0..256).step_by(5) {
            for y in (0..256).step_by(3) {
                for k in (0..256).step_by(11) {
                    let x = color::CMYK {
                        c: c as u8,
                        m: m as u8,
                        y: y as u8,
                        k: k as u8,
                    };
                    let (r0, g0, b0, _) = x.rgba();
                    assert_eq!(
                        ((r0 >> 8) as u8, (g0 >> 8) as u8, (b0 >> 8) as u8),
                        color::cmyk_to_rgb(x.c, x.m, x.y, x.k)
                    );
                }
            }
        }
    }
}

// Go: color TestPalette
#[test]
fn palette_index_convert() {
    let rgba = |r, g, b, a| Color::RGBA(color::RGBA { r, g, b, a });
    let p = Palette(vec![
        rgba(0xff, 0xff, 0xff, 0xff),
        rgba(0x80, 0x00, 0x00, 0xff),
        rgba(0x7f, 0x00, 0x00, 0x7f),
        rgba(0x00, 0x00, 0x00, 0x7f),
        rgba(0x00, 0x00, 0x00, 0x00),
        rgba(0x40, 0x40, 0x40, 0x40),
    ]);
    for (i, &c) in p.iter().enumerate() {
        assert_eq!(p.index(c), i);
    }
    assert_eq!(
        p.convert(rgba(0x80, 0x00, 0x00, 0x80)),
        Some(rgba(0x7f, 0x00, 0x00, 0x7f))
    );
    assert_eq!(Palette::default().convert(rgba(1, 2, 3, 4)), None);
    assert_eq!(palette::plan9().len(), 256);
    assert_eq!(palette::web_safe().len(), 216);
}

#[test]
fn subsample_ratio_string_and_opaque() {
    assert_eq!(
        YCbCrSubsampleRatio::Ratio420.to_string(),
        "YCbCrSubsampleRatio420"
    );
    let mut m = NRGBA::new(rect(0, 0, 2, 2));
    assert!(!m.opaque());
    assert_eq!((&m as &dyn Image).try_opaque(), Some(false));
    for p in m.pix.chunks_mut(4) {
        p[3] = 0xff;
    }
    assert!(m.opaque());
    assert_eq!((&rect(0, 0, 1, 1) as &dyn Image).try_opaque(), None);
    let mut p = Paletted::new(rect(0, 0, 2, 2), Palette(vec![OPAQUE, TRANSPARENT]));
    assert!(p.opaque());
    p.set_color_index(1, 1, 1);
    assert!(!p.opaque());
    // with_sub_image_mut emulates Go's aliasing SubImage.
    let mut m = RGBA::new(rect(0, 0, 4, 4));
    m.with_sub_image_mut(rect(1, 1, 3, 3), |s| {
        DrawImage::set(s, 2, 2, Color::Gray(color::Gray { y: 9 }));
    });
    assert_eq!(
        m.rgba_at(2, 2),
        color::RGBA {
            r: 9,
            g: 9,
            b: 9,
            a: 255
        }
    );
    assert_eq!(m.pix.len(), 64);
}
