//! Port of gift v1.2.1 `pixels_test.go`.

use go_image::color::{self, Color, Palette};
use go_image::{
    Alpha, Gray, Gray16, Image, NRGBA, NRGBA64, Paletted, RGBA, RGBA64, Uniform, YCbCr,
    YCbCrSubsampleRatio, draw, rect,
};

use crate::pixels::{
    GetterImg, ImageType, Pixel, PixelGetter, PixelSetter, SetterImg, clampi32, f32u8, f32u16,
};

// Go: pixels_test.go:TestNewPixelGetter
#[test]
fn test_new_pixel_getter() {
    let r = rect(0, 0, 1, 1);
    let check = |img: &dyn Image, it: ImageType, name: &str| {
        let pg = PixelGetter::new(img);
        // pg.nrgba != nil etc.: the typed reference matches the tag.
        let ok_ptr = matches!(
            (it, pg.img),
            (ImageType::NRGBA, GetterImg::NRGBA(_))
                | (ImageType::NRGBA64, GetterImg::NRGBA64(_))
                | (ImageType::RGBA, GetterImg::RGBA(_))
                | (ImageType::RGBA64, GetterImg::RGBA64(_))
                | (ImageType::Gray, GetterImg::Gray(_))
                | (ImageType::Gray16, GetterImg::Gray16(_))
                | (ImageType::YCbCr, GetterImg::YCbCr(_))
                | (ImageType::Generic, GetterImg::Generic(_))
        );
        assert!(
            pg.it == it && ok_ptr && img.bounds().eq(pg.bounds),
            "newPixelGetter {name}"
        );
    };
    check(&NRGBA::new(r), ImageType::NRGBA, "NRGBA");
    check(&NRGBA64::new(r), ImageType::NRGBA64, "NRGBA64");
    check(&RGBA::new(r), ImageType::RGBA, "RGBA");
    check(&RGBA64::new(r), ImageType::RGBA64, "RGBA64");
    check(&Gray::new(r), ImageType::Gray, "Gray");
    check(&Gray16::new(r), ImageType::Gray16, "Gray16");
    check(
        &YCbCr::new(r, YCbCrSubsampleRatio::Ratio422),
        ImageType::YCbCr,
        "YCbCr",
    );
    check(
        &Uniform::new(Color::NRGBA64(color::NRGBA64 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        })),
        ImageType::Generic,
        "Generic(Uniform)",
    );
    check(&Alpha::new(r), ImageType::Generic, "Generic(Alpha)");
}

/// Go: pixels_test.go:comparePixels
fn compare_pixels(px1: Pixel, px2: Pixel, dif: f64) -> bool {
    if (px1.r as f64 - px2.r as f64).abs() > dif {
        return false;
    }
    if (px1.g as f64 - px2.g as f64).abs() > dif {
        return false;
    }
    if (px1.b as f64 - px2.b as f64).abs() > dif {
        return false;
    }
    if (px1.a as f64 - px2.a as f64).abs() > dif {
        return false;
    }
    true
}

/// Go: pixels_test.go:compareColorsNRGBA
fn compare_colors_nrgba(c1: color::NRGBA, c2: color::NRGBA, dif: i64) -> bool {
    if (c1.r as f64 - c2.r as f64).abs() > dif as f64 {
        return false;
    }
    if (c1.g as f64 - c2.g as f64).abs() > dif as f64 {
        return false;
    }
    if (c1.b as f64 - c2.b as f64).abs() > dif as f64 {
        return false;
    }
    if (c1.a as f64 - c2.a as f64).abs() > dif as f64 {
        return false;
    }
    true
}

/// `color.NRGBAModel.Convert(img.At(x, y)).(color.NRGBA)`
fn nrgba_at(img: &dyn Image, x: i64, y: i64) -> color::NRGBA {
    match color::NRGBA_MODEL.convert(img.at(x, y)) {
        Color::NRGBA(c) => c,
        c => panic!("not NRGBA: {c:?}"),
    }
}

fn test_palette() -> Palette {
    Palette(vec![
        Color::NRGBA(color::NRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }),
        Color::NRGBA(color::NRGBA {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }),
        Color::NRGBA(color::NRGBA {
            r: 50,
            g: 100,
            b: 150,
            a: 255,
        }),
        Color::NRGBA(color::NRGBA {
            r: 150,
            g: 100,
            b: 50,
            a: 200,
        }),
    ])
}

