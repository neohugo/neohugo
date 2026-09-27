// Port of go1.27.1 src/internal/strconv/uscale.go.
//
// Floating point binary<->decimal conversion by fast unrounded scaling.
// See "Floating-Point Printing and Parsing Can Be Simple And Fast",
// https://research.swtch.com/fp

use super::Error;
use super::deps::{float32frombits, float64frombits};
use super::ftoa::{FLOAT32_MANT_BITS, FLOAT32_MIN_EXP, FLOAT64_MANT_BITS, FLOAT64_MIN_EXP};
use super::itoa::format_base10;
use super::pow10tab::{POW10_MIN, POW10_TAB};

/// bits.Len64
#[inline]
pub(crate) fn len64(x: u64) -> i64 {
    64 - x.leading_zeros() as i64
}

/// bits.Mul64: returns (hi, lo) of the 128-bit product x*y.
#[inline]
fn mul64(x: u64, y: u64) -> (u64, u64) {
    let p = (x as u128) * (y as u128);
    ((p >> 64) as u64, p as u64)
}

/// bool2 converts b to an integer: 1 for true, 0 for false.
#[inline]
pub(crate) fn bool2(b: bool) -> u64 {
    b as u64
}

// Go: internal/strconv/uscale.go:pack64
/// pack64 takes m, e and returns f = m * 2**e.
/// It assumes the caller has provided a 53-bit mantissa m
/// and an exponent that is in range for the mantissa.
pub(crate) fn pack64(m: u64, e: i64) -> (f64, Option<Error>) {
    if m & (1 << 52) == 0 {
        return (float64frombits(m), None);
    }
    if e >= 0x7FF - 1075 {
        return (
            float64frombits(m & (1 << 63) | 0x7ff << 52),
            Some(Error::Range),
        );
    }
    (
        float64frombits(m & !(1 << 52) | ((1075 + e) as u64) << 52),
        None,
    )
}

// Go: internal/strconv/uscale.go:pack32
/// pack32 takes m, e and returns f = m * 2**e.
/// It assumes the caller has provided a 24-bit mantissa m
/// and an exponent that is in range for the mantissa.
pub(crate) fn pack32(m: u32, e: i64) -> (f32, Option<Error>) {
    if m & (1 << 23) == 0 {
        return (float32frombits(m), None);
    }
    if e >= 0xFF - 150 {
        return (
            float32frombits(m & (1 << 31) | 0xff << 23),
            Some(Error::Range),
        );
    }
    (
        float32frombits(m & !(1 << 23) | ((150 + e) as u32) << 23),
        None,
    )
}

/// An Unrounded represents an unrounded value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Unrounded(pub(crate) u64);

impl Unrounded {
    #[inline]
    pub(crate) fn floor(self) -> u64 {
        self.0.wrapping_add(0) >> 2
    }
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn round_half_down(self) -> u64 {
        self.0.wrapping_add(1) >> 2
    }
    #[inline]
    pub(crate) fn round(self) -> u64 {
        self.0.wrapping_add(1).wrapping_add((self.0 >> 2) & 1) >> 2
    }
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn round_half_up(self) -> u64 {
        self.0.wrapping_add(2) >> 2
    }
    #[inline]
    pub(crate) fn ceil(self) -> u64 {
        self.0.wrapping_add(3) >> 2
    }
    #[inline]
    pub(crate) fn nudge(self, delta: i64) -> Unrounded {
        Unrounded(self.0.wrapping_add(delta as u64))
    }

    #[inline]
    pub(crate) fn div(self, d: u64) -> Unrounded {
        let x = self.0;
        Unrounded((x / d) | self.0 & 1 | bool2(x % d != 0))
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn rsh(self, s: u32) -> Unrounded {
        let u = self.0;
        Unrounded(u >> s | u & 1 | bool2(u & ((1u64 << s) - 1) != 0))
    }
}

// Go: internal/strconv/uscale.go:log10Pow2
/// log10Pow2(x) returns floor(log10 2**x) = floor(x * log10 2).
#[inline]
pub fn log10_pow2(x: i64) -> i64 {
    // log10 2 ~= 0.30102999566 ~= 78913 / 2^18
    (x.wrapping_mul(78913)) >> 18
}

// Go: internal/strconv/uscale.go:log2Pow10
/// log2Pow10(x) returns floor(log2 10**x) = floor(x * log2 10).
#[inline]
pub fn log2_pow10(x: i64) -> i64 {
    // log2 10 ~= 3.32192809489 ~= 108853 / 2^15
    (x.wrapping_mul(108853)) >> 15
}

/// uint64pow10[x] is 10**x.
pub(crate) static UINT64POW10: [u64; 20] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    10000000000,
    100000000000,
    1000000000000,
    10000000000000,
    100000000000000,
    1000000000000000,
    10000000000000000,
    100000000000000000,
    1000000000000000000,
    10000000000000000000,
];

