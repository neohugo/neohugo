//! Go: math/pow10.go (go1.27.1). `Pow10` is a product/quotient of two table
//! entries, which is *not* always the correctly rounded 10^n, so it is ported
//! verbatim instead of using `10f64.powi(n)`.

static POW10TAB: [f64; 32] = [
    1e00, 1e01, 1e02, 1e03, 1e04, 1e05, 1e06, 1e07, 1e08, 1e09, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
    1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28, 1e29, 1e30, 1e31,
];

static POW10POSTAB32: [f64; 10] = [
    1e00, 1e32, 1e64, 1e96, 1e128, 1e160, 1e192, 1e224, 1e256, 1e288,
];

static POW10NEGTAB32: [f64; 11] = [
    1e-00, 1e-32, 1e-64, 1e-96, 1e-128, 1e-160, 1e-192, 1e-224, 1e-256, 1e-288, 1e-320,
];

// Go: math/pow10.go:Pow10
/// Returns 10**n. `Pow10(n) = 0` for `n < -323`, `+Inf` for `n > 308`.
pub fn pow10(n: i64) -> f64 {
    if (0..=308).contains(&n) {
        return POW10POSTAB32[(n as u64 / 32) as usize] * POW10TAB[(n as u64 % 32) as usize];
    }
    if (-323..0).contains(&n) {
        let m = n.wrapping_neg() as u64;
        return POW10NEGTAB32[(m / 32) as usize] / POW10TAB[(m % 32) as usize];
    }
    if n > 0 {
        return f64::INFINITY;
    }
    0.0
}
