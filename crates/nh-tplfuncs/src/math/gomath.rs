//! The Go 1.27.1 `math` functions `tpl/math` calls, as compiled for darwin/arm64 (the golden
//! toolchain). A copy of `crates/gift/src/gomath.rs` (verified there) plus `Tan`, `Asin`,
//! `Acos`, `Atan`, `Atan2` and `Min` (fused sites read from the arm64 disassembly; T18).
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! * `Exp` is the arm64 assembly `archExp` (`math/exp_arm64.s`), ported
//!   instruction by instruction (it uses `FMADDD`/`FMSUBD`/`FNMSUBD`).
//! * `Log`, `Pow`, `Sin`, `Cos`, `Sincos`, `Frexp`, `Ldexp` are the pure Go
//!   implementations (`haveArchLog` etc. are false on arm64). The Go compiler
//!   fuses their `x*y + z` expressions into FMA instructions; every fused site
//!   below was read from `go tool objdump` of the oracle binary (identical to
//!   the golden binary's code) and is written with `mul_add`.
//! * `Sqrt`, `Floor`, `Ceil`, `Trunc`, `Abs` are exact IEEE operations
//!   (arm64 intrinsics `FSQRTD`, `FRINTMD`, `FRINTPD`, `FRINTZD`, `FABSD`), the
//!   same as Rust's.
//! * `Max` is the arm64 assembly `archMax` (`math/dim_arm64.s`).

// Go's constants and range tests are kept as written.
#![allow(
    clippy::excessive_precision,
    clippy::approx_constant,
    clippy::manual_range_contains
)]

const MASK: u64 = 0x7FF;
const SHIFT: u64 = 64 - 11 - 1;
const BIAS: i64 = 1023;
const SIGN_MASK: u64 = 1 << 63;
const UVNAN: u64 = 0x7FF8000000000001;
const UVINF: u64 = 0x7FF0000000000000;
const UVNEGINF: u64 = 0xFFF0000000000000;

/// Go: math/bits.go:NaN
#[inline]
pub fn nan() -> f64 {
    f64::from_bits(UVNAN)
}

/// Go: math/bits.go:Inf
#[inline]
pub fn inf(sign: i64) -> f64 {
    if sign >= 0 {
        f64::from_bits(UVINF)
    } else {
        f64::from_bits(UVNEGINF)
    }
}

/// Go: math/bits.go:IsNaN
#[inline]
#[allow(clippy::eq_op)]
pub fn is_nan(f: f64) -> bool {
    f != f
}

/// Go: math/bits.go:IsInf
#[inline]
pub fn is_inf(f: f64, sign: i64) -> bool {
    sign >= 0 && f > f64::MAX || sign <= 0 && f < -f64::MAX
}

/// Go: math/signbit.go:Signbit
#[inline]
pub fn signbit(x: f64) -> bool {
    (x.to_bits() as i64) < 0
}

/// Go: math/copysign.go:Copysign
#[inline]
pub fn copysign(f: f64, sign: f64) -> f64 {
    f64::from_bits(f.to_bits() & !SIGN_MASK | sign.to_bits() & SIGN_MASK)
}

/// Go: math/bits.go:normalize
#[inline]
fn normalize(x: f64) -> (f64, i64) {
    const SMALLEST_NORMAL: f64 = 2.2250738585072014e-308; // 2**-1022
    if x.abs() < SMALLEST_NORMAL {
        return (x * (1u64 << 52) as f64, -52);
    }
    (x, 0)
}

/// Go: math/frexp.go:frexp
pub fn frexp(f: f64) -> (f64, i64) {
    // special cases
    if f == 0.0 {
        return (f, 0); // correctly return -0
    }
    if is_inf(f, 0) || is_nan(f) {
        return (f, 0);
    }
    let (f, mut exp) = normalize(f);
    let mut x = f.to_bits();
    exp += ((x >> SHIFT) & MASK) as i64 - BIAS + 1;
    x &= !(MASK << SHIFT);
    x |= ((-1 + BIAS) as u64) << SHIFT;
    (f64::from_bits(x), exp)
}

/// Go: math/ldexp.go:ldexp
pub fn ldexp(frac: f64, exp: i64) -> f64 {
    // special cases
    if frac == 0.0 {
        return frac; // correctly return -0
    }
    if is_inf(frac, 0) || is_nan(frac) {
        return frac;
    }
    let (frac, e) = normalize(frac);
    let mut exp = exp.wrapping_add(e);
    let mut x = frac.to_bits();
    exp = exp
        .wrapping_add((x >> SHIFT) as i64 & MASK as i64)
        .wrapping_sub(BIAS);
    if exp < -1075 {
        return copysign(0.0, frac); // underflow
    }
    if exp > 1023 {
        // overflow
        if frac < 0.0 {
            return inf(-1);
        }
        return inf(1);
    }
    let mut m: f64 = 1.0;
    if exp < -1022 {
        // denormal
        exp += 53;
        m = 1.0 / (1u64 << 53) as f64; // 2**-53
    }
    x &= !(MASK << SHIFT);
    x |= ((exp + BIAS) as u64) << SHIFT;
    m * f64::from_bits(x)
}

