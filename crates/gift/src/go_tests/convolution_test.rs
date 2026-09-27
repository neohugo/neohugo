//! Port of gift v1.2.1 `convolution_test.go`.

use go_image::{Gray, NRGBA, Rectangle, rect};

use super::{check_bounds_and_pix, gray, nrgba};
use crate::convolution::{
    convolve_1dh, convolve_1dv, convolve_line, prepare_convolution_weights,
    prepare_convolution_weights_1d,
};
use crate::gift::{DEFAULT_OPTIONS, Options};
use crate::{convolution, gaussian_blur, mean, sobel, unsharp_mask};

// Go: convolution_test.go:TestConvolution
#[test]
fn test_convolution() {
    struct Case {
        desc: &'static str,
        kernel: &'static [f32],
        normalize: bool,
        alpha: bool,
        abs: bool,
        delta: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "convolution (0x0, false, false, false, 0)",
            kernel: &[],
            normalize: false,
            alpha: false,
            abs: false,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (3x3, false, false, false, 0)",
            kernel: &[
                0.0, 0.0, 1.0, //
                0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, //
            ],
            normalize: false,
            alpha: false,
            abs: false,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x00, 0xA0, 0xA0, 0xA0, 0x00, 0x20, 0x40, 0x60, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x80, 0x60, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (3x3, true, false, false, 0)",
            kernel: &[
                0.0, 0.0, 1.0, //
                0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, //
            ],
            normalize: true,
            alpha: false,
            abs: false,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0x00, 0x10, 0x20, 0x30, 0x80, //
                0x40, 0x30, 0x20, 0x00, 0x50, 0x50, 0x50, 0x00, 0x10, 0x20, 0x30, 0x00, //
                0x40, 0x30, 0x20, 0x20, 0x40, 0x30, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (3x3, false, true, false, 0)",
            kernel: &[
                0.0, 0.0, 1.0, //
                0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, //
            ],
            normalize: false,
            alpha: true,
            abs: false,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0xA0, 0xA0, 0xA0, 0xA0, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (3x3, true, true, true, 0)",
            kernel: &[
                0.0, 0.0, 0.0, //
                -0.5, 0.0, 0.5, //
                0.0, 0.0, 0.0, //
            ],
            normalize: true,
            alpha: true,
            abs: true,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0x60, 0x20, 0x20, 0x60, 0x20, 0x40, 0x60, 0x80, //
                0x80, 0x60, 0x40, 0x20, 0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (3x3, false, true, false, 3 / 255.0)",
            kernel: &[
                0.0, 0.0, 1.0, //
                0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, //
            ],
            normalize: false,
            alpha: true,
            abs: false,
            delta: 3.0 / 255.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x03, 0x03, 0x03, 0x03, 0x23, 0x43, 0x63, 0x83, 0x23, 0x43, 0x63, 0x83, //
                0x83, 0x63, 0x43, 0x23, 0xA3, 0xA3, 0xA3, 0xA3, 0x23, 0x43, 0x63, 0x83, //
                0x83, 0x63, 0x43, 0x23, 0x83, 0x63, 0x43, 0x23, 0x03, 0x03, 0x03, 0x03, //
            ],
        },
        Case {
            desc: "convolution (3x3, false, false, true, 0)",
            kernel: &[
                0.0, 0.0, -0.5, //
                0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, //
            ],
            normalize: false,
            alpha: false,
            abs: true,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0x00, 0x10, 0x20, 0x30, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0x00, 0x10, 0x20, 0x30, 0x00, //
                0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "convolution (7x7, false, true, false, 0)",
            kernel: &[
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
            ],
            normalize: false,
            alpha: true,
            abs: false,
            delta: 0.0,
            srcb: rect(-1, -1, 2, 2),
            dstb: rect(0, 0, 3, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x40, 0x60, 0x80, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x80, 0x60, 0x40, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, //
                0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, //
                0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, 0xA0, //
            ],
        },
    ];

    for d in &test_data {
        for parallel in [true, false] {
            let src = nrgba(d.srcb, d.src_pix);

            let f = convolution(d.kernel.to_vec(), d.normalize, d.alpha, d.abs, d.delta);
            let mut dst = NRGBA::new(f.bounds(src.bounds()));
            f.draw(
                &mut dst,
                &src,
                Some(&Options {
                    parallelization: parallel,
                }),
            );

            assert!(
                check_bounds_and_pix(dst.bounds(), d.dstb, &dst.pix, d.dst_pix),
                "test [{}] failed: {:?}, {:?}",
                d.desc,
                dst.bounds(),
                dst.pix
            );
        }
    }

    struct Case1 {
        klen: i64,
        size: i64,
    }
    let test_kernel_sizes: Vec<Case1> = vec![
        Case1 { klen: 0, size: 0 },
        Case1 { klen: 1, size: 1 },
        Case1 { klen: 3, size: 1 },
        Case1 { klen: 8, size: 1 },
        Case1 { klen: 9, size: 3 },
        Case1 { klen: 16, size: 3 },
        Case1 { klen: 24, size: 3 },
        Case1 { klen: 25, size: 5 },
        Case1 { klen: 40, size: 5 },
        Case1 { klen: 48, size: 5 },
    ];

    for d in &test_kernel_sizes {
        let tmp = vec![0f32; d.klen as usize];
        let (sz, _) = prepare_convolution_weights(&tmp, true);
        assert_eq!(sz, d.size, "unexpected kernel size: {} {}", d.klen, sz);
    }

    // check no panics (Go passes nil options; convolve1dh/v only read them
    // through copyimage, which substitutes the defaults)
    convolution(vec![], true, true, true, 1.0).draw(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        None,
    );
    convolve_1dh(
        &mut Gray::new(rect(0, 0, 1, 1)),
        &Gray::new(rect(0, 0, 1, 1)),
        &[],
        &DEFAULT_OPTIONS,
    );
    convolve_1dv(
        &mut Gray::new(rect(0, 0, 1, 1)),
        &Gray::new(rect(0, 0, 1, 1)),
        &[],
        &DEFAULT_OPTIONS,
    );
    convolve_1dh(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        &[],
        &DEFAULT_OPTIONS,
    );
    convolve_1dv(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        &[],
        &DEFAULT_OPTIONS,
    );
    convolve_line(&mut [], &[], &[]);
    prepare_convolution_weights_1d(&[0.0, 0.0]);
    prepare_convolution_weights_1d(&[]);
}

