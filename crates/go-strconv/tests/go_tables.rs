//! Ports of go1.27.1's own strconv tests:
//! src/internal/strconv/{ftoa,atof,atoi,itoa,decimal,fp,atob,atoc,ctoa,math}_test.go
//! and src/strconv/{quote,number,strconv}_test.go.

#![allow(clippy::excessive_precision)]
#![allow(clippy::approx_constant)]
#![allow(clippy::unusual_byte_groupings)]
#![allow(clippy::type_complexity)]

mod common;

use common::Rng;
use go_strconv as sc;
use go_strconv::internal::{self, Decimal, Error};

fn ffmt(f: f64, fmt: u8, prec: i64, bs: i64) -> String {
    sc::format_float(f, fmt, prec, bs)
}

fn f32of(f: f64) -> f64 {
    internal::f32_to_f64(internal::f64_to_f32(f))
}

// ---------------------------------------------------------------------------
// ftoa_test.go

fn fdiv(a: f64, b: f64) -> f64 {
    std::hint::black_box(a) / std::hint::black_box(b)
}

const BELOW1E23: f64 = 99999999999999974834176.0;
const ABOVE1E23: f64 = 100000000000000008388608.0;

fn ftoatests() -> Vec<(f64, u8, i64, &'static str)> {
    let p2 = |e: i32| 2f64.powi(e);
    vec![
        (1.0, b'e', 5, "1.00000e+00"),
        (1.0, b'f', 5, "1.00000"),
        (1.0, b'g', 5, "1"),
        (1.0, b'g', -1, "1"),
        (1.0, b'x', -1, "0x1p+00"),
        (1.0, b'x', 5, "0x1.00000p+00"),
        (20.0, b'g', -1, "20"),
        (20.0, b'x', -1, "0x1.4p+04"),
        (1234567.8, b'g', -1, "1.2345678e+06"),
        (1234567.8, b'x', -1, "0x1.2d687cccccccdp+20"),
        (200000.0, b'g', -1, "200000"),
        (200000.0, b'x', -1, "0x1.86ap+17"),
        (200000.0, b'X', -1, "0X1.86AP+17"),
        (2000000.0, b'g', -1, "2e+06"),
        (1e10, b'g', -1, "1e+10"),
        // f conversion basic cases
        (12345.0, b'f', 2, "12345.00"),
        (1234.5, b'f', 2, "1234.50"),
        (123.45, b'f', 2, "123.45"),
        (12.345, b'f', 2, "12.35"),
        (1.2345, b'f', 2, "1.23"),
        (0.12345, b'f', 2, "0.12"),
        (0.12945, b'f', 2, "0.13"),
        (0.012345, b'f', 2, "0.01"),
        (0.015, b'f', 2, "0.01"),
        (0.016, b'f', 2, "0.02"),
        (0.0052345, b'f', 2, "0.01"),
        (0.0012345, b'f', 2, "0.00"),
        (0.00012345, b'f', 2, "0.00"),
        (0.000012345, b'f', 2, "0.00"),
        (0.996644984, b'f', 6, "0.996645"),
        (0.996644984, b'f', 5, "0.99664"),
        (0.996644984, b'f', 4, "0.9966"),
        (0.996644984, b'f', 3, "0.997"),
        (0.996644984, b'f', 2, "1.00"),
        (0.996644984, b'f', 1, "1.0"),
        // g conversion and zero suppression
        (400.0, b'g', 2, "4e+02"),
        (40.0, b'g', 2, "40"),
        (4.0, b'g', 2, "4"),
        (0.4, b'g', 2, "0.4"),
        (0.04, b'g', 2, "0.04"),
        (0.004, b'g', 2, "0.004"),
        (0.0004, b'g', 2, "0.0004"),
        (0.00004, b'g', 2, "4e-05"),
        (0.000004, b'g', 2, "4e-06"),
        (0.0, b'e', 5, "0.00000e+00"),
        (0.0, b'f', 5, "0.00000"),
        (0.0, b'g', 5, "0"),
        (0.0, b'g', -1, "0"),
        (0.0, b'x', 5, "0x0.00000p+00"),
        (-1.0, b'e', 5, "-1.00000e+00"),
        (-1.0, b'f', 5, "-1.00000"),
        (-1.0, b'g', 5, "-1"),
        (-1.0, b'g', -1, "-1"),
        (12.0, b'e', 5, "1.20000e+01"),
        (12.0, b'f', 5, "12.00000"),
        (12.0, b'g', 5, "12"),
        (12.0, b'g', -1, "12"),
        (123456700.0, b'e', 5, "1.23457e+08"),
        (123456700.0, b'f', 5, "123456700.00000"),
        (123456700.0, b'g', 5, "1.2346e+08"),
        (123456700.0, b'g', -1, "1.234567e+08"),
        (1.2345e6, b'e', 5, "1.23450e+06"),
        (1.2345e6, b'f', 5, "1234500.00000"),
        (1.2345e6, b'g', 5, "1.2345e+06"),
        // Round to even
        (1.2345e6, b'e', 3, "1.234e+06"),
        (1.2355e6, b'e', 3, "1.236e+06"),
        (1.2345, b'f', 3, "1.234"),
        (1.2355, b'f', 3, "1.236"),
        (1234567890123456.5, b'e', 15, "1.234567890123456e+15"),
        (1234567890123457.5, b'e', 15, "1.234567890123458e+15"),
        (108678236358137.625, b'g', -1, "1.0867823635813762e+14"),
        (1e23, b'e', 17, "9.99999999999999916e+22"),
        (1e23, b'f', 17, "99999999999999991611392.00000000000000000"),
        (1e23, b'g', 17, "9.9999999999999992e+22"),
        (1e23, b'e', -1, "1e+23"),
        (1e23, b'f', -1, "100000000000000000000000"),
        (1e23, b'g', -1, "1e+23"),
        (BELOW1E23, b'e', 17, "9.99999999999999748e+22"),
        (
            BELOW1E23,
            b'f',
            17,
            "99999999999999974834176.00000000000000000",
        ),
        (BELOW1E23, b'g', 17, "9.9999999999999975e+22"),
        (BELOW1E23, b'e', -1, "9.999999999999997e+22"),
        (BELOW1E23, b'f', -1, "99999999999999970000000"),
        (BELOW1E23, b'g', -1, "9.999999999999997e+22"),
        (ABOVE1E23, b'e', 17, "1.00000000000000008e+23"),
        (
            ABOVE1E23,
            b'f',
            17,
            "100000000000000008388608.00000000000000000",
        ),
        (ABOVE1E23, b'g', 17, "1.0000000000000001e+23"),
        (ABOVE1E23, b'e', -1, "1.0000000000000001e+23"),
        (ABOVE1E23, b'f', -1, "100000000000000010000000"),
        (ABOVE1E23, b'g', -1, "1.0000000000000001e+23"),
        (fdiv(5e-304, 1e20), b'g', -1, "5e-324"),
        (fdiv(-5e-304, 1e20), b'g', -1, "-5e-324"),
        (fdiv(5e-304, 1e20), b'e', -1, "5e-324"),
        (32.0, b'g', -1, "32"),
        (32.0, b'g', 0, "3e+01"),
        (100.0, b'x', -1, "0x1.9p+06"),
        (100.0, b'y', -1, "%y"),
        (f64::NAN, b'g', -1, "NaN"),
        (-f64::NAN, b'g', -1, "NaN"),
        (f64::INFINITY, b'g', -1, "+Inf"),
        (f64::NEG_INFINITY, b'g', -1, "-Inf"),
        (-f64::INFINITY, b'g', -1, "-Inf"),
        (-1.0, b'b', -1, "-4503599627370496p-52"),
        // fixed bugs
        (0.9, b'f', 1, "0.9"),
        (0.09, b'f', 1, "0.1"),
        (0.0999, b'f', 1, "0.1"),
        (0.05, b'f', 1, "0.1"),
        (0.05, b'f', 0, "0"),
        (0.5, b'f', 1, "0.5"),
        (0.5, b'f', 0, "0"),
        (1.5, b'f', 0, "2"),
        (2.2250738585072012e-308, b'g', -1, "2.2250738585072014e-308"),
        (2.2250738585072011e-308, b'g', -1, "2.225073858507201e-308"),
        // Issue 2625.
        (383260575764816448.0, b'f', 0, "383260575764816448"),
        (383260575764816448.0, b'g', -1, "3.8326057576481645e+17"),
        // Issue 29491.
        (498484681984085570.0, b'f', -1, "498484681984085570"),
        (-5.8339553793802237e+23, b'g', -1, "-5.8339553793802237e+23"),
        // Issue 52187
        (123.45, b'?', 0, "%?"),
        (123.45, b'?', 1, "%?"),
        (123.45, b'?', -1, "%?"),
        // rounding
        (2.275555555555555, b'x', -1, "0x1.23456789abcdep+01"),
        (2.275555555555555, b'x', 0, "0x1p+01"),
        (2.275555555555555, b'x', 2, "0x1.23p+01"),
        (2.275555555555555, b'x', 16, "0x1.23456789abcde000p+01"),
        (2.275555555555555, b'x', 21, "0x1.23456789abcde00000000p+01"),
        (2.2755555510520935, b'x', -1, "0x1.2345678p+01"),
        (2.2755555510520935, b'x', 6, "0x1.234568p+01"),
        (2.275555431842804, b'x', -1, "0x1.2345668p+01"),
        (2.275555431842804, b'x', 6, "0x1.234566p+01"),
        (3.999969482421875, b'x', -1, "0x1.ffffp+01"),
        (3.999969482421875, b'x', 4, "0x1.ffffp+01"),
        (3.999969482421875, b'x', 3, "0x1.000p+02"),
        (3.999969482421875, b'x', 2, "0x1.00p+02"),
        (3.999969482421875, b'x', 1, "0x1.0p+02"),
        (3.999969482421875, b'x', 0, "0x1p+02"),
        // Cases that Java once mishandled, from David Chase.
        (1.801439850948199e+16, b'g', -1, "1.801439850948199e+16"),
        (5.960464477539063e-08, b'g', -1, "5.960464477539063e-08"),
        (1.012e-320, b'g', -1, "1.012e-320"),
        // Cases from TestFtoaRandom that caught bugs in fixedFtoa.
        (8177880169308380. * p2(1), b'e', 14, "1.63557603386168e+16"),
        (8393378656576888. * p2(1), b'e', 15, "1.678675731315378e+16"),
        (
            8738676561280626. * p2(4),
            b'e',
            16,
            "1.3981882498049002e+17",
        ),
        (8291032395191335. / p2(30), b'e', 5, "7.72163e+06"),
        (
            8880392441509914. / p2(80),
            b'e',
            16,
            "7.3456884594794477e-09",
        ),
        // Exercise divisiblePow5 case in fixedFtoa
        (2384185791015625. * p2(12), b'e', 5, "9.76562e+18"),
        (2384185791015625. * p2(13), b'e', 5, "1.95312e+19"),
        // Exercise potential mistakes in fixedFtoa.
        (
            (0x1000000000005u64 as f64) * p2(23),
            b'e',
            16,
            "2.3611832414348645e+21",
        ), // 0x1.000000000005p+71
        (p2(-27), b'e', 17, "7.45058059692382812e-09"), // 0x1.0000p-27
        (p2(-41), b'e', 17, "4.54747350886464119e-13"), // 0x1.0000p-41
        // go.dev/issue/79591; used NULs instead of trailing zeros
        (
            0.00000000000000000093564868367555,
            b'f',
            150,
            "0.000000000000000000935648683675550014820074270847655141588693848715273682775661612254225474316626787185668945312500000000000000000000000000000000000000",
        ),
    ]
}

