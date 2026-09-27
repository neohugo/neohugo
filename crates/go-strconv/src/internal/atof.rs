// Port of go1.27.1 src/internal/strconv/atof.go.
//
// decimal to binary floating point conversion.
// Algorithm:
//   1) Store input in multiprecision decimal.
//   2) Multiply/divide decimal by powers of two until in range [0.5, 1)
//   3) Multiply by 2^precision and round to get mantissa.

use std::cell::Cell;

use super::Error;
use super::atoi::underscore_ok;
use super::decimal::Decimal;
use super::deps::{f32_to_f64, f64_to_f32, float32frombits, float64frombits, inf, nan};
use super::ftoa::{
    FLOAT32_BIAS, FLOAT32_EXP_BITS, FLOAT32_MANT_BITS, FLOAT64_BIAS, FLOAT64_EXP_BITS,
    FLOAT64_MANT_BITS, lower,
};
use super::uscale::{bool2, parse_float32, parse_float64};

pub(crate) struct FloatInfo {
    mantbits: u32,
    expbits: u32,
    bias: i64,
}

static FLOAT32INFO: FloatInfo = FloatInfo {
    mantbits: FLOAT32_MANT_BITS as u32,
    expbits: FLOAT32_EXP_BITS as u32,
    bias: FLOAT32_BIAS,
};
static FLOAT64INFO: FloatInfo = FloatInfo {
    mantbits: FLOAT64_MANT_BITS as u32,
    expbits: FLOAT64_EXP_BITS as u32,
    bias: FLOAT64_BIAS,
};

thread_local! {
    // Go: `var optimize = true // set to false to force slow-path conversions for testing`
    // Thread-local so that tests toggling it cannot race with other tests.
    static OPTIMIZE: Cell<bool> = const { Cell::new(true) };
}

#[inline]
pub(crate) fn optimize() -> bool {
    OPTIMIZE.with(|o| o.get())
}

/// Test hook (Go export_test.go SetOptimize): sets the current thread's
/// optimize flag and returns the old value. Not for production use.
#[doc(hidden)]
pub fn set_optimize(b: bool) -> bool {
    OPTIMIZE.with(|o| o.replace(b))
}

// Go: internal/strconv/atof.go:commonPrefixLenIgnoreCase
/// commonPrefixLenIgnoreCase returns the length of the common
/// prefix of s and prefix, with the character case of s ignored.
/// The prefix argument must be all lower-case.
fn common_prefix_len_ignore_case(s: &[u8], prefix: &[u8]) -> usize {
    let n = prefix.len().min(s.len());
    for i in 0..n {
        let mut c = s[i];
        if c.is_ascii_uppercase() {
            c += b'a' - b'A';
        }
        if c != prefix[i] {
            return i;
        }
    }
    n
}

// Go: internal/strconv/atof.go:special
/// special returns the floating-point value for the special,
/// possibly signed floating-point representations inf, infinity,
/// and NaN. The result is ok if a prefix of s contains one
/// of these representations and n is the length of that prefix.
/// The character case is ignored.
fn special(s: &[u8]) -> (f64, usize, bool) {
    if s.is_empty() {
        return (0.0, 0, false);
    }
    let mut s = s;
    let mut sign = 1;
    let mut nsign = 0;
    let c = s[0];
    let mut fallthrough = false;
    if c == b'+' || c == b'-' {
        if s[0] == b'-' {
            sign = -1;
        }
        nsign = 1;
        s = &s[1..];
        fallthrough = true;
    }
    if fallthrough || c == b'i' || c == b'I' {
        let mut n = common_prefix_len_ignore_case(s, b"infinity");
        // Anything longer than "inf" is ok, but if we
        // don't have "infinity", only consume "inf".
        if 3 < n && n < 8 {
            n = 3;
        }
        if n == 3 || n == 8 {
            return (inf(sign), nsign + n, true);
        }
    } else if (c == b'n' || c == b'N') && common_prefix_len_ignore_case(s, b"nan") == 3 {
        return (nan(), 3, true);
    }
    (0.0, 0, false)
}