// Go: pixels_test.go:TestGetPixel (the getter is created after each Set)
#[test]
fn test_get_pixel() {
    // RGBA, NRGBA, RGBA64, NRGBA64
    let r = rect(-1, -2, 3, 4);
    let mut images1: Vec<Box<dyn draw::Image>> = vec![
        Box::new(RGBA::new(r)),
        Box::new(RGBA64::new(r)),
        Box::new(NRGBA::new(r)),
        Box::new(NRGBA64::new(r)),
        Box::new(Paletted::new(r, test_palette())),
    ];

    struct Case {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors1: Vec<Case> = vec![
        Case {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 0.0),
        },
        Case {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case {
            c: color::NRGBA {
                r: 50,
                g: 100,
                b: 150,
                a: 255,
            },
            px: Pixel::new(0.196, 0.392, 0.588, 1.0),
        },
        Case {
            c: color::NRGBA {
                r: 150,
                g: 100,
                b: 50,
                a: 200,
            },
            px: Pixel::new(0.588, 0.392, 0.196, 0.784),
        },
    ];

    for img in images1.iter_mut() {
        for k in &colors1 {
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    img.set(x, y, Color::NRGBA(k.c));
                    let pg = PixelGetter::new(&**img);
                    let px = pg.get_pixel(x, y);
                    assert!(
                        compare_pixels(k.px, px, 0.005),
                        "getPixel {:?} {x}x{y} {:?} {:?}",
                        k.c,
                        k.px,
                        px
                    );
                }
            }
        }
    }

    // Uniform (Generic)

    for k in &colors1 {
        let img = Uniform::new(Color::NRGBA(k.c));
        let pg = PixelGetter::new(&img);
        for x in [-1i64, 0, 2] {
            for y in [-2i64, 0, 3] {
                let px = pg.get_pixel(x, y);
                assert!(
                    compare_pixels(k.px, px, 0.005),
                    "getPixel Uniform {:?} {x}x{y} {:?} {:?}",
                    k.c,
                    k.px,
                    px
                );
            }
        }
    }

    // YCbCr

    struct Case1 {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors2: Vec<Case1> = vec![
        Case1 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 50,
                g: 100,
                b: 150,
                a: 255,
            },
            px: Pixel::new(0.196, 0.392, 0.588, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 150,
                g: 100,
                b: 50,
                a: 255,
            },
            px: Pixel::new(0.588, 0.392, 0.196, 1.0),
        },
    ];

    for k in &colors2 {
        for sr in [
            YCbCrSubsampleRatio::Ratio444,
            YCbCrSubsampleRatio::Ratio422,
            YCbCrSubsampleRatio::Ratio420,
            YCbCrSubsampleRatio::Ratio440,
            YCbCrSubsampleRatio::Ratio410,
            YCbCrSubsampleRatio::Ratio411,
        ] {
            let mut img = YCbCr::new(r, sr);
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    let iy = img.y_offset(x, y) as usize;
                    let ic = img.c_offset(x, y) as usize;
                    (img.y[iy], img.cb[ic], img.cr[ic]) = color::rgb_to_ycbcr(k.c.r, k.c.g, k.c.b);
                    let pg = PixelGetter::new(&img);
                    let px = pg.get_pixel(x, y);
                    assert!(
                        compare_pixels(k.px, px, 0.005),
                        "getPixel YCbCr {sr:?} {:?} {x}x{y} {:?} {:?}",
                        k.c,
                        k.px,
                        px
                    );
                }
            }
        }
    }

    // Gray, Gray16

    let mut images2: Vec<Box<dyn draw::Image>> =
        vec![Box::new(Gray::new(r)), Box::new(Gray16::new(r))];

    struct Case2 {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors3: Vec<Case2> = vec![
        Case2 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case2 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case2 {
            c: color::NRGBA {
                r: 50,
                g: 100,
                b: 150,
                a: 255,
            },
            px: Pixel::new(0.356, 0.356, 0.356, 1.0),
        },
        Case2 {
            c: color::NRGBA {
                r: 150,
                g: 100,
                b: 50,
                a: 200,
            },
            px: Pixel::new(0.337, 0.337, 0.337, 1.0),
        },
    ];

    for img in images2.iter_mut() {
        for k in &colors3 {
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    img.set(x, y, Color::NRGBA(k.c));
                    let pg = PixelGetter::new(&**img);
                    let px = pg.get_pixel(x, y);
                    assert!(
                        compare_pixels(k.px, px, 0.005),
                        "getPixel {:?} {x}x{y} {:?} {:?}",
                        k.c,
                        k.px,
                        px
                    );
                }
            }
        }
    }
}

/// Go: pixels_test.go:comparePixelSlices (it starts at index 1, as in Go).
fn compare_pixel_slices(s1: &[Pixel], s2: &[Pixel], dif: f64) -> bool {
    if s1.len() != s2.len() {
        return false;
    }
    for i in 1..s1.len() {
        if !compare_pixels(s1[i], s2[i], dif) {
            return false;
        }
    }
    true
}

