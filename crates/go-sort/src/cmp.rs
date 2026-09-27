//! Port of Go's `cmp` package (go1.27.1 `src/cmp/cmp.go`) plus the
//! semantics of the `min`/`max` builtins (`src/runtime/minmax.go` for
//! strings; the arm64 `FMIN`/`FMAX` lowering for floats), as used by
//! `slices.Sort`, `slices.Min`, `slices.Max` and `slices.BinarySearch`.

/// Go `cmp.Ordered`: integers, floats and strings. Strings compare bytewise,
/// which is also Rust's `Ord` for `str`/`[u8]`.
pub trait Ordered {
    /// `x != x` (always false for non-floats).
    fn go_is_nan(&self) -> bool;
    /// Go `x < y`.
    fn go_lt(&self, other: &Self) -> bool;
    /// Go `x == y`.
    fn go_eq(&self, other: &Self) -> bool;
    /// Go builtin `min(x, y)`.
    fn go_min(self, other: Self) -> Self;
    /// Go builtin `max(x, y)`.
    fn go_max(self, other: Self) -> Self;
}

// Go: cmp/cmp.go:Less
/// Reports whether x is less than y. NaN is less than any non-NaN; -0.0 is
/// not less than (is equal to) 0.0.
pub fn less<T: Ordered + ?Sized>(x: &T, y: &T) -> bool {
    (x.go_is_nan() && !y.go_is_nan()) || x.go_lt(y)
}

// Go: cmp/cmp.go:Compare
/// Returns -1 if x < y, 0 if x == y, +1 if x > y, with NaN ordered first.
pub fn compare<T: Ordered + ?Sized>(x: &T, y: &T) -> i32 {
    let x_nan = x.go_is_nan();
    let y_nan = y.go_is_nan();
    if x_nan {
        if y_nan {
            return 0;
        }
        return -1;
    }
    if y_nan {
        return 1;
    }
    if x.go_lt(y) {
        return -1;
    }
    if y.go_lt(x) {
        return 1;
    }
    0
}

macro_rules! ordered_int {
    ($($t:ty),*) => {$(
        impl Ordered for $t {
            fn go_is_nan(&self) -> bool { false }
            fn go_lt(&self, other: &Self) -> bool { *self < *other }
            fn go_eq(&self, other: &Self) -> bool { *self == *other }
            fn go_min(self, other: Self) -> Self { if other < self { other } else { self } }
            fn go_max(self, other: Self) -> Self { if other > self { other } else { self } }
        }
    )*};
}
ordered_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

macro_rules! ordered_float {
    ($($t:ty, $quiet_bit:expr);*) => {$(
        impl Ordered for $t {
            #[allow(clippy::eq_op)]
            fn go_is_nan(&self) -> bool { *self != *self }
            fn go_lt(&self, other: &Self) -> bool { *self < *other }
            #[allow(clippy::float_cmp)]
            fn go_eq(&self, other: &Self) -> bool { *self == *other }
            // Go: builtin min on floats. On arm64 (the golden platform) the
            // compiler lowers it to FMIN{S,D} (ssagen minMax -> OpMin64F ->
            // FMIND arg0, arg1), not to runtime/minmax.go:fmin; the two only
            // differ in which NaN is returned. See arm64_process_nans.
            fn go_min(self, other: Self) -> Self {
                let (x, y) = (self, other);
                if let Some(n) = arm64_process_nans!($t, $quiet_bit, x, y) {
                    return n;
                }
                if x == 0.0 && y == 0.0 {
                    // min(-0, +0) = -0
                    return <$t>::from_bits(x.to_bits() | y.to_bits());
                }
                if x < y { x } else { y }
            }
            // Go: builtin max on floats (FMAX{S,D} on arm64, see go_min).
            fn go_max(self, other: Self) -> Self {
                let (x, y) = (self, other);
                if let Some(n) = arm64_process_nans!($t, $quiet_bit, x, y) {
                    return n;
                }
                if x == 0.0 && y == 0.0 {
                    // max(-0, +0) = +0
                    return <$t>::from_bits(x.to_bits() & y.to_bits());
                }
                if x > y { x } else { y }
            }
        }
    )*};
}

/// ARM `FPProcessNaNs(op1, op2)` with FPCR.DN = 0 (the darwin/arm64
/// default): a signaling NaN operand wins (quieted), op1 before op2; then a
/// quiet NaN operand, op1 before op2. Verified against the oracle
/// (`slices.Min`/`slices.Max` with NaN payloads).
macro_rules! arm64_process_nans {
    ($t:ty, $quiet_bit:expr, $x:expr, $y:expr) => {{
        let is_snan = |v: $t| v.is_nan() && v.to_bits() & $quiet_bit == 0;
        let quiet = |v: $t| <$t>::from_bits(v.to_bits() | $quiet_bit);
        if is_snan($x) {
            Some(quiet($x))
        } else if is_snan($y) {
            Some(quiet($y))
        } else if $x.is_nan() {
            Some($x)
        } else if $y.is_nan() {
            Some($y)
        } else {
            None
        }
    }};
}
ordered_float!(f32, 1u32 << 22; f64, 1u64 << 51);

macro_rules! ordered_str {
    ($($t:ty => |$v:ident| $bytes:expr),* $(,)?) => {$(
        impl Ordered for $t {
            fn go_is_nan(&self) -> bool { false }
            fn go_lt(&self, other: &Self) -> bool {
                let a: &[u8] = { let $v = self; $bytes };
                let b: &[u8] = { let $v = other; $bytes };
                a < b
            }
            fn go_eq(&self, other: &Self) -> bool {
                let a: &[u8] = { let $v = self; $bytes };
                let b: &[u8] = { let $v = other; $bytes };
                a == b
            }
            // Go: runtime/minmax.go:strmin
            fn go_min(self, other: Self) -> Self { if other.go_lt(&self) { other } else { self } }
            // Go: runtime/minmax.go:strmax
            fn go_max(self, other: Self) -> Self { if self.go_lt(&other) { other } else { self } }
        }
    )*};
}
ordered_str!(
    String => |v| v.as_bytes(),
    &str => |v| v.as_bytes(),
    Box<str> => |v| v.as_bytes(),
    std::sync::Arc<str> => |v| v.as_bytes(),
    Vec<u8> => |v| v.as_slice(),
    &[u8] => |v| v,
    Box<[u8]> => |v| v,
    std::sync::Arc<[u8]> => |v| v,
);