impl Decimal {
    // Go: internal/strconv/atof.go:(*decimal).set
    pub(crate) fn set(&mut self, s: &[u8]) -> bool {
        let b = self;
        let mut i = 0usize;
        b.neg = false;
        b.trunc = false;

        // optional sign
        if i >= s.len() {
            return false;
        }
        match s[i] {
            b'+' => i += 1,
            b'-' => {
                i += 1;
                b.neg = true;
            }
            _ => {}
        }

        // digits
        let mut sawdot = false;
        let mut sawdigits = false;
        while i < s.len() {
            if s[i] == b'_' {
                // readFloat already checked underscores
                i += 1;
                continue;
            } else if s[i] == b'.' {
                if sawdot {
                    return false;
                }
                sawdot = true;
                b.dp = b.nd;
                i += 1;
                continue;
            } else if s[i].is_ascii_digit() {
                sawdigits = true;
                if s[i] == b'0' && b.nd == 0 {
                    // ignore leading zeros
                    b.dp -= 1;
                    i += 1;
                    continue;
                }
                if (b.nd as usize) < b.d.len() {
                    b.d[b.nd as usize] = s[i];
                    b.nd += 1;
                } else if s[i] != b'0' {
                    b.trunc = true;
                }
                i += 1;
                continue;
            }
            break;
        }
        if !sawdigits {
            return false;
        }
        if !sawdot {
            b.dp = b.nd;
        }

        // optional exponent moves decimal point.
        // if we read a very large, very long number,
        // just be sure to move the decimal point by
        // a lot (say, 100000).  it doesn't matter if it's
        // not the exact number.
        if i < s.len() && lower(s[i]) == b'e' {
            i += 1;
            if i >= s.len() {
                return false;
            }
            let mut esign = 1;
            match s[i] {
                b'+' => i += 1,
                b'-' => {
                    i += 1;
                    esign = -1;
                }
                _ => {}
            }
            if i >= s.len() || s[i] < b'0' || s[i] > b'9' {
                return false;
            }
            let mut e: i64 = 0;
            while i < s.len() && (s[i].is_ascii_digit() || s[i] == b'_') {
                if s[i] == b'_' {
                    // readFloat already checked underscores
                    i += 1;
                    continue;
                }
                if e < 10000 {
                    e = e * 10 + s[i] as i64 - b'0' as i64;
                }
                i += 1;
            }
            b.dp += e * esign;
        }

        if i != s.len() {
            return false;
        }

        true
    }
}

/// Result of readFloat.
struct ReadFloat {
    mantissa: u64,
    exp: i64,
    neg: bool,
    trunc: bool,
    hex: bool,
    i: usize,
    ok: bool,
}

