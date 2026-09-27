//! Port of gift v1.2.1 `colors_test.go`.

use std::sync::Arc;

use go_image::color::{self, Color};
use go_image::{Gray, Gray16, NRGBA, Rectangle, draw, rect};

use super::{check_bounds_and_pix, gray, nrgba};
use crate::colors::{get_from_lut, normalize_hue, prepare_lut};
use crate::utils::absf32;
use crate::{
    Filter, brightness, color_balance, color_func, colorize, colorspace_linear_to_srgb,
    colorspace_srgb_to_linear, contrast, gamma, grayscale, hue, invert, new, saturation, sepia,
    sigmoid_filter, threshold,
};

// Go: colors_test.go:TestLut
#[test]
fn test_lut() {
    let f = |v: f32| v;
    for size in [10i64, 100, 1000] {
        let lut = prepare_lut(size, &f);
        let l = lut.len();
        assert_eq!(l as i64, size, "LUT bad size");
        assert_eq!(lut[0], 0.0, "LUT bad start value");
        assert_eq!(lut[l - 1], 1.0, "LUT bad end value");
    }
    let lut = prepare_lut(10000, &f);
    for u in [0f32, 0.0001, 0.5555, 0.9999, 1.0] {
        let v = get_from_lut(&lut, u);
        assert!(
            ((v - u) as f64).abs() <= 0.0001,
            "LUT bad value: expected {u} got {v}"
        );
    }
}

// Go: colors_test.go:TestInvert
#[test]
fn test_invert() {
    let mut src = Gray::new(rect(0, 0, 256, 1));
    for i in 0..=255 {
        src.pix[i] = i as u8;
    }
    let g = new(vec![invert()]);
    let mut dst = Gray::new(g.bounds(src.bounds()));
    g.draw(&mut dst, &src);

    for i in 0..=255 {
        assert_eq!(dst.pix[i], 255 - src.pix[i], "InvertColors: index {i}");
    }
}

/// The body of TestColorspaceSRGBToLinear and TestColorspaceLinearToSRGB.
fn check_colorspace(name: &str, f: fn() -> Arc<dyn Filter>, vals: &[f32; 11]) {
    let mut imgs: Vec<Box<dyn draw::Image>> = vec![
        Box::new(Gray::new(rect(0, 0, 11, 11))),
        Box::new(Gray::new(rect(0, 0, 111, 111))),
        Box::new(Gray16::new(rect(0, 0, 11, 11))),
        Box::new(Gray16::new(rect(0, 0, 1111, 1111))),
    ];
    for img in imgs.iter_mut() {
        for i in 0..=10i64 {
            // color.Gray{uint8(255 * i / 10.0)}: integer division.
            img.set(
                i,
                0,
                Color::Gray(color::Gray {
                    y: (255 * i / 10) as u8,
                }),
            );
        }
        let mut img2 = Gray::new(img.bounds());
        new(vec![f()]).draw(&mut img2, &**img);
        assert!(
            img2.bounds().size().eq(img.bounds().size()),
            "{name} bad result size"
        );
        for i in 0..=10 {
            // uint8(vals[i]*255 + 0.5), fused on arm64 (FMADDS).
            let expected = vals[i].mul_add(255.0, 0.5) as u8;
            let c = img2.gray_at(i as i64, 0);
            assert!(
                (c.y as f64 - expected as f64).abs() <= 1.0,
                "{name} bad color value at index {i} expected {expected} got {}",
                c.y
            );
        }
    }
}

// Go: colors_test.go:TestColorspaceSRGBToLinear
#[test]
fn test_colorspace_srgb_to_linear() {
    let vals = [
        0.00000, 0.01002, 0.03310, 0.07324, 0.13287, 0.21404, 0.31855, 0.44799, 0.60383, 0.78741,
        1.00000,
    ];
    check_colorspace("ColorspaceSRGBToLinear", colorspace_srgb_to_linear, &vals);
}

// Go: colors_test.go:TestColorspaceLinearToSRGB
#[test]
fn test_colorspace_linear_to_srgb() {
    let vals = [
        0.00000, 0.34919, 0.48453, 0.58383, 0.66519, 0.73536, 0.79774, 0.85431, 0.90633, 0.95469,
        1.00000,
    ];
    check_colorspace(
        "ColorspaceLinearRGBToSRGB",
        colorspace_linear_to_srgb,
        &vals,
    );
}

