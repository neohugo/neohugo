//! Port of gift v1.2.1 `gift_test.go` (TestGolden is `tests/go_golden.rs`).

use std::sync::Arc;

use go_image::color::{self, Color};
use go_image::{
    Gray, Gray16, Image, NRGBA, NRGBA64, Point, RGBA, RGBA64, Rectangle, draw, pt, rect,
};

use super::check_bounds_and_pix;
use crate::gift::get_sub_image;
use crate::resize::NEAREST_NEIGHBOR_RESAMPLING;
use crate::transform::{CENTER_ANCHOR, NEAREST_NEIGHBOR_INTERPOLATION};
use crate::{
    COPY_OPERATOR, DEFAULT_OPTIONS, Filter, GIFT, OVER_OPERATOR, Operator, Options, brightness,
    color_balance, color_func, colorize, colorspace_linear_to_srgb, colorspace_srgb_to_linear,
    contrast, convolution, crop, crop_to_size, flip_horizontal, flip_vertical, gamma,
    gaussian_blur, grayscale, hue, invert, maximum, mean, median, minimum, new, pixelate, resize,
    resize_to_fill, resize_to_fit, rotate, rotate90, rotate180, rotate270, saturation, sepia,
    sigmoid_filter, sobel, transpose, transverse, unsharp_mask,
};

/// Go: gift_test.go:testFilter
struct TestFilter {
    z: i64,
}

impl Filter for TestFilter {
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx() + self.z, src_bounds.dy() + self.z * 2)
    }

    fn draw(&self, dst: &mut dyn draw::Image, _src: &dyn Image, _options: Option<&Options>) {
        let b = dst.bounds();
        dst.set(b.min.x, b.min.y, Color::Gray(color::Gray { y: 123 }));
    }
}

fn test_filter(z: i64) -> Arc<dyn Filter> {
    Arc::new(TestFilter { z })
}

// Go: gift_test.go:TestGIFT
#[test]
fn test_gift() {
    let mut g = new(vec![]);
    assert_eq!(
        g.parallelization(),
        DEFAULT_OPTIONS.parallelization,
        "unexpected parallelization property"
    );
    g.set_parallelization(true);
    assert!(g.parallelization(), "unexpected parallelization property");
    g.set_parallelization(false);
    assert!(!g.parallelization(), "unexpected parallelization property");

    let mut g = new(vec![test_filter(1), test_filter(2), test_filter(3)]);
    assert_eq!(g.filters.len(), 3, "unexpected filters count");

    g.add([test_filter(4), test_filter(5), test_filter(6)]);
    assert_eq!(g.filters.len(), 6, "unexpected filters count");
    let b = g.bounds(rect(0, 0, 1, 2));
    assert!(b.eq(rect(0, 0, 22, 44)), "unexpected gift bounds");

    g.empty();
    assert_eq!(g.filters.len(), 0, "unexpected filters count");
    let b = g.bounds(rect(0, 0, 1, 2));
    assert!(b.eq(rect(0, 0, 1, 2)), "unexpected gift bounds");

    // g = &GIFT{}: the zero value (no filters, Parallelization false).
    let mut g = GIFT {
        filters: vec![],
        options: Options {
            parallelization: false,
        },
    };
    let mut src = Gray::new(rect(-1, -1, 1, 1));
    src.pix = vec![1, 2, 3, 4];
    let mut dst = Gray::new(g.bounds(src.bounds()));
    g.draw(&mut dst, &src);
    assert!(
        dst.bounds().size().eq(src.bounds().size()),
        "unexpected dst bounds"
    );
    for i in 0..dst.pix.len() {
        assert_eq!(dst.pix[i], src.pix[i], "unexpected dst pix");
    }

    g.add([test_filter(1)]);
    g.add([test_filter(2)]);
    let mut dst = Gray::new(g.bounds(src.bounds()));
    g.draw(&mut dst, &src);
    assert!(
        dst.bounds().dx() == src.bounds().dx() + 3 && dst.bounds().dy() == src.bounds().dy() + 6,
        "unexpected dst bounds"
    );
    assert_eq!(dst.pix[0], 123, "unexpected dst pix");
}

