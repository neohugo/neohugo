//! Go: parse/strconv/float.go
//!
//! `ParseFloat` here is tdewolff's fast parser, which is NOT correctly
//! rounded (it multiplies/divides by powers of ten); it must not be replaced
//! by `str::parse::<f64>`.

use crate::gobytes::{ByteView, GoBytes, SubView};
use crate::gomath::pow10;
use crate::strconv::int::{len_int, parse_int};

pub(crate) static FLOAT64_POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

// Go: parse/strconv/float.go:ParseFloat
/// Parses a byte-slice and returns the float it represents and the number of
/// bytes consumed. If an invalid character is encountered, it stops there.
pub fn parse_float<B: ByteView + ?Sized>(b: &B) -> (f64, usize) {
    let mut i: usize = 0;
    let mut neg = false;
    if i < b.len() && (b.at(i) == b'+' || b.at(i) == b'-') {
        neg = b.at(i) == b'-';
        i += 1;
    }
    let start = i;
    let mut dot: isize = -1;
    let mut trunk: isize = -1;
    let mut n: u64 = 0;
    while i < b.len() {
        let c = b.at(i);
        if c.is_ascii_digit() {
            if trunk == -1 {
                if u64::MAX / 10 < n {
                    trunk = i as isize;
                } else {
                    n = n.wrapping_mul(10);
                    n = n.wrapping_add((c - b'0') as u64);
                }
            }
        } else if dot == -1 && c == b'.' {
            dot = i as isize;
        } else {
            break;
        }
        i += 1;
    }
    if i == start || i == start + 1 && dot == start as isize {
        return (0.0, 0);
    }

    let mut f = n as f64;
    if neg {
        f = -f;
    }

    let mut mant_exp: i64 = 0;
    if dot != -1 {
        if trunk == -1 {
            trunk = i as isize;
        }
        mant_exp = (trunk - dot - 1) as i64;
    } else if trunk != -1 {
        mant_exp = (trunk - i as isize) as i64;
    }
    let mut exp_exp: i64 = 0;
    if i < b.len() && (b.at(i) == b'e' || b.at(i) == b'E') {
        let start_exp = i;
        i += 1;
        let (e, exp_len) = parse_int(&SubView::new(b, i));
        if 0 < exp_len {
            exp_exp = e;
            i += exp_len;
        } else {
            i = start_exp;
        }
    }
    let mut exp = exp_exp.wrapping_sub(mant_exp);

    // copied from strconv/atof.go
    if exp == 0 {
        return (f, i);
    } else if 0 < exp && exp <= 15 + 22 {
        // int * 10^k
        // If exponent is big but number of digits is not,
        // can move a few zeros into the integer part.
        if 22 < exp {
            f *= FLOAT64_POW10[(exp - 22) as usize];
            exp = 22;
        }
        if (-1e15..=1e15).contains(&f) {
            return (f * FLOAT64_POW10[exp as usize], i);
        }
    } else if (-22..0).contains(&exp) {
        // int / 10^k
        return (f / FLOAT64_POW10[(-exp) as usize], i);
    }
    f *= pow10(mant_exp.wrapping_neg());
    (f * pow10(exp_exp), i)
}

#[allow(clippy::approx_constant)] // Go's literal, kept verbatim
const LOG2: f64 = 0.3010299956639812;

// Go: parse/strconv/float.go:float64exp
fn float64exp(f: f64) -> isize {
    let mut exp2: isize = 0;
    if f != 0.0 {
        let x = f.to_bits();
        exp2 = ((x >> (64 - 11 - 1)) as isize & 0x7FF) - 1023 + 1;
    }

    let mut exp10 = exp2 as f64 * LOG2;
    if exp10 < 0.0 {
        // FMA: Go/arm64 fuses `float64(exp2)*log2 - 1.0` (FNMSUBD, float.go:105);
        // the comparison above uses the rounded product.
        exp10 = (exp2 as f64).mul_add(LOG2, -1.0);
    }
    exp10 as isize
}

