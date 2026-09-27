// Port of go1.27.1 src/internal/strconv/deps.go.
//
// Implementations to avoid importing other dependencies (package math),
// plus the float32<->float64 conversions Go performs implicitly.

// Go: internal/strconv/deps.go:float64frombits
#[inline]
pub(crate) fn float64frombits(b: u64) -> f64 {
    f64::from_bits(b)
}

// Go: internal/strconv/deps.go:float32frombits
#[inline]
pub(crate) fn float32frombits(b: u32) -> f32 {
    f32::from_bits(b)
}

// Go: internal/strconv/deps.go:float64bits
#[inline]
pub(crate) fn float64bits(f: f64) -> u64 {
    f.to_bits()
}

// Go: internal/strconv/deps.go:float32bits
#[inline]
pub(crate) fn float32bits(f: f32) -> u32 {
    f.to_bits()
}

// Go: internal/strconv/deps.go:inf
pub(crate) fn inf(sign: i64) -> f64 {
    let v: u64 = if sign >= 0 {
        0x7FF0000000000000
    } else {
        0xFFF0000000000000
    };
    float64frombits(v)
}

// Go: internal/strconv/deps.go:isNaN
#[allow(dead_code)]
#[inline]
pub(crate) fn is_nan(f: f64) -> bool {
    f != f
}

// Go: internal/strconv/deps.go:nan
pub(crate) fn nan() -> f64 {
    float64frombits(0x7FF8000000000001)
}

/// Go `float32(f)` for a float64 `f`, as executed by FCVT on darwin/arm64
/// (FPCR.DN = 0): IEEE round-to-nearest-even for numbers; a NaN keeps its
/// sign and the top 22 bits of its payload and gets the quiet bit set.
///
/// Rust `as f32` compiles to the same instruction, but Rust does not
/// guarantee NaN payloads for casts (and constant folding may differ), so
/// NaNs are converted explicitly.
#[inline]
pub fn f64_to_f32(f: f64) -> f32 {
    if f.is_nan() {
        let b = f.to_bits();
        let sign = ((b >> 63) as u32) << 31;
        let frac = ((b >> 29) & 0x3F_FFFF) as u32; // fraction bits 50..29
        return f32::from_bits(sign | 0x7F80_0000 | 0x0040_0000 | frac);
    }
    f as f32
}

/// Go `float64(f)` for a float32 `f`, as executed by FCVT on darwin/arm64:
/// exact for numbers; a NaN keeps sign and payload (shifted up by 29 bits)
/// and gets the quiet bit set.
#[inline]
pub fn f32_to_f64(f: f32) -> f64 {
    if f.is_nan() {
        let b = f.to_bits() as u64;
        let sign = (b >> 31) << 63;
        let frac = (b & 0x7F_FFFF) << 29;
        return f64::from_bits(sign | 0x7FF0_0000_0000_0000 | 0x0008_0000_0000_0000 | frac);
    }
    f as f64
}