/// Go: math/modf.go:Modf (Trunc is the arm64 FRINTZD intrinsic).
#[inline]
pub fn modf(f: f64) -> (f64, f64) {
    let integer = f.trunc();
    let fractional = copysign(f - integer, f);
    (integer, fractional)
}

/// Go: math/exp_arm64.s:archExp (math.Exp on arm64).
pub fn exp(x: f64) -> f64 {
    const LN2HI: f64 = 6.93147180369123816490e-01;
    const LN2LO: f64 = 1.90821492927058770002e-10;
    const LOG2E: f64 = 1.44269504088896338700e+00;
    const OVERFLOW: f64 = 7.09782712893383973096e+02;
    const UNDERFLOW: f64 = -7.45133219101941108420e+02;
    const NEAR_ZERO: u64 = 0x3e30000000000000; // 2**-28
    const POS_INF: u64 = 0x7ff0000000000000;
    const FRAC_MASK: u64 = 0x000fffffffffffff;
    const C1: u64 = 0x3cb0000000000000; // 2**-52
    const P1: f64 = 1.66666666666666657415e-01; // 0x3FC55555; 0x55555555
    const P2: f64 = -2.77777777770155933842e-03; // 0xBF66C16C; 0x16BEBD93
    const P3: f64 = 6.61375632143793436117e-05; // 0x3F11566A; 0xAF25DE2C
    const P4: f64 = -1.65339022054652515390e-06; // 0xBEBBBD41; 0xC5D26BF1
    const P5: f64 = 4.13813679705723846039e-08; // 0x3E663769; 0x72BEA4D0

    // FCMPD F0, F0; BNE isNaN
    if is_nan(x) {
        return x;
    }
    // x > Overflow, return PosInf
    if x > OVERFLOW {
        return f64::from_bits(POS_INF);
    }
    // x < Underflow, return 0
    if x < UNDERFLOW {
        return 0.0;
    }
    let one = 1.0f64;
    // fabs(x) < NearZero, return 1 + x
    if x.abs() < f64::from_bits(NEAR_ZERO) {
        return x + one;
    }
    // argument reduction, x = k*ln2 + r,  |r| <= 0.5*ln2
    // computed as r = hi - lo for extra precision.
    let km = LOG2E.mul_add(x, -0.5); // FNMSUBD: Log2e*x - 0.5
    let kp = LOG2E.mul_add(x, 0.5); // FMADDD: Log2e*x + 0.5
    let k = if x < 0.0 { km } else { kp }; // FCSELD LT
    let ik = k as i64; // FCVTZSD (saturating, NaN -> 0)
    let fk = ik as f64; // SCVTFD
    let hi = (-fk).mul_add(LN2HI, x); // FMSUBD: x - float64(int(k))*Ln2Hi
    let lo = fk * LN2LO;
    let r = hi - lo;
    let t = r * r;
    // compute y
    let mut c = t.mul_add(P5, P4);
    c = t.mul_add(c, P3);
    c = t.mul_add(c, P2);
    c = t.mul_add(c, P1);
    c = (-t).mul_add(c, r); // FMSUBD: r - t*(...)
    let d = 2.0 - c;
    let mut y2 = r * c;
    y2 /= d; // (r*c)/(2-c)
    y2 = lo - y2;
    y2 -= hi;
    let y = one - y2; // y = 1-((lo-(r*c)/(2-c))-hi)
    // inline Ldexp(y, k)
    let r0 = y.to_bits();
    let r2 = r0 & FRAC_MASK; // fraction
    let mut r5 = (r0 >> 52) as i64; // exponent (LSR: logical)
    r5 = r5.wrapping_add(ik);
    let mut m = one;
    if r5 < 1 {
        // denormal
        r5 = r5.wrapping_add(52);
        m = f64::from_bits(C1); // m = 2**-52
    }
    let bits = r2 | (r5 as u64).wrapping_shl(52);
    f64::from_bits(bits) * m
}