const ROW_COLORS: [color::NRGBA; 4] = [
    color::NRGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    color::NRGBA {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    },
    color::NRGBA {
        r: 50,
        g: 100,
        b: 150,
        a: 255,
    },
    color::NRGBA {
        r: 150,
        g: 100,
        b: 50,
        a: 200,
    },
];

const ROW_PIXELS: [Pixel; 4] = [
    Pixel::new(0.0, 0.0, 0.0, 0.0),
    Pixel::new(1.0, 1.0, 1.0, 1.0),
    Pixel::new(0.196, 0.392, 0.588, 1.0),
    Pixel::new(0.588, 0.392, 0.196, 0.784),
];

// Go: pixels_test.go:TestGetPixelRow
#[test]
fn test_get_pixel_row() {
    let mut img = NRGBA::new(rect(-1, -2, 3, 2));
    let mut row: Vec<Pixel> = Vec::new();
    for y in img.bounds().min.y..img.bounds().max.y {
        for x in img.bounds().min.x..img.bounds().max.x {
            let c = ROW_COLORS[(x - img.bounds().min.x) as usize];
            img.set(x, y, Color::NRGBA(c));
        }
        let pg = PixelGetter::new(&img);
        pg.get_pixel_row(y, &mut row);
        assert!(
            compare_pixel_slices(&row, &ROW_PIXELS, 0.005),
            "getPixelRow y={y} {row:?} {ROW_PIXELS:?}"
        );
    }
}

// Go: pixels_test.go:TestGetPixelColumn
#[test]
fn test_get_pixel_column() {
    let mut img = NRGBA::new(rect(-1, -2, 3, 2));
    let mut column: Vec<Pixel> = Vec::new();
    for x in img.bounds().min.x..img.bounds().max.x {
        for y in img.bounds().min.y..img.bounds().max.y {
            let c = ROW_COLORS[(y - img.bounds().min.y) as usize];
            img.set(x, y, Color::NRGBA(c));
        }
        let pg = PixelGetter::new(&img);
        pg.get_pixel_column(x, &mut column);
        assert!(
            compare_pixel_slices(&column, &ROW_PIXELS, 0.005),
            "getPixelColumn x={x} {column:?} {ROW_PIXELS:?}"
        );
    }
}

// Go: pixels_test.go:TestF32u8 (f32u8 of a plain value: nothing to fuse)
#[test]
fn test_f32u8() {
    struct Case {
        x: f32,
        y: u8,
    }
    let test_data: Vec<Case> = vec![
        Case { x: -1.0, y: 0 },
        Case { x: 0.0, y: 0 },
        Case { x: 100.0, y: 100 },
        Case { x: 255.0, y: 255 },
        Case { x: 256.0, y: 255 },
    ];

    for p in &test_data {
        let v = f32u8(p.x);
        assert_eq!(v, p.y, "f32u8({}) != {}: {}", p.x, p.y, v);
    }
}

// Go: pixels_test.go:TestF32u16
#[test]
fn test_f32u16() {
    struct Case {
        x: f32,
        y: u16,
    }
    let test_data: Vec<Case> = vec![
        Case { x: -1.0, y: 0 },
        Case { x: 0.0, y: 0 },
        Case { x: 1.0, y: 1 },
        Case {
            x: 10000.0,
            y: 10000,
        },
        Case {
            x: 65535.0,
            y: 65535,
        },
        Case {
            x: 65536.0,
            y: 65535,
        },
    ];

    for p in &test_data {
        let v = f32u16(p.x);
        assert_eq!(v, p.y, "f32u16({}) != {}: {}", p.x, p.y, v);
    }
}

// Go: pixels_test.go:TestClampi32
#[test]
fn test_clampi32() {
    struct Case {
        x: i32,
        y: i32,
    }
    let test_data: Vec<Case> = vec![
        Case { x: -1, y: 0 },
        Case { x: 0, y: 0 },
        Case { x: 1, y: 1 },
        Case { x: 99, y: 99 },
        Case { x: 100, y: 100 },
        Case { x: 101, y: 100 },
    ];

    for p in &test_data {
        let v = clampi32(p.x, 0, 100);
        assert_eq!(v, p.y, "clampi32({}) != {}: {}", p.x, p.y, v);
    }
}