#[test]
fn test_ftoa() {
    for (f, fmt, prec, want) in ftoatests() {
        let s = ffmt(f, fmt, prec, 64);
        assert_eq!(s, want, "testN=64 {f} {} {prec}", fmt as char);
        let mut x = b"abc".to_vec();
        sc::append_float(&mut x, f, fmt, prec, 64);
        assert_eq!(x, format!("abc{want}").into_bytes());
        if f32of(f) == f && fmt != b'b' {
            if f == 5.960464477539063e-08 {
                // This test is an exact float32 but asking for float64 precision in the string.
                continue;
            }
            let s = ffmt(f, fmt, prec, 32);
            assert_eq!(s, want, "testN=32 {f} {} {prec}", fmt as char);
            let mut x = b"abc".to_vec();
            sc::append_float(&mut x, f, fmt, prec, 32);
            assert_eq!(x, format!("abc{want}").into_bytes());
        }
    }
}

#[test]
fn test_ftoa_powers_of_two() {
    for exp in -2048..=2048 {
        let f = 2f64.powi(exp); // math.Ldexp(1, exp)
        let f = if exp < -1074 { 0.0 } else { f };
        if !f.is_infinite() {
            let s = ffmt(f, b'e', -1, 64);
            let (x, _) = internal::parse_float(s.as_bytes(), 64);
            assert_eq!(x, f, "failed roundtrip {f} => {s} => {x}");
        }
        let f32v = internal::f64_to_f32(f);
        if !f32v.is_infinite() {
            let s = ffmt(f32v as f64, b'e', -1, 32);
            let (x, _) = internal::parse_float(s.as_bytes(), 32);
            assert_eq!(
                internal::f64_to_f32(x),
                f32v,
                "failed roundtrip {f32v} => {s} => {x}"
            );
        }
    }
}

#[test]
fn test_ftoa_random() {
    let mut r = Rng::new(0xF70A);
    for _ in 0..100_000 {
        let bits = r.next();
        let x = f64::from_bits(bits);

        let short_fast = ffmt(x, b'g', -1, 64);
        internal::set_optimize(false);
        let short_slow = ffmt(x, b'g', -1, 64);
        internal::set_optimize(true);
        assert_eq!(short_slow, short_fast, "{bits:016x}");

        let prec = r.intn(12) + 5;
        let fast = ffmt(x, b'e', prec, 64);
        internal::set_optimize(false);
        let slow = ffmt(x, b'e', prec, 64);
        internal::set_optimize(true);
        assert_eq!(slow, fast, "{bits:016x} %.{prec}e");
    }
}

#[test]
#[should_panic(expected = "strconv: illegal FormatFloat bitSize")]
fn test_format_float_invalid_bit_size() {
    let _ = ffmt(3.14, b'g', -1, 100);
}

#[test]
#[should_panic(expected = "strconv: illegal AppendFloat bitSize")]
fn test_append_float_invalid_bit_size() {
    let mut v = Vec::new();
    sc::append_float(&mut v, 3.14, b'g', -1, 100);
}

#[test]
fn test_ftoa_benches_cases() {
    // ftoaBenches inputs, checked for fast == slow.
    let cases: &[(f64, u8, i64, i64)] = &[
        (33909.0, b'g', -1, 64),
        (339.7784, b'g', -1, 64),
        (-5.09e75, b'g', -1, 64),
        (-5.11e-95, b'g', -1, 64),
        (1.234567890123456e-78, b'g', -1, 64),
        (123456789123456789123456789.0, b'g', -1, 64),
        (-1.0, b'b', -1, 64),
        (33909.0, b'g', -1, 32),
        (3.375, b'g', -1, 32),
        (339.7784, b'g', -1, 32),
        (-5.09e25, b'g', -1, 32),
        (-5.11e-25, b'g', -1, 32),
        (1.234567e-8, b'g', -1, 32),
        (15961084.0 * 2f64.powi(-125), b'e', 8, 32),
        (14855922.0 * 2f64.powi(-83), b'e', 9, 32),
        (123456.0, b'e', 3, 64),
        (123.456, b'e', 3, 64),
        (1.2345e+06, b'e', 3, 64),
        (1.23456e+78, b'e', 3, 64),
        (1.23456e-78, b'e', 3, 64),
        (4.096e+25, b'e', 5, 64),
        (1.23456e-78, b'e', 12, 64),
        (1.23456e-78, b'e', 16, 64),
        (6965949469487146.0 * 2f64.powi(-249), b'e', 12, 64),
        (8887055249355788.0 * 2f64.powi(665), b'e', 17, 64),
        (6994187472632449.0 * 2f64.powi(690), b'e', 18, 64),
        (123.456, b'f', 6, 64),
        (0.0123, b'f', 6, 64),
        (12.3456, b'f', 2, 64),
        (8.03413753080882349e+43, b'e', -1, 64),
        (622666234635.3213e-320, b'e', -1, 64),
        (3.90625e-3, b'e', -1, 32),
        (3.90625e-3, b'e', -1, 64),
    ];
    for &(f, fmt, prec, bs) in cases {
        let fast = ffmt(f, fmt, prec, bs);
        internal::set_optimize(false);
        let slow = ffmt(f, fmt, prec, bs);
        internal::set_optimize(true);
        assert_eq!(fast, slow);
    }
    assert_eq!(
        ffmt(8.03413753080882349e+43, b'e', -1, 64),
        "8.034137530808823e+43"
    );
}

// ---------------------------------------------------------------------------
// atof_test.go