/// Go: math/log.go:log (math.Log on arm64; haveArchLog is false).
pub fn log(x: f64) -> f64 {
    const LN2HI: f64 = 6.93147180369123816490e-01; /* 3fe62e42 fee00000 */
    const LN2LO: f64 = 1.90821492927058770002e-10; /* 3dea39ef 35793c76 */
    const L1: f64 = 6.666666666666735130e-01; /* 3FE55555 55555593 */
    const L2: f64 = 3.999999999940941908e-01; /* 3FD99999 9997FA04 */
    const L3: f64 = 2.857142874366239149e-01; /* 3FD24924 94229359 */
    const L4: f64 = 2.222219843214978396e-01; /* 3FCC71C5 1D8E78AF */
    const L5: f64 = 1.818357216161805012e-01; /* 3FC74664 96CB03DE */
    const L6: f64 = 1.531383769920937332e-01; /* 3FC39A09 D078C69F */
    const L7: f64 = 1.479819860511658591e-01; /* 3FC2F112 DF3E5244 */
    const SQRT2: f64 = 1.41421356237309504880168872420969807856967187537694807317667974;

    // special cases
    if is_nan(x) || is_inf(x, 1) {
        return x;
    }
    if x < 0.0 {
        return nan();
    }
    if x == 0.0 {
        return inf(-1);
    }

    // reduce
    let (mut f1, mut ki) = frexp(x);
    if f1 < SQRT2 / 2.0 {
        f1 *= 2.0;
        ki -= 1;
    }
    let f = f1 - 1.0;
    let k = ki as f64;

    // compute
    let s = f / (2.0 + f);
    let s2 = s * s;
    let s4 = s2 * s2;
    // t1 := s2 * (L1 + s4*(L3+s4*(L5+s4*L7))) -- the outer product is fused below
    let t1p = s4.mul_add(s4.mul_add(s4.mul_add(L7, L5), L3), L1);
    // t2 := s4 * (L2 + s4*(L4+s4*L6))
    let t2 = s4 * s4.mul_add(s4.mul_add(L6, L4), L2);
    // R := t1 + t2 (FMADDD: t2 + s2*t1p)
    let rr = s2.mul_add(t1p, t2);
    // hfsq := 0.5 * f * f (the second product is fused into both uses)
    let hf = 0.5 * f;
    // return k*Ln2Hi - ((hfsq - (s*(hfsq+R) + k*Ln2Lo)) - f)
    let a = hf.mul_add(f, rr); // hfsq + R
    let b = s.mul_add(a, k * LN2LO); // s*(hfsq+R) + k*Ln2Lo
    let c = hf.mul_add(f, -b); // hfsq - (...)
    let d = c - f;
    k.mul_add(LN2HI, -d) // k*Ln2Hi - (...)
}

/// Go: math/pow.go:isOddInt
fn is_odd_int(x: f64) -> bool {
    if x.abs() >= (1u64 << 53) as f64 {
        // 1 << 53 is the largest exact integer in the float64 format.
        return false;
    }
    let (xi, xf) = modf(x);
    xf == 0.0 && (xi as i64) & 1 == 1
}

/// Go: math/pow.go:pow (math.Pow on arm64; haveArchPow is false).
pub fn pow(x: f64, y: f64) -> f64 {
    if y == 0.0 || x == 1.0 {
        return 1.0;
    } else if y == 1.0 {
        return x;
    } else if is_nan(x) || is_nan(y) {
        return nan();
    } else if x == 0.0 {
        if y < 0.0 {
            if signbit(x) && is_odd_int(y) {
                return inf(-1);
            }
            return inf(1);
        } else if y > 0.0 {
            if signbit(x) && is_odd_int(y) {
                return x;
            }
            return 0.0;
        }
    } else if is_inf(y, 0) {
        if x == -1.0 {
            return 1.0;
        } else if (x.abs() < 1.0) == is_inf(y, 1) {
            return 0.0;
        } else {
            return inf(1);
        }
    } else if is_inf(x, 0) {
        if is_inf(x, -1) {
            return pow(1.0 / x, -y); // Pow(-0, -y)
        }
        if y < 0.0 {
            return 0.0;
        } else if y > 0.0 {
            return inf(1);
        }
    } else if y == 0.5 {
        return sqrt(x);
    } else if y == -0.5 {
        return 1.0 / sqrt(x);
    }

    let (mut yi, mut yf) = modf(y.abs());
    if yf != 0.0 && x < 0.0 {
        return nan();
    }
    if yi >= 9.223372036854775808e18 {
        // 1<<63
        // yi is a large even int that will lead to overflow (or underflow to 0)
        // for all x except -1 (x == 1 was handled earlier)
        if x == -1.0 {
            return 1.0;
        } else if (x.abs() < 1.0) == (y > 0.0) {
            return 0.0;
        } else {
            return inf(1);
        }
    }

    // ans = a1 * 2**ae (= 1 for now).
    let mut a1: f64 = 1.0;
    let mut ae: i64 = 0;

    // ans *= x**yf
    if yf != 0.0 {
        if yf > 0.5 {
            yf -= 1.0;
            yi += 1.0;
        }
        a1 = exp(yf * log(x));
    }

    // ans *= x**yi
    // by multiplying in successive squarings
    // of x according to bits of yi.
    // accumulate powers of two into exp.
    let (mut x1, mut xe) = frexp(x);
    let mut i = yi as i64;
    while i != 0 {
        if xe < -(1 << 12) || (1 << 12) < xe {
            // catch xe before it overflows the left shift below
            ae = ae.wrapping_add(xe);
            break;
        }
        if i & 1 == 1 {
            a1 *= x1;
            ae = ae.wrapping_add(xe);
        }
        let x0 = x1;
        x1 *= x1;
        xe <<= 1;
        if x1 < 0.5 {
            // x1 += x1, with the square fused (FMADDD x0, x1, x0).
            x1 = x0.mul_add(x0, x1);
            xe -= 1;
        }
        i >>= 1;
    }

    // ans = a1*2**ae
    // if y < 0 { ans = 1 / ans }
    // but in the opposite order
    if y < 0.0 {
        a1 = 1.0 / a1;
        ae = ae.wrapping_neg();
    }
    ldexp(a1, ae)
}