// Go: gift_test.go:TestDrawAt
#[test]
fn test_draw_at() {
    struct Case {
        desc: &'static str,
        filters: Vec<Arc<dyn Filter>>,
        pt: Point,
        op: Operator,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix0: &'static [u8],
        dst_pix1: &'static [u8],
    }
    let test_data_gray: Vec<Case> = vec![
        Case {
            desc: "draw at (Gray, [], -2, -2, copy)",
            filters: vec![],
            pt: pt(-2, -2),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 0, 0, 0, 0, 0],
        },
        Case {
            desc: "draw at (Gray, [], -1, -1, copy)",
            filters: vec![],
            pt: pt(-1, -1),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[0, 0, 0, 0, 0, 1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9],
        },
        Case {
            desc: "draw at (Gray, [], 0, 0, copy)",
            filters: vec![],
            pt: pt(0, 0),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 4, 5],
        },
        Case {
            desc: "draw at (Gray, [], 2, 2, copy)",
            filters: vec![],
            pt: pt(2, 2),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        Case {
            desc: "draw at (Gray, [], 0, -10, copy)",
            filters: vec![],
            pt: pt(0, -10),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        Case {
            desc: "draw at (Gray, [], -3, -3, copy)",
            filters: vec![],
            pt: pt(-3, -3),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[5, 6, 0, 0, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        Case {
            desc: "draw at (Gray, [], -3, -3, over)",
            filters: vec![],
            pt: pt(-3, -3),
            op: OVER_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[5, 6, 0, 0, 8, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        Case {
            desc: "draw at (Gray, [Resize], -2, -2, copy)",
            filters: vec![resize(6, 6, &NEAREST_NEIGHBOR_RESAMPLING)],
            pt: pt(-2, -2),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[1, 1, 2, 2, 1, 1, 2, 2, 4, 4, 5, 5, 4, 4, 5, 5],
        },
        Case {
            desc: "draw at (Gray, [Resize], -3, -3, copy)",
            filters: vec![resize(6, 6, &NEAREST_NEIGHBOR_RESAMPLING)],
            pt: pt(-3, -3),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[1, 2, 2, 3, 4, 5, 5, 6, 4, 5, 5, 6, 7, 8, 8, 9],
        },
        Case {
            desc: "draw at (Gray, [Resize], -1, -1, copy)",
            filters: vec![resize(6, 6, &NEAREST_NEIGHBOR_RESAMPLING)],
            pt: pt(-1, -1),
            op: COPY_OPERATOR,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(-2, -2, 2, 2),
            src_pix: &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            dst_pix0: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            dst_pix1: &[0, 0, 0, 0, 0, 1, 1, 2, 0, 1, 1, 2, 0, 4, 4, 5],
        },
        Case {
            desc: "draw at (Gray, [Resize], -1, -1, copy, empty)",
            filters: vec![resize(6, 6, &NEAREST_NEIGHBOR_RESAMPLING)],
            pt: pt(-1, -1),
            op: COPY_OPERATOR,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix0: &[],
            dst_pix1: &[],
        },
    ];

    for d in test_data_gray {
        let mut src = Gray::new(d.srcb);
        src.pix = d.src_pix.to_vec();

        let g = new(d.filters);

        let mut dst = Gray::new(d.dstb);
        dst.pix = d.dst_pix0.to_vec();

        g.draw_at(&mut dst, &src, d.pt, d.op);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix1),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }

    struct Case1 {
        desc: &'static str,
        filters: Vec<Arc<dyn Filter>>,
        pt: Point,
        op: Operator,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix0: &'static [u8],
        dst_pix1: &'static [u8],
    }
    let test_data_nrgba: Vec<Case1> = vec![
        Case1 {
            desc: "draw at (NRGBA, [], 1, 1, over, 0% 100% alpha)",
            filters: vec![],
            pt: pt(1, 1),
            op: OVER_OPERATOR,
            srcb: rect(0, 0, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                10, 20, 30, 255, 40, 50, 60, 255, //
                100, 200, 0, 255, 0, 250, 200, 255, //
            ],
            dst_pix0: &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            ],
            dst_pix1: &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 10, 20, 30, 255, 40, 50, 60, 255, //
                0, 0, 0, 0, 100, 200, 0, 255, 0, 250, 200, 255, //
            ],
        },
        Case1 {
            desc: "draw at (NRGBA, [], 1, 1, over, 0% 50% alpha)",
            filters: vec![],
            pt: pt(1, 1),
            op: OVER_OPERATOR,
            srcb: rect(0, 0, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                10, 20, 30, 127, 40, 50, 60, 127, //
                100, 200, 0, 127, 0, 250, 200, 127, //
            ],
            dst_pix0: &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            ],
            dst_pix1: &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 0, 0, 0, 10, 20, 30, 127, 40, 50, 60, 127, //
                0, 0, 0, 0, 100, 200, 0, 127, 0, 250, 200, 127, //
            ],
        },
        Case1 {
            desc: "draw at (NRGBA, [], 1, 1, over, 100% 50% alpha)",
            filters: vec![],
            pt: pt(1, 1),
            op: OVER_OPERATOR,
            srcb: rect(0, 0, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                10, 20, 30, 128, 40, 50, 60, 128, //
                100, 200, 0, 128, 0, 250, 200, 128, //
            ],
            dst_pix0: &[
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, //
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, //
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, //
            ],
            dst_pix1: &[
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, //
                0, 0, 0, 255, 5, 10, 15, 255, 20, 25, 30, 255, //
                0, 0, 0, 255, 50, 100, 0, 255, 0, 125, 100, 255, //
            ],
        },
        Case1 {
            desc: "draw at (NRGBA, [], 1, 1, over, 100% 25% alpha)",
            filters: vec![],
            pt: pt(1, 1),
            op: OVER_OPERATOR,
            srcb: rect(0, 0, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                20, 40, 80, 64, 40, 80, 120, 64, //
                100, 200, 0, 64, 0, 100, 200, 64, //
            ],
            dst_pix0: &[
                0, 0, 0, 255, 1, 2, 3, 255, 0, 0, 0, 255, //
                0, 0, 0, 255, 40, 80, 120, 255, 40, 40, 40, 255, //
                0, 0, 0, 255, 200, 200, 12, 255, 0, 0, 0, 255, //
            ],
            dst_pix1: &[
                0, 0, 0, 255, 1, 2, 3, 255, 0, 0, 0, 255, //
                0, 0, 0, 255, 35, 70, 110, 255, 40, 50, 60, 255, //
                0, 0, 0, 255, 175, 200, 9, 255, 0, 25, 50, 255, //
            ],
        },
        Case1 {
            desc: "draw at (NRGBA, [], 1, 1, over, shape)",
            filters: vec![],
            pt: pt(1, 1),
            op: OVER_OPERATOR,
            srcb: rect(0, 0, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                100, 100, 100, 255, 100, 100, 100, 255, //
                100, 100, 100, 255, 100, 100, 100, 0, //
            ],
            dst_pix0: &[
                10, 10, 10, 255, 10, 10, 10, 255, 10, 10, 10, 255, //
                10, 10, 10, 255, 10, 10, 10, 255, 10, 10, 10, 255, //
                10, 10, 10, 255, 10, 10, 10, 255, 10, 10, 10, 255, //
            ],
            dst_pix1: &[
                10, 10, 10, 255, 10, 10, 10, 255, 10, 10, 10, 255, //
                10, 10, 10, 255, 100, 100, 100, 255, 100, 100, 100, 255, //
                10, 10, 10, 255, 100, 100, 100, 255, 10, 10, 10, 255, //
            ],
        },
    ];

    for d in test_data_nrgba {
        let mut src = NRGBA::new(d.srcb);
        src.pix = d.src_pix.to_vec();

        let g = new(d.filters);

        let mut dst = NRGBA::new(d.dstb);
        dst.pix = d.dst_pix0.to_vec();

        g.draw_at(&mut dst, &src, d.pt, d.op);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix1),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }
}

