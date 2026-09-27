//! Go: parse/strconv/decimal.go

use crate::gobytes::{ByteView, GoBytes};
use crate::gomath::pow10;
use crate::strconv::float::FLOAT64_POW10;
use crate::strconv::int::{INT64_POW10, len_int, len_uint};

// Go: parse/strconv/decimal.go:ParseDecimal
/// Parses a number of the format `1.2` (no exponent).
#[allow(clippy::impossible_comparisons)]
pub fn parse_decimal<B: ByteView + ?Sized>(b: &B) -> (f64, usize) {
    // float64 has up to 17 significant decimal digits and an exponent in [-1022,1023]
    let mut i: usize = 0;
    let mut sign = 1.0f64;
    if 0 < b.len() && b.at(0) == b'-' {
        sign = -1.0;
        i += 1;
    }

    let mut start: isize = -1;
    let mut dot: isize = -1;
    let mut n: u64 = 0;
    while i < b.len() {
        // parse up to 18 significant digits (with dot will be 17) ignoring zeros before/after
        let c = b.at(i);
        if c.is_ascii_digit() {
            if start == -1 {
                if (b'1'..=b'9').contains(&c) {
                    n = (c - b'0') as u64;
                    start = i as isize;
                }
            } else if (i as isize) - start < 18 {
                n *= 10;
                n += (c - b'0') as u64;
            }
        } else if c == b'.' {
            if dot != -1 {
                break;
            }
            dot = i as isize;
        } else {
            break;
        }
        i += 1;
    }
    if i == 1 && dot == 0 {
        return (0.0, 0); // only dot
    } else if start == -1 {
        return (0.0, i); // only zeros and dot
    } else if dot == -1 {
        dot = i as isize;
    }

    let mut exp = (dot - start) - len_uint(n) as isize;
    if dot < start {
        exp += 1;
    }
    if 1023 < exp {
        if sign == 1.0 {
            return (f64::INFINITY, i);
        } else {
            return (f64::NEG_INFINITY, i);
        }
    } else if exp < -1022 {
        return (0.0, i);
    }

    let f = sign * n as f64;
    if (0..23).contains(&exp) {
        return (f * FLOAT64_POW10[exp as usize], i);
    } else if 23 < exp && exp < 0 {
        // unreachable in Go too (kept for a line-by-line port)
        return (f / FLOAT64_POW10[exp as usize], i);
    }
    (f * pow10(exp as i64), i)
}

// Go: parse/strconv/decimal.go:AppendDecimal
/// Appends a float to `b` with `dec` the maximum number of decimals.
pub fn append_decimal(b: GoBytes, mut f: f64, mut dec: isize) -> GoBytes {
    if f.is_nan() || f.is_infinite() {
        return b;
    }

    if !(0..=17).contains(&dec) {
        dec = 17;
    }
    let p = pow10(dec as i64);

    // correct rounding
    // FMA: Go/arm64 fuses `f*p + 0.5` (FMADDD, decimal.go:86) and `f*p - 0.5`
    // (FNMSUBD, decimal.go:88); the comparison uses the rounded product.
    let prod = f * p;
    if 0.0 <= prod {
        f = f.mul_add(p, 0.5);
    } else {
        f = f.mul_add(p, -0.5);
    }

    // calculate mantissa and exponent
    let mut num = f as i64;
    if num == 0 {
        return b.append(b"0");
    }
    while 0 < dec && num % 10 == 0 {
        num /= 10;
        dec -= 1; // remove trailing zeros
    }

    let (mut i, mut n) = (b.len() as isize, len_int(num) as isize);
    if 0 < dec {
        if n < dec {
            n = dec; // number has zero after dot
        }
        n += 1; // dot
        let lim = INT64_POW10[dec as usize];
        if 0 < num && num < lim || num < 0 && -lim < num {
            n += 1; // zero at beginning
        }
    }
    let b = if (b.cap() as isize) < i + n {
        b.append(&vec![0u8; n as usize])
    } else {
        b.slice_to((i + n) as usize)
    };

    // print sign
    if num < 0 {
        num = num.wrapping_neg();
        b.set(i as usize, b'-');
    }
    i += n - 1;

    // print number
    if 0 < dec {
        b.set(i as usize, ((num % 10) as u8).wrapping_add(b'0'));
        num /= 10;
        dec -= 1;
        i -= 1;
        while 0 < dec {
            b.set(i as usize, ((num % 10) as u8).wrapping_add(b'0'));
            num /= 10;
            dec -= 1;
            i -= 1;
        }
        b.set(i as usize, b'.');
        i -= 1;
    }
    if num == 0 {
        b.set(i as usize, b'0');
    } else {
        while num != 0 {
            b.set(i as usize, ((num % 10) as u8).wrapping_add(b'0'));
            num /= 10;
            i -= 1;
        }
    }
    b
}