// Go: convolution_test.go:TestGaussianBlur
#[test]
fn test_gaussian_blur() {
    struct Case {
        desc: &'static str,
        sigma: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "blur (0)",
            sigma: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xC0, 0x00, 0xC0, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xC0, 0x00, 0xC0, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
        },
        Case {
            desc: "blur (0.3)",
            sigma: 0.3,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xC0, 0x00, 0xC0, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x01, 0x00, 0x01, 0x00, //
                0x01, 0xBD, 0x01, 0xBD, 0x01, //
                0x00, 0x01, 0x00, 0x01, 0x00, //
            ],
        },
        Case {
            desc: "blur (1)",
            sigma: 1.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xC0, 0x00, 0xC0, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x0B, 0x15, 0x16, 0x15, 0x0B, //
                0x13, 0x23, 0x25, 0x23, 0x13, //
                0x0B, 0x15, 0x16, 0x15, 0x0B, //
            ],
        },
        Case {
            desc: "blur (3)",
            sigma: 3.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xC0, 0x00, 0xC0, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, //
            ],
            dst_pix: &[
                0x05, 0x06, 0x06, 0x06, 0x05, //
                0x05, 0x06, 0x06, 0x06, 0x05, //
                0x05, 0x06, 0x06, 0x06, 0x05, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = gaussian_blur(d.sigma);
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

    // check no panics
    gaussian_blur(0.5).draw(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        None,
    );
}

