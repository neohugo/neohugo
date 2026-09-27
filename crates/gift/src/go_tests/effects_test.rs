//! Port of gift v1.2.1 `effects_test.go`.

use go_image::{Gray, Rectangle, rect};

use super::{check_bounds_and_pix, gray};
use crate::pixelate;

// Go: effects_test.go:TestPixelate
#[test]
fn test_pixelate() {
    struct Case {
        desc: &'static str,
        size: i64,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "pixelate (0)",
            size: 0,
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
            desc: "pixelate (1)",
            size: 1,
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
            desc: "pixelate (2)",
            size: 2,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x54, 0x54, 0x64, 0x64, 0x30, //
                0x54, 0x54, 0x64, 0x64, 0x30, //
                0x40, 0x40, 0x40, 0x40, 0x00, //
            ],
        },
        Case {
            desc: "pixelate (3)",
            size: 3,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x45, 0x45, 0x45, 0x4d, 0x4d, //
                0x45, 0x45, 0x45, 0x4d, 0x4d, //
                0x45, 0x45, 0x45, 0x4d, 0x4d, //
            ],
        },
        Case {
            desc: "pixelate (10)",
            size: 10,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x49, 0x49, 0x49, 0x49, 0x49, //
                0x49, 0x49, 0x49, 0x49, 0x49, //
                0x49, 0x49, 0x49, 0x49, 0x49, //
            ],
        },
        Case {
            desc: "pixelate 0x0",
            size: 3,
            srcb: rect(-1, -1, -1, -1),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = pixelate(d.size);
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