// sin coefficients
const SIN: [f64; 6] = [
    1.58962301576546568060e-10, // 0x3de5d8fd1fd19ccd
    -2.50507477628578072866e-8, // 0xbe5ae5e5a9291f5d
    2.75573136213857245213e-6,  // 0x3ec71de3567d48a1
    -1.98412698295895385996e-4, // 0xbf2a01a019bfdf03
    8.33333333332211858878e-3,  // 0x3f8111111110f7d0
    -1.66666666666666307295e-1, // 0xbfc5555555555548
];

// cos coefficients
const COS: [f64; 6] = [
    -1.13585365213876817300e-11, // 0xbda8fa49a0861a9b
    2.08757008419747316778e-9,   // 0x3e21ee9d7b4e3f05
    -2.75573141792967388112e-7,  // 0xbe927e4f7eac4bc6
    2.48015872888517045348e-5,   // 0x3efa01a019c844f5
    -1.38888888888730564116e-3,  // 0xbf56c16c16c14f91
    4.16666666666665929218e-2,   // 0x3fa555555555554b
];

const PI4A: f64 = 7.85398125648498535156e-1; // 0x3fe921fb40000000, Pi/4 split into three parts
const PI4B: f64 = 3.77489470793079817668e-8; // 0x3e64442d00000000,
const PI4C: f64 = 2.69515142907905952645e-15; // 0x3ce8469898cc5170,
/// Go: `4 / Pi` rounded once to float64.
const FOUR_OVER_PI: f64 = 1.2732395447351628; // 0x3ff45f306dc9c883
const REDUCE_THRESHOLD: f64 = (1u64 << 29) as f64;

/// The sin polynomial as compiled: z + (z*zz)*P(zz), Horner steps fused.
#[inline]
fn sin_poly(z: f64, zz: f64) -> f64 {
    let p = zz.mul_add(
        zz.mul_add(
            zz.mul_add(zz.mul_add(zz.mul_add(SIN[0], SIN[1]), SIN[2]), SIN[3]),
            SIN[4],
        ),
        SIN[5],
    );
    (z * zz).mul_add(p, z)
}

/// The cos polynomial as compiled: (1 - 0.5*zz) + (zz*zz)*P(zz), all fused.
#[inline]
fn cos_poly(zz: f64) -> f64 {
    let a = (-0.5f64).mul_add(zz, 1.0); // FMSUBD: 1.0 - zz*0.5
    let p = zz.mul_add(
        zz.mul_add(
            zz.mul_add(zz.mul_add(zz.mul_add(COS[0], COS[1]), COS[2]), COS[3]),
            COS[4],
        ),
        COS[5],
    );
    (zz * zz).mul_add(p, a)
}

/// Octant reduction shared by sin, cos and Sincos (x >= 0, x < reduceThreshold).
#[inline]
fn reduce_small(x: f64) -> (u64, f64) {
    let mut j = (x * FOUR_OVER_PI) as u64; // integer part of x/(Pi/4)
    let mut y = j as f64; // integer part of x/(Pi/4), as float
    if j & 1 == 1 {
        j = j.wrapping_add(1);
        y += 1.0;
    }
    j &= 7; // octant modulo 2Pi radians (360 degrees)
    // z = ((x - y*PI4A) - y*PI4B) - y*PI4C, each step fused (FMSUBD).
    let z = (-y).mul_add(PI4C, (-y).mul_add(PI4B, (-y).mul_add(PI4A, x)));
    (j, z)
}