// Go: pixels_test.go:TestNewPixelSetter
#[test]
fn test_new_pixel_setter() {
    let r = rect(0, 0, 1, 1);
    let check = |img: &mut dyn draw::Image, it: ImageType, name: &str| {
        let b = img.bounds();
        let ps = PixelSetter::new(img);
        let ok_ptr = matches!(
            (it, &ps.img),
            (ImageType::NRGBA, SetterImg::NRGBA(_))
                | (ImageType::NRGBA64, SetterImg::NRGBA64(_))
                | (ImageType::RGBA, SetterImg::RGBA(_))
                | (ImageType::RGBA64, SetterImg::RGBA64(_))
                | (ImageType::Gray, SetterImg::Gray(_))
                | (ImageType::Gray16, SetterImg::Gray16(_))
                | (ImageType::Paletted, SetterImg::Paletted(_))
                | (ImageType::Generic, SetterImg::Generic(_))
        );
        assert!(
            ps.it == it && ok_ptr && b.eq(ps.bounds),
            "newPixelSetter {name}"
        );
    };
    check(&mut NRGBA::new(r), ImageType::NRGBA, "NRGBA");
    check(&mut NRGBA64::new(r), ImageType::NRGBA64, "NRGBA64");
    check(&mut RGBA::new(r), ImageType::RGBA, "RGBA");
    check(&mut RGBA64::new(r), ImageType::RGBA64, "RGBA64");
    check(&mut Gray::new(r), ImageType::Gray, "Gray");
    check(&mut Gray16::new(r), ImageType::Gray16, "Gray16");
    check(
        &mut Paletted::new(r, Palette(vec![])),
        ImageType::Paletted,
        "Paletted",
    );
    check(&mut Alpha::new(r), ImageType::Generic, "Generic(Alpha)");
}

/// setPixel on a fresh setter of img, then read back as NRGBA (Go keeps one
/// setter for the whole loop; it holds no state besides the palette).
fn set_and_get(img: &mut dyn draw::Image, x: i64, y: i64, px: Pixel) -> color::NRGBA {
    PixelSetter::new(img).set_pixel(x, y, px);
    nrgba_at(&*img, x, y)
}