// Go: internal/strconv/uscale.go:fixedWidthFloat
/// fixedWidthFloat returns the n-digit decimal form of f = m * 2**e as d * 10**p.
/// n can be at most 18.
/// If fmt == 'f' then n is a conservative estimate of the number of digits,
/// and digits are discarded to match prec.
pub(crate) fn fixed_width_float(m: u64, e: i64, n: i64, prec: i64, fmt: u8) -> (u64, i64) {
    let mut p = n - 1 - log10_pow2(e + 63);
    let mut pre = Scaler::default();
    prescale(&mut pre, e, p, log2_pow10(p));
    let mut u = uscale(m, &pre);
    if u >= unmin(UINT64POW10[n as usize]) {
        u = u.div(10);
        p -= 1;
    }
    if fmt == b'f' {
        while p > prec {
            u = u.div(10);
            p -= 1;
        }
    }
    (u.round(), -p)
}

// Go: internal/strconv/uscale.go:parseFloat64
/// parseFloat64 rounds d * 10**p to the nearest float64 f.
/// d can have at most 19 digits.
/// It returns ErrRange if the result rounds to infinity.
pub(crate) fn parse_float64(d: u64, p: i64, sign: u64) -> (f64, Option<Error>) {
    let b = len64(d);
    let lp = log2_pow10(p);
    let mut e = 1074.min(53 - b - lp);
    let mut pre = Scaler::default();
    prescale(&mut pre, e - (64 - b), p, lp);
    if pre.s >= 64 {
        return (float64frombits(sign | 0), None);
    }
    let mut u = uscale(d << (64 - b), &pre);

    // This block is branch-free code for:
    //	if u.round() >= 1<<53 {
    //		u = u.rsh(1)
    //		e = e - 1
    //	}
    let s = bool2(u >= unmin(1 << 53));
    u = Unrounded(u.0 >> s | u.0 & 1);
    e -= s as i64;

    pack64(sign | u.round(), -e)
}

// Go: internal/strconv/uscale.go:parseFloat32
/// parseFloat32 rounds d * 10**p to the nearest float32 f.
/// d can have at most 19 digits.
/// It returns ErrRange if the result rounds to infinity.
pub(crate) fn parse_float32(d: u64, p: i64, sign: u32) -> (f32, Option<Error>) {
    let b = len64(d);
    let lp = log2_pow10(p);
    let mut e = 149.min(24 - b - lp);
    let mut pre = Scaler::default();
    prescale(&mut pre, e - (64 - b), p, lp);
    if pre.s >= 64 {
        return (float32frombits(sign | 0), None);
    }
    let mut u = uscale(d << (64 - b), &pre);

    // This block is branch-free code for:
    //	if u.round() >= 1<<24 {
    //		u = u.rsh(1)
    //		e = e - 1
    //	}
    let s = bool2(u >= unmin(1 << 24));
    u = Unrounded(u.0 >> s | u.0 & 1);
    e -= s as i64;

    pack32(sign | u.round() as u32, -e)
}

// Go: internal/strconv/uscale.go:unmin
/// unmin returns the minimum unrounded that rounds to x.
#[inline]
pub(crate) fn unmin(x: u64) -> Unrounded {
    Unrounded((x << 2).wrapping_sub(2))
}

/// Parameterized constants for shortFloat[F float32 | float64].
#[derive(Clone, Copy)]
pub(crate) struct FloatParams {
    pub(crate) mant_bits: i64,
    pub(crate) min_exp: i64,
}

pub(crate) const SHORT_F32: FloatParams = FloatParams {
    mant_bits: FLOAT32_MANT_BITS,
    min_exp: FLOAT32_MIN_EXP,
};
pub(crate) const SHORT_F64: FloatParams = FloatParams {
    mant_bits: FLOAT64_MANT_BITS,
    min_exp: FLOAT64_MIN_EXP,
};