/// Go: math/sin.go:cos
pub fn cos(x: f64) -> f64 {
    if is_nan(x) || is_inf(x, 0) {
        return nan();
    }

    // make argument positive
    let mut sign = false;
    let x = x.abs();

    let (mut j, z) = if x >= REDUCE_THRESHOLD {
        trig_reduce(x)
    } else {
        reduce_small(x)
    };

    if j > 3 {
        j -= 4;
        sign = !sign;
    }
    if j > 1 {
        sign = !sign;
    }

    let zz = z * z;
    let mut y = if j == 1 || j == 2 {
        sin_poly(z, zz)
    } else {
        cos_poly(zz)
    };
    if sign {
        y = -y;
    }
    y
}

/// Go: math/sin.go:sin
pub fn sin(x: f64) -> f64 {
    // special cases
    if x == 0.0 || is_nan(x) {
        return x; // return ±0 || NaN()
    }
    if is_inf(x, 0) {
        return nan();
    }

    // make argument positive but save the sign
    let mut sign = false;
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sign = true;
    }

    let (mut j, z) = if x >= REDUCE_THRESHOLD {
        trig_reduce(x)
    } else {
        reduce_small(x)
    };
    // reflect in x axis
    if j > 3 {
        sign = !sign;
        j -= 4;
    }
    let zz = z * z;
    let mut y = if j == 1 || j == 2 {
        cos_poly(zz)
    } else {
        sin_poly(z, zz)
    };
    if sign {
        y = -y;
    }
    y
}

/// Go: math/sincos.go:Sincos
pub fn sincos(x: f64) -> (f64, f64) {
    // special cases
    if x == 0.0 {
        return (x, 1.0); // return ±0.0, 1.0
    }
    if is_nan(x) || is_inf(x, 0) {
        return (nan(), nan());
    }

    // make argument positive
    let (mut sin_sign, mut cos_sign) = (false, false);
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sin_sign = true;
    }

    let (mut j, z) = if x >= REDUCE_THRESHOLD {
        trig_reduce(x)
    } else {
        reduce_small(x)
    };
    if j > 3 {
        // reflect in x axis
        j -= 4;
        sin_sign = !sin_sign;
        cos_sign = !cos_sign;
    }
    if j > 1 {
        cos_sign = !cos_sign;
    }

    let zz = z * z;
    let mut cos = cos_poly(zz);
    let mut sin = sin_poly(z, zz);
    if j == 1 || j == 2 {
        std::mem::swap(&mut sin, &mut cos);
    }
    if cos_sign {
        cos = -cos;
    }
    if sin_sign {
        sin = -sin;
    }
    (sin, cos)
}

// mPi4 is the binary digits of 4/pi as a uint64 array,
// that is, 4/pi = Sum mPi4[i]*2^(-64*i)
// 19 64-bit digits and the leading one bit give 1217 bits
// of precision to handle the largest possible float64 exponent.
const M_PI4: [u64; 20] = [
    0x0000000000000001,
    0x45f306dc9c882a53,
    0xf84eafa3ea69bb81,
    0xb6c52b3278872083,
    0xfca2c757bd778ac3,
    0x6e48dc74849ba5c0,
    0x0c925dd413a32439,
    0xfc3bd63962534e7d,
    0xd1046bea5d768909,
    0xd338e04d68befc82,
    0x7323ac7306a673e9,
    0x3908bf177bf25076,
    0x3ff12fffbc0b301f,
    0xde5e2316b414da3e,
    0xda6cfd9e4f96136e,
    0x9e8c7ecd3cbfd45a,
    0xea4f758fd7cbe2f6,
    0x7a0e73ef14a525d4,
    0xd7f6bf623f1aba10,
    0xac06608df8f6d757,
];

#[inline]
fn go_shl(x: u64, s: u64) -> u64 {
    if s >= 64 { 0 } else { x << s }
}

#[inline]
fn go_shr(x: u64, s: u64) -> u64 {
    if s >= 64 { 0 } else { x >> s }
}