// Go: pixels_test.go:TestSetPixel
#[test]
fn test_set_pixel() {
    // RGBA, NRGBA, RGBA64, NRGBA64
    let r = rect(-1, -2, 3, 4);
    let mut images1: Vec<Box<dyn draw::Image>> = vec![
        Box::new(RGBA::new(r)),
        Box::new(RGBA64::new(r)),
        Box::new(NRGBA::new(r)),
        Box::new(NRGBA64::new(r)),
    ];

    struct Case {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors1: Vec<Case> = vec![
        Case {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 0.0),
        },
        Case {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case {
            c: color::NRGBA {
                r: 50,
                g: 100,
                b: 150,
                a: 255,
            },
            px: Pixel::new(0.196, 0.392, 0.588, 1.0),
        },
        Case {
            c: color::NRGBA {
                r: 150,
                g: 100,
                b: 50,
                a: 200,
            },
            px: Pixel::new(0.588, 0.392, 0.196, 0.784),
        },
    ];

    for img in images1.iter_mut() {
        for k in &colors1 {
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    let c = set_and_get(&mut **img, x, y, k.px);
                    assert!(
                        compare_colors_nrgba(c, k.c, 1),
                        "setPixel {:?} {x}x{y} {:?} {:?}",
                        k.px,
                        k.c,
                        c
                    );
                }
            }
        }
    }

    // Gray, Gray16

    let mut images2: Vec<Box<dyn draw::Image>> =
        vec![Box::new(Gray::new(r)), Box::new(Gray16::new(r))];

    struct Case1 {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors2: Vec<Case1> = vec![
        Case1 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 110,
                g: 110,
                b: 110,
                a: 255,
            },
            px: Pixel::new(0.2, 0.5, 0.7, 1.0),
        },
        Case1 {
            c: color::NRGBA {
                r: 55,
                g: 55,
                b: 55,
                a: 255,
            },
            px: Pixel::new(0.2, 0.5, 0.7, 0.5),
        },
    ];

    for img in images2.iter_mut() {
        for k in &colors2 {
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    let c = set_and_get(&mut **img, x, y, k.px);
                    assert!(
                        compare_colors_nrgba(c, k.c, 1),
                        "setPixel {:?} {x}x{y} {:?} {:?}",
                        k.px,
                        k.c,
                        c
                    );
                }
            }
        }
    }

    // Generic(Alpha)

    struct Case2 {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors3: Vec<Case2> = vec![
        Case2 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case2 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 127,
            },
            px: Pixel::new(0.2, 0.5, 0.7, 0.5),
        },
        Case2 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 63,
            },
            px: Pixel::new(0.1, 0.2, 0.3, 0.25),
        },
    ];

    let mut img = Alpha::new(r);
    for k in &colors3 {
        for x in [-1i64, 0, 2] {
            for y in [-2i64, 0, 3] {
                let c = set_and_get(&mut img, x, y, k.px);
                assert!(
                    compare_colors_nrgba(c, k.c, 1),
                    "setPixel Alpha {:?} {x}x{y} {:?} {:?}",
                    k.px,
                    k.c,
                    c
                );
            }
        }
    }

    // Paletted

    let nrgba = |r, g, b, a| Color::NRGBA(color::NRGBA { r, g, b, a });
    let mut images4: Vec<Box<dyn draw::Image>> = vec![Box::new(Paletted::new(
        r,
        Palette(vec![
            nrgba(0, 0, 0, 0),
            nrgba(0, 0, 0, 255),
            nrgba(255, 255, 255, 255),
            nrgba(50, 100, 150, 255),
            nrgba(150, 100, 50, 200),
            nrgba(1, 255, 255, 255),
            nrgba(2, 255, 255, 255),
            nrgba(3, 255, 255, 255),
        ]),
    ))];

    // Go's `k.001 / 255` is an untyped constant rounded once to float32; the
    // f32::from_bits values are go1.27.1's (an f32 division would round
    // 3.001 first and give 0x3c40d133).
    struct Case3 {
        c: color::NRGBA,
        px: Pixel,
    }
    let colors4: Vec<Case3> = vec![
        Case3 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 0.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 1.0, 1.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 50,
                g: 100,
                b: 150,
                a: 255,
            },
            px: Pixel::new(0.196, 0.392, 0.588, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 150,
                g: 100,
                b: 50,
                a: 200,
            },
            px: Pixel::new(0.588, 0.392, 0.196, 0.784),
        },
        Case3 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            px: Pixel::new(0.1, 0.01, 0.001, 0.1),
        },
        Case3 {
            c: color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            px: Pixel::new(0.0, 0.0, 0.0, 0.9),
        },
        Case3 {
            c: color::NRGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(1.0, 0.9, 1.0, 0.9),
        },
        Case3 {
            c: color::NRGBA {
                r: 1,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(f32::from_bits(0x36839605), 1.0, 1.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 1,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(f32::from_bits(0x3b80a166), 1.0, 1.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 2,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(f32::from_bits(0x3c0090f3), 1.0, 1.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 3,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(f32::from_bits(0x3c40d134), 1.0, 1.0, 1.0),
        },
        Case3 {
            c: color::NRGBA {
                r: 3,
                g: 255,
                b: 255,
                a: 255,
            },
            px: Pixel::new(f32::from_bits(0x3c8088ba), 1.0, 1.0, 1.0),
        },
    ];

    for img in images4.iter_mut() {
        for k in &colors4 {
            for x in [-1i64, 0, 2] {
                for y in [-2i64, 0, 3] {
                    let c = set_and_get(&mut **img, x, y, k.px);
                    assert!(
                        compare_colors_nrgba(c, k.c, 0),
                        "setPixel Paletted {:?} {x}x{y} {:?} {:?}",
                        k.px,
                        k.c,
                        c
                    );
                }
            }
        }
    }
}

// Go: pixels_test.go:TestSetPixelRow
#[test]
fn test_set_pixel_row() {
    let mut img = NRGBA::new(rect(-1, -2, 3, 2));
    for y in img.bounds().min.y..img.bounds().max.y {
        PixelSetter::new(&mut img).set_pixel_row(y, &ROW_PIXELS);
        for x in img.bounds().min.x..img.bounds().max.x {
            let c = img.nrgba_at(x, y);
            let wanted_color = ROW_COLORS[(x - img.bounds().min.x) as usize];
            assert!(
                compare_colors_nrgba(wanted_color, c, 1),
                "setPixelRow y={y} x={x} {wanted_color:?} {c:?}"
            );
        }
    }
}

// Go: pixels_test.go:TestSetPixelColumn
#[test]
fn test_set_pixel_column() {
    let mut img = NRGBA::new(rect(-1, -2, 3, 2));
    for x in img.bounds().min.x..img.bounds().max.x {
        PixelSetter::new(&mut img).set_pixel_column(x, &ROW_PIXELS);
        for y in img.bounds().min.y..img.bounds().max.y {
            let c = img.nrgba_at(x, y);
            let wanted_color = ROW_COLORS[(y - img.bounds().min.y) as usize];
            assert!(
                compare_colors_nrgba(wanted_color, c, 1),
                "setPixelColumn x={x} y={y} {wanted_color:?} {c:?}"
            );
        }
    }
}