// Go: internal/strconv/atof.go:readFloat
/// readFloat reads a decimal or hexadecimal mantissa and exponent from a float
/// string representation in s; the number may be followed by other characters.
/// readFloat reports the number of bytes consumed (i), and whether the number
/// is valid (ok).
fn read_float(s: &[u8]) -> ReadFloat {
    let mut r = ReadFloat {
        mantissa: 0,
        exp: 0,
        neg: false,
        trunc: false,
        hex: false,
        i: 0,
        ok: false,
    };
    let mut underscores = false;

    // optional sign
    if r.i >= s.len() {
        return r;
    }
    match s[r.i] {
        b'+' => r.i += 1,
        b'-' => {
            r.i += 1;
            r.neg = true;
        }
        _ => {}
    }

    // digits
    let mut base: u64 = 10;
    let mut max_mant_digits = 19; // 10^19 fits in uint64
    let mut exp_char = b'e';
    if r.i + 2 < s.len() && s[r.i] == b'0' && lower(s[r.i + 1]) == b'x' {
        base = 16;
        max_mant_digits = 16; // 16^16 fits in uint64
        r.i += 2;
        exp_char = b'p';
        r.hex = true;
    }
    let mut sawdot = false;
    let mut sawdigits = false;
    let mut nd: i64 = 0;
    let mut nd_mant: i64 = 0;
    let mut dp: i64 = 0;
    while r.i < s.len() {
        let c = s[r.i];
        if c == b'_' {
            underscores = true;
            r.i += 1;
            continue;
        } else if c == b'.' {
            if sawdot {
                break;
            }
            sawdot = true;
            dp = nd;
            r.i += 1;
            continue;
        } else if c.is_ascii_digit() {
            sawdigits = true;
            if c == b'0' && nd == 0 {
                // ignore leading zeros
                dp -= 1;
                r.i += 1;
                continue;
            }
            nd += 1;
            if nd_mant < max_mant_digits {
                r.mantissa = r.mantissa.wrapping_mul(base);
                r.mantissa = r.mantissa.wrapping_add((c - b'0') as u64);
                nd_mant += 1;
            } else if c != b'0' {
                r.trunc = true;
            }
            r.i += 1;
            continue;
        } else if base == 16 && b'a' <= lower(c) && lower(c) <= b'f' {
            sawdigits = true;
            nd += 1;
            if nd_mant < max_mant_digits {
                r.mantissa = r.mantissa.wrapping_mul(16);
                r.mantissa = r.mantissa.wrapping_add((lower(c) - b'a' + 10) as u64);
                nd_mant += 1;
            } else {
                r.trunc = true;
            }
            r.i += 1;
            continue;
        }
        break;
    }
    if !sawdigits {
        return r;
    }
    if !sawdot {
        dp = nd;
    }

    if base == 16 {
        dp *= 4;
        nd_mant *= 4;
    }

    // optional exponent moves decimal point.
    // if we read a very large, very long number,
    // just be sure to move the decimal point by
    // a lot (say, 100000).  it doesn't matter if it's
    // not the exact number.
    if r.i < s.len() && lower(s[r.i]) == exp_char {
        r.i += 1;
        if r.i >= s.len() {
            return r;
        }
        let mut esign = 1;
        match s[r.i] {
            b'+' => r.i += 1,
            b'-' => {
                r.i += 1;
                esign = -1;
            }
            _ => {}
        }
        if r.i >= s.len() || s[r.i] < b'0' || s[r.i] > b'9' {
            return r;
        }
        let mut e: i64 = 0;
        while r.i < s.len() && (s[r.i].is_ascii_digit() || s[r.i] == b'_') {
            if s[r.i] == b'_' {
                underscores = true;
                r.i += 1;
                continue;
            }
            if e < 10000 {
                e = e * 10 + s[r.i] as i64 - b'0' as i64;
            }
            r.i += 1;
        }
        dp += e * esign;
    } else if base == 16 {
        // Must have exponent.
        return r;
    }

    if r.mantissa != 0 {
        r.exp = dp - nd_mant;
    }

    if underscores && !underscore_ok(&s[..r.i]) {
        return r;
    }

    r.ok = true;
    r
}

/// decimal power of ten to binary power of two.
static POWTAB: [i64; 9] = [1, 3, 6, 9, 13, 16, 19, 23, 26];

impl Decimal {
    // Go: internal/strconv/atof.go:(*decimal).floatBits
    fn float_bits(&mut self, flt: &FloatInfo) -> (u64, bool) {
        let d = self;
        let mut exp: i64;
        let mut mant: u64;
        let mut overflow = false;

        // The Go code uses gotos; the labelled block below reproduces them:
        // `break 'out` is `goto out`, `overflow_label = true; break 'out` is
        // `goto overflow`.
        let mut goto_overflow = false;
        'out: {
            // Zero is always a special case.
            if d.nd == 0 {
                mant = 0;
                exp = flt.bias;
                break 'out;
            }

            // Obvious overflow/underflow.
            // These bounds are for 64-bit floats.
            // Will have to change if we want to support 80-bit floats in the future.
            if d.dp > 310 {
                goto_overflow = true;
                mant = 0;
                exp = 0;
                break 'out;
            }
            if d.dp < -330 {
                // zero
                mant = 0;
                exp = flt.bias;
                break 'out;
            }

            // Scale by powers of two until in range [0.5, 1.0)
            exp = 0;
            while d.dp > 0 {
                let n = if d.dp >= POWTAB.len() as i64 {
                    27
                } else {
                    POWTAB[d.dp as usize]
                };
                d.shift(-n);
                exp += n;
            }
            while d.dp < 0 || d.dp == 0 && d.d[0] < b'5' {
                let n = if -d.dp >= POWTAB.len() as i64 {
                    27
                } else {
                    POWTAB[(-d.dp) as usize]
                };
                d.shift(n);
                exp -= n;
            }

            // Our range is [0.5,1) but floating point range is [1,2).
            exp -= 1;

            // Minimum representable exponent is flt.bias+1.
            // If the exponent is smaller, move it up and
            // adjust d accordingly.
            if exp < flt.bias + 1 {
                let n = flt.bias + 1 - exp;
                d.shift(-n);
                exp += n;
            }

            if exp - flt.bias >= (1 << flt.expbits) - 1 {
                goto_overflow = true;
                mant = 0;
                break 'out;
            }

            // Extract 1+flt.mantbits bits.
            d.shift(1 + flt.mantbits as i64);
            mant = d.rounded_integer();

            // Rounding might have added a bit; shift down.
            if mant == 2u64 << flt.mantbits {
                mant >>= 1;
                exp += 1;
                if exp - flt.bias >= (1 << flt.expbits) - 1 {
                    goto_overflow = true;
                    break 'out;
                }
            }

            // Denormalized?
            if mant & (1u64 << flt.mantbits) == 0 {
                exp = flt.bias;
            }
            break 'out;
        }

