//! Port of gift v1.2.1 `transform_test.go`.

use std::sync::Arc;

use go_image::color::{self, Color};
use go_image::{Gray, Rectangle, pt, rect};

use super::{check_bounds_and_pix, gray};
use crate::transform::{
    BOTTOM_ANCHOR, BOTTOM_LEFT_ANCHOR, BOTTOM_RIGHT_ANCHOR, CENTER_ANCHOR, CUBIC_INTERPOLATION,
    LEFT_ANCHOR, LINEAR_INTERPOLATION, NEAREST_NEIGHBOR_INTERPOLATION, RIGHT_ANCHOR, TOP_ANCHOR,
    TOP_LEFT_ANCHOR, TOP_RIGHT_ANCHOR,
};
use crate::{
    Anchor, Filter, Interpolation, crop, crop_to_size, flip_horizontal, flip_vertical, rotate,
    rotate90, rotate180, rotate270, transpose, transverse,
};

/// The body shared by Go's TestRotate90 ... TestTransverse: the filter on a
/// 4x2 Gray image at (-1,-1).
fn check_transform(f: Arc<dyn Filter>, exp_rect: Rectangle, exp_pix: &[u8]) {
    let img0 = gray(rect(-1, -1, 3, 1), &[1, 2, 3, 4, 5, 6, 7, 8]);
    let img1_exp = gray(exp_rect, exp_pix);

    let mut img1 = Gray::new(f.bounds(img0.bounds()));
    f.draw(&mut img1, &img0, None);

    assert!(
        img1.bounds().size().eq(img1_exp.bounds().size()),
        "expected {:?} got {:?}",
        img1_exp.bounds().size(),
        img1.bounds().size()
    );
    assert_eq!(img1_exp.pix, img1.pix);
}

// Go: transform_test.go:TestRotate90
#[test]
fn test_rotate90() {
    check_transform(rotate90(), rect(0, 0, 2, 4), &[4, 8, 3, 7, 2, 6, 1, 5]);
}

// Go: transform_test.go:TestRotate180
#[test]
fn test_rotate180() {
    check_transform(rotate180(), rect(0, 0, 4, 2), &[8, 7, 6, 5, 4, 3, 2, 1]);
}

// Go: transform_test.go:TestRotate270
#[test]
fn test_rotate270() {
    check_transform(rotate270(), rect(0, 0, 2, 4), &[5, 1, 6, 2, 7, 3, 8, 4]);
}

// Go: transform_test.go:TestFlipHorizontal
#[test]
fn test_flip_horizontal() {
    check_transform(
        flip_horizontal(),
        rect(0, 0, 4, 2),
        &[4, 3, 2, 1, 8, 7, 6, 5],
    );
}

// Go: transform_test.go:TestFlipVertical
#[test]
fn test_flip_vertical() {
    check_transform(flip_vertical(), rect(0, 0, 4, 2), &[5, 6, 7, 8, 1, 2, 3, 4]);
}

// Go: transform_test.go:TestTranspose
#[test]
fn test_transpose() {
    check_transform(transpose(), rect(0, 0, 2, 4), &[1, 5, 2, 6, 3, 7, 4, 8]);
}

// Go: transform_test.go:TestTransverse
#[test]
fn test_transverse() {
    check_transform(transverse(), rect(0, 0, 2, 4), &[8, 4, 7, 3, 6, 2, 5, 1]);
}