/// Go: math/trig_reduce.go:trigReduce
pub fn trig_reduce(x: f64) -> (u64, f64) {
    const PI4: f64 = std::f64::consts::PI / 4.0;
    if x < PI4 {
        return (0, x);
    }
    // Extract out the integer and exponent such that,
    // x = ix * 2 ** exp.
    let mut ix = x.to_bits();
    let exp = ((ix >> SHIFT) & MASK) as i64 - BIAS - SHIFT as i64;
    ix &= !(MASK << SHIFT);
    ix |= 1 << SHIFT;
    // Use the exponent to extract the 3 appropriate uint64 digits from mPi4,
    // B ~ (z0, z1, z2), such that the product leading digit has the exponent -61.
    // Note, exp >= -53 since x >= PI4 and exp < 971 for maximum float64.
    let (digit, bitshift) = (((exp + 61) as u64) / 64, ((exp + 61) as u64) % 64);
    let digit = digit as usize;
    let z0 = go_shl(M_PI4[digit], bitshift) | go_shr(M_PI4[digit + 1], 64 - bitshift);
    let z1 = go_shl(M_PI4[digit + 1], bitshift) | go_shr(M_PI4[digit + 2], 64 - bitshift);
    let z2 = go_shl(M_PI4[digit + 2], bitshift) | go_shr(M_PI4[digit + 3], 64 - bitshift);
    // Multiply mantissa by the digits and extract the upper two digits (hi, lo).
    let z2hi = ((z2 as u128 * ix as u128) >> 64) as u64;
    let z1p = z1 as u128 * ix as u128;
    let (z1hi, z1lo) = ((z1p >> 64) as u64, z1p as u64);
    let z0lo = z0.wrapping_mul(ix);
    let (lo, c) = z1lo.overflowing_add(z2hi);
    let hi = z0lo.wrapping_add(z1hi).wrapping_add(c as u64);
    // The top 3 bits of hi give j.
    let mut j = hi >> 61;
    // Extract the fraction and find its magnitude.
    let mut hi = hi << 3 | lo >> 61;
    let lz = hi.leading_zeros() as u64;
    let e = (BIAS as u64).wrapping_sub(lz + 1);
    // Clear implicit mantissa bit and shift into place.
    hi = go_shl(hi, lz + 1) | go_shr(lo, 64u64.wrapping_sub(lz + 1));
    hi >>= 64 - SHIFT;
    // Include the exponent and convert to a float.
    hi |= e << SHIFT;
    let mut z = f64::from_bits(hi);
    // Map zeros to origin.
    if j & 1 == 1 {
        j += 1;
        j &= 7;
        z -= 1.0;
    }
    // Multiply the fractional part by pi/4.
    (j, z * PI4)
}

/// Go: math/dim_arm64.s:archMax (math.Max on arm64): +Inf wins, then FMAXD
/// (NaN if either operand is NaN; FMAX(+0, -0) = +0).
pub fn max(x: f64, y: f64) -> f64 {
    const POS_INF: u64 = 0x7ff0000000000000;
    if x.to_bits() == POS_INF || y.to_bits() == POS_INF {
        return f64::from_bits(POS_INF);
    }
    // FMAXD Fn=y, Fm=x: a NaN operand propagates (quieted), Fn first. The
    // payload never reaches gift's output (it is converted with int()).
    if is_nan(y) {
        return f64::from_bits(y.to_bits() | 0x0008000000000000);
    }
    if is_nan(x) {
        return f64::from_bits(x.to_bits() | 0x0008000000000000);
    }
    if x == 0.0 && y == 0.0 {
        // FMAX(+0, -0) = +0.
        if signbit(x) {
            return y;
        }
        return x;
    }
    if x > y { x } else { y }
}

// ---------------------------------------------------------------------------
// arm64 default NaN

/// The NaN an arm64 floating point instruction produces for an invalid operation on non-NaN
/// operands (FPCR.DN clear: +qNaN with a zero payload); x86-64 produces `0xFFF8000000000000`.
pub const DEFAULT_NAN: u64 = 0x7FF8000000000000;

/// Go: math.Sqrt on arm64 (the FSQRTD intrinsic): a negative operand gives the default NaN.
pub fn sqrt(x: f64) -> f64 {
    if x < 0.0 {
        return f64::from_bits(DEFAULT_NAN);
    }
    x.sqrt()
}

/// The result `r` of an arm64 binary operation on `a` and `b`: the default NaN when the
/// operation was invalid (a NaN operand propagates unchanged, as on x86-64).
pub fn arm_result(r: f64, a: f64, b: f64) -> f64 {
    if r.is_nan() && !a.is_nan() && !b.is_nan() {
        return f64::from_bits(DEFAULT_NAN);
    }
    r
}

// ---------------------------------------------------------------------------
// Added for tpl/math (T18): Tan, Asin, Acos, Atan, Atan2, Min. The fused sites were read from
// `go tool objdump` of a linux/arm64 go1.27.1 build (`math.tan`, `math.asin`, `math.satan`).

