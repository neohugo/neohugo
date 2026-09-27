//! Port of gift v1.2.1 `resize_test.go`.

use go_image::{Gray, Rectangle, rect};

use super::{check_bounds_and_pix, gray, nrgba};
use crate::resize::{
    BOX_RESAMPLING, CUBIC_RESAMPLING, LANCZOS_RESAMPLING, LINEAR_RESAMPLING,
    NEAREST_NEIGHBOR_RESAMPLING, Resamp, Resampling, bcspline, resize, resize_to_fill,
    resize_to_fit, sinc,
};
use crate::transform::{BOTTOM_ANCHOR, BOTTOM_RIGHT_ANCHOR, CENTER_ANCHOR, TOP_ANCHOR};
use crate::{Anchor, GIFT, new};

// Go: resize_test.go:TestResize
#[test]
fn test_resize() {
    // Testing various sizes and parallelization settings
    let (w, h) = (10i64, 20i64);
    let img0 = Gray::new(rect(0, 0, w, h));
    let sz: [(i64, i64, i64, i64); 12] = [
        (w, h, w, h),
        (w * 2, h, w * 2, h),
        (w, h * 2, w, h * 2),
        (w * 2, h * 2, w * 2, h * 2),
        (w / 2, h, w / 2, h),
        (w, 0, w, h),
        (0, h, w, h),
        (w * 2, 0, w * 2, h * 2),
        (0, h / 2, w / 2, h / 2),
        (0, 0, 0, 0),
        (1, -1, 0, 0),
        (-1, 1, 0, 0),
    ];
    let rfilters: [&'static Resamp; 5] = [
        &NEAREST_NEIGHBOR_RESAMPLING,
        &BOX_RESAMPLING,
        &LINEAR_RESAMPLING,
        &CUBIC_RESAMPLING,
        &LANCZOS_RESAMPLING,
    ];
    for prlz in [true, false] {
        for &(w0, h0, w1, h1) in &sz {
            for f in rfilters {
                let mut g: GIFT = new(vec![resize(w0, h0, f)]);
                g.set_parallelization(prlz);
                let mut img1 = Gray::new(g.bounds(img0.bounds()));
                g.draw(&mut img1, &img0);
                let (w2, h2) = (img1.bounds().dx(), img1.bounds().dy());
                assert!(
                    w2 == w1 && h2 == h1,
                    "resize {f} {w0}x{h0}: expected {w1}x{h1} got {w2}x{h2}"
                );
            }
        }
    }

    // Nearest filter resize
    let img0 = gray(rect(-1, -1, 4, 1), &[1, 2, 3, 4, 5, 6, 7, 8, 0, 1]);
    let img1_exp = gray(rect(0, 0, 2, 2), &[2, 4, 7, 0]);
    let f = resize(2, 2, &NEAREST_NEIGHBOR_RESAMPLING);
    let mut img1 = Gray::new(f.bounds(img0.bounds()));
    f.draw(&mut img1, &img0, None);
    assert!(img1.bounds().size().eq(img1_exp.bounds().size()));
    assert_eq!(img1_exp.pix, img1.pix);

    // Box Filter resize
    let img0 = gray(rect(-1, -1, 3, 1), &[1, 2, 2, 1, 4, 5, 8, 9]);
    let img1_exp = gray(rect(0, 0, 2, 1), &[3, 5]);
    let f = resize(2, 1, &BOX_RESAMPLING);
    let mut img1 = Gray::new(f.bounds(img0.bounds()));
    f.draw(&mut img1, &img0, None);
    assert!(img1.bounds().size().eq(img1_exp.bounds().size()));
    assert_eq!(img1_exp.pix, img1.pix);

    // Empty image should remain empty and not panic
    let img0 = Gray::default();
    let f = resize(100, 100, &BOX_RESAMPLING);
    let mut img1 = Gray::new(f.bounds(img0.bounds()));
    f.draw(&mut img1, &img0, None);
    assert!(
        img1.bounds().dx() == 0 && img1.bounds().dy() == 0,
        "empty image resized is not empty"
    );

    // Testing kernel values outside the window
    for f in rfilters {
        assert_eq!(
            f.kernel(f.support() + 0.000001),
            0.0,
            "filter {f} value outside support != 0"
        );
    }

    // Testing spline and sinc edge cases
    assert_eq!(sinc(0.0), 1.0, "sinc(0) != 1");
    assert_eq!(bcspline(-2.0, 0.0, 0.5), 0.0, "bcspline(-2, ...) != 0");

    // resamp{name: "test"}: Go's zero support and nil kernel.
    fn nil_kernel(_: f32) -> f32 {
        unreachable!()
    }
    let r = Resamp {
        name: "test",
        support: 0.0,
        kernel: nil_kernel,
    };
    assert_eq!(r.to_string(), "test", "resamplingStruct String() fail");

    struct Case {
        desc: &'static str,
        w: i64,
        h: i64,
        r: &'static dyn Resampling,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "resize (1, 2 -> 1, 1; box; non-alpha)",
            w: 1,
            h: 1,
            r: &BOX_RESAMPLING,
            srcb: rect(0, 0, 1, 2),
            dstb: rect(0, 0, 1, 1),
            src_pix: &[
                0xff, 0x00, 0x00, 0xff, //
                0x00, 0xff, 0x00, 0xff, //
            ],
            dst_pix: &[0x80, 0x80, 0x00, 0xff],
        },
        Case {
            desc: "resize (1, 2 -> 1, 1; box; alpha)",
            w: 1,
            h: 1,
            r: &BOX_RESAMPLING,
            srcb: rect(0, 0, 1, 2),
            dstb: rect(0, 0, 1, 1),
            src_pix: &[
                0xff, 0x00, 0x00, 0xff, //
                0x00, 0xff, 0x00, 0x00, //
            ],
            dst_pix: &[0xff, 0x00, 0x00, 0x80],
        },
        Case {
            desc: "resize (1, 2 -> 1, 3; linear; alpha)",
            w: 1,
            h: 3,
            r: &LINEAR_RESAMPLING,
            srcb: rect(0, 0, 1, 2),
            dstb: rect(0, 0, 1, 3),
            src_pix: &[
                0xff, 0x00, 0x00, 0xff, //
                0x00, 0xff, 0x00, 0x00, //
            ],
            dst_pix: &[
                0xff, 0x00, 0x00, 0xff, //
                0xff, 0x00, 0x00, 0x80, //
                0x00, 0x00, 0x00, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        let src = nrgba(d.srcb, d.src_pix);

        let f = resize(d.w, d.h, d.r);
        let mut dst = go_image::NRGBA::new(f.bounds(src.bounds()));
        f.draw(&mut dst, &src, None);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }
}

// Go: resize_test.go:TestResizeToFit
#[test]
fn test_resize_to_fit() {
    struct Case {
        desc: &'static str,
        w: i64,
        h: i64,
        r: &'static dyn Resampling,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "resize to fit (0, 0, nearest)",
            w: 0,
            h: 0,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[],
        },
        Case {
            desc: "resize to fit (1, 1, nearest)",
            w: 1,
            h: 1,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 1, 1),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[0x0c],
        },
        Case {
            desc: "resize to fit (2, 3, box)",
            w: 2,
            h: 3,
            r: &BOX_RESAMPLING,
            srcb: rect(-1, -1, 3, 1),
            dstb: rect(0, 0, 2, 1),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, //
                0x05, 0x06, 0x07, 0x08, //
            ],
            dst_pix: &[0x03, 0x05],
        },
        Case {
            desc: "resize to fit (3, 2, box)",
            w: 3,
            h: 2,
            r: &BOX_RESAMPLING,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 1, 2),
            src_pix: &[
                0x00, 0x01, //
                0x05, 0x06, //
                0x02, 0x03, //
                0x07, 0x08, //
            ],
            dst_pix: &[0x03, 0x05],
        },
        Case {
            desc: "resize to fit (2, 4, box)",
            w: 2,
            h: 4,
            r: &BOX_RESAMPLING,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x01, //
                0x05, 0x06, //
                0x02, 0x03, //
                0x07, 0x08, //
            ],
            dst_pix: &[
                0x00, 0x01, //
                0x05, 0x06, //
                0x02, 0x03, //
                0x07, 0x08, //
            ],
        },
        Case {
            desc: "resize to fit (3, 10, box)",
            w: 3,
            h: 10,
            r: &BOX_RESAMPLING,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 2, 4),
            src_pix: &[
                0x00, 0x01, //
                0x05, 0x06, //
                0x02, 0x03, //
                0x07, 0x08, //
            ],
            dst_pix: &[
                0x00, 0x01, //
                0x05, 0x06, //
                0x02, 0x03, //
                0x07, 0x08, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = resize_to_fit(d.w, d.h, d.r);
        let mut dst = Gray::new(f.bounds(src.bounds()));
        f.draw(&mut dst, &src, None);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }
}