// Go: convolution_test.go:TestUnsharpMask
#[test]
fn test_unsharp_mask() {
    struct Case {
        desc: &'static str,
        sigma: f32,
        amount: f32,
        threshold: f32,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "unsharp mask (0.3, 1, 0)",
            sigma: 0.3,
            amount: 1.0,
            threshold: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB1, 0xA1, 0xB1, 0x60, //
                0x00, 0x81, 0x00, 0x81, 0x00, //
            ],
        },
        Case {
            desc: "unsharp mask (1, 1, 0)",
            sigma: 1.0,
            amount: 1.0,
            threshold: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x45, 0x00, 0x45, 0x00, //
                0x82, 0xFF, 0xE4, 0xFF, 0x82, //
                0x00, 0xB2, 0x00, 0xB2, 0x00, //
            ],
        },
        Case {
            desc: "unsharp mask (1, 0.5, 0)",
            sigma: 1.0,
            amount: 0.5,
            threshold: 0.0,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x42, 0x00, 0x42, 0x00, //
                0x71, 0xDD, 0xC2, 0xDD, 0x71, //
                0x00, 0x99, 0x00, 0x99, 0x00, //
            ],
        },
        Case {
            desc: "unsharp mask (1, 1, 0.05)",
            sigma: 1.0,
            amount: 1.0,
            threshold: 0.05,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x60, 0xB0, 0xA0, 0xB0, 0x60, //
                0x00, 0x80, 0x00, 0x80, 0x00, //
            ],
            dst_pix: &[
                0x00, 0x40, 0x00, 0x40, 0x00, //
                0x82, 0xFF, 0xE4, 0xFF, 0x82, //
                0x00, 0xB2, 0x00, 0xB2, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = unsharp_mask(d.sigma, d.amount, d.threshold);
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

    // check no panics
    unsharp_mask(0.5, 1.0, 0.0).draw(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        None,
    );
}

// Go: convolution_test.go:TestMean
#[test]
fn test_mean() {
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
            desc: "mean (0x0 false)",
            ksize: 0,
            disk: false,
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
            desc: "mean (1x1 false)",
            ksize: 1,
            disk: false,
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
            desc: "mean (2x2 true)",
            ksize: 2,
            disk: true,
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
            desc: "mean (3x3 false)",
            ksize: 3,
            disk: false,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x10, 0x40, 0x00, 0x40, 0x10, //
                0x20, 0x50, 0x00, 0x50, 0x20, //
                0x30, 0x60, 0x00, 0x60, 0x30, //
            ],
            dst_pix: &[
                0x25, 0x1E, 0x2E, 0x1E, 0x25, //
                0x30, 0x25, 0x35, 0x25, 0x30, //
                0x3B, 0x2C, 0x3C, 0x2C, 0x3B, //
            ],
        },
        Case {
            desc: "mean (3x3 true)",
            ksize: 3,
            disk: true,
            srcb: rect(-1, -1, 4, 2),
            dstb: rect(0, 0, 5, 3),
            src_pix: &[
                0x10, 0x40, 0x00, 0x40, 0x10, //
                0x20, 0x50, 0x00, 0x50, 0x20, //
                0x30, 0x60, 0x00, 0x60, 0x30, //
            ],
            dst_pix: &[
                0x1D, 0x2D, 0x1A, 0x2D, 0x1D, //
                0x2A, 0x36, 0x20, 0x36, 0x2A, //
                0x36, 0x40, 0x26, 0x40, 0x36, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = mean(d.ksize, d.disk);
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

    // check no panics
    mean(5, false).draw(
        &mut Gray::new(rect(0, 0, 0, 0)),
        &Gray::new(rect(0, 0, 0, 0)),
        None,
    );
}

// Go: convolution_test.go:TestSobel
#[test]
fn test_sobel() {
    struct Case {
        desc: &'static str,
        srcb: Rectangle,
        dstb: Rectangle,
        src_pix: &'static [u8],
        dst_pix: &'static [u8],
    }
    let test_data: Vec<Case> = vec![
        Case {
            desc: "sobel 0x0",
            srcb: rect(0, 0, 0, 0),
            dstb: rect(0, 0, 0, 0),
            src_pix: &[],
            dst_pix: &[],
        },
        Case {
            desc: "sobel 6x6",
            srcb: rect(-1, -1, 5, 5),
            dstb: rect(0, 0, 6, 6),
            src_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, 0x99, 0x99, 0x99, 0x99, //
                0x00, 0x00, 0x99, 0x99, 0x99, 0x99, //
                0x00, 0x00, 0x99, 0x99, 0x99, 0x99, //
                0x00, 0x00, 0x99, 0x99, 0x99, 0x99, //
            ],
            dst_pix: &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0xd8, 0xff, 0xff, 0xff, 0xff, //
                0x00, 0xff, 0xff, 0xff, 0xff, 0xff, //
                0x00, 0xff, 0xff, 0x00, 0x00, 0x00, //
                0x00, 0xff, 0xff, 0x00, 0x00, 0x00, //
                0x00, 0xff, 0xff, 0x00, 0x00, 0x00, //
            ],
        },
    ];

    for d in &test_data {
        let src = gray(d.srcb, d.src_pix);

        let f = sobel();
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