// Go: colors_test.go:TestAdjustGamma
#[test]
fn test_adjust_gamma() {
    let mut src = Gray::new(rect(0, 0, 256, 1));
    let mut dst = Gray::new(rect(0, 0, 256, 1));
    for i in 0..=255 {
        src.pix[i] = i as u8;
    }
    let ag = gamma(2.0);
    ag.draw(&mut dst, &src, None);

    for i in 100..=150 {
        assert!(dst.pix[i] > src.pix[i], "Gamma unexpected color");
    }

    let ag = gamma(0.5);
    ag.draw(&mut dst, &src, None);

    for i in 100..=150 {
        assert!(dst.pix[i] < src.pix[i], "Gamma unexpected color");
    }

    let ag = gamma(1.0);
    ag.draw(&mut dst, &src, None);

    for i in 100..=150 {
        assert_eq!(dst.pix[i], src.pix[i], "Gamma unexpected color");
    }
}

/// The loop of the Gray-table tests: `f(d.p)` from a Gray source into a Gray
/// destination.
fn check_gray(
    desc: &str,
    f: Arc<dyn Filter>,
    srcb: Rectangle,
    dstb: Rectangle,
    src_pix: &[u8],
    dst_pix: &[u8],
) {
    let src = gray(srcb, src_pix);
    let mut dst = Gray::new(f.bounds(src.bounds()));
    f.draw(&mut dst, &src, None);
    assert!(
        check_bounds_and_pix(dst.bounds(), dstb, &dst.pix, dst_pix),
        "test [{desc}] failed: {:?}, {:?}",
        dst.bounds(),
        dst.pix
    );
}

/// The loop of the NRGBA-table tests.
fn check_nrgba(
    desc: &str,
    f: Arc<dyn Filter>,
    srcb: Rectangle,
    dstb: Rectangle,
    src_pix: &[u8],
    dst_pix: &[u8],
) {
    let src = nrgba(srcb, src_pix);
    let mut dst = NRGBA::new(f.bounds(src.bounds()));
    f.draw(&mut dst, &src, None);
    assert!(
        check_bounds_and_pix(dst.bounds(), dstb, &dst.pix, dst_pix),
        "test [{desc}] failed: {:?}, {:?}",
        dst.bounds(),
        dst.pix
    );
}

// Go: colors_test.go:TestContrast
#[test]
fn test_contrast() {
    struct Case {
        desc: &'static str,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "contrast (0)",
            p: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "contrast (30)",
            p: 30.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x25, 0x00, 0x25, 0x00, //
                0x53, 0xC5, 0xAE, 0xC5, 0x53, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "contrast (-30)",
            p: -30.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x26, 0x53, 0x26, 0x53, 0x26, //
                0x69, 0xA1, 0x96, 0xA1, 0x69, //
                0x26, 0x80, 0x26, 0x80, 0x26, //
            ],
        },
        Case {
            desc: "contrast (100)",
            p: 100.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xFF, 0xFF, 0xFF, 0x00, //
                0x00, 0xFF, 0x00, 0xFF, 0x00, //
            ],
        },
        Case {
            desc: "contrast (200)",
            p: 200.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xFF, 0xFF, 0xFF, 0x00, //
                0x00, 0xFF, 0x00, 0xFF, 0x00, //
            ],
        },
        Case {
            desc: "contrast (-100)",
            p: -100.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x80, 0x80, 0x80, 0x80, 0x80, //
                0x80, 0x80, 0x80, 0x80, 0x80, //
                0x80, 0x80, 0x80, 0x80, 0x80, //
            ],
        },
        Case {
            desc: "contrast (-200)",
            p: -200.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x80, 0x80, 0x80, 0x80, 0x80, //
                0x80, 0x80, 0x80, 0x80, 0x80, //
                0x80, 0x80, 0x80, 0x80, 0x80, //
            ],
        },
    ];

    for d in &test_data {
        check_gray(d.desc, contrast(d.p), d.srcb, d.dstb, d.src_pix, d.dst_pix);
    }
}