// Go: transform_test.go:TestCrop
#[test]
fn test_crop() {
    struct Case {
        desc: &'static str,
        r: Rectangle,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "crop (0, 0, 0, 0)",
            r: rect(0, 0, 0, 0),
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[],
        },
        Case {
            desc: "crop (1, 1, -1, -1)",
            r: Rectangle {
                min: pt(1, 1),
                max: pt(-1, -1),
            },
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[],
        },
        Case {
            desc: "crop (-1, 0, 3, 2)",
            r: rect(-1, 0, 3, 2),
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 4, 2),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x60, 0xB0, 0xA0, 0xB0, //
                0x00, 0x80, 0x00, 0x80, //
            ],
        },
        Case {
            desc: "crop (-100, -100, 2, 2)",
            r: rect(-100, -100, 2, 2),
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, //
                0x00, 0x80, 0x00, //
            ],
        },
        Case {
            desc: "crop (-100, -100, 100, 100)",
            r: rect(-100, -100, 100, 100),
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
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = crop(d.r);
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

// Go: transform_test.go:TestCropToSize
#[test]
fn test_crop_to_size() {
    struct Case {
        desc: &'static str,
        w: i64,
        h: i64,
        anchor: Anchor,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "crop to size (0, 0, center)",
            w: 0,
            h: 0,
            anchor: CENTER_ANCHOR,
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
            desc: "crop to size (3, 3, center)",
            w: 3,
            h: 3,
            anchor: CENTER_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x06, 0x07, 0x08, //
                0x0b, 0x0c, 0x0d, //
                0x10, 0x11, 0x12, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, top-left)",
            w: 3,
            h: 3,
            anchor: TOP_LEFT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x00, 0x01, 0x02, //
                0x05, 0x06, 0x07, //
                0x0a, 0x0b, 0x0c, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, top)",
            w: 3,
            h: 3,
            anchor: TOP_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x01, 0x02, 0x03, //
                0x06, 0x07, 0x08, //
                0x0b, 0x0c, 0x0d, //
            ],
        },
        Case {
            desc: "crop to size (3, 3,, top-right)",
            w: 3,
            h: 3,
            anchor: TOP_RIGHT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x02, 0x03, 0x04, //
                0x07, 0x08, 0x09, //
                0x0c, 0x0d, 0x0e, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, left)",
            w: 3,
            h: 3,
            anchor: LEFT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x05, 0x06, 0x07, //
                0x0a, 0x0b, 0x0c, //
                0x0f, 0x10, 0x11, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, right)",
            w: 3,
            h: 3,
            anchor: RIGHT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x07, 0x08, 0x09, //
                0x0c, 0x0d, 0x0e, //
                0x11, 0x12, 0x13, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, bottom-left)",
            w: 3,
            h: 3,
            anchor: BOTTOM_LEFT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x0a, 0x0b, 0x0c, //
                0x0f, 0x10, 0x11, //
                0x14, 0x15, 0x16, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, bottom)",
            w: 3,
            h: 3,
            anchor: BOTTOM_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x0b, 0x0c, 0x0d, //
                0x10, 0x11, 0x12, //
                0x15, 0x16, 0x17, //
            ],
        },
        Case {
            desc: "crop to size (3, 3, bottom-right)",
            w: 3,
            h: 3,
            anchor: BOTTOM_RIGHT_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x0c, 0x0d, 0x0e, //
                0x11, 0x12, 0x13, //
                0x16, 0x17, 0x18, //
            ],
        },
        Case {
            desc: "crop to size (100, 100, center)",
            w: 100,
            h: 100,
            anchor: CENTER_ANCHOR,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 5, 5),
            src_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
            dst_pix: &[
                0x00, 0x01, 0x02, 0x03, 0x04, //
                0x05, 0x06, 0x07, 0x08, 0x09, //
                0x0a, 0x0b, 0x0c, 0x0d, 0x0e, //
                0x0f, 0x10, 0x11, 0x12, 0x13, //
                0x14, 0x15, 0x16, 0x17, 0x18, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = crop_to_size(d.w, d.h, d.anchor);
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

// Go: transform_test.go:TestRotate
#[test]
fn test_rotate() {
    struct Case {
        desc: &'static str,
        a: f32,
        bg: Color,
        interp: Interpolation,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "rotate 0x0 90 white nearest",
            a: 90.0,
            bg: Color::Gray16(color::WHITE),
            interp: NEAREST_NEIGHBOR_INTERPOLATION,
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "rotate 1x1 90 white nearest",
            a: 90.0,
            bg: Color::Gray16(color::WHITE),
            interp: NEAREST_NEIGHBOR_INTERPOLATION,
            srcb: rect(-1, -1, 0, 0),
            dstb: rect(0, 0, 1, 1),
            src_pix: &[0x80],
            dst_pix: &[0x80],
        },
        Case {
            desc: "rotate 3x3 -90 white nearest",
            a: -90.0,
            bg: Color::Gray16(color::WHITE),
            interp: NEAREST_NEIGHBOR_INTERPOLATION,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x10, 0x20, 0x30, //
                0x40, 0x50, 0x60, //
                0x70, 0x80, 0x90, //
            ],
            dst_pix: &[
                0x70, 0x40, 0x10, //
                0x80, 0x50, 0x20, //
                0x90, 0x60, 0x30, //
            ],
        },
        Case {
            desc: "rotate 3x3 -90 white linear",
            a: -90.0,
            bg: Color::Gray16(color::WHITE),
            interp: LINEAR_INTERPOLATION,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x10, 0x20, 0x30, //
                0x40, 0x50, 0x60, //
                0x70, 0x80, 0x90, //
            ],
            dst_pix: &[
                0x70, 0x40, 0x10, //
                0x80, 0x50, 0x20, //
                0x90, 0x60, 0x30, //
            ],
        },
        Case {
            desc: "rotate 3x3 45 black nearest",
            a: 45.0,
            bg: Color::Gray16(color::BLACK),
            interp: NEAREST_NEIGHBOR_INTERPOLATION,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 5, 5),
            src_pix: &[
                0x10, 0x20, 0x30, //
                0x40, 0x50, 0x60, //
                0x70, 0x80, 0x90, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x30, 0x00, 0x00, //
                0x00, 0x20, 0x30, 0x60, 0x00, //
                0x10, 0x10, 0x50, 0x90, 0x90, //
                0x00, 0x40, 0x70, 0x80, 0x00, //
                0x00, 0x00, 0x70, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "rotate 5x5 45 black linear",
            a: 45.0,
            bg: Color::Gray16(color::BLACK),
            interp: LINEAR_INTERPOLATION,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 8, 8),
            src_pix: &[
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x26, 0x26, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x2c, 0xe0, 0xe0, 0x2c, 0x00, 0x00, //
                0x00, 0x2c, 0xe0, 0xff, 0xff, 0xe0, 0x2c, 0x00, //
                0x26, 0xe0, 0xff, 0xff, 0xff, 0xff, 0xe0, 0x26, //
                0x26, 0xe0, 0xff, 0xff, 0xff, 0xff, 0xe0, 0x26, //
                0x00, 0x2c, 0xe0, 0xff, 0xff, 0xe0, 0x2c, 0x00, //
                0x00, 0x00, 0x2c, 0xe0, 0xe0, 0x2c, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x26, 0x26, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "rotate 5x5 45 black cubic",
            a: 45.0,
            bg: Color::Gray16(color::BLACK),
            interp: CUBIC_INTERPOLATION,
            srcb: rect(-1, -1, 4, 4),
            dstb: rect(0, 0, 8, 8),
            src_pix: &[
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
                0xff, 0xff, 0xff, 0xff, 0xff, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x23, 0x23, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x28, 0xf1, 0xf1, 0x28, 0x00, 0x00, //
                0x00, 0x28, 0xe3, 0xff, 0xff, 0xe3, 0x28, 0x00, //
                0x23, 0xf1, 0xff, 0xff, 0xff, 0xff, 0xf1, 0x23, //
                0x23, 0xf1, 0xff, 0xff, 0xff, 0xff, 0xf1, 0x23, //
                0x00, 0x28, 0xe3, 0xff, 0xff, 0xe3, 0x28, 0x00, //
                0x00, 0x00, 0x28, 0xf1, 0xf1, 0x28, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x23, 0x23, 0x00, 0x00, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = rotate(d.a, d.bg, d.interp);
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