fn atoftests() -> Vec<(String, &'static str, Option<Error>)> {
    let syn = Some(Error::Syntax);
    let rng = Some(Error::Range);
    let mut v: Vec<(String, &'static str, Option<Error>)> = vec![];
    let mut add = |a: &str, b: &'static str, e: Option<Error>| v.push((a.to_string(), b, e));
    add("", "0", syn);
    add("1", "1", None);
    add("+1", "1", None);
    add("1x", "0", syn);
    add("1.1.", "0", syn);
    add("1e23", "1e+23", None);
    add("1E23", "1e+23", None);
    add("100000000000000000000000", "1e+23", None);
    add("1e-100", "1e-100", None);
    add("123456700", "1.234567e+08", None);
    add("99999999999999974834176", "9.999999999999997e+22", None);
    add("100000000000000000000001", "1.0000000000000001e+23", None);
    add("100000000000000008388608", "1.0000000000000001e+23", None);
    add("100000000000000016777215", "1.0000000000000001e+23", None);
    add("100000000000000016777216", "1.0000000000000003e+23", None);
    add("-1", "-1", None);
    add("-0.1", "-0.1", None);
    add("-0", "-0", None);
    add("1e-20", "1e-20", None);
    add("625e-3", "0.625", None);
    // Hexadecimal floating-point.
    add("0x1p0", "1", None);
    add("0x1p1", "2", None);
    add("0x1p-1", "0.5", None);
    add("0x1ep-1", "15", None);
    add("-0x1ep-1", "-15", None);
    add("-0x1_ep-1", "-15", None);
    add("0x1p-200", "6.223015277861142e-61", None);
    add("0x1p200", "1.6069380442589903e+60", None);
    add("0x1fFe2.p0", "131042", None);
    add("0x1fFe2.P0", "131042", None);
    add("-0x2p3", "-16", None);
    add("0x0.fp4", "15", None);
    add("0x0.fp0", "0.9375", None);
    add("0x1e2", "0", syn);
    add("1p2", "0", syn);
    // zeros
    add("0", "0", None);
    add("0e0", "0", None);
    add("-0e0", "-0", None);
    add("+0e0", "0", None);
    add("0e-0", "0", None);
    add("-0e-0", "-0", None);
    add("+0e-0", "0", None);
    add("0e+0", "0", None);
    add("-0e+0", "-0", None);
    add("+0e+0", "0", None);
    add("0e+01234567890123456789", "0", None);
    add("0.00e-01234567890123456789", "0", None);
    add("-0e+01234567890123456789", "-0", None);
    add("-0.00e-01234567890123456789", "-0", None);
    add("0x0p+01234567890123456789", "0", None);
    add("0x0.00p-01234567890123456789", "0", None);
    add("-0x0p+01234567890123456789", "-0", None);
    add("-0x0.00p-01234567890123456789", "-0", None);
    add("0e291", "0", None);
    add("0e292", "0", None);
    add("0e347", "0", None);
    add("0e348", "0", None);
    add("-0e291", "-0", None);
    add("-0e292", "-0", None);
    add("-0e347", "-0", None);
    add("-0e348", "-0", None);
    for e in [
        "126", "127", "128", "129", "130", "1022", "1023", "1024", "1025", "1026",
    ] {
        add(&format!("0x0p{e}"), "0", None);
    }
    for e in [
        "126", "127", "128", "129", "130", "1022", "1023", "1024", "1025", "1026",
    ] {
        add(&format!("-0x0p{e}"), "-0", None);
    }
    // NaNs
    add("nan", "NaN", None);
    add("NaN", "NaN", None);
    add("NAN", "NaN", None);
    // Infs
    add("inf", "+Inf", None);
    add("-Inf", "-Inf", None);
    add("+INF", "+Inf", None);
    add("-Infinity", "-Inf", None);
    add("+INFINITY", "+Inf", None);
    add("Infinity", "+Inf", None);
    // largest float64
    add("1.7976931348623157e308", "1.7976931348623157e+308", None);
    add("-1.7976931348623157e308", "-1.7976931348623157e+308", None);
    add("0x1.fffffffffffffp1023", "1.7976931348623157e+308", None);
    add("-0x1.fffffffffffffp1023", "-1.7976931348623157e+308", None);
    add("0x1fffffffffffffp+971", "1.7976931348623157e+308", None);
    add("-0x1fffffffffffffp+971", "-1.7976931348623157e+308", None);
    add("0x.1fffffffffffffp1027", "1.7976931348623157e+308", None);
    add("-0x.1fffffffffffffp1027", "-1.7976931348623157e+308", None);
    // next float64 - too large
    add("1.7976931348623159e308", "+Inf", rng);
    add("-1.7976931348623159e308", "-Inf", rng);
    add("0x1p1024", "+Inf", rng);
    add("-0x1p1024", "-Inf", rng);
    add("0x2p1023", "+Inf", rng);
    add("-0x2p1023", "-Inf", rng);
    add("0x.1p1028", "+Inf", rng);
    add("-0x.1p1028", "-Inf", rng);
    add("0x.2p1027", "+Inf", rng);
    add("-0x.2p1027", "-Inf", rng);
    // borderline - okay
    add("1.7976931348623158e308", "1.7976931348623157e+308", None);
    add("-1.7976931348623158e308", "-1.7976931348623157e+308", None);
    add(
        "0x1.fffffffffffff7fffp1023",
        "1.7976931348623157e+308",
        None,
    );
    add(
        "-0x1.fffffffffffff7fffp1023",
        "-1.7976931348623157e+308",
        None,
    );
    // borderline - too large
    add("1.797693134862315808e308", "+Inf", rng);
    add("-1.797693134862315808e308", "-Inf", rng);
    add("0x1.fffffffffffff8p1023", "+Inf", rng);
    add("-0x1.fffffffffffff8p1023", "-Inf", rng);
    add("0x1fffffffffffff.8p+971", "+Inf", rng);
    add("-0x1fffffffffffff8p+967", "-Inf", rng);
    add("0x.1fffffffffffff8p1027", "+Inf", rng);
    add("-0x.1fffffffffffff9p1027", "-Inf", rng);
    // a little too large
    add("1e308", "1e+308", None);
    add("2e308", "+Inf", rng);
    add("1e309", "+Inf", rng);
    add("0x1p1025", "+Inf", rng);
    // way too large
    add("1e310", "+Inf", rng);
    add("-1e310", "-Inf", rng);
    add("1e400", "+Inf", rng);
    add("-1e400", "-Inf", rng);
    add("1e400000", "+Inf", rng);
    add("-1e400000", "-Inf", rng);
    add("0x1p1030", "+Inf", rng);
    add("0x1p2000", "+Inf", rng);
    add("0x1p2000000000", "+Inf", rng);
    add("-0x1p1030", "-Inf", rng);
    add("-0x1p2000", "-Inf", rng);
    add("-0x1p2000000000", "-Inf", rng);
    // denormalized
    add("1e-305", "1e-305", None);
    add("1e-306", "1e-306", None);
    add("1e-307", "1e-307", None);
    add("1e-308", "1e-308", None);
    add("1e-309", "1e-309", None);
    add("1e-310", "1e-310", None);
    add("1e-322", "1e-322", None);
    // smallest denormal
    add("5e-324", "5e-324", None);
    add("4e-324", "5e-324", None);
    add("3e-324", "5e-324", None);
    // too small
    add("2e-324", "0", None);
    // way too small
    add("1e-350", "0", None);
    add("1e-400000", "0", None);
    add("1e-345", "0", None);
    add("1e-343", "0", None);
    add("9.999999999999999999e-343", "0", None);
    // Near denormals and denormals.
    add("0x2.00000000000000p-1010", "1.8227805048890994e-304", None);
    add("0x1.fffffffffffff0p-1010", "1.8227805048890992e-304", None);
    add("0x1.fffffffffffff7p-1010", "1.8227805048890992e-304", None);
    add("0x1.fffffffffffff8p-1010", "1.8227805048890994e-304", None);
    add("0x1.fffffffffffff9p-1010", "1.8227805048890994e-304", None);
    add("0x2.00000000000000p-1022", "4.450147717014403e-308", None);
    add("0x1.fffffffffffff0p-1022", "4.4501477170144023e-308", None);
    add("0x1.fffffffffffff7p-1022", "4.4501477170144023e-308", None);
    add("0x1.fffffffffffff8p-1022", "4.450147717014403e-308", None);
    add("0x1.fffffffffffff9p-1022", "4.450147717014403e-308", None);
    add("0x1.00000000000000p-1022", "2.2250738585072014e-308", None);
    add("0x0.fffffffffffff0p-1022", "2.225073858507201e-308", None);
    add("0x0.ffffffffffffe0p-1022", "2.2250738585072004e-308", None);
    add("0x0.ffffffffffffe7p-1022", "2.2250738585072004e-308", None);
    add("0x1.ffffffffffffe8p-1023", "2.225073858507201e-308", None);
    add("0x1.ffffffffffffe9p-1023", "2.225073858507201e-308", None);
    add("0x0.00000003fffff0p-1022", "2.072261e-317", None);
    add("0x0.00000003456780p-1022", "1.694649e-317", None);
    add("0x0.00000003456787p-1022", "1.694649e-317", None);
    add("0x0.00000003456788p-1022", "1.694649e-317", None);
    add("0x0.00000003456790p-1022", "1.6946496e-317", None);
    add("0x0.00000003456789p-1022", "1.6946496e-317", None);
    add(
        "0x0.0000000345678800000000000000000000000001p-1022",
        "1.6946496e-317",
        None,
    );
    add("0x0.000000000000f0p-1022", "7.4e-323", None);
    add("0x0.00000000000060p-1022", "3e-323", None);
    add("0x0.00000000000058p-1022", "3e-323", None);
    add("0x0.00000000000057p-1022", "2.5e-323", None);
    add("0x0.00000000000050p-1022", "2.5e-323", None);
    add("0x0.00000000000010p-1022", "5e-324", None);
    add("0x0.000000000000081p-1022", "5e-324", None);
    add("0x0.00000000000008p-1022", "0", None);
    add("0x0.00000000000007fp-1022", "0", None);
    // try to overflow exponent
    add("1e-4294967296", "0", None);
    add("1e+4294967296", "+Inf", rng);
    add("1e-18446744073709551616", "0", None);
    add("1e+18446744073709551616", "+Inf", rng);
    add("0x1p-4294967296", "0", None);
    add("0x1p+4294967296", "+Inf", rng);
    add("0x1p-18446744073709551616", "0", None);
    add("0x1p+18446744073709551616", "+Inf", rng);
    // Parse errors
    add("1e", "0", syn);
    add("1e-", "0", syn);
    add(".e-1", "0", syn);
    add("1\x00.2", "0", syn);
    add("0x", "0", syn);
    add("0x.", "0", syn);
    add("0x1", "0", syn);
    add("0x.1", "0", syn);
    add("0x1p", "0", syn);
    add("0x.1p", "0", syn);
    add("0x1p+", "0", syn);
    add("0x.1p+", "0", syn);
    add("0x1p-", "0", syn);
    add("0x.1p-", "0", syn);
    add("0x1p+2", "4", None);
    add("0x.1p+2", "0.25", None);
    add("0x1p-2", "0.25", None);
    add("0x.1p-2", "0.015625", None);
    add("2.2250738585072012e-308", "2.2250738585072014e-308", None);
    add("2.2250738585072011e-308", "2.225073858507201e-308", None);
    // A very large number (initially wrongly parsed by the fast algorithm).
    add("4.630813248087435e+307", "4.630813248087435e+307", None);
    // A different kind of very large number.
    add("22.222222222222222", "22.22222222222222", None);
    add(
        &format!("2.{}e+1", "2".repeat(4000)),
        "22.22222222222222",
        None,
    );
    add("0x1.1111111111111p222", "7.18931911124017e+66", None);
    add("0x2.2222222222222p221", "7.18931911124017e+66", None);
    add(
        &format!("0x2.{}p221", "2".repeat(4000)),
        "7.18931911124017e+66",
        None,
    );
    // Exactly halfway between 1 and math.Nextafter(1, 2).
    add(
        "1.00000000000000011102230246251565404236316680908203125",
        "1",
        None,
    );
    add("0x1.00000000000008p0", "1", None);
    add(
        "1.00000000000000011102230246251565404236316680908203124",
        "1",
        None,
    );
    add("0x1.00000000000007Fp0", "1", None);
    add(
        "1.00000000000000011102230246251565404236316680908203126",
        "1.0000000000000002",
        None,
    );
    add("0x1.000000000000081p0", "1.0000000000000002", None);
    add("0x1.00000000000009p0", "1.0000000000000002", None);
    add(
        &format!(
            "1.00000000000000011102230246251565404236316680908203125{}1",
            "0".repeat(10000)
        ),
        "1.0000000000000002",
        None,
    );
    add(
        &format!("0x1.00000000000008{}1p0", "0".repeat(10000)),
        "1.0000000000000002",
        None,
    );
    add(
        "1.00000000000000033306690738754696212708950042724609375",
        "1.0000000000000004",
        None,
    );
    add("0x1.00000000000018p0", "1.0000000000000004", None);
    add(
        "1090544144181609348671888949248",
        "1.0905441441816093e+30",
        None,
    );
    add(
        "1090544144181609348835077142190",
        "1.0905441441816094e+30",
        None,
    );
    // Underscores.
    add("1_23.50_0_0e+1_2", "1.235e+14", None);
    for s in [
        "-_123.5e+12",
        "+_123.5e+12",
        "_123.5e+12",
        "1__23.5e+12",
        "123_.5e+12",
        "123._5e+12",
        "123.5_e+12",
        "123.5__0e+12",
        "123.5e_+12",
        "123.5e+_12",
        "123.5e_-12",
        "123.5e-_12",
        "123.5e+1__2",
        "123.5e+12_",
    ] {
        add(s, "0", syn);
    }
    add("0x_1_2.3_4_5p+1_2", "74565", None);
    for s in [
        "-_0x12.345p+12",
        "+_0x12.345p+12",
        "_0x12.345p+12",
        "0x__12.345p+12",
        "0x1__2.345p+12",
        "0x12_.345p+12",
        "0x12._345p+12",
        "0x12.3__45p+12",
        "0x12.345_p+12",
        "0x12.345p_+12",
        "0x12.345p+_12",
        "0x12.345p_-12",
        "0x12.345p-_12",
        "0x12.345p+1__2",
        "0x12.345p+12_",
    ] {
        add(s, "0", syn);
    }
    add("1e100x", "0", syn);
    add("1e1000x", "0", syn);
    v
}

fn atof32tests() -> Vec<(String, &'static str, Option<Error>)> {
    let rng = Some(Error::Range);
    let mut v: Vec<(String, &'static str, Option<Error>)> = vec![];
    let mut add = |a: &str, b: &'static str, e: Option<Error>| v.push((a.to_string(), b, e));
    add("0x1p-100", "7.888609e-31", None);
    add("0x1p100", "1.2676506e+30", None);
    add("1.000000059604644775390625", "1", None);
    add("0x1.000001p0", "1", None);
    add("1.000000059604644775390624", "1", None);
    add("0x1.0000008p0", "1", None);
    add("0x1.000000fp0", "1", None);
    add("1.000000059604644775390626", "1.0000001", None);
    add("0x1.000002p0", "1.0000001", None);
    add("0x1.0000018p0", "1.0000001", None);
    add("0x1.0000011p0", "1.0000001", None);
    add(
        &format!("1.000000059604644775390625{}1", "0".repeat(10000)),
        "1.0000001",
        None,
    );
    add(
        &format!("0x1.000001{}1p0", "0".repeat(10000)),
        "1.0000001",
        None,
    );
    add(
        "340282346638528859811704183484516925440",
        "3.4028235e+38",
        None,
    );
    add(
        "-340282346638528859811704183484516925440",
        "-3.4028235e+38",
        None,
    );
    add("0x.ffffffp128", "3.4028235e+38", None);
    add(
        "-340282346638528859811704183484516925440",
        "-3.4028235e+38",
        None,
    );
    add("-0x.ffffffp128", "-3.4028235e+38", None);
    add("3.4028236e38", "+Inf", rng);
    add("-3.4028236e38", "-Inf", rng);
    add("0x1.0p128", "+Inf", rng);
    add("-0x1.0p128", "-Inf", rng);
    add("3.402823567e38", "3.4028235e+38", None);
    add("-3.402823567e38", "-3.4028235e+38", None);
    add("0x.ffffff7fp128", "3.4028235e+38", None);
    add("-0x.ffffff7fp128", "-3.4028235e+38", None);
    add("3.4028235678e38", "+Inf", rng);
    add("-3.4028235678e38", "-Inf", rng);
    add("0x.ffffff8p128", "+Inf", rng);
    add("-0x.ffffff8p128", "-Inf", rng);
    add("1e-38", "1e-38", None);
    add("1e-39", "1e-39", None);
    add("1e-40", "1e-40", None);
    add("1e-41", "1e-41", None);
    add("1e-42", "1e-42", None);
    add("1e-43", "1e-43", None);
    add("1e-44", "1e-44", None);
    add("6e-45", "6e-45", None);
    add("5e-45", "6e-45", None);
    add("1e-45", "1e-45", None);
    add("2e-45", "1e-45", None);
    add("3e-45", "3e-45", None);
    add("0x0.89aBcDp-125", "1.2643093e-38", None);
    add("0x0.8000000p-125", "1.1754944e-38", None);
    add("0x0.1234560p-125", "1.671814e-39", None);
    add("0x0.1234567p-125", "1.671814e-39", None);
    add("0x0.1234568p-125", "1.671814e-39", None);
    add("0x0.1234569p-125", "1.671815e-39", None);
    add("0x0.1234570p-125", "1.671815e-39", None);
    add("0x0.0000010p-125", "1e-45", None);
    add("0x0.00000081p-125", "1e-45", None);
    add("0x0.0000008p-125", "0", None);
    add("0x0.0000007p-125", "0", None);
    add("1e-70", "0", None);
    add("1e-65", "0", None);
    add("1e-64", "0", None);
    add("9.999999999999999999e-64", "0", None);
    add("4951760157141521099596496896", "4.9517602e+27", None);
    v
}

#[test]
fn test_parse_float_prefix() {
    for (input, _, err) in atoftests() {
        if err.is_some() {
            continue;
        }
        for suffix in [" ", "q", "+", "-", "<", "=", ">", "(", ")", "i", "init"] {
            let s = format!("{input}{suffix}");
            let (_, n, err) = internal::parse_float_prefix(s.as_bytes(), 64);
            assert_eq!(err, None, "ParseFloatPrefix({s:?}, 64)");
            assert_eq!(n, input.len(), "ParseFloatPrefix({s:?}, 64)");
        }
    }
}

fn test_atof(opt: bool) {
    let oldopt = internal::set_optimize(opt);
    for (input, want, want_err) in atoftests() {
        let (out, err) = internal::parse_float(input.as_bytes(), 64);
        let outs = ffmt(out, b'g', -1, 64);
        assert_eq!(
            (outs.as_str(), err),
            (want, want_err),
            "ParseFloat({input:?}, 64) opt={opt}"
        );
        // public API error wrapping
        match sc::parse_float(&input, 64) {
            Ok(v) => {
                assert!(want_err.is_none());
                assert_eq!(v.to_bits(), out.to_bits());
            }
            Err(e) => {
                assert_eq!(e.func, "ParseFloat");
                assert_eq!(e.num, input.as_bytes());
                assert_eq!(
                    e.err,
                    match want_err.unwrap() {
                        Error::Range => sc::Error::Range,
                        _ => sc::Error::Syntax,
                    }
                );
            }
        }
        if f32of(out) == out {
            let (out, err) = internal::parse_float(input.as_bytes(), 32);
            let out32 = internal::f64_to_f32(out);
            assert_eq!(out32 as f64, out, "ParseFloat({input:?}, 32) not a float32");
            let outs = ffmt(out32 as f64, b'g', -1, 32);
            assert_eq!(
                (outs.as_str(), err),
                (want, want_err),
                "ParseFloat({input:?}, 32) opt={opt}"
            );
        }
    }
    for (input, want, want_err) in atof32tests() {
        let (out, err) = internal::parse_float(input.as_bytes(), 32);
        let out32 = internal::f64_to_f32(out);
        assert_eq!(out32 as f64, out, "ParseFloat({input:?}, 32) not a float32");
        let outs = ffmt(out32 as f64, b'g', -1, 32);
        assert_eq!(
            (outs.as_str(), err),
            (want, want_err),
            "ParseFloat({input:?}, 32) opt={opt}"
        );
    }
    internal::set_optimize(oldopt);
}

#[test]
fn test_atof_fast() {
    test_atof(true);
}

#[test]
fn test_atof_slow() {
    test_atof(false);
}

#[test]
fn test_atof_random() {
    let mut r = Rng::new(0xA70F);
    for _ in 0..100_000 {
        let x = f64::from_bits(r.next());
        let s = ffmt(x, b'g', -1, 64);
        let (y, _) = internal::parse_float(s.as_bytes(), 64);
        assert!(
            y == x || (x.is_nan() && y.is_nan()),
            "number {s} badly parsed"
        );
    }
}

#[test]
fn test_round_trip() {
    // Go: {8865794286000691 << 39, ...} is an exact untyped constant.
    let cases = [
        (8865794286000691u64, "4.87402195346389e+27"),
        (8865794286000692u64, "4.8740219534638903e+27"),
    ];
    for (i, s) in cases {
        let f = i as f64 * 2f64.powi(39);
        for opt in [false, true] {
            let old = internal::set_optimize(opt);
            assert_eq!(ffmt(f, b'g', -1, 64), s);
            let (g, err) = internal::parse_float(s.as_bytes(), 64);
            assert_eq!((g, err), (f, None));
            internal::set_optimize(old);
        }
    }
}

#[test]
fn test_round_trip32() {
    let step = 997u32;
    let mut count = 0;
    let mut i = 0u32;
    while i < 0xff << 23 {
        let mut f = f32::from_bits(i);
        if i & 1 == 1 {
            f = -f; // negative
        }
        let s = ffmt(f as f64, b'g', -1, 32);
        let (parsed, err) = internal::parse_float(s.as_bytes(), 32);
        let parsed32 = internal::f64_to_f32(parsed);
        assert_eq!(err, None, "ParseFloat({s:?}, 32)");
        assert_eq!(
            parsed32 as f64, parsed,
            "ParseFloat({s:?}, 32) not a float32"
        );
        assert_eq!(parsed32.to_bits(), f.to_bits(), "ParseFloat({s:?}, 32)");
        count += 1;
        i += step;
    }
    assert!(count > 2_000_000);
}

#[test]
fn test_parse_float_incorrect_bit_size() {
    for bs in [0, 10, 100, 128] {
        assert_eq!(sc::parse_float("1.5e308", bs), Ok(1.5e308));
    }
}

// ---------------------------------------------------------------------------
// math_test.go

#[test]
fn test_log10_pow2() {
    use std::f64::consts::{LN_2, LN_10};
    for x in -1600i64..=1600 {
        let i = internal::export_test::log10_pow2(x);
        let f = (x as f64 * LN_2 / LN_10).floor() as i64;
        assert_eq!(i, f, "log10Pow2({x})");
    }
    for x in -500i64..=500 {
        let i = internal::export_test::log2_pow10(x);
        let f = (x as f64 * LN_10 / LN_2).floor() as i64;
        assert_eq!(i, f, "log2Pow10({x})");
    }
}

// ---------------------------------------------------------------------------
// atoi_test.go

fn syn<T: Default>() -> (T, Option<Error>) {
    (T::default(), Some(Error::Syntax))
}

fn parse_uint64_tests() -> Vec<(&'static str, u64, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, s),
        ("0", 0, None),
        ("1", 1, None),
        ("12345", 12345, None),
        ("012345", 12345, None),
        ("12345x", 0, s),
        ("98765432100", 98765432100, None),
        ("18446744073709551615", u64::MAX, None),
        ("18446744073709551616", u64::MAX, r),
        ("18446744073709551620", u64::MAX, r),
        ("1_2_3_4_5", 0, s),
        ("_12345", 0, s),
        ("1__2345", 0, s),
        ("12345_", 0, s),
        ("-0", 0, s),
        ("-1", 0, s),
        ("+1", 0, s),
    ]
}

fn parse_uint64_base_tests() -> Vec<(&'static str, i64, u64, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, 0, s),
        ("0", 0, 0, None),
        ("0x", 0, 0, s),
        ("0X", 0, 0, s),
        ("1", 0, 1, None),
        ("12345", 0, 12345, None),
        ("012345", 0, 0o12345, None),
        ("0x12345", 0, 0x12345, None),
        ("0X12345", 0, 0x12345, None),
        ("12345x", 0, 0, s),
        ("0xabcdefg123", 0, 0, s),
        ("123456789abc", 0, 0, s),
        ("98765432100", 0, 98765432100, None),
        ("18446744073709551615", 0, u64::MAX, None),
        ("18446744073709551616", 0, u64::MAX, r),
        ("18446744073709551620", 0, u64::MAX, r),
        ("0xFFFFFFFFFFFFFFFF", 0, u64::MAX, None),
        ("0x10000000000000000", 0, u64::MAX, r),
        ("01777777777777777777777", 0, u64::MAX, None),
        ("01777777777777777777778", 0, 0, s),
        ("02000000000000000000000", 0, u64::MAX, r),
        ("0200000000000000000000", 0, 1 << 61, None),
        ("0b", 0, 0, s),
        ("0B", 0, 0, s),
        ("0b101", 0, 5, None),
        ("0B101", 0, 5, None),
        ("0o", 0, 0, s),
        ("0O", 0, 0, s),
        ("0o377", 0, 255, None),
        ("0O377", 0, 255, None),
        // underscores allowed with base == 0 only
        ("1_2_3_4_5", 0, 12345, None),
        ("_12345", 0, 0, s),
        ("1__2345", 0, 0, s),
        ("12345_", 0, 0, s),
        ("1_2_3_4_5", 10, 0, s),
        ("_12345", 10, 0, s),
        ("1__2345", 10, 0, s),
        ("12345_", 10, 0, s),
        ("0x_1_2_3_4_5", 0, 0x12345, None),
        ("_0x12345", 0, 0, s),
        ("0x__12345", 0, 0, s),
        ("0x1__2345", 0, 0, s),
        ("0x1234__5", 0, 0, s),
        ("0x12345_", 0, 0, s),
        ("1_2_3_4_5", 16, 0, s),
        ("_12345", 16, 0, s),
        ("1__2345", 16, 0, s),
        ("1234__5", 16, 0, s),
        ("12345_", 16, 0, s),
        ("0_1_2_3_4_5", 0, 0o12345, None),
        ("_012345", 0, 0, s),
        ("0__12345", 0, 0, s),
        ("01234__5", 0, 0, s),
        ("012345_", 0, 0, s),
        ("0o_1_2_3_4_5", 0, 0o12345, None),
        ("_0o12345", 0, 0, s),
        ("0o__12345", 0, 0, s),
        ("0o1234__5", 0, 0, s),
        ("0o12345_", 0, 0, s),
        ("0_1_2_3_4_5", 8, 0, s),
        ("_012345", 8, 0, s),
        ("0__12345", 8, 0, s),
        ("01234__5", 8, 0, s),
        ("012345_", 8, 0, s),
        ("0b_1_0_1", 0, 5, None),
        ("_0b101", 0, 0, s),
        ("0b__101", 0, 0, s),
        ("0b1__01", 0, 0, s),
        ("0b10__1", 0, 0, s),
        ("0b101_", 0, 0, s),
        ("1_0_1", 2, 0, s),
        ("_101", 2, 0, s),
        ("1_01", 2, 0, s),
        ("10_1", 2, 0, s),
        ("101_", 2, 0, s),
    ]
}