        if goto_overflow {
            // overflow:
            // ±Inf
            mant = 0;
            exp = (1 << flt.expbits) - 1 + flt.bias;
            overflow = true;
        }

        // out:
        // Assemble bits.
        let mut bits = mant & ((1u64 << flt.mantbits) - 1);
        bits |= (((exp - flt.bias) & ((1 << flt.expbits) - 1)) as u64) << flt.mantbits;
        if d.neg {
            bits |= 1u64 << flt.mantbits << flt.expbits;
        }
        (bits, overflow)
    }
}

/// Exact powers of 10.
static FLOAT64POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];
static FLOAT32POW10: [f32; 11] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];

// Go: internal/strconv/atof.go:atof64exact
/// If possible to convert decimal representation to 64-bit float f exactly,
/// entirely in floating-point math, do so, avoiding the expense of decimalToFloatBits.
/// Three common cases:
///
/// - value is exact integer
/// - value is exact integer * exact power of ten
/// - value is exact integer / exact power of ten
///
/// These all produce potentially inexact but correctly rounded answers.
fn atof64exact(mantissa: u64, exp: i64, neg: bool) -> (f64, bool) {
    if mantissa >> FLOAT64INFO.mantbits != 0 {
        return (0.0, false);
    }
    let mut f = mantissa as f64;
    if neg {
        f = -f;
    }
    let mut exp = exp;
    if exp == 0 {
        // an integer.
        return (f, true);
    } else if exp > 0 && exp <= 15 + 22 {
        // int * 10^k
        // If exponent is big but number of digits is not,
        // can move a few zeros into the integer part.
        if exp > 22 {
            f *= FLOAT64POW10[(exp - 22) as usize];
            exp = 22;
        }
        if !(-1e15..=1e15).contains(&f) {
            // the exponent was really too large.
            return (f, false);
        }
        return (f * FLOAT64POW10[exp as usize], true);
    } else if exp < 0 && exp >= -22 {
        // int / 10^k
        return (f / FLOAT64POW10[(-exp) as usize], true);
    }
    (f, false)
}

// Go: internal/strconv/atof.go:atof32exact
/// If possible to compute mantissa*10^exp to 32-bit float f exactly,
/// entirely in floating-point math, do so, avoiding the machinery above.
fn atof32exact(mantissa: u64, exp: i64, neg: bool) -> (f32, bool) {
    if mantissa >> FLOAT32_MANT_BITS != 0 {
        return (0.0, false);
    }
    let mut f = mantissa as f32;
    if neg {
        f = -f;
    }
    let mut exp = exp;
    if exp == 0 {
        return (f, true);
    } else if exp > 0 && exp <= 7 + 10 {
        // int * 10^k
        // If exponent is big but number of digits is not,
        // can move a few zeros into the integer part.
        if exp > 10 {
            f *= FLOAT32POW10[(exp - 10) as usize];
            exp = 10;
        }
        if !(-1e7..=1e7).contains(&f) {
            // the exponent was really too large.
            return (f, false);
        }
        return (f * FLOAT32POW10[exp as usize], true);
    } else if exp < 0 && exp >= -10 {
        // int / 10^k
        return (f / FLOAT32POW10[(-exp) as usize], true);
    }
    (f, false)
}