// Go: parse/strconv/float.go:AppendFloat
/// Appends a float to `b` with precision `prec` (number of decimals). Returns
/// the new slice and whether successful.
pub fn append_float(b: GoBytes, mut f: f64, mut prec: isize) -> (GoBytes, bool) {
    if f.is_nan() || f.is_infinite() {
        return (b, false);
    }

    let mut neg = false;
    if f < 0.0 {
        f = -f;
        neg = true;
    }
    if !(0..=17).contains(&prec) {
        prec = 17; // maximum number of significant digits in double
    }
    prec -= float64exp(f); // number of digits in front of the dot
    f *= pow10(prec as i64);

    // calculate mantissa and exponent
    let mut mant = f as i64;
    let mut mant_len = len_int(mant) as isize;
    let mant_exp = mant_len - prec - 1;
    if mant == 0 {
        return (b.append(b"0"), true);
    }

    // expLen is zero for positive exponents, because positive exponents are determined later on in the big conversion loop
    let mut exp: isize = 0;
    let mut exp_len: isize = 0;
    if 0 < mant_exp {
        // positive exponent is determined in the loop below
        // but if we initially decreased the exponent to fit in an integer, we can't set the new exponent in the loop alone,
        // since the number of zeros at the end determines the positive exponent in the loop, and we just artificially lost zeros
        if prec < 0 {
            exp = mant_exp;
        }
        exp_len = 1 + len_int(exp as i64) as isize; // e + digits
    } else if mant_exp < -3 {
        exp = mant_exp;
        exp_len = 1 + len_int(exp as i64) as isize; // e + minus + digits
    } else if mant_exp < -1 {
        mant_len += -mant_exp - 1; // extra zero between dot and first digit
    }

    // reserve space in b
    let mut i = b.len() as isize;
    let mut max_len = 1 + mant_len + exp_len; // dot + mantissa digits + exponent
    if neg {
        max_len += 1;
    }
    let b = if (b.cap() as isize) < i + max_len {
        b.append(&vec![0u8; max_len as usize])
    } else {
        b.slice_to((i + max_len) as usize)
    };
    let set = |j: isize, c: u8| b.set(j as usize, c);

    // write to string representation
    if neg {
        set(i, b'-');
        i += 1;
    }

    // big conversion loop, start at the end and move to the front
    // initially print trailing zeros and remove them later on
    // for example if the first non-zero digit is three positions in front of the dot, it will overwrite the zeros with a positive exponent
    let mut zero = true;
    let mut last = i + mant_len; // right-most position of digit that is non-zero + dot
    let mut dot = last - prec - exp; // position of dot
    let mut j = last;
    while 0 < mant {
        if j == dot {
            set(j, b'.');
            j -= 1;
        }
        let new_mant = mant / 10;
        let digit = mant - 10 * new_mant;
        if zero && 0 < digit {
            // first non-zero digit, if we are still behind the dot we can trim the end to this position
            // otherwise trim to the dot (including the dot)
            if dot < j {
                i = j + 1;
                // decrease negative exponent further to get rid of dot
                if exp < 0 {
                    let new_exp = exp - (j - dot);
                    // getting rid of the dot shouldn't lower the exponent to more digits (e.g. -9 -> -10)
                    if len_int(new_exp as i64) == len_int(exp as i64) {
                        exp = new_exp;
                        dot = j;
                        j -= 1;
                        i -= 1;
                    }
                }
            } else {
                i = dot;
            }
            last = j;
            zero = false;
        }
        set(j, b'0' + digit as u8);
        j -= 1;
        mant = new_mant;
    }

    if dot < j {
        // extra zeros behind the dot
        while dot < j {
            set(j, b'0');
            j -= 1;
        }
        set(j, b'.');
    } else if last + 3 < dot {
        // add positive exponent because we have 3 or more zeros in front of the dot
        i = last + 1;
        exp = dot - last - 1;
    } else if j == dot {
        // handle 0.1
        set(j, b'.');
    }

    // exponent
    if exp != 0 {
        if exp == 1 {
            set(i, b'0');
            i += 1;
        } else if exp == 2 {
            set(i, b'0');
            set(i + 1, b'0');
            i += 2;
        } else {
            set(i, b'e');
            i += 1;
            if exp < 0 {
                set(i, b'-');
                i += 1;
                exp = -exp;
            }
            i += len_int(exp as i64) as isize;
            let mut j = i;
            while 0 < exp {
                let new_exp = exp / 10;
                let digit = exp - 10 * new_exp;
                j -= 1;
                set(j, b'0' + digit as u8);
                exp = new_exp;
            }
        }
    }
    (b.slice_to(i as usize), true)
}