fn parse_int64_tests() -> Vec<(&'static str, i64, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, s),
        ("0", 0, None),
        ("-0", 0, None),
        ("+0", 0, None),
        ("1", 1, None),
        ("-1", -1, None),
        ("+1", 1, None),
        ("12345", 12345, None),
        ("-12345", -12345, None),
        ("012345", 12345, None),
        ("-012345", -12345, None),
        ("98765432100", 98765432100, None),
        ("-98765432100", -98765432100, None),
        ("9223372036854775807", i64::MAX, None),
        ("-9223372036854775807", -i64::MAX, None),
        ("9223372036854775808", i64::MAX, r),
        ("-9223372036854775808", i64::MIN, None),
        ("9223372036854775809", i64::MAX, r),
        ("-9223372036854775809", i64::MIN, r),
        ("-1_2_3_4_5", 0, s),
        ("-_12345", 0, s),
        ("_12345", 0, s),
        ("1__2345", 0, s),
        ("12345_", 0, s),
        ("123%45", 0, s),
    ]
}

fn parse_int64_base_tests() -> Vec<(&'static str, i64, i64, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, 0, s),
        ("0", 0, 0, None),
        ("-0", 0, 0, None),
        ("1", 0, 1, None),
        ("-1", 0, -1, None),
        ("12345", 0, 12345, None),
        ("-12345", 0, -12345, None),
        ("012345", 0, 0o12345, None),
        ("-012345", 0, -0o12345, None),
        ("0x12345", 0, 0x12345, None),
        ("-0X12345", 0, -0x12345, None),
        ("12345x", 0, 0, s),
        ("-12345x", 0, 0, s),
        ("98765432100", 0, 98765432100, None),
        ("-98765432100", 0, -98765432100, None),
        ("9223372036854775807", 0, i64::MAX, None),
        ("-9223372036854775807", 0, -i64::MAX, None),
        ("9223372036854775808", 0, i64::MAX, r),
        ("-9223372036854775808", 0, i64::MIN, None),
        ("9223372036854775809", 0, i64::MAX, r),
        ("-9223372036854775809", 0, i64::MIN, r),
        // other bases
        ("g", 17, 16, None),
        ("10", 25, 25, None),
        (
            "holycow",
            35,
            (((((17 * 35 + 24) * 35 + 21) * 35 + 34) * 35 + 12) * 35 + 24) * 35 + 32,
            None,
        ),
        (
            "holycow",
            36,
            (((((17 * 36 + 24) * 36 + 21) * 36 + 34) * 36 + 12) * 36 + 24) * 36 + 32,
            None,
        ),
        // base 2
        ("0", 2, 0, None),
        ("-1", 2, -1, None),
        ("1010", 2, 10, None),
        ("1000000000000000", 2, 1 << 15, None),
        (
            "111111111111111111111111111111111111111111111111111111111111111",
            2,
            i64::MAX,
            None,
        ),
        (
            "1000000000000000000000000000000000000000000000000000000000000000",
            2,
            i64::MAX,
            r,
        ),
        (
            "-1000000000000000000000000000000000000000000000000000000000000000",
            2,
            i64::MIN,
            None,
        ),
        (
            "-1000000000000000000000000000000000000000000000000000000000000001",
            2,
            i64::MIN,
            r,
        ),
        // base 8
        ("-10", 8, -8, None),
        ("57635436545", 8, 0o57635436545, None),
        ("100000000", 8, 1 << 24, None),
        // base 16
        ("10", 16, 16, None),
        ("-123456789abcdef", 16, -0x123456789abcdef, None),
        ("7fffffffffffffff", 16, i64::MAX, None),
        // underscores
        ("-0x_1_2_3_4_5", 0, -0x12345, None),
        ("0x_1_2_3_4_5", 0, 0x12345, None),
        ("-_0x12345", 0, 0, s),
        ("_-0x12345", 0, 0, s),
        ("_0x12345", 0, 0, s),
        ("0x__12345", 0, 0, s),
        ("0x1__2345", 0, 0, s),
        ("0x1234__5", 0, 0, s),
        ("0x12345_", 0, 0, s),
        ("-0_1_2_3_4_5", 0, -0o12345, None),
        ("0_1_2_3_4_5", 0, 0o12345, None),
        ("-_012345", 0, 0, s),
        ("_-012345", 0, 0, s),
        ("_012345", 0, 0, s),
        ("0__12345", 0, 0, s),
        ("01234__5", 0, 0, s),
        ("012345_", 0, 0, s),
        ("+0xf", 0, 0xf, None),
        ("-0xf", 0, -0xf, None),
        ("0x+f", 0, 0, s),
        ("0x-f", 0, 0, s),
    ]
}