/// Go: gift_test.go:fakeDrawImage
struct FakeDrawImage {
    r: Rectangle,
}

impl Image for FakeDrawImage {
    fn color_model(&self) -> color::Model {
        color::NRGBA_MODEL
    }
    fn bounds(&self) -> Rectangle {
        self.r
    }
    fn at(&self, _x: i64, _y: i64) -> Color {
        Color::NRGBA(color::NRGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl draw::Image for FakeDrawImage {
    fn set(&mut self, _x: i64, _y: i64, _c: Color) {}
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Go: gift_test.go:TestSubImage. getSubImage runs a callback on the
// aliasing sub-image; the Set must be visible in the parent afterwards.
#[test]
fn test_sub_image() {
    let r = rect(0, 0, 10, 10);
    let test_data: Vec<(&str, Box<dyn draw::Image>, bool)> = vec![
        ("sub image (Gray)", Box::new(Gray::new(r)), true),
        ("sub image (Gray16)", Box::new(Gray16::new(r)), true),
        ("sub image (RGBA)", Box::new(RGBA::new(r)), true),
        ("sub image (RGBA64)", Box::new(RGBA64::new(r)), true),
        ("sub image (NRGBA)", Box::new(NRGBA::new(r)), true),
        ("sub image (NRGBA64)", Box::new(NRGBA64::new(r)), true),
        ("sub image (fake)", Box::new(FakeDrawImage { r }), false),
    ];

    for (desc, mut img, want_ok) in test_data {
        let mut sub_bounds = None;
        let ok = get_sub_image(&mut *img, pt(3, 3), |simg| {
            sub_bounds = Some(simg.bounds());
            simg.set(
                5,
                5,
                Color::NRGBA(color::NRGBA {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                }),
            );
        });
        assert_eq!(ok, want_ok, "test [{desc}] failed");
        assert_eq!(ok, sub_bounds.is_some(), "test [{desc}]: callback");
        if ok {
            assert!(
                sub_bounds.unwrap().eq(rect(3, 3, 10, 10)),
                "test [{desc}]: sub-image bounds"
            );
            let (r, g, b, a) = img.at(5, 5).rgba();
            assert!(
                r == 0xffff && g == 0xffff && b == 0xffff && a == 0xffff,
                "test [{desc}] failed: expected (0xffff, 0xffff, 0xffff, 0xffff), got ({r}, {g}, {b}, {a})"
            );
        }
    }
}

// Go: gift_test.go:TestDraw
#[test]
fn test_draw() {
    let nn = &NEAREST_NEIGHBOR_RESAMPLING;
    let filters: Vec<Vec<Arc<dyn Filter>>> = vec![
        vec![],
        vec![resize(2, 2, nn), crop(rect(0, 0, 1, 1))],
        vec![resize(2, 2, nn), crop_to_size(1, 1, CENTER_ANCHOR)],
        vec![flip_horizontal()],
        vec![flip_vertical()],
        vec![resize(2, 2, nn), resize(1, 1, nn)],
        vec![resize(2, 2, nn), resize_to_fit(1, 1, nn)],
        vec![resize(2, 2, nn), resize_to_fill(1, 1, nn, CENTER_ANCHOR)],
        vec![rotate(
            45.0,
            Color::NRGBA(color::NRGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            }),
            NEAREST_NEIGHBOR_INTERPOLATION,
        )],
        vec![rotate90()],
        vec![rotate180()],
        vec![rotate270()],
        vec![transpose()],
        vec![transverse()],
        vec![brightness(10.0)],
        vec![color_balance(10.0, 10.0, 10.0)],
        vec![color_func(|_r0, _g0, _b0, _a0| (1.0, 1.0, 1.0, 1.0))],
        vec![colorize(240.0, 50.0, 100.0)],
        vec![colorspace_linear_to_srgb()],
        vec![colorspace_srgb_to_linear()],
        vec![contrast(10.0)],
        vec![convolution(
            vec![-1.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 1.0],
            false,
            false,
            false,
            0.0,
        )],
        vec![gamma(1.1)],
        vec![gaussian_blur(3.0)],
        vec![grayscale()],
        vec![hue(90.0)],
        vec![invert()],
        vec![maximum(3, true)],
        vec![minimum(3, true)],
        vec![mean(3, true)],
        vec![median(3, true)],
        vec![pixelate(3)],
        vec![saturation(10.0)],
        vec![sepia(10.0)],
        vec![sigmoid_filter(0.5, 5.0)],
        vec![sobel()],
        vec![unsharp_mask(1.0, 1.5, 0.001)],
    ];

    let zero = color::NRGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    for (i, f) in filters.into_iter().enumerate() {
        let mut src = NRGBA::new(rect(1, 1, 2, 2));
        src.pix = vec![255, 255, 255, 255];
        let g = new(f);
        let mut dst = NRGBA::new(rect(-100, -100, -95, -95));
        g.draw(&mut dst, &src);
        for x in dst.bounds().min.x..dst.bounds().max.x {
            for y in dst.bounds().min.y..dst.bounds().max.y {
                let c = match color::NRGBA_MODEL.convert(dst.at(x, y)) {
                    Color::NRGBA(c) => c,
                    c => panic!("{c:?}"),
                };
                let failed = if x == -100 && y == -100 {
                    c == zero
                } else {
                    c != zero
                };
                assert!(!failed, "test draw pos failed: {i} {:?}", dst.pix);
            }
        }
    }
}