const TAN_P: [f64; 3] = [
    -1.30936939181383777646e4, // 0xc0c992d8d24f3f38
    1.15351664838587416140e6,  // 0x413199eca5fc9ddd
    -1.79565251976484877988e7, // 0xc1711fead3299176
];
const TAN_Q: [f64; 5] = [
    1.00000000000000000000e0,
    1.36812963470692954678e4,  // 0x40cab8a5eeb36572
    -1.32089234440210967447e6, // 0xc13427bc582abc96
    2.50083801823357915839e7,  // 0x4177d98fc2ead8ef
    -5.38695755929454629881e7, // 0xc189afe03cbe5a31
];

/// Go: math/tan.go:tan (math.Tan on arm64; haveArchTan is false).
pub fn tan(x: f64) -> f64 {
    if x == 0.0 || is_nan(x) {
        return x; // return ±0 || NaN()
    }
    if is_inf(x, 0) {
        return nan();
    }

    let mut sign = false;
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sign = true;
    }
    let (j, z) = if x >= REDUCE_THRESHOLD {
        trig_reduce(x)
    } else {
        // FMULD + FCVTZUD (saturating) + UCVTFD.
        let mut j = (x * FOUR_OVER_PI) as u64; // integer part of x/(Pi/4)
        let mut y = j as f64;

        /* map zeros and singularities to origin */
        if j & 1 == 1 {
            j = j.wrapping_add(1);
            y += 1.0;
        }

        // z = ((x - y*PI4A) - y*PI4B) - y*PI4C, each step fused (FMSUBD).
        let z = (-y).mul_add(PI4C, (-y).mul_add(PI4B, (-y).mul_add(PI4A, x)));
        (j, z)
    };
    let zz = z * z;

    let mut y = if zz > 1e-14 {
        // Numerator: FMADDD, FMADDD, FMULD.
        let num = zz * zz.mul_add(zz.mul_add(TAN_P[0], TAN_P[1]), TAN_P[2]);
        // Denominator: (zz+Q1) is fused from z*z (FMADDD z, z, Q1), then FMADDD x3.
        let mut den = z.mul_add(z, TAN_Q[1]);
        den = den.mul_add(zz, TAN_Q[2]);
        den = den.mul_add(zz, TAN_Q[3]);
        den = den.mul_add(zz, TAN_Q[4]);
        // z + z*(num/den): FDIVD, FMADDD.
        z.mul_add(num / den, z)
    } else {
        z
    };
    if j & 2 == 2 {
        y = -1.0 / y;
    }
    if sign {
        y = -y;
    }
    y
}

/// Go: math/atan.go:xatan (inlined into satan; every copy fuses the same sites).
fn xatan(x: f64) -> f64 {
    const P0: f64 = -8.750608600031904122785e-01;
    const P1: f64 = -1.615753718733365076637e+01;
    const P2: f64 = -7.500855792314704667340e+01;
    const P3: f64 = -1.228866684490136173410e+02;
    const P4: f64 = -6.485021904942025371773e+01;
    const Q0: f64 = 2.485846490142306297962e+01;
    const Q1: f64 = 1.650270098316988542046e+02;
    const Q2: f64 = 4.328810604912902668951e+02;
    const Q3: f64 = 4.853903996359136964868e+02;
    const Q4: f64 = 1.945506571482613964425e+02;
    let z = x * x;
    // Numerator: 4 x FMADDD then FMULD.
    let num = z * z.mul_add(z.mul_add(z.mul_add(z.mul_add(P0, P1), P2), P3), P4);
    // Denominator: (z+Q0) fused from x*x (FMADDD x, x, Q0), then 4 x FMADDD.
    let mut den = x.mul_add(x, Q0);
    den = den.mul_add(z, Q1);
    den = den.mul_add(z, Q2);
    den = den.mul_add(z, Q3);
    den = den.mul_add(z, Q4);
    let z = num / den;
    // z = x*z + x (FMADDD).
    x.mul_add(z, x)
}

/// Go: math/atan.go:satan
fn satan(x: f64) -> f64 {
    const MOREBITS: f64 = 6.123233995736765886130e-17; // pi/2 = PIO2 + Morebits
    const TAN3PIO8: f64 = 2.41421356237309504880; // tan(3*pi/8)
    if x <= 0.66 {
        return xatan(x);
    }
    if x > TAN3PIO8 {
        return std::f64::consts::FRAC_PI_2 - xatan(1.0 / x) + MOREBITS;
    }
    std::f64::consts::FRAC_PI_4 + xatan((x - 1.0) / (x + 1.0)) + 0.5 * MOREBITS
}