fn parse_uint32_tests() -> Vec<(&'static str, u32, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, s),
        ("0", 0, None),
        ("1", 1, None),
        ("12345", 12345, None),
        ("012345", 12345, None),
        ("12345x", 0, s),
        ("987654321", 987654321, None),
        ("4294967295", u32::MAX, None),
        ("4294967296", u32::MAX, r),
        ("1_2_3_4_5", 0, s),
        ("_12345", 0, s),
        ("_12345", 0, s),
        ("1__2345", 0, s),
        ("12345_", 0, s),
    ]
}

fn parse_int32_tests() -> Vec<(&'static str, i32, Option<Error>)> {
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    vec![
        ("", 0, s),
        ("0", 0, None),
        ("-0", 0, None),
        ("1", 1, None),
        ("-1", -1, None),
        ("12345", 12345, None),
        ("-12345", -12345, None),
        ("012345", 12345, None),
        ("-012345", -12345, None),
        ("12345x", 0, s),
        ("-12345x", 0, s),
        ("987654321", 987654321, None),
        ("-987654321", -987654321, None),
        ("2147483647", i32::MAX, None),
        ("-2147483647", -i32::MAX, None),
        ("2147483648", i32::MAX, r),
        ("-2147483648", i32::MIN, None),
        ("2147483649", i32::MAX, r),
        ("-2147483649", i32::MIN, r),
        ("-1_2_3_4_5", 0, s),
        ("-_12345", 0, s),
        ("_12345", 0, s),
        ("1__2345", 0, s),
        ("12345_", 0, s),
        ("123%45", 0, s),
    ]
}

fn pub_err(e: Option<Error>) -> Option<sc::Error> {
    e.map(|e| match e {
        Error::Range => sc::Error::Range,
        Error::Syntax => sc::Error::Syntax,
        other => panic!("unexpected {other:?}"),
    })
}

#[test]
fn test_parse_uint32() {
    for (input, out, err) in parse_uint32_tests() {
        assert_eq!(
            internal::parse_uint(input.as_bytes(), 10, 32),
            (out as u64, err),
            "ParseUint({input:?}, 10, 32)"
        );
        let r = sc::parse_uint(input, 10, 32);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
        if let Err(e) = r {
            assert_eq!((e.func, e.num.as_slice()), ("ParseUint", input.as_bytes()));
        }
    }
}

#[test]
fn test_parse_uint64() {
    for (input, out, err) in parse_uint64_tests() {
        assert_eq!(
            internal::parse_uint(input.as_bytes(), 10, 64),
            (out, err),
            "ParseUint({input:?}, 10, 64)"
        );
        assert_eq!(
            internal::parse_uint(input.as_bytes(), 10, 0),
            (out, err),
            "ParseUint({input:?}, 10, 0)"
        );
        let r = sc::parse_uint(input, 10, 64);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
    }
}

#[test]
fn test_parse_uint64_base() {
    for (input, base, out, err) in parse_uint64_base_tests() {
        assert_eq!(
            internal::parse_uint(input.as_bytes(), base, 64),
            (out, err),
            "ParseUint({input:?}, {base}, 64)"
        );
        let r = sc::parse_uint(input, base, 64);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
    }
}

#[test]
fn test_parse_int32() {
    for (input, out, err) in parse_int32_tests() {
        assert_eq!(
            internal::parse_int(input.as_bytes(), 10, 32),
            (out as i64, err),
            "ParseInt({input:?}, 10, 32)"
        );
        let r = sc::parse_int(input, 10, 32);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
        if let Err(e) = r {
            assert_eq!((e.func, e.num.as_slice()), ("ParseInt", input.as_bytes()));
        }
    }
}

#[test]
fn test_parse_int64() {
    for (input, out, err) in parse_int64_tests() {
        assert_eq!(
            internal::parse_int(input.as_bytes(), 10, 64),
            (out, err),
            "ParseInt({input:?}, 10, 64)"
        );
        assert_eq!(
            internal::parse_int(input.as_bytes(), 10, 0),
            (out, err),
            "ParseInt({input:?}, 10, 0)"
        );
        let r = sc::parse_int(input, 10, 64);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
    }
}

#[test]
fn test_parse_int64_base() {
    for (input, base, out, err) in parse_int64_base_tests() {
        assert_eq!(
            internal::parse_int(input.as_bytes(), base, 64),
            (out, err),
            "ParseInt({input:?}, {base}, 64)"
        );
        let r = sc::parse_int(input, base, 64);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
    }
}

#[test]
fn test_atoi() {
    for (input, out, err) in parse_int64_tests() {
        assert_eq!(
            internal::atoi(input.as_bytes()),
            (out, err),
            "Atoi({input:?})"
        );
        let r = sc::atoi(input);
        assert_eq!(r.as_ref().err().map(|e| e.err), pub_err(err));
        if let Err(e) = r {
            assert_eq!((e.func, e.num.as_slice()), ("Atoi", input.as_bytes()));
        }
    }
}

#[test]
fn test_parse_bit_size_and_base() {
    let bit_size_tests = [
        (-1, Some(Error::BitSize)),
        (0, None),
        (64, None),
        (65, Some(Error::BitSize)),
    ];
    for (arg, err) in bit_size_tests {
        assert_eq!(internal::parse_int(b"0", 0, arg).1, err);
        assert_eq!(internal::parse_uint(b"0", 0, arg).1, err);
        if err.is_some() {
            assert_eq!(
                sc::parse_int("0", 0, arg).unwrap_err().err,
                sc::Error::BitSize(arg)
            );
            assert_eq!(
                sc::parse_uint("0", 0, arg).unwrap_err().to_string(),
                format!("strconv.ParseUint: parsing \"0\": invalid bit size {arg}")
            );
        }
    }
    let base_tests = [
        (-1, Some(Error::Base)),
        (0, None),
        (1, Some(Error::Base)),
        (2, None),
        (36, None),
        (37, Some(Error::Base)),
    ];
    for (arg, err) in base_tests {
        assert_eq!(internal::parse_int(b"0", arg, 0).1, err);
        assert_eq!(internal::parse_uint(b"0", arg, 0).1, err);
        if err.is_some() {
            assert_eq!(
                sc::parse_int("0", arg, 0).unwrap_err().err,
                sc::Error::Base(arg)
            );
            assert_eq!(
                sc::parse_int("0", arg, 0).unwrap_err().to_string(),
                format!("strconv.ParseInt: parsing \"0\": invalid base {arg}")
            );
        }
    }
    let _ = syn::<i64>();
}

// ---------------------------------------------------------------------------
// itoa_test.go

fn itob64tests() -> Vec<(i64, i64, &'static str)> {
    vec![
        (0, 10, "0"),
        (1, 10, "1"),
        (-1, 10, "-1"),
        (12345678, 10, "12345678"),
        (-987654321, 10, "-987654321"),
        ((1 << 31) - 1, 10, "2147483647"),
        (-(1 << 31) + 1, 10, "-2147483647"),
        (1 << 31, 10, "2147483648"),
        (-(1 << 31), 10, "-2147483648"),
        ((1 << 31) + 1, 10, "2147483649"),
        (-(1 << 31) - 1, 10, "-2147483649"),
        ((1 << 32) - 1, 10, "4294967295"),
        (-(1 << 32) + 1, 10, "-4294967295"),
        (1 << 32, 10, "4294967296"),
        (-(1 << 32), 10, "-4294967296"),
        ((1 << 32) + 1, 10, "4294967297"),
        (-(1 << 32) - 1, 10, "-4294967297"),
        (1 << 50, 10, "1125899906842624"),
        (i64::MAX, 10, "9223372036854775807"),
        (-i64::MAX, 10, "-9223372036854775807"),
        (i64::MIN, 10, "-9223372036854775808"),
        (0, 2, "0"),
        (10, 2, "1010"),
        (-1, 2, "-1"),
        (1 << 15, 2, "1000000000000000"),
        (-8, 8, "-10"),
        (0o57635436545, 8, "57635436545"),
        (1 << 24, 8, "100000000"),
        (16, 16, "10"),
        (-0x123456789abcdef, 16, "-123456789abcdef"),
        (i64::MAX, 16, "7fffffffffffffff"),
        (
            i64::MAX,
            2,
            "111111111111111111111111111111111111111111111111111111111111111",
        ),
        (
            i64::MIN,
            2,
            "-1000000000000000000000000000000000000000000000000000000000000000",
        ),
        (16, 17, "g"),
        (25, 25, "10"),
        (
            (((((17 * 35 + 24) * 35 + 21) * 35 + 34) * 35 + 12) * 35 + 24) * 35 + 32,
            35,
            "holycow",
        ),
        (
            (((((17 * 36 + 24) * 36 + 21) * 36 + 34) * 36 + 12) * 36 + 24) * 36 + 32,
            36,
            "holycow",
        ),
    ]
}