// Go: internal/strconv/atof.go:atofHex
/// atofHex converts the hex floating-point string s
/// to a rounded float32 or float64 value (depending on flt==&float32info or flt==&float64info)
/// and returns it as a float64.
/// The string s has already been parsed into a mantissa, exponent, and sign (neg==true for negative).
/// If trunc is true, trailing non-zero bits have been omitted from the mantissa.
fn atof_hex(
    flt: &'static FloatInfo,
    mantissa: u64,
    exp: i64,
    neg: bool,
    trunc: bool,
) -> (f64, Option<Error>) {
    let max_exp = (1i64 << flt.expbits) + flt.bias - 2;
    let min_exp = flt.bias + 1;
    let mut exp = exp + flt.mantbits as i64; // mantissa now implicitly divided by 2^mantbits.
    let mut mantissa = mantissa;

    // Shift mantissa and exponent to bring representation into float range.
    // Eventually we want a mantissa with a leading 1-bit followed by mantbits other bits.
    // For rounding, we need two more, where the bottom bit represents
    // whether that bit or any later bit was non-zero.
    // (If the mantissa has already lost non-zero bits, trunc is true,
    // and we OR in a 1 below after shifting left appropriately.)
    while mantissa != 0 && mantissa >> (flt.mantbits + 2) == 0 {
        mantissa <<= 1;
        exp -= 1;
    }
    if trunc {
        mantissa |= 1;
    }
    while mantissa >> (1 + flt.mantbits + 2) != 0 {
        mantissa = mantissa >> 1 | mantissa & 1;
        exp += 1;
    }

    // If exponent is too negative,
    // denormalize in hopes of making it representable.
    // (The -2 is for the rounding bits.)
    while mantissa > 1 && exp < min_exp - 2 {
        mantissa = mantissa >> 1 | mantissa & 1;
        exp += 1;
    }

    // Round using two bottom bits.
    let mut round = mantissa & 3;
    mantissa >>= 2;
    round |= mantissa & 1; // round to even (round up if mantissa is odd)
    exp += 2;
    if round == 3 {
        mantissa += 1;
        if mantissa == 1u64 << (1 + flt.mantbits) {
            mantissa >>= 1;
            exp += 1;
        }
    }

    if mantissa >> flt.mantbits == 0 {
        // Denormal or zero.
        exp = flt.bias;
    }
    let mut err = None;
    if exp > max_exp {
        // infinity and range error
        mantissa = 1u64 << flt.mantbits;
        exp = max_exp + 1;
        err = Some(Error::Range);
    }

    let mut bits = mantissa & ((1u64 << flt.mantbits) - 1);
    bits |= (((exp - flt.bias) & ((1 << flt.expbits) - 1)) as u64) << flt.mantbits;
    if neg {
        bits |= 1u64 << flt.mantbits << flt.expbits;
    }
    if std::ptr::eq(flt, &FLOAT32INFO) {
        return (f32_to_f64(float32frombits(bits as u32)), err);
    }
    (float64frombits(bits), err)
}

// Go: internal/strconv/atof.go:atof32
fn atof32(s: &[u8]) -> (f32, usize, Option<Error>) {
    {
        let (val, n, ok) = special(s);
        if ok {
            return (f64_to_f32(val), n, None);
        }
    }

    let r = read_float(s);
    let (d, p, neg, trunc, hex, n, ok) = (r.mantissa, r.exp, r.neg, r.trunc, r.hex, r.i, r.ok);
    if !ok {
        return (0.0, n, Some(Error::Syntax));
    }

    if hex {
        let (f, err) = atof_hex(&FLOAT32INFO, d, p, neg, trunc);
        return (f64_to_f32(f), n, err);
    }

    if optimize() {
        let sign = (bool2(neg) as u32) << 31;
        if d == 0 {
            return (float32frombits(sign | 0), n, None);
        }
        if p > 40 {
            // overflow to ±Inf
            return (float32frombits(sign | 0xff << 23), n, Some(Error::Range));
        }
        if p < -70 {
            // underflow to ±0
            return (float32frombits(sign | 0), n, None);
        }
        if !trunc {
            // Exact rounding with single multiplication or division.
            let (f, ok) = atof32exact(d, p, neg);
            if ok {
                return (f, n, None);
            }
        }
        // Use fast unrounded scaling.
        // The only possible err is ErrRange, when the result overflows to ±Inf.
        let (f, err) = parse_float32(d, p, sign);
        if !trunc {
            return (f, n, err);
        }
        // If additional digits were truncated from d
        // but d+1 converts to the same value,
        // then the additional digits don't matter.
        let (f1, _) = parse_float32(d.wrapping_add(1), p, sign);
        if f == f1 {
            return (f, n, err);
        }
    }

    // Slow fallback.
    let mut dec = Box::<Decimal>::default();
    if !dec.set(&s[..n]) {
        return (0.0, n, Some(Error::Syntax));
    }
    let (b, ovf) = dec.float_bits(&FLOAT32INFO);
    let f = float32frombits(b as u32);
    let mut err = None;
    if ovf {
        err = Some(Error::Range);
    }
    (f, n, err)
}