// Go: internal/strconv/uscale.go:shortFloat
/// shortFloat computes the shortest formatting of f,
/// using as few digits as possible that will still round trip
/// back to the original float.
pub(crate) fn short_float(fp: FloatParams, m: u64, e: i64) -> (u64, i64) {
    let mant_bits = fp.mant_bits;
    let min_exp = fp.min_exp;

    // Note: these cases could be factored a little more,
    // but in the first two branches, z is a constant,
    // allowing the compiler to greatly simplify the code.
    let min: u64;
    let max: u64;
    let odd: i64;
    let p: i64;
    let mut z = 63 - mant_bits;
    if m == 1 << 63 && e > min_exp {
        p = -skewed(e + z);
        min = m.wrapping_sub(1 << (z - 2)); // min = m - 1/4 * 2**(e+z)
        max = m.wrapping_add(1 << (z - 1)); // max = m + 1/2 * 2**(e+z)
        odd = (m >> z) as i64 & 1;
    } else if e >= min_exp {
        p = -log10_pow2(e + z);
        min = m.wrapping_sub(1 << (z - 1)); // min = m - 1/2 * 2**(e+z)
        max = m.wrapping_add(1 << (z - 1)); // max = m + 1/2 * 2**(e+z)
        odd = (m >> z) as i64 & 1;
    } else {
        z += min_exp - e;
        p = -log10_pow2(e + z);
        min = m.wrapping_sub(1 << (z - 1)); // min = m - 1/2 * 2**(e+z)
        max = m.wrapping_add(1 << (z - 1)); // max = m + 1/2 * 2**(e+z)
        odd = (m >> z) as i64 & 1;
    }

    let mut pre = Scaler::default();
    prescale(&mut pre, e, p, log2_pow10(p));
    let dmin = uscale(min, &pre).nudge(odd).ceil();
    let dmax = uscale(max, &pre).nudge(-odd).floor();

    let mut d = dmax / 10;
    if d.wrapping_mul(10) >= dmin {
        return (d, -(p - 1));
    }
    d = dmin;
    if d < dmax {
        d = uscale(m, &pre).round();
    }
    (d, -p)
}

// Go: internal/strconv/uscale.go:skewed
/// skewed computes the skewed footprint of m * 2**e,
/// which is floor(log10 3/4 * 2**e) = floor(e*(log10 2)-(log10 4/3)).
#[inline]
fn skewed(e: i64) -> i64 {
    (e.wrapping_mul(631305) - 261663) >> 21
}

/// A PmHiLo represents hi<<64 - lo.
#[derive(Clone, Copy)]
pub(crate) struct PmHiLo {
    pub(crate) hi: u64,
    pub(crate) lo: u64,
}

/// A Scaler holds derived scaling constants for a given e, p pair.
#[derive(Default, Clone, Copy)]
pub(crate) struct Scaler {
    pm_hi: u64,
    pm_lo: u64,
    s: i64,
}

// Go: internal/strconv/uscale.go:prescale
/// prescale returns the scaling constants for e, p.
/// lp must be log2Pow10(p).
/// The caller is responsible for either avoiding e, p pairs
/// that cause pre.s < 0 or pre.s >= 64, or else handling
/// those cases before passing the result to uscale.
/// In practice, pre.s < 0 would indicate a buggy caller
/// and pre.s >= 64 can only happen for parsing and is
/// picked off at those call sites.
#[inline]
fn prescale(pre: &mut Scaler, e: i64, p: i64, lp: i64) {
    let idx = (p - POW10_MIN) as usize;
    pre.pm_hi = POW10_TAB[idx].hi;
    pre.pm_lo = POW10_TAB[idx].lo;
    pre.s = -(e + lp + 3);
}

// Go: internal/strconv/uscale.go:uscale
/// uscale returns unround(x * 2**e * 10**p).
/// The caller should pass &pre for prescale(&pre, e, p, log2Pow10(p))
/// and should have left-justified x so its high bit is set.
/// The caller is also responsible for checking that c.s < 64.
/// For formatting, that's always true.
/// For parsing, the caller needs to pick it off early and return a signed 0.
#[inline]
fn uscale(x: u64, c: &Scaler) -> Unrounded {
    let (mut hi, mid) = mul64(x, c.pm_hi);
    let s = (c.s & 63) as u32; // make shifts cheaper
    if hi >> s << s != hi {
        return Unrounded(hi >> s | 1);
    }
    let (mid2, _) = mul64(x, c.pm_lo);
    hi = hi.wrapping_sub(bool2(mid < mid2));
    Unrounded(hi >> s | bool2(mid.wrapping_sub(mid2) > 1))
}

// Go: internal/strconv/uscale.go:setDigits
/// setDigits sets digs to the nd digits described by d, p.
pub(crate) fn set_digits(s: &mut [u8], d: u64, p: i64, nd: i64) -> (i64, i64) {
    let mut dp = 0;
    let mut nd = nd;
    // Note: nd <= len(s) is guaranteed by caller.
    if nd as usize <= s.len() {
        format_base10(&mut s[..nd as usize], d);
        dp = nd + p;
        while nd > 0 && s[(nd - 1) as usize] == b'0' {
            nd -= 1;
        }
    }
    (dp, nd)
}

// Go: internal/strconv/uscale.go:numDigits
/// numDigits returns the number of decimal digits in d.
/// It requires d >= 1.
#[inline]
pub(crate) fn num_digits(d: u64) -> i64 {
    let nd = log10_pow2(len64(d));
    nd + bool2(d >= UINT64POW10[nd as usize]) as i64
}