#[test]
fn test_itoa() {
    for (input, base, out) in itob64tests() {
        assert_eq!(sc::format_int(input, base), out);
        let mut x = b"abc".to_vec();
        sc::append_int(&mut x, input, base);
        assert_eq!(x, format!("abc{out}").into_bytes());
        if input >= 0 {
            assert_eq!(sc::format_uint(input as u64, base), out);
            let mut x = Vec::new();
            sc::append_uint(&mut x, input as u64, base);
            assert_eq!(x, out.as_bytes());
        }
        if base == 10 && input >= 0 {
            let mut buf = [0u8; 32];
            let i = internal::runtime_format_base10(&mut buf, input as u64);
            assert_eq!(&buf[i..], out.as_bytes());
        }
        if base == 10 {
            assert_eq!(sc::itoa(input), out);
        }
    }
}

#[test]
#[should_panic(expected = "strconv: illegal AppendInt/FormatInt base")]
fn test_itoa_illegal_base() {
    sc::format_uint(12345678, 1);
}

#[test]
fn test_uitoa() {
    let tests: [(u64, i64, &str); 6] = [
        ((1 << 63) - 1, 10, "9223372036854775807"),
        (1 << 63, 10, "9223372036854775808"),
        ((1 << 63) + 1, 10, "9223372036854775809"),
        (u64::MAX - 1, 10, "18446744073709551614"),
        (u64::MAX, 10, "18446744073709551615"),
        (
            u64::MAX,
            2,
            "1111111111111111111111111111111111111111111111111111111111111111",
        ),
    ];
    for (input, base, out) in tests {
        assert_eq!(sc::format_uint(input, base), out);
        let mut x = b"abc".to_vec();
        sc::append_uint(&mut x, input, base);
        assert_eq!(x, format!("abc{out}").into_bytes());
        if base == 10 {
            let mut buf = [0u8; 32];
            let i = internal::runtime_format_base10(&mut buf, input);
            assert_eq!(&buf[i..], out.as_bytes());
        }
    }
}

#[test]
fn test_format_uint_varlen() {
    let mut n: u64 = 0;
    let mut s = String::new();
    for d in 1..=20u64 {
        n = n * 10 + d % 10;
        s.push(char::from(b'0' + (d % 10) as u8));
        assert_eq!(sc::format_uint(n, 10), s);
    }
}

// ---------------------------------------------------------------------------
// decimal_test.go

#[test]
fn test_decimal_shift() {
    let tests: [(u64, i64, &str); 8] = [
        (0, -100, "0"),
        (0, 100, "0"),
        (1, 100, "1267650600228229401496703205376"),
        (
            1,
            -100,
            "0.0000000000000000000000000000007888609052210118054117285652827862296732064351090230047702789306640625",
        ),
        (12345678, 8, "3160493568"),
        (12345678, -8, "48225.3046875"),
        (195312, 9, "99999744"),
        (1953125, 9, "1000000000"),
    ];
    for (i, shift, out) in tests {
        let mut d = Decimal::new(i);
        d.shift(shift);
        assert_eq!(d.to_string(), out, "Decimal {i} << {shift}");
    }
}

#[test]
fn test_decimal_round() {
    let tests: [(u64, i64, &str, &str, &str, u64); 13] = [
        (0, 4, "0", "0", "0", 0),
        (12344999, 4, "12340000", "12340000", "12350000", 12340000),
        (12345000, 4, "12340000", "12340000", "12350000", 12340000),
        (12345001, 4, "12340000", "12350000", "12350000", 12350000),
        (23454999, 4, "23450000", "23450000", "23460000", 23450000),
        (23455000, 4, "23450000", "23460000", "23460000", 23460000),
        (23455001, 4, "23450000", "23460000", "23460000", 23460000),
        (99994999, 4, "99990000", "99990000", "100000000", 99990000),
        (99995000, 4, "99990000", "100000000", "100000000", 100000000),
        (99999999, 4, "99990000", "100000000", "100000000", 100000000),
        (12994999, 4, "12990000", "12990000", "13000000", 12990000),
        (12995000, 4, "12990000", "13000000", "13000000", 13000000),
        (12999999, 4, "12990000", "13000000", "13000000", 13000000),
    ];
    for (i, nd, down, round, up, int) in tests {
        let mut d = Decimal::new(i);
        d.round_down(nd);
        assert_eq!(d.to_string(), down);
        let mut d = Decimal::new(i);
        d.round(nd);
        assert_eq!(d.to_string(), round);
        assert_eq!(d.rounded_integer(), int);
        let mut d = Decimal::new(i);
        d.round_up(nd);
        assert_eq!(d.to_string(), up);
    }
}

#[test]
fn test_decimal_rounded_integer() {
    let tests: [(u64, i64, u64); 10] = [
        (0, 100, 0),
        (512, -8, 2),
        (513, -8, 2),
        (640, -8, 2),
        (641, -8, 3),
        (384, -8, 2),
        (385, -8, 2),
        (383, -8, 1),
        (1, 100, u64::MAX),
        (1000, 0, 1000),
    ];
    for (i, shift, int) in tests {
        let mut d = Decimal::new(i);
        d.shift(shift);
        assert_eq!(d.rounded_integer(), int, "Decimal {i} >> {shift}");
    }
}

// ---------------------------------------------------------------------------
// fp_test.go

fn pow2(i: i64) -> f64 {
    if i < 0 {
        return 1.0 / pow2(-i);
    }
    match i {
        0 => 1.0,
        1 => 2.0,
        _ => pow2(i / 2) * pow2(i - i / 2),
    }
}

fn myatof64(s: &str) -> Option<f64> {
    if let Some((mant, exp)) = s.split_once('p') {
        let n = sc::parse_int(mant, 10, 64).ok()?;
        let mut e = sc::atoi(exp).ok()?;
        let mut v = n as f64;
        if e <= -1000 {
            v *= pow2(-1000);
            e += 1000;
            while e < 0 {
                v /= 2.0;
                e += 1;
            }
            return Some(v);
        }
        if e >= 1000 {
            v *= pow2(1000);
            e -= 1000;
            while e > 0 {
                v *= 2.0;
                e -= 1;
            }
            return Some(v);
        }
        return Some(v * pow2(e));
    }
    sc::parse_float(s, 64).ok()
}

fn myatof32(s: &str) -> Option<f32> {
    if let Some((mant, exp)) = s.split_once('p') {
        let n = sc::atoi(mant).ok()?;
        let e = sc::atoi(exp).ok()?;
        return Some(internal::f64_to_f32(n as f64 * pow2(e)));
    }
    sc::parse_float(s, 32).ok().map(internal::f64_to_f32)
}

#[test]
fn test_fp() {
    let data = common::fixture("go-testdata/testfp.txt");
    let mut n = 0;
    for (lineno, line) in data.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let a: Vec<&str> = line.split(' ').collect();
        assert_eq!(a.len(), 4, "testfp.txt:{}", lineno + 1);
        let (v, bs) = match a[0] {
            "float64" => (myatof64(a[2]).expect("atof64"), 64),
            "float32" => (myatof32(a[2]).expect("atof32") as f64, 32),
            _ => unreachable!(),
        };
        let s = if a[1] == "%b" {
            ffmt(v, b'b', -1, bs)
        } else {
            let p: i64 = a[1]
                .trim_start_matches("%.")
                .trim_end_matches('e')
                .parse()
                .unwrap();
            ffmt(v, b'e', p, bs)
        };
        assert_eq!(s, a[3], "testfp.txt:{}: {line}", lineno + 1);
        n += 1;
    }
    assert!(n > 100);
}

#[test]
fn test_parse_float_testdata() {
    let data = common::fixture("go-testdata/atof1k.txt");
    for line in data.lines() {
        let s = line.trim();
        if s.starts_with('#') || s.is_empty() {
            continue;
        }
        internal::set_optimize(false);
        let (want, err1) = internal::parse_float(s.as_bytes(), 64);
        internal::set_optimize(true);
        let (have, err2) = internal::parse_float(s.as_bytes(), 64);
        assert_eq!(err1, None);
        assert_eq!(err2, None);
        assert_eq!(have.to_bits(), want.to_bits(), "ParseFloat({s})");
    }
}

#[test]
fn test_format_float_testdata() {
    let data = common::fixture("go-testdata/ftoa1k.txt");
    for line in data.lines() {
        let s = line.trim();
        if s.starts_with('#') || s.is_empty() {
            continue;
        }
        let f = sc::parse_float(s, 64).unwrap();
        for i in 0..19 {
            internal::set_optimize(false);
            let want = ffmt(f, b'e', i, 64);
            internal::set_optimize(true);
            let have = ffmt(f, b'e', i, 64);
            assert_eq!(have, want, "FormatFloat({s}, 'e', {i})");
        }
    }
}

// ---------------------------------------------------------------------------
// atob_test.go

#[test]
fn test_parse_bool() {
    let tests: [(&str, bool, Option<Error>); 14] = [
        ("", false, Some(Error::Syntax)),
        ("asdf", false, Some(Error::Syntax)),
        ("0", false, None),
        ("f", false, None),
        ("F", false, None),
        ("FALSE", false, None),
        ("false", false, None),
        ("False", false, None),
        ("1", true, None),
        ("t", true, None),
        ("T", true, None),
        ("TRUE", true, None),
        ("true", true, None),
        ("True", true, None),
    ];
    for (input, out, err) in tests {
        assert_eq!(
            internal::parse_bool(input.as_bytes()),
            (out, err),
            "ParseBool({input})"
        );
        match sc::parse_bool(input) {
            Ok(b) => assert_eq!((b, err), (out, None)),
            Err(e) => {
                assert_eq!(e.err, sc::Error::Syntax);
                assert_eq!(e.func, "ParseBool");
            }
        }
    }
    assert_eq!(sc::format_bool(true), "true");
    assert_eq!(sc::format_bool(false), "false");
    let mut v = b"foo ".to_vec();
    sc::append_bool(&mut v, true);
    assert_eq!(v, b"foo true");
    let mut v = b"foo ".to_vec();
    sc::append_bool(&mut v, false);
    assert_eq!(v, b"foo false");
}

// ---------------------------------------------------------------------------
// atoc_test.go / ctoa_test.go

fn same_complex(a: (f64, f64), b: (f64, f64)) -> bool {
    let nan = |c: (f64, f64)| c.0.is_nan() || c.1.is_nan();
    (nan(a) && nan(b)) || a == b
}

// Go: internal/strconv/atoc_test.go:TestParseComplexIncorrectBitSize
#[test]
fn test_parse_complex_incorrect_bit_size() {
    let s = "1.5e308+1.0e307i";
    for bit_size in [0, 10, 100, 256] {
        let c = sc::parse_complex(s, bit_size)
            .unwrap_or_else(|e| panic!("ParseComplex({s:?}, {bit_size}) gave error {e}"));
        assert_eq!(c, (1.5e308, 1.0e307), "ParseComplex({s:?}, {bit_size})");
    }
}