// Go: internal/strconv/atof.go:atof64
fn atof64(s: &[u8]) -> (f64, usize, Option<Error>) {
    {
        let (val, n, ok) = special(s);
        if ok {
            return (val, n, None);
        }
    }

    let r = read_float(s);
    let (d, p, neg, trunc, hex, n, ok) = (r.mantissa, r.exp, r.neg, r.trunc, r.hex, r.i, r.ok);
    if !ok {
        return (0.0, n, Some(Error::Syntax));
    }
    if hex {
        let (f, err) = atof_hex(&FLOAT64INFO, d, p, neg, trunc);
        return (f, n, err);
    }
    if optimize() {
        let sign = bool2(neg) << 63;
        if d == 0 {
            return (float64frombits(sign | 0), n, None);
        }
        if p > 310 {
            // overflow to ±Inf
            return (float64frombits(sign | 0x7ff << 52), n, Some(Error::Range));
        }
        if p < -345 {
            // underflow to ±0
            return (float64frombits(sign | 0), n, None);
        }
        if !trunc {
            // Exact rounding with single multiplication or division.
            let (f, ok) = atof64exact(d, p, neg);
            if ok {
                return (f, n, None);
            }
        }
        // Use fast unrounded scaling.
        // The only possible err is ErrRange, when the result overflows to ±Inf.
        let (f, err) = parse_float64(d, p, sign);
        if !trunc {
            return (f, n, err);
        }
        // If additional digits were truncated from d
        // but d+1 converts to the same value,
        // then the additional digits don't matter.
        let (f1, _) = parse_float64(d.wrapping_add(1), p, sign);
        if f == f1 {
            return (f, n, err);
        }
    }

    // Slow fallback.
    let mut dec = Box::<Decimal>::default();
    if !dec.set(&s[..n]) {
        return (0.0, n, Some(Error::Syntax));
    }
    let (b, ovf) = dec.float_bits(&FLOAT64INFO);
    let f = float64frombits(b);
    let mut err = None;
    if ovf {
        err = Some(Error::Range);
    }
    (f, n, err)
}

// Go: internal/strconv/atof.go:ParseFloat
/// ParseFloat converts the string s to a floating-point number
/// with the precision specified by bitSize: 32 for float32, or 64 for float64.
/// When bitSize=32, the result still has type float64, but it will be
/// convertible to float32 without changing its value.
///
/// Returns Go's `(float64, error)` pair: the value is meaningful even when
/// the error is `Some(Error::Range)` (±Inf).
pub fn parse_float(s: &[u8], bit_size: i64) -> (f64, Option<Error>) {
    let (f, n, err) = parse_float_prefix(s, bit_size);
    if n != s.len() {
        return (0.0, Some(Error::Syntax));
    }
    (f, err)
}

// Go: internal/strconv/atof.go:parseFloatPrefix
/// Parses the longest float prefix of s; returns the value, the number of
/// bytes consumed and the error.
pub fn parse_float_prefix(s: &[u8], bit_size: i64) -> (f64, usize, Option<Error>) {
    if bit_size == 32 {
        let (f, n, err) = atof32(s);
        return (f32_to_f64(f), n, err);
    }
    atof64(s)
}
