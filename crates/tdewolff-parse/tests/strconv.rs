//! Differential tests for the strconv package against
//! tests/fixtures/strconv.txt, plus the upstream test tables ported literally.

#![allow(clippy::excessive_precision, clippy::type_complexity)]
mod common;
use common::*;

use std::collections::HashMap;

use tdewolff_parse::GoBytes;
use tdewolff_parse::css::hsl2rgb;
use tdewolff_parse::gomath::pow10;
use tdewolff_parse::strconv::*;

fn bits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

fn fb(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn eq(what: &str, key: &str, want: String, got: String) -> Result<(), String> {
    if want == got {
        Ok(())
    } else {
        Err(format!("{} {}: want {} got {}", what, key, want, got))
    }
}

#[test]
fn strconv_fixtures() {
    check_strconv(&records("strconv.txt"));
}

/// Randomized records from the oracle's `fnfuzz` mode (checked-in small set,
/// or `TDEWOLFF_PARSE_FNFUZZ=<dir>` for a large one).
#[test]
fn strconv_fnfuzz() {
    check_strconv(&fnfuzz_records("strconv.txt"));
}

fn check_strconv(recs: &[Vec<String>]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    check_all(recs, |r| {
        *counts.entry(r[0].clone()).or_default() += 1;
        let op = r[0].as_str();
        match op {
            "parsefloat" | "parseint" | "parseuint" | "parsedecimal" | "parsenumber"
            | "parsenumber2" | "parsenumber3" => {
                let input = unhex_opt(&r[1]).unwrap_or_default();
                let key = format!("{:?}", String::from_utf8_lossy(&input));
                let want = r[2..].join(" ");
                let got = match op {
                    "parsefloat" => {
                        let (f, n) = parse_float(&input[..]);
                        format!("{} {}", fb(f), n)
                    }
                    "parseint" => {
                        let (v, n) = parse_int(&input[..]);
                        format!("{} {}", v, n)
                    }
                    "parseuint" => {
                        let (v, n) = parse_uint(&input[..]);
                        format!("{} {}", v, n)
                    }
                    "parsedecimal" => {
                        let (f, n) = parse_decimal(&input[..]);
                        format!("{} {}", fb(f), n)
                    }
                    "parsenumber" => {
                        let (num, dec, n) = parse_number(&input[..], '.' as i32, ',' as i32);
                        format!("{} {} {}", num, dec, n)
                    }
                    "parsenumber2" => {
                        let (num, dec, n) = parse_number(&input[..], ',' as i32, '.' as i32);
                        format!("{} {} {}", num, dec, n)
                    }
                    _ => {
                        let (num, dec, n) = parse_number(&input[..], 0x00b7, 0xfffd);
                        format!("{} {} {}", num, dec, n)
                    }
                };
                eq(op, &key, want, got)
            }
            "appendfloat" => {
                let f = bits(&r[1]);
                let prec: isize = r[2].parse().unwrap();
                let (b, ok) = append_float(GoBytes::empty(), f, prec);
                eq(
                    op,
                    &format!("{:e} prec {}", f, prec),
                    format!("{} {}", r[3], r[4]),
                    format!("{} {}", hx(&b), ok as u8),
                )
            }
            "appenddecimal" => {
                let f = bits(&r[1]);
                let dec: isize = r[2].parse().unwrap();
                let b = append_decimal(GoBytes::nil(), f, dec);
                eq(op, &format!("{:e} dec {}", f, dec), r[3].clone(), hx(&b))
            }
            "appendint" => {
                let v: i64 = r[1].parse().unwrap();
                eq(op, &r[1], r[2].clone(), hx(&append_int(GoBytes::nil(), v)))
            }
            "appendint_cap" => {
                let v: i64 = r[1].parse().unwrap();
                eq(
                    op,
                    &r[1],
                    r[2].clone(),
                    hx(&append_int(GoBytes::make(3, 10), v)),
                )
            }
            "lenint" => {
                let v: i64 = r[1].parse().unwrap();
                eq(op, &r[1], r[2].clone(), len_int(v).to_string())
            }
            "appendnumber" => {
                let v: i64 = r[1].parse().unwrap();
                let dec: isize = r[2].parse().unwrap();
                let gs: isize = r[3].parse().unwrap();
                let gsym: i32 = r[4].parse().unwrap();
                let dsym: i32 = r[5].parse().unwrap();
                let b = append_number(GoBytes::make(0, 4), v, dec, gs, gsym, dsym);
                eq(op, &r[1..6].join(" "), r[6].clone(), hx(&b))
            }
            "pow10" => {
                let n: i64 = r[1].parse().unwrap();
                eq(op, &r[1], r[2].clone(), fb(pow10(n)))
            }
            "hsl" => {
                let (h, s, l) = (bits(&r[1]), bits(&r[2]), bits(&r[3]));
                let (rr, g, b) = hsl2rgb(h, s, l);
                eq(
                    op,
                    &format!("{} {} {}", h, s, l),
                    r[4..7].join(" "),
                    format!("{} {} {}", fb(rr), fb(g), fb(b)),
                )
            }
            _ => Err(format!("unknown op {}", op)),
        }
    });
    let mut c: Vec<_> = counts.into_iter().collect();
    c.sort();
    eprintln!("strconv ops: {:?}", c);
}

// Upstream tables (strconv/*_test.go, css/util_test.go), ported literally.

#[test]
fn upstream_parse_float() {
    let tests: &[(&str, f64)] = &[
        ("5", 5.0),
        ("5.1", 5.1),
        ("-5.1", -5.1),
        ("5.1e-2", 5.1e-2),
        ("5.1e+2", 5.1e+2),
        ("0.0e1", 0.0e1),
        ("18446744073709551620", 18446744073709551620.0),
        ("1e23", 1e23),
    ];
    for &(s, want) in tests {
        let (f, n) = parse_float(s.as_bytes());
        assert_eq!(n, s.len(), "{}", s);
        assert_eq!(f, want, "{}", s);
    }
    let errs: &[(&str, usize, f64)] = &[
        ("e1", 0, 0.0),
        (".", 0, 0.0),
        ("1e", 1, 1.0),
        ("1e+", 1, 1.0),
        ("1e+1", 4, 10.0),
    ];
    for &(s, n0, want) in errs {
        let (f, n) = parse_float(s.as_bytes());
        assert_eq!(n, n0, "{}", s);
        assert_eq!(f, want, "{}", s);
    }
}

#[test]
fn upstream_append_float() {
    let tests: &[(f64, isize, &str)] = &[
        (0.0, 6, "0"),
        (1.0, 6, "1"),
        (9.0, 6, "9"),
        (9.99999, 6, "9.99999"),
        (123.0, 6, "123"),
        (0.123456, 6, ".123456"),
        (0.066, 6, ".066"),
        (0.0066, 6, ".0066"),
        (12e2, 6, "1200"),
        (12e3, 6, "12e3"),
        (0.1, 6, ".1"),
        (0.001, 6, ".001"),
        (0.0001, 6, "1e-4"),
        (-1.0, 6, "-1"),
        (-123.0, 6, "-123"),
        (-123.456, 6, "-123.456"),
        (-12e3, 6, "-12e3"),
        (-0.1, 6, "-.1"),
        (-0.0001, 6, "-1e-4"),
        (0.000100009, 10, "100009e-9"),
        (0.0001000009, 10, "1.000009e-4"),
        (1e18, 0, "1e18"),
        (1e1, 0, "10"),
        (1e2, 1, "100"),
        (1e3, 2, "1e3"),
        (1e10, -1, "1e10"),
        (1e15, -1, "1e15"),
        (1e-5, 6, "1e-5"),
        (f64::NAN, 0, ""),
        (f64::INFINITY, 0, ""),
        (f64::NEG_INFINITY, 0, ""),
        (0.0, 19, "0"),
        (0.000923361977200859392, -1, "9.23361977200859392e-4"),
        (1234.0, 2, "1.23e3"),
        (12345.0, 2, "1.23e4"),
        (12.345, 2, "12.3"),
        (12.345, 3, "12.34"),
    ];
    for &(f, prec, want) in tests {
        let (b, _) = append_float(GoBytes::empty(), f, prec);
        assert_eq!(b.to_vec(), want.as_bytes(), "{} {}", f, prec);
    }
    let b = GoBytes::make(0, 22);
    let _ = append_float(b.clone(), 12.34, -1);
    assert_eq!(b.slice_to(5).to_vec(), b"12.34", "in buffer");
}

#[test]
fn upstream_ints() {
    let tests: &[(&str, i64)] = &[
        ("5", 5),
        ("99", 99),
        ("999", 999),
        ("-5", -5),
        ("+5", 5),
        ("9223372036854775807", 9223372036854775807),
        ("-9223372036854775807", -9223372036854775807),
        ("-9223372036854775808", i64::MIN),
    ];
    for &(s, want) in tests {
        assert_eq!(parse_int(s.as_bytes()), (want, s.len()), "{}", s);
    }
    for s in [
        "a",
        "+",
        "9223372036854775808",
        "-9223372036854775809",
        "18446744073709551620",
    ] {
        assert_eq!(parse_int(s.as_bytes()), (0, 0), "{}", s);
    }
    let tests: &[(i64, &str)] = &[
        (0, "0"),
        (5, "5"),
        (99, "99"),
        (999, "999"),
        (-5, "-5"),
        (9223372036854775807, "9223372036854775807"),
        (-9223372036854775807, "-9223372036854775807"),
        (i64::MIN, "-9223372036854775808"),
    ];
    for &(v, want) in tests {
        assert_eq!(append_int(GoBytes::nil(), v).to_vec(), want.as_bytes());
    }
    for (s, want) in [
        ("5", 5u64),
        ("99", 99),
        ("999", 999),
        ("18446744073709551615", u64::MAX),
    ] {
        assert_eq!(parse_uint(s.as_bytes()), (want, s.len()));
    }
    for s in ["a", "18446744073709551616", "-1"] {
        assert_eq!(parse_uint(s.as_bytes()), (0, 0), "{}", s);
    }
    let lens: &[(i64, usize)] = &[
        (0, 1),
        (1, 1),
        (10, 2),
        (99, 2),
        (9223372036854775807, 19),
        (i64::MIN, 20),
        (100, 3),
        (1000, 4),
        (10000, 5),
        (100000, 6),
        (1000000, 7),
        (10000000, 8),
        (100000000, 9),
        (1000000000, 10),
        (10000000000, 11),
        (100000000000, 12),
        (1000000000000, 13),
        (10000000000000, 14),
        (100000000000000, 15),
        (1000000000000000, 16),
        (10000000000000000, 17),
        (100000000000000000, 18),
        (1000000000000000000, 19),
    ];
    for &(v, want) in lens {
        assert_eq!(len_int(v), want, "{}", v);
    }
}

#[test]
fn upstream_decimal() {
    let tests: &[(&str, f64)] = &[
        ("5", 5.0),
        ("5.1", 5.1),
        ("0.0000000000000000000000000005", 5e-28),
        ("18446744073709551620", 18446744073709551620.0),
        ("1000000000000000000000000.0000", 1e24),
        ("1000000000000000000000000000000000000000000", 1e42),
    ];
    for &(s, want) in tests {
        let (f, n) = parse_decimal(s.as_bytes());
        assert_eq!(n, s.len());
        // test.Float: relative tolerance
        assert!(((f - want) / want).abs() < 1e-6, "{}: {} vs {}", s, f, want);
    }
    let errs: &[(&str, usize, f64)] = &[
        ("+1", 0, 0.0),
        ("-1", 2, -1.0),
        (".", 0, 0.0),
        ("1e1", 1, 1.0),
    ];
    for &(s, n0, want) in errs {
        assert_eq!(parse_decimal(s.as_bytes()), (want, n0), "{}", s);
    }
    let tests: &[(f64, isize, &str)] = &[
        (0.0, 0, "0"),
        (1.0, 2, "1"),
        (-1.0, 2, "-1"),
        (1.2, 2, "1.2"),
        (1.23, 2, "1.23"),
        (1.234, 2, "1.23"),
        (1.235, 2, "1.24"),
        (0.1, 2, "0.1"),
        (0.01, 2, "0.01"),
        (0.001, 2, "0"),
        (0.005, 2, "0.01"),
        (-75.8077501, 6, "-75.80775"),
    ];
    for &(f, dec, want) in tests {
        assert_eq!(
            append_decimal(GoBytes::nil(), f, dec).to_vec(),
            want.as_bytes(),
            "{}",
            f
        );
    }
}

#[test]
fn upstream_number() {
    let tests: &[(&str, i64, isize, usize)] = &[
        ("5", 5, 0, 1),
        ("-5", -5, 0, 2),
        ("5,0", 50, 1, 3),
        ("5,0a", 50, 1, 3),
        ("-1000,00", -100000, 2, 8),
        ("9223372036854775807", 9223372036854775807, 0, 19),
        ("-9223372036854775807", -9223372036854775807, 0, 20),
        ("-9223372036854775808", i64::MIN, 0, 20),
        ("92233720368547758070", 9223372036854775807, 0, 19),
        ("-92233720368547758080", i64::MIN, 0, 20),
    ];
    for &(s, num, dec, n) in tests {
        assert_eq!(
            parse_number(s.as_bytes(), '.' as i32, ',' as i32),
            (num, dec, n),
            "{}",
            s
        );
    }
    let tests: &[(i64, isize, &str)] = &[
        (0, 0, "0"),
        (0, -1, "0"),
        (0, 2, "0,00"),
        (1, 2, "0,01"),
        (-1, 2, "-0,01"),
        (100, 2, "1,00"),
        (-100, 2, "-1,00"),
        (-450, 0, "-450"),
        (1000, 0, "1.000"),
        (100000, 2, "1.000,00"),
        (123456789012, 2, "1.234.567.890,12"),
        (9223372036854775807, 2, "92.233.720.368.547.758,07"),
        (i64::MIN, 2, "-92.233.720.368.547.758,08"),
    ];
    for &(num, dec, want) in tests {
        let b = append_number(GoBytes::make(0, 4), num, dec, 3, '.' as i32, ',' as i32);
        assert_eq!(b.to_vec(), want.as_bytes(), "{}", want);
    }
    let b = append_number(GoBytes::make(0, 7), 12345, 1, 3, -1, -1);
    assert_eq!(b.to_vec(), b"1.234,5");
}

#[test]
fn upstream_hsl2rgb() {
    assert_eq!(hsl2rgb(0.0, 1.0, 0.5), (1.0, 0.0, 0.0));
    assert_eq!(hsl2rgb(1.0, 1.0, 0.5), (1.0, 0.0, 0.0));
    assert_eq!(hsl2rgb(0.66, 0.0, 1.0), (1.0, 1.0, 1.0));
}