// Go: resize_test.go:TestResizeToFill
#[test]
fn test_resize_to_fill() {
    struct Case {
        desc: &'static str,
        w: i64,
        h: i64,
        r: &'static dyn Resampling,
        anchor: Anchor,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "resize to fill (0, 0, nearest, center)",
            w: 0,
            h: 0,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: CENTER_ANCHOR,
            srcb: rect(-1, -1, 3, 1),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, //
                0x04, 0x05, 0x06, 0x07, //
            ],
            dst_pix: &[],
        },
        Case {
            desc: "resize to fill (4, 2, nearest, center)",
            w: 4,
            h: 2,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: CENTER_ANCHOR,
            srcb: rect(-1, -1, 3, 1),
            dstb: rect(0, 0, 4, 2),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, //
                0x04, 0x05, 0x06, 0x07, //
            ],
            dst_pix: &[
                0x00, 0x01, 0x02, 0x03, //
                0x04, 0x05, 0x06, 0x07, //
            ],
        },
        Case {
            desc: "resize to fill (4, 4, nearest, center)",
            w: 4,
            h: 4,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: CENTER_ANCHOR,
            srcb: rect(-1, -1, 3, 1),
            dstb: rect(0, 0, 4, 4),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, //
                0x04, 0x05, 0x06, 0x07, //
            ],
            dst_pix: &[
                0x01, 0x01, 0x02, 0x02, //
                0x01, 0x01, 0x02, 0x02, //
                0x05, 0x05, 0x06, 0x06, //
                0x05, 0x05, 0x06, 0x06, //
            ],
        },
        Case {
            desc: "resize to fill (4, 4, nearest, bottom-right)",
            w: 4,
            h: 4,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: BOTTOM_RIGHT_ANCHOR,
            srcb: rect(-1, -1, 1, 3),
            dstb: rect(0, 0, 4, 4),
            src_pix: &[
                0x00, 0x01, //
                0x02, 0x03, //
                0x04, 0x05, //
                0x06, 0x07, //
            ],
            dst_pix: &[
                0x04, 0x04, 0x05, 0x05, //
                0x04, 0x04, 0x05, 0x05, //
                0x06, 0x06, 0x07, 0x07, //
                0x06, 0x06, 0x07, 0x07, //
            ],
        },
        Case {
            desc: "resize to fill (2, 1, nearest, bottom)",
            w: 2,
            h: 1,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: BOTTOM_ANCHOR,
            srcb: rect(-1, -1, 5, 5),
            dstb: rect(0, 0, 2, 1),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, //
                0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, //
                0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, //
                0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, //
                0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, //
                0xac, 0xad, 0xae, 0xaf, 0xb0, 0xb1, //
            ],
            dst_pix: &[0xa7, 0xaa],
        },
        Case {
            desc: "resize to fill (2, 1, nearest, top)",
            w: 2,
            h: 1,
            r: &NEAREST_NEIGHBOR_RESAMPLING,
            anchor: TOP_ANCHOR,
            srcb: rect(-1, -1, 5, 5),
            dstb: rect(0, 0, 2, 1),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, //
                0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, //
                0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, //
                0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, //
                0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, //
                0xac, 0xad, 0xae, 0xaf, 0xb0, 0xb1, //
            ],
            dst_pix: &[0x07, 0x0a],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = resize_to_fill(d.w, d.h, d.r, d.anchor);
        let mut dst = Gray::new(f.bounds(src.bounds()));
        f.draw(&mut dst, &src, None);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }
}