// Go: colors_test.go:TestBrightness
#[test]
fn test_brightness() {
    struct Case {
        desc: &'static str,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "brightness (0)",
            p: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "brightness (30)",
            p: 30.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x4D, 0x8D, 0x4D, 0x8D, 0x4D, //
                0xAD, 0xFD, 0xED, 0xFD, 0xAD, //
                0x4D, 0xCD, 0x4D, 0xCD, 0x4D, //
            ],
        },
        Case {
            desc: "brightness (-30)",
            p: -30.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA1, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x14, 0x64, 0x55, 0x64, 0x14, //
                0x00, 0x34, 0x00, 0x34, 0x00, //
            ],
        },
        Case {
            desc: "brightness (100)",
            p: 100.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
        },
        Case {
            desc: "brightness (200)",
            p: 200.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
        },
        Case {
            desc: "brightness (-100)",
            p: -100.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "brightness (-200)",
            p: -200.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        check_gray(
            d.desc,
            brightness(d.p),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestSigmoid
#[test]
fn test_sigmoid() {
    struct Case {
        desc: &'static str,
        midpoint: f32,
        factor: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "sigmoid (0.5, 0)",
            midpoint: 0.5,
            factor: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "sigmoid (0.5, 3)",
            midpoint: 0.5,
            factor: 3.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x38, 0x00, 0x38, 0x00, //
                0x5B, 0xB7, 0xA5, 0xB7, 0x5B, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "sigmoid (0.5, -3)",
            midpoint: 0.5,
            factor: -3.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x48, 0x00, 0x48, 0x00, //
                0x65, 0xA9, 0x9B, 0xA9, 0x65, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        check_gray(
            d.desc,
            sigmoid_filter(d.midpoint, d.factor),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestGrayscale
#[test]
fn test_grayscale() {
    struct Case {
        desc: &'static str,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "grayscale 0x0",
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "grayscale 2x3",
            srcb: rect(-1, -1, 1, 2),
            dstb: rect(0, 0, 2, 3),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0xFF, 0x00, 0x88, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
            dst_pix: &[
                0x0D, 0x0D, 0x0D, 0x30, 0x5C, 0x5C, 0x5C, 0xFF, //
                0xE3, 0xE3, 0xE3, 0xC0, 0x56, 0x56, 0x56, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(d.desc, grayscale(), d.srcb, d.dstb, d.src_pix, d.dst_pix);
    }
}

// Go: colors_test.go:TestSepia
#[test]
fn test_sepia() {
    struct Case {
        desc: &'static str,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "sepia 100 0x0",
            p: 100.0,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "sepia 0 2x3",
            p: 0.0,
            srcb: rect(-1, -1, 1, 2),
            dstb: rect(0, 0, 2, 3),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0xFF, 0x00, 0x88, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0xFF, 0x00, 0x88, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
        },
        Case {
            desc: "sepia 100 2x3",
            p: 100.0,
            srcb: rect(-1, -1, 1, 2),
            dstb: rect(0, 0, 2, 3),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0xFF, 0x00, 0x88, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
            dst_pix: &[
                0x12, 0x10, 0x0D, 0x30, 0x7E, 0x70, 0x57, 0xFF, //
                0xFF, 0xFF, 0xD4, 0xC0, 0x78, 0x6B, 0x54, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xEF, 0xFF, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(d.desc, sepia(d.p), d.srcb, d.dstb, d.src_pix, d.dst_pix);
    }
}

// Go: colors_test.go:TestNormalizeHue
#[test]
fn test_normalize_hue() {
    struct Case {
        h0: f32,
        h1: f32,
    }
    let test_data: Vec<Case> = vec![
        Case { h0: 0.0, h1: 0.0 },
        Case { h0: 0.1, h1: 0.1 },
        Case { h0: 0.5, h1: 0.5 },
        Case { h0: 0.9, h1: 0.9 },
        Case { h0: 1.1, h1: 0.1 },
        Case { h0: 3.0, h1: 0.0 },
        Case { h0: 5.7, h1: 0.7 },
        Case { h0: -0.1, h1: 0.9 },
        Case { h0: -0.5, h1: 0.5 },
        Case { h0: -0.9, h1: 0.1 },
        Case { h0: -3.0, h1: 0.0 },
        Case { h0: -5.7, h1: 0.3 },
    ];

    for d in &test_data {
        let h = normalize_hue(d.h0);
        assert!(
            absf32(h - d.h1) <= 0.00001,
            "normalizeHue({}) failed: {} expected: {}",
            d.h0,
            h,
            d.h1
        );
    }
}

// Go: colors_test.go:TestHue
#[test]
fn test_hue() {
    struct Case {
        desc: &'static str,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "hue 0 0x0",
            p: 0.0,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "hue 0",
            p: 0.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
        },
        Case {
            desc: "hue -720",
            p: -720.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
        },
        Case {
            desc: "hue -90",
            p: -90.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x20, 0x00, 0x30, 0x99, 0x33, 0x99, 0xFF, //
                0xF0, 0xD0, 0xF0, 0xC0, 0x11, 0xBB, 0x11, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0x50, 0x51, 0xEE, 0x77, 0xFE, 0xFE, 0xFD, 0xFD, //
            ],
        },
        Case {
            desc: "hue 630",
            p: 630.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x20, 0x00, 0x30, 0x99, 0x33, 0x99, 0xFF, //
                0xF0, 0xD0, 0xF0, 0xC0, 0x11, 0xBB, 0x11, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0x50, 0x51, 0xEE, 0x77, 0xFE, 0xFE, 0xFD, 0xFD, //
            ],
        },
        Case {
            desc: "hue 90",
            p: 90.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x20, 0x00, 0x20, 0x30, 0x33, 0x99, 0x33, 0xFF, //
                0xD0, 0xF0, 0xD0, 0xC0, 0xBB, 0x11, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0xED, 0x50, 0x77, 0xFD, 0xFE, 0xFE, 0xFD, //
            ],
        },
        Case {
            desc: "hue 3690",
            p: 3690.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0x50, 0xA0, 0x77, 0xFD, 0xFE, 0xFD, 0xFD, //
            ],
            dst_pix: &[
                0x20, 0x00, 0x20, 0x30, 0x33, 0x99, 0x33, 0xFF, //
                0xD0, 0xF0, 0xD0, 0xC0, 0xBB, 0x11, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xEE, 0xED, 0x50, 0x77, 0xFD, 0xFE, 0xFE, 0xFD, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(d.desc, hue(d.p), d.srcb, d.dstb, d.src_pix, d.dst_pix);
    }
}

// Go: colors_test.go:TestSaturation
#[test]
fn test_saturation() {
    struct Case {
        desc: &'static str,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "saturation 0 0x0",
            p: 0.0,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "saturation 0",
            p: 0.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
        },
        Case {
            desc: "saturation -50",
            p: -50.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x08, 0x10, 0x18, 0x30, 0x80, 0x66, 0x4D, 0xFF, //
                0xE8, 0xE0, 0xD8, 0xC0, 0x3B, 0x66, 0x91, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xBC, 0x74, 0x9C, 0x77, 0xF2, 0xFA, 0xF2, 0xFD, //
            ],
        },
        Case {
            desc: "saturation 100",
            p: 100.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0xCC, 0x66, 0x00, 0xFF, //
                0xFF, 0xE0, 0xC1, 0xC0, 0x00, 0x66, 0xCC, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0x31, 0xA3, 0x77, 0xED, 0xFF, 0xED, 0xFD, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(
            d.desc,
            saturation(d.p),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestColorize
#[test]
fn test_colorize() {
    struct Case {
        desc: &'static str,
        h: f32,
        s: f32,
        p: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "colorize 0, 0, 0, 0x0",
            h: 0.0,
            s: 0.0,
            p: 0.0,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "colorize 0, 100, 0",
            h: 0.0,
            s: 100.0,
            p: 0.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
        },
        Case {
            desc: "colorize 0, 100, 100",
            h: 0.0,
            s: 100.0,
            p: 100.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x20, 0x00, 0x00, 0x30, 0xCC, 0x00, 0x00, 0xFF, //
                0xFF, 0xC1, 0xC1, 0xC0, 0xCC, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0x31, 0x31, 0x77, 0xFF, 0xED, 0xED, 0xFD, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(
            d.desc,
            colorize(d.h, d.s, d.p),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestColorBalance
#[test]
fn test_color_balance() {
    struct Case {
        desc: &'static str,
        r: f32,
        g: f32,
        b: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "color balance 0, 0, 0, 0x0",
            r: 0.0,
            g: 0.0,
            b: 0.0,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "color balance 0, 0, 0",
            r: 0.0,
            g: 0.0,
            b: 0.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
        },
        Case {
            desc: "color balance 10, -20, 200",
            r: 10.0,
            g: -20.0,
            b: 200.0,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
                0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
            ],
            dst_pix: &[
                0x00, 0x0D, 0x60, 0x30, 0xA8, 0x52, 0x99, 0xFF, //
                0xFF, 0xB3, 0xFF, 0xC0, 0x13, 0x52, 0xFF, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xCC, 0xFF, 0xFF, //
                0xF6, 0x40, 0xFF, 0x77, 0xFF, 0xCB, 0xFF, 0xFD, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(
            d.desc,
            color_balance(d.r, d.g, d.b),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestThreshold
#[test]
fn test_threshold() {
    struct Case {
        desc: &'static str,
        percentage: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "threshold -1",
            percentage: -1.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0xff, 0xff, 0xff, 0xff, 0xff],
        },
        Case {
            desc: "threshold 0",
            percentage: 0.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0xff, 0xff, 0xff, 0xff, 0xff],
        },
        Case {
            desc: "threshold 30",
            percentage: 30.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0x00, 0xff, 0xff, 0xff, 0xff],
        },
        Case {
            desc: "threshold 50",
            percentage: 50.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0x00, 0x00, 0xff, 0xff, 0xff],
        },
        Case {
            desc: "threshold 90",
            percentage: 90.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0x00, 0x00, 0x00, 0x00, 0xff],
        },
        Case {
            desc: "threshold 100",
            percentage: 100.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        },
        Case {
            desc: "threshold 101",
            percentage: 101.0,
            srcb: rect(-1, -1, 5, 0),
            dstb: rect(0, 0, 6, 1),
            src_pix: &[0x00, 0x33, 0x66, 0x99, 0xcc, 0xff],
            dst_pix: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        },
    ];

    for d in &test_data {
        check_gray(
            d.desc,
            threshold(d.percentage),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}

// Go: colors_test.go:TestColorFunc (the callbacks have no fusable
// expressions, so rustc and the arm64 Go compiler agree on them).
#[test]
fn test_color_func() {
    type Fn4 = fn(f32, f32, f32, f32) -> (f32, f32, f32, f32);
    struct Case {
        desc: &'static str,
        f: Fn4,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let src_pix: &'static [u8] = &[
        0x00, 0x10, 0x20, 0x30, 0x99, 0x66, 0x33, 0xFF, //
        0xF0, 0xE0, 0xD0, 0xC0, 0x11, 0x66, 0xBB, 0x00, //
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
        0xE0, 0x50, 0xA0, 0x77, 0xEE, 0xFE, 0xEE, 0xFD, //
    ];
    let test_data: Vec<Case> = vec![
        Case {
            desc: "color func 0x0",
            f: |r0, g0, b0, a0| (r0, g0, b0, a0),
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "color func swap channels",
            f: |r0, g0, b0, a0| (a0, b0, g0, r0),
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix,
            dst_pix: &[
                0x30, 0x20, 0x10, 0x00, 0xFF, 0x33, 0x66, 0x99, //
                0xC0, 0xD0, 0xE0, 0xF0, 0x00, 0xBB, 0x66, 0x11, //
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, //
                0x77, 0xA0, 0x50, 0xE0, 0xFD, 0xEE, 0xFE, 0xEE, //
            ],
        },
        Case {
            desc: "color func invert all",
            f: |r0, g0, b0, a0| (1.0 - r0, 1.0 - g0, 1.0 - b0, 1.0 - a0),
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix,
            dst_pix: &[
                0xFF, 0xEF, 0xDF, 0xCF, 0x66, 0x99, 0xCC, 0x00, //
                0x0F, 0x1F, 0x2F, 0x3F, 0xEE, 0x99, 0x44, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, //
                0x1F, 0xAF, 0x5F, 0x88, 0x11, 0x01, 0x11, 0x02, //
            ],
        },
    ];

    for d in &test_data {
        check_nrgba(
            d.desc,
            color_func(d.f),
            d.srcb,
            d.dstb,
            d.src_pix,
            d.dst_pix,
        );
    }
}