#[test]
fn test_parse_complex() {
    let inf = f64::INFINITY;
    let ninf = f64::NEG_INFINITY;
    let nan = f64::NAN;
    let s = Some(Error::Syntax);
    let r = Some(Error::Range);
    let p2 = |e: i32| 2f64.powi(e);
    let h103 = (0x103 as f64) * p2(-4); // 0x10.3
    let tests: Vec<(&str, (f64, f64), Option<Error>)> = vec![
        ("", (0.0, 0.0), s),
        (" ", (0.0, 0.0), s),
        ("(", (0.0, 0.0), s),
        (")", (0.0, 0.0), s),
        ("i", (0.0, 0.0), s),
        ("+i", (0.0, 0.0), s),
        ("-i", (0.0, 0.0), s),
        ("1I", (0.0, 0.0), s),
        ("10  + 5i", (0.0, 0.0), s),
        ("3+", (0.0, 0.0), s),
        ("3+5", (0.0, 0.0), s),
        ("3+5+5i", (0.0, 0.0), s),
        ("()", (0.0, 0.0), s),
        ("(i)", (0.0, 0.0), s),
        ("(0)", (0.0, 0.0), None),
        ("(1i)", (0.0, 1.0), None),
        ("(3.0+5.5i)", (3.0, 5.5), None),
        ("(1)+1i", (0.0, 0.0), s),
        ("(3.0+5.5i", (0.0, 0.0), s),
        ("3.0+5.5i)", (0.0, 0.0), s),
        ("NaN", (nan, 0.0), None),
        ("NANi", (0.0, nan), None),
        ("nan+nAni", (nan, nan), None),
        ("+NaN", (0.0, 0.0), s),
        ("-NaN", (0.0, 0.0), s),
        ("NaN-NaNi", (0.0, 0.0), s),
        ("Inf", (inf, 0.0), None),
        ("+inf", (inf, 0.0), None),
        ("-inf", (ninf, 0.0), None),
        ("Infinity", (inf, 0.0), None),
        ("+INFINITY", (inf, 0.0), None),
        ("-infinity", (ninf, 0.0), None),
        ("+infi", (0.0, inf), None),
        ("0-infinityi", (0.0, ninf), None),
        ("Inf+Infi", (inf, inf), None),
        ("+Inf-Infi", (inf, ninf), None),
        ("-Infinity+Infi", (ninf, inf), None),
        ("inf-inf", (0.0, 0.0), s),
        ("0", (0.0, 0.0), None),
        ("0i", (0.0, 0.0), None),
        ("-0.0i", (0.0, 0.0), None),
        ("0+0.0i", (0.0, 0.0), None),
        ("0e+0i", (0.0, 0.0), None),
        ("0e-0+0i", (0.0, 0.0), None),
        ("-0.0-0.0i", (0.0, 0.0), None),
        ("0e+012345", (0.0, 0.0), None),
        ("0x0p+012345i", (0.0, 0.0), None),
        ("0x0.00p-012345i", (0.0, 0.0), None),
        ("+0e-0+0e-0i", (0.0, 0.0), None),
        ("0e+0+0e+0i", (0.0, 0.0), None),
        ("-0e+0-0e+0i", (0.0, 0.0), None),
        ("0.1", (0.1, 0.0), None),
        ("0.1i", (0.0, 0.1), None),
        ("0.123", (0.123, 0.0), None),
        ("0.123i", (0.0, 0.123), None),
        ("0.123+0.123i", (0.123, 0.123), None),
        ("99", (99.0, 0.0), None),
        ("+99", (99.0, 0.0), None),
        ("-99", (-99.0, 0.0), None),
        ("+1i", (0.0, 1.0), None),
        ("-1i", (0.0, -1.0), None),
        ("+3+1i", (3.0, 1.0), None),
        ("30+3i", (30.0, 3.0), None),
        ("+3e+3-3e+3i", (3e3, -3e3), None),
        ("+3e+3+3e+3i", (3e3, 3e3), None),
        ("+3e+3+3e+3i+", (0.0, 0.0), s),
        ("0.1_2_3", (0.123, 0.0), None),
        ("+0x_3p3i", (0.0, 24.0), None),
        ("0_0+0x_0p0i", (0.0, 0.0), None),
        ("0x_10.3p-8+0x3p3i", (h103 * p2(-8), 24.0), None),
        ("+0x_1_0.3p-8+0x_3_0p3i", (h103 * p2(-8), 384.0), None),
        ("0x1_0.3p+8-0x_3p3i", (h103 * p2(8), -24.0), None),
        ("0x10.3p-8+0x3p3i", (h103 * p2(-8), 24.0), None),
        ("+0x10.3p-8+0x3p3i", (h103 * p2(-8), 24.0), None),
        ("0x10.3p+8-0x3p3i", (h103 * p2(8), -24.0), None),
        ("0x1p0", (1.0, 0.0), None),
        ("0x1p1", (2.0, 0.0), None),
        ("0x1p-1", (0.5, 0.0), None),
        ("0x1ep-1", (15.0, 0.0), None),
        ("-0x1ep-1", (-15.0, 0.0), None),
        ("-0x2p3", (-16.0, 0.0), None),
        ("0x1e2", (0.0, 0.0), s),
        ("1p2", (0.0, 0.0), s),
        ("0x1e2i", (0.0, 0.0), s),
        ("+0x1p1024", (inf, 0.0), r),
        ("-0x1p1024", (ninf, 0.0), r),
        ("+0x1p1024i", (0.0, inf), r),
        ("-0x1p1024i", (0.0, ninf), r),
        ("+0x1p1024+0x1p1024i", (inf, inf), r),
        ("+0x1p1024-0x1p1024i", (inf, ninf), r),
        ("-0x1p1024+0x1p1024i", (ninf, inf), r),
        ("-0x1p1024-0x1p1024i", (ninf, ninf), r),
        (
            "+0x1.fffffffffffff7fffp1023+0x1.fffffffffffff7fffp1023i",
            (f64::MAX, f64::MAX),
            None,
        ),
        (
            "+0x1.fffffffffffff7fffp1023-0x1.fffffffffffff7fffp1023i",
            (f64::MAX, -f64::MAX),
            None,
        ),
        (
            "-0x1.fffffffffffff7fffp1023+0x1.fffffffffffff7fffp1023i",
            (-f64::MAX, f64::MAX),
            None,
        ),
        (
            "-0x1.fffffffffffff7fffp1023-0x1.fffffffffffff7fffp1023i",
            (-f64::MAX, -f64::MAX),
            None,
        ),
        ("+0x1.fffffffffffff8p1023", (inf, 0.0), r),
        ("-0x1fffffffffffff.8p+971", (ninf, 0.0), r),
        ("+0x1.fffffffffffff8p1023i", (0.0, inf), r),
        ("-0x1fffffffffffff.8p+971i", (0.0, ninf), r),
        (
            "+0x1.fffffffffffff8p1023+0x1.fffffffffffff8p1023i",
            (inf, inf),
            r,
        ),
        (
            "+0x1.fffffffffffff8p1023-0x1.fffffffffffff8p1023i",
            (inf, ninf),
            r,
        ),
        (
            "-0x1fffffffffffff.8p+971+0x1fffffffffffff.8p+971i",
            (ninf, inf),
            r,
        ),
        (
            "-0x1fffffffffffff8p+967-0x1fffffffffffff8p+967i",
            (ninf, ninf),
            r,
        ),
        ("1e308+1e308i", (1e308, 1e308), None),
        ("2e308+2e308i", (inf, inf), r),
        ("1e309+1e309i", (inf, inf), r),
        ("0x1p1025+0x1p1025i", (inf, inf), r),
        ("2e308", (inf, 0.0), r),
        ("1e309", (inf, 0.0), r),
        ("0x1p1025", (inf, 0.0), r),
        ("2e308i", (0.0, inf), r),
        ("1e309i", (0.0, inf), r),
        ("0x1p1025i", (0.0, inf), r),
        ("+1e310+1e310i", (inf, inf), r),
        ("+1e310-1e310i", (inf, ninf), r),
        ("-1e310+1e310i", (ninf, inf), r),
        ("-1e310-1e310i", (ninf, ninf), r),
        ("1e-4294967296", (0.0, 0.0), None),
        ("1e-4294967296i", (0.0, 0.0), None),
        ("1e-4294967296+1i", (0.0, 1.0), None),
        ("1+1e-4294967296i", (1.0, 0.0), None),
        ("1e-4294967296+1e-4294967296i", (0.0, 0.0), None),
        ("1e+4294967296", (inf, 0.0), r),
        ("1e+4294967296i", (0.0, inf), r),
        ("1e+4294967296+1e+4294967296i", (inf, inf), r),
        ("1e+4294967296-1e+4294967296i", (inf, ninf), r),
    ];
    for (input, out, err) in tests {
        let (c, e) = internal::parse_complex(input.as_bytes(), 128);
        assert!(
            same_complex(c, out) && e == err,
            "ParseComplex({input}, 128) = {c:?}, {e:?}"
        );
        match sc::parse_complex(input, 128) {
            Ok(v) => assert!(err.is_none() && same_complex(v, out)),
            Err(ne) => {
                assert_eq!(ne.func, "ParseComplex");
                assert_eq!(Some(ne.err), pub_err(err));
            }
        }
        if f32of(out.0).to_bits() == out.0.to_bits() && f32of(out.1).to_bits() == out.1.to_bits()
            || (out.0 == f32of(out.0) && out.1 == f32of(out.1))
        {
            let (c, e) = internal::parse_complex(input.as_bytes(), 64);
            let c64 = (f32of(c.0), f32of(c.1));
            assert!(
                same_complex(c64, out) && e == err,
                "ParseComplex({input}, 64) = {c:?}, {e:?}"
            );
        }
    }
    for bs in [0, 10, 100, 256] {
        assert_eq!(
            sc::parse_complex("1.5e308+1.0e307i", bs),
            Ok((1.5e308, 1.0e307))
        );
    }
}

#[test]
fn test_format_complex() {
    let tests: [((f64, f64), u8, i64, i64, &str); 9] = [
        ((1.0, 2.0), b'g', -1, 128, "(1+2i)"),
        ((3.0, -4.0), b'g', -1, 128, "(3-4i)"),
        ((-5.0, 6.0), b'g', -1, 128, "(-5+6i)"),
        ((-7.0, -8.0), b'g', -1, 128, "(-7-8i)"),
        ((3.14159, 0.00123), b'e', 3, 128, "(3.142e+00+1.230e-03i)"),
        ((3.14159, 0.00123), b'f', 3, 128, "(3.142+0.001i)"),
        ((3.14159, 0.00123), b'g', 3, 128, "(3.14+0.00123i)"),
        (
            (1.2345678901234567, 9.876543210987654),
            b'f',
            -1,
            128,
            "(1.2345678901234567+9.876543210987654i)",
        ),
        (
            (1.2345678901234567, 9.876543210987654),
            b'f',
            -1,
            64,
            "(1.2345679+9.876543i)",
        ),
    ];
    for (c, fmt, prec, bs, out) in tests {
        assert_eq!(sc::format_complex(c, fmt, prec, bs), out);
    }
}

#[test]
#[should_panic(expected = "invalid bitSize")]
fn test_format_complex_invalid_bit_size() {
    let _ = sc::format_complex((1.0, 2.0), b'g', -1, 100);
}

// ---------------------------------------------------------------------------
// quote_test.go

struct QuoteTest {
    input: &'static [u8],
    out: &'static str,
    ascii: &'static str,
    graphic: &'static str,
}

