//! Port of gift v1.2.1 `rank_test.go`.

use go_image::{Gray, NRGBA, Rectangle, rect};

use super::{check_bounds_and_pix, gray, nrgba};
use crate::{maximum, median, minimum};

// Go: rank_test.go:TestMedian
#[test]
fn test_median() {
    struct Case {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "median (0, false)",
            ksize: 0,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "median (1, false)",
            ksize: 1,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "median (2, false)",
            ksize: 2,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "median (3, false)",
            ksize: 3,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x44, 0x55, 0x55, 0x55, 0x66, //
                0x44, 0x77, 0x88, 0x88, 0x66, //
                0x44, 0x77, 0x88, 0xBB, 0xCC, //
            ],
        },
        Case {
            desc: "median (3, true)",
            ksize: 3,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x55, 0x55, 0x55, 0x66, //
                0x44, 0x99, 0xBB, 0x88, 0x66, //
                0x33, 0x77, 0xBB, 0xBB, 0xEE, //
            ],
        },
        Case {
            desc: "median (4, true)",
            ksize: 4,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x55, 0x55, 0x55, 0x66, //
                0x44, 0x99, 0xBB, 0x88, 0x66, //
                0x33, 0x77, 0xBB, 0xBB, 0xEE, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = median(d.ksize, d.disk);
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

    struct Case1 {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data_nrgba: Vec<Case1> = vec![Case1 {
        desc: "median nrgba (3, true)",
        ksize: 3,
        disk: true,
        srcb: rect(-1, -1, 4, 2),
        dstb: rect(0, 0, 5, 3),
        src_pix: &[
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x99, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00,
            0x00, 0x22, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xCC, 0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00,
            0x00, 0x88, 0x00, 0x00, 0x00, 0xEE, //
        ],
        dst_pix: &[
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00,
            0x00, 0x55, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x99, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00,
            0x00, 0x88, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00,
            0x00, 0xBB, 0x00, 0x00, 0x00, 0xEE, //
        ],
    }];

    for d in &test_data_nrgba {
        let src = nrgba(d.srcb, d.src_pix);

        let f = median(d.ksize, d.disk);
        let mut dst = NRGBA::new(f.bounds(src.bounds()));
        f.draw(&mut dst, &src, None);

        assert!(
            check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix),
            "test [{}] failed: {:?}, {:?}",
            d.desc,
            dst.bounds(),
            dst.pix
        );
    }

    // check no panics
    median(5, false).draw(
        &mut Gray::new(rect(0, 0, 1, 1)),
        &Gray::new(rect(0, 0, 1, 1)),
        None,
    );
    median(5, false).draw(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        None,
    );
}

// Go: rank_test.go:TestMinimum
#[test]
fn test_minimum() {
    struct Case {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "minimum (0, false)",
            ksize: 0,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "minimum (1, false)",
            ksize: 1,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "minimum (2, false)",
            ksize: 2,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "minimum (3, false)",
            ksize: 3,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x11, 0x22, 0x00, 0x00, //
                0x11, 0x11, 0x22, 0x00, 0x00, //
                0x33, 0x33, 0x44, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "minimum (3, true)",
            ksize: 3,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x11, 0x22, 0x22, 0x00, //
                0x11, 0x44, 0x44, 0x00, 0x00, //
                0x33, 0x33, 0x77, 0x88, 0x00, //
            ],
        },
        Case {
            desc: "minimum (4, true)",
            ksize: 4,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x11, 0x22, 0x22, 0x00, //
                0x11, 0x44, 0x44, 0x00, 0x00, //
                0x33, 0x33, 0x77, 0x88, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = minimum(d.ksize, d.disk);
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

    struct Case1 {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data_nrgba: Vec<Case1> = vec![Case1 {
        desc: "minimum nrgba (3, true)",
        ksize: 3,
        disk: true,
        srcb: rect(-1, -1, 4, 2),
        dstb: rect(0, 0, 5, 3),
        src_pix: &[
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x99, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00,
            0x00, 0x22, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xCC, 0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00,
            0x00, 0x88, 0x00, 0x00, 0x00, 0xEE, //
        ],
        dst_pix: &[
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00,
            0x00, 0x22, 0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00,
            0x00, 0x88, 0x00, 0x00, 0x00, 0x00, //
        ],
    }];

    for d in &test_data_nrgba {
        let src = nrgba(d.srcb, d.src_pix);

        let f = minimum(d.ksize, d.disk);
        let mut dst = NRGBA::new(f.bounds(src.bounds()));
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

// Go: rank_test.go:TestMaximum
#[test]
fn test_maximum() {
    struct Case {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "maximum (0, false)",
            ksize: 0,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "maximum (1, false)",
            ksize: 1,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "maximum (2, false)",
            ksize: 2,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
        },
        Case {
            desc: "maximum (3, false)",
            ksize: 3,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0xFF, 0xFF, 0xFF, 0xFF, 0xCC, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xEE, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xEE, //
            ],
        },
        Case {
            desc: "maximum (3, true)",
            ksize: 3,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0xFF, 0x99, 0xFF, 0xCC, 0x66, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xEE, //
                0xFF, 0xBB, 0xFF, 0xEE, 0xEE, //
            ],
        },
        Case {
            desc: "maximum (4, true)",
            ksize: 4,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x11, 0x99, 0x55, 0x22, 0x66, //
                0xFF, 0x44, 0xFF, 0xCC, 0x00, //
                0x33, 0x77, 0xBB, 0x88, 0xEE, //
            ],
            dst_pix: &[
                0xFF, 0x99, 0xFF, 0xCC, 0x66, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xEE, //
                0xFF, 0xBB, 0xFF, 0xEE, 0xEE, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = maximum(d.ksize, d.disk);
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

    struct Case1 {
        desc: &'static str,
        ksize: i64,
        disk: bool,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data_nrgba: Vec<Case1> = vec![Case1 {
        desc: "maximum nrgba (3, true)",
        ksize: 3,
        disk: true,
        srcb: rect(-1, -1, 4, 2),
        dstb: rect(0, 0, 5, 3),
        src_pix: &[
            0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x99, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00,
            0x00, 0x22, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xCC, 0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00,
            0x00, 0x88, 0x00, 0x00, 0x00, 0xEE, //
        ],
        dst_pix: &[
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x99, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xCC, 0x00, 0x00, 0x00, 0x66, //
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xFF, 0x00, 0x00, 0x00, 0xEE, //
            0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xBB, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00,
            0x00, 0xEE, 0x00, 0x00, 0x00, 0xEE, //
        ],
    }];

    for d in &test_data_nrgba {
        let src = nrgba(d.srcb, d.src_pix);

        let f = maximum(d.ksize, d.disk);
        let mut dst = NRGBA::new(f.bounds(src.bounds()));
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