/// Go: math/atan.go:atan
pub fn atan(x: f64) -> f64 {
    if x == 0.0 {
        return x;
    }
    if x > 0.0 {
        return satan(x);
    }
    -satan(-x)
}

/// Go: math/asin.go:asin
pub fn asin(x: f64) -> f64 {
    if x == 0.0 {
        return x; // special case
    }
    let mut sign = false;
    let mut x = x;
    if x < 0.0 {
        x = -x;
        sign = true;
    }
    if x > 1.0 {
        return nan(); // special case
    }

    // Sqrt(1 - x*x): FMSUBD then FSQRTD.
    let mut temp = (-x).mul_add(x, 1.0).sqrt();
    if x > 0.7 {
        temp = std::f64::consts::FRAC_PI_2 - satan(temp / x);
    } else {
        temp = satan(x / temp);
    }

    if sign {
        temp = -temp;
    }
    temp
}

/// Go: math/asin.go:acos
pub fn acos(x: f64) -> f64 {
    std::f64::consts::FRAC_PI_2 - asin(x)
}

/// Go: math/atan2.go:atan2
pub fn atan2(y: f64, x: f64) -> f64 {
    use std::f64::consts::PI;
    if is_nan(y) || is_nan(x) {
        return nan();
    }
    if y == 0.0 {
        if x >= 0.0 && !signbit(x) {
            return copysign(0.0, y);
        }
        return copysign(PI, y);
    }
    if x == 0.0 {
        return copysign(PI / 2.0, y);
    }
    if is_inf(x, 0) {
        if is_inf(x, 1) {
            if is_inf(y, 0) {
                return copysign(PI / 4.0, y);
            }
            return copysign(0.0, y);
        }
        if is_inf(y, 0) {
            return copysign(3.0 * PI / 4.0, y);
        }
        return copysign(PI, y);
    }
    if is_inf(y, 0) {
        return copysign(PI / 2.0, y);
    }

    let q = atan(y / x);
    if x < 0.0 {
        if q <= 0.0 {
            return q + PI;
        }
        return q - PI;
    }
    q
}

/// Go: math/dim_arm64.s:archMin (math.Min on arm64): -Inf wins, then FMIND
/// (NaN if either operand is NaN; FMIN(+0, -0) = -0).
pub fn min(x: f64, y: f64) -> f64 {
    const NEG_INF: u64 = 0xfff0000000000000;
    if x.to_bits() == NEG_INF || y.to_bits() == NEG_INF {
        return f64::from_bits(NEG_INF);
    }
    // FMIND Fn=y, Fm=x: a NaN operand propagates (quieted), Fn first.
    if is_nan(y) {
        return f64::from_bits(y.to_bits() | 0x0008000000000000);
    }
    if is_nan(x) {
        return f64::from_bits(x.to_bits() | 0x0008000000000000);
    }
    if x == 0.0 && y == 0.0 {
        // FMIN(+0, -0) = -0.
        if signbit(x) {
            return x;
        }
        return y;
    }
    if x < y { x } else { y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(FOUR_OVER_PI.to_bits(), 0x3ff45f306dc9c883);
        assert_eq!(PI4A.to_bits(), 0x3fe921fb40000000);
        assert_eq!(PI4B.to_bits(), 0x3e64442d00000000);
        assert_eq!(PI4C.to_bits(), 0x3ce8469898cc5170);
        assert_eq!(SIN[0].to_bits(), 0x3de5d8fd1fd19ccd);
        assert_eq!(SIN[5].to_bits(), 0xbfc5555555555548);
        assert_eq!(COS[0].to_bits(), 0xbda8fa49a0861a9b);
        assert_eq!(COS[5].to_bits(), 0x3fa555555555554b);
    }

    #[test]
    fn go_math_special_cases() {
        assert_eq!(exp(0.0), 1.0);
        assert_eq!(exp(1.0).to_bits(), std::f64::consts::E.to_bits());
        assert!(is_nan(log(-1.0)));
        assert_eq!(log(0.0), f64::NEG_INFINITY);
        assert_eq!(pow(2.0, 10.0), 1024.0);
        assert_eq!(pow(-8.0, 1.0 / 3.0).to_bits(), nan().to_bits());
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(sincos(0.0), (0.0, 1.0));
        assert_eq!(max(1.0, f64::NEG_INFINITY), 1.0);
        assert_eq!(max(-0.0, 0.0).to_bits(), 0);
        assert_eq!(max(0.0, -0.0).to_bits(), 0);
        assert!(is_nan(max(1.0, f64::NAN)));
    }
}