fn quotetests() -> Vec<QuoteTest> {
    vec![
        QuoteTest {
            input: b"\x07\x08\x0c\r\n\t\x0b",
            out: r#""\a\b\f\r\n\t\v""#,
            ascii: r#""\a\b\f\r\n\t\v""#,
            graphic: r#""\a\b\f\r\n\t\v""#,
        },
        QuoteTest {
            input: b"\\",
            out: r#""\\""#,
            ascii: r#""\\""#,
            graphic: r#""\\""#,
        },
        QuoteTest {
            input: b"abc\xffdef",
            out: r#""abc\xffdef""#,
            ascii: r#""abc\xffdef""#,
            graphic: r#""abc\xffdef""#,
        },
        QuoteTest {
            input: "\u{263a}".as_bytes(),
            out: "\"\u{263a}\"",
            ascii: r#""\u263a""#,
            graphic: "\"\u{263a}\"",
        },
        QuoteTest {
            input: "\u{10ffff}".as_bytes(),
            out: r#""\U0010ffff""#,
            ascii: r#""\U0010ffff""#,
            graphic: r#""\U0010ffff""#,
        },
        QuoteTest {
            input: b"\x04",
            out: r#""\x04""#,
            ascii: r#""\x04""#,
            graphic: r#""\x04""#,
        },
        // Some non-printable but graphic runes. Final column is double-quoted.
        QuoteTest {
            input: "!\u{00a0}!\u{2000}!\u{3000}!".as_bytes(),
            out: r#""!\u00a0!\u2000!\u3000!""#,
            ascii: r#""!\u00a0!\u2000!\u3000!""#,
            graphic: "\"!\u{00a0}!\u{2000}!\u{3000}!\"",
        },
        QuoteTest {
            input: b"\x7f",
            out: r#""\x7f""#,
            ascii: r#""\x7f""#,
            graphic: r#""\x7f""#,
        },
    ]
}

#[test]
fn test_quote() {
    for tt in quotetests() {
        assert_eq!(sc::quote(tt.input), tt.out);
        let mut out = b"abc".to_vec();
        sc::append_quote(&mut out, tt.input);
        assert_eq!(out, format!("abc{}", tt.out).into_bytes());

        assert_eq!(sc::quote_to_ascii(tt.input), tt.ascii);
        let mut out = b"abc".to_vec();
        sc::append_quote_to_ascii(&mut out, tt.input);
        assert_eq!(out, format!("abc{}", tt.ascii).into_bytes());

        assert_eq!(sc::quote_to_graphic(tt.input), tt.graphic);
        let mut out = b"abc".to_vec();
        sc::append_quote_to_graphic(&mut out, tt.input);
        assert_eq!(out, format!("abc{}", tt.graphic).into_bytes());
    }
}

#[test]
fn test_quote_rune() {
    let tests: [(i32, &str, &str, &str); 13] = [
        ('a' as i32, "'a'", "'a'", "'a'"),
        (7, r"'\a'", r"'\a'", r"'\a'"),
        ('\\' as i32, r"'\\'", r"'\\'", r"'\\'"),
        (0xFF, "'\u{ff}'", r"'\u00ff'", "'\u{ff}'"),
        (0x263a, "'\u{263a}'", r"'\u263a'", "'\u{263a}'"),
        (0xdead, "'\u{fffd}'", r"'\ufffd'", "'\u{fffd}'"),
        (0xfffd, "'\u{fffd}'", r"'\ufffd'", "'\u{fffd}'"),
        (
            0x0010ffff,
            r"'\U0010ffff'",
            r"'\U0010ffff'",
            r"'\U0010ffff'",
        ),
        (0x0010ffff + 1, "'\u{fffd}'", r"'\ufffd'", "'\u{fffd}'"),
        (0x04, r"'\x04'", r"'\x04'", r"'\x04'"),
        (0x00a0, r"'\u00a0'", r"'\u00a0'", "'\u{00a0}'"),
        (0x2000, r"'\u2000'", r"'\u2000'", "'\u{2000}'"),
        (0x3000, r"'\u3000'", r"'\u3000'", "'\u{3000}'"),
    ];
    for (r, out, ascii, graphic) in tests {
        assert_eq!(sc::quote_rune(r), out);
        let mut v = b"abc".to_vec();
        sc::append_quote_rune(&mut v, r);
        assert_eq!(v, format!("abc{out}").into_bytes());
        assert_eq!(sc::quote_rune_to_ascii(r), ascii);
        let mut v = b"abc".to_vec();
        sc::append_quote_rune_to_ascii(&mut v, r);
        assert_eq!(v, format!("abc{ascii}").into_bytes());
        assert_eq!(sc::quote_rune_to_graphic(r), graphic);
        let mut v = b"abc".to_vec();
        sc::append_quote_rune_to_graphic(&mut v, r);
        assert_eq!(v, format!("abc{graphic}").into_bytes());
    }
}

#[test]
fn test_can_backquote() {
    for c in 0u8..=31 {
        assert_eq!(sc::can_backquote([c]), c == 9, "{c}");
    }
    let tests: [(&[u8], bool); 12] = [
        (b"`", false),
        (b"\x7f", false),
        (br##"' !"#$%&'()*+,-./:;<=>?@[\]^_{|}~"##, true),
        (b"0123456789", true),
        (b"ABCDEFGHIJKLMNOPQRSTUVWXYZ", true),
        (b"abcdefghijklmnopqrstuvwxyz", true),
        ("\u{263a}".as_bytes(), true),
        (b"\x80", false),
        (b"a\xe0\xa0z", false),
        ("\u{feff}abc".as_bytes(), false),
        ("a\u{feff}z".as_bytes(), false),
        (b"", true),
    ];
    for (s, out) in tests {
        assert_eq!(
            sc::can_backquote(s),
            out,
            "{:?}",
            String::from_utf8_lossy(s)
        );
    }
}

fn unquotetests() -> Vec<(&'static [u8], &'static [u8])> {
    vec![
        (br#""""#, b""),
        (br#""a""#, b"a"),
        (br#""abc""#, b"abc"),
        ("\"\u{263a}\"".as_bytes(), "\u{263a}".as_bytes()),
        (br#""hello world""#, b"hello world"),
        (br#""\xFF""#, b"\xFF"),
        (br#""\377""#, b"\xff"),
        (br#""\u1234""#, "\u{1234}".as_bytes()),
        (br#""\U00010111""#, "\u{10111}".as_bytes()),
        (br#""\U0001011111""#, "\u{10111}11".as_bytes()),
        (br#""\a\b\f\n\r\t\v\\\"""#, b"\x07\x08\x0c\n\r\t\x0b\\\""),
        (br#""'""#, b"'"),
        (br"'a'", b"a"),
        ("'\u{2639}'".as_bytes(), "\u{2639}".as_bytes()),
        (br"'\a'", b"\x07"),
        (br"'\x10'", b"\x10"),
        (br"'\377'", b"\xff"),
        (br"'\u1234'", "\u{1234}".as_bytes()),
        (br"'\U00010111'", "\u{10111}".as_bytes()),
        (br"'\t'", b"\t"),
        (br"' '", b" "),
        (br"'\''", b"'"),
        (br#"'"'"#, b"\""),
        (b"``", b""),
        (b"`a`", b"a"),
        (b"`abc`", b"abc"),
        ("`\u{263a}`".as_bytes(), "\u{263a}".as_bytes()),
        (b"`hello world`", b"hello world"),
        (b"`\\xFF`", br"\xFF"),
        (b"`\\377`", br"\377"),
        (b"`\\`", br"\"),
        (b"`\n`", b"\n"),
        (b"`\t`", b"\t"),
        (b"` `", b" "),
        (b"`a\rb`", b"ab"),
    ]
}

fn misquoted() -> Vec<&'static [u8]> {
    vec![
        b"",
        br#"""#,
        br#""a"#,
        br#""'"#,
        br#"b""#,
        br#""\""#,
        br#""\9""#,
        br#""\19""#,
        br#""\129""#,
        br"'\'",
        br"'\9'",
        br"'\19'",
        br"'\129'",
        br"'ab'",
        br#""\x1!""#,
        br#""\U12345678""#,
        br#""\z""#,
        b"`",
        b"`xxx",
        b"``x\r",
        b"`\"",
        br#""\'""#,
        br#"'\"'"#,
        b"\"\n\"",
        b"\"\\n\n\"",
        b"'\n'",
        br#""\udead""#,
        br#""\ud83d\ude4f""#,
    ]
}

fn test_unquote_one(input: &[u8], want: &[u8], want_err: Option<sc::Error>) {
    // Test Unquote.
    let r = sc::unquote(input);
    let (got, got_err) = match &r {
        Ok(v) => (v.as_slice(), None),
        Err(e) => (&b""[..], Some(*e)),
    };
    assert_eq!(
        (got, got_err),
        (want, want_err),
        "Unquote({:?})",
        String::from_utf8_lossy(input)
    );

    // Test QuotedPrefix.
    let mut want = want.to_vec();
    let mut want_err = want_err;
    if got_err.is_none() {
        want = input.to_vec();
    }
    let mut suffix: Vec<u8> = b"\n\r\\\"`'".to_vec();
    if !input.is_empty() {
        suffix.retain(|&c| c != input[0]);
    }
    let mut inp = input.to_vec();
    inp.extend_from_slice(&suffix);
    let r = sc::quoted_prefix(&inp);
    let (got, got_err) = match &r {
        Ok(v) => (v.to_vec(), None),
        Err(e) => (Vec::new(), Some(*e)),
    };
    if got_err.is_none() && want_err.is_some() {
        // original input had trailing junk, reparse with only valid prefix
        want_err = sc::unquote(&got).err();
        want = got.clone();
    }
    assert_eq!(
        (got, got_err),
        (want, want_err),
        "QuotedPrefix({:?})",
        String::from_utf8_lossy(&inp)
    );
}

#[test]
fn test_unquote() {
    for (input, out) in unquotetests() {
        test_unquote_one(input, out, None);
    }
    for tt in quotetests() {
        test_unquote_one(tt.out.as_bytes(), tt.input, None);
    }
    for s in misquoted() {
        test_unquote_one(s, b"", Some(sc::Error::Syntax));
    }
}

#[test]
fn test_unquote_invalid_utf8() {
    let tests: [(&[u8], &[u8], Option<sc::Error>); 5] = [
        (br#""foo""#, b"foo", None),
        (br#""foo"#, b"", Some(sc::Error::Syntax)),
        (b"\"\xc0\"", b"\xef\xbf\xbd", None),
        (b"\"a\xc0\"", b"a\xef\xbf\xbd", None),
        (b"\"\\t\xc0\"", b"\t\xef\xbf\xbd", None),
    ];
    for (input, want, err) in tests {
        test_unquote_one(input, want, err);
    }
}

// ---------------------------------------------------------------------------
// strconv_test.go / number_test.go

#[test]
fn test_error_prefixes() {
    assert_eq!(sc::atoi("INVALID").unwrap_err().func, "Atoi");
    assert_eq!(sc::parse_bool("INVALID").unwrap_err().func, "ParseBool");
    assert_eq!(
        sc::parse_float("INVALID", 64).unwrap_err().func,
        "ParseFloat"
    );
    assert_eq!(
        sc::parse_int("INVALID", 10, 64).unwrap_err().func,
        "ParseInt"
    );
    assert_eq!(
        sc::parse_uint("INVALID", 10, 64).unwrap_err().func,
        "ParseUint"
    );
}

#[test]
fn test_num_error() {
    let cases: [(&[u8], &str); 3] = [
        (b"0", r#"strconv.ParseFloat: parsing "0": invalid syntax"#),
        (b"`", "strconv.ParseFloat: parsing \"`\": invalid syntax"),
        (
            b"1\x00.2",
            r#"strconv.ParseFloat: parsing "1\x00.2": invalid syntax"#,
        ),
    ];
    for (num, want) in cases {
        let e = sc::NumError {
            func: "ParseFloat",
            num: num.to_vec(),
            err: sc::Error::Syntax,
        };
        assert_eq!(e.to_string(), want);
        use std::error::Error as _;
        assert!(e.source().is_some());
    }
}
