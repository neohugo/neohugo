//! Faithful port of Go's `strconv` package as shipped in go1.27.1
//! (`src/strconv` + `src/internal/strconv`), for byte-for-byte parity with
//! the Go build of neohugo.
//!
//! Go strings are byte strings, so every parsing/quoting entry point accepts
//! `impl AsRef<[u8]>` (`&str`, `&[u8]`, `&String`, `&Vec<u8>` ...). Go `int`
//! is `i64`, Go `rune` is [`Rune`] (`i32`, so invalid code points can be
//! passed like in Go). Formatting output is always ASCII and is returned as
//! `String`; quoting output is always valid UTF-8 and is returned as
//! `String`; unquoting output may be arbitrary bytes and is `Vec<u8>`.
//!
//! The crate root mirrors package `strconv` (errors are [`NumError`], like
//! Go's `*strconv.NumError`). The [`internal`] module mirrors package
//! `internal/strconv`, which keeps Go's `(value, error)` shape for callers
//! that need the value returned alongside an error (e.g. `±Inf` with
//! `ErrRange`, or the clamped value from `ParseInt`).
//!
//! All float formatting and parsing is Go's own algorithm (unrounded
//! scaling, research.swtch.com/fp, plus the multiprecision `decimal`
//! fallback), not Rust's float formatting.

// Lints that fight a faithful line-by-line port.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::identity_op)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::eq_op)]
#![allow(clippy::same_item_push)]
#![allow(clippy::manual_is_multiple_of)]
// Go conditions kept verbatim, e.g. `!(len(in) > 0 && in[0] == quote)`.
#![allow(clippy::nonminimal_bool)]

pub mod internal;
mod isprint;
mod quote;
mod utf8;

pub use quote::{
    append_quote, append_quote_rune, append_quote_rune_to_ascii, append_quote_rune_to_graphic,
    append_quote_to_ascii, append_quote_to_graphic, can_backquote, is_graphic, is_print, quote,
    quote_rune, quote_rune_to_ascii, quote_rune_to_graphic, quote_to_ascii, quote_to_graphic,
    quoted_prefix, unquote, unquote_char,
};

/// Go `rune` (`int32`). Values outside the valid Unicode range are allowed,
/// exactly as in Go.
pub type Rune = i32;

/// IntSize is the size in bits of an int or uint value.
pub const INT_SIZE: i64 = internal::INT_SIZE;

// Go: strconv/number.go:ErrRange, ErrSyntax, baseError, bitSizeError
/// The `Err` of a [`NumError`] (and the bare error returned by
/// [`unquote`] / [`unquote_char`] / [`quoted_prefix`], which is always
/// [`Error::Syntax`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Error {
    /// `strconv.ErrRange`: "value out of range".
    Range,
    /// `strconv.ErrSyntax`: "invalid syntax".
    Syntax,
    /// `errors.New("invalid base " + Itoa(base))`.
    Base(i64),
    /// `errors.New("invalid bit size " + Itoa(bitSize))`.
    BitSize(i64),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Range => f.write_str("value out of range"),
            Error::Syntax => f.write_str("invalid syntax"),
            Error::Base(b) => write!(f, "invalid base {}", itoa(*b)),
            Error::BitSize(b) => write!(f, "invalid bit size {}", itoa(*b)),
        }
    }
}

impl std::error::Error for Error {}

// Go: strconv/number.go:NumError
/// A NumError records a failed conversion.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct NumError {
    /// the failing function (ParseBool, ParseInt, ParseUint, ParseFloat, ParseComplex, Atoi)
    pub func: &'static str,
    /// the input
    pub num: Vec<u8>,
    /// the reason the conversion failed (e.g. ErrRange, ErrSyntax, etc.)
    pub err: Error,
}

impl std::fmt::Display for NumError {
    // Go: strconv/number.go:(*NumError).Error
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "strconv.{}: parsing {}: {}",
            self.func,
            quote(&self.num),
            self.err
        )
    }
}

impl std::error::Error for NumError {
    // Go: strconv/number.go:(*NumError).Unwrap
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.err)
    }
}

// Go: strconv/number.go:toError
/// toError converts from internal/strconv.Error to the error guaranteed by this package's APIs.
fn to_error(
    func: &'static str,
    s: &[u8],
    base: i64,
    bit_size: i64,
    err: internal::Error,
) -> NumError {
    let err = match err {
        internal::Error::Syntax => Error::Syntax,
        internal::Error::Range => Error::Range,
        internal::Error::Base => Error::Base(base),
        internal::Error::BitSize => Error::BitSize(bit_size),
    };
    NumError {
        func,
        num: s.to_vec(),
        err,
    }
}

// Go: strconv/number.go:ParseBool
/// ParseBool returns the boolean value represented by the string.
/// It accepts 1, t, T, TRUE, true, True, 0, f, F, FALSE, false, False.
/// Any other value returns an error.
pub fn parse_bool(str: impl AsRef<[u8]>) -> Result<bool, NumError> {
    let str = str.as_ref();
    match internal::parse_bool(str) {
        (_, Some(err)) => Err(to_error("ParseBool", str, 0, 0, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:FormatBool
/// FormatBool returns "true" or "false" according to the value of b.
pub fn format_bool(b: bool) -> &'static str {
    internal::format_bool(b)
}

// Go: strconv/number.go:AppendBool
/// AppendBool appends "true" or "false", according to the value of b, to dst.
pub fn append_bool(dst: &mut Vec<u8>, b: bool) {
    internal::append_bool(dst, b)
}

// Go: strconv/number.go:ParseComplex
/// ParseComplex converts the string s to a complex number `(real, imag)`
/// with the precision specified by bitSize: 64 for complex64, or 128 for complex128.
/// When bitSize=64, the result still has type complex128, but it will be
/// convertible to complex64 without changing its value.
///
/// On error, Go also returns a value (±Inf components for ErrRange); use
/// [`internal::parse_complex`] when that value is needed.
pub fn parse_complex(s: impl AsRef<[u8]>, bit_size: i64) -> Result<(f64, f64), NumError> {
    let s = s.as_ref();
    match internal::parse_complex(s, bit_size) {
        (_, Some(err)) => Err(to_error("ParseComplex", s, 0, bit_size, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:ParseFloat
/// ParseFloat converts the string s to a floating-point number
/// with the precision specified by bitSize: 32 for float32, or 64 for float64.
/// When bitSize=32, the result still has type float64, but it will be
/// convertible to float32 without changing its value.
///
/// ParseFloat accepts decimal and hexadecimal floating-point numbers
/// as defined by the Go syntax for floating-point literals.
/// If s is well-formed and near a valid floating-point number,
/// ParseFloat returns the nearest floating-point number rounded
/// using IEEE754 unbiased rounding.
///
/// If s is not syntactically well-formed, ParseFloat returns err.Err = ErrSyntax.
///
/// If s is syntactically well-formed but is more than 1/2 ULP
/// away from the largest floating point number of the given size,
/// Go returns f = ±Inf, err.Err = ErrRange (use [`internal::parse_float`]
/// to get the ±Inf value together with the error).
///
/// ParseFloat recognizes the string "NaN", and the (possibly signed) strings "Inf" and "Infinity"
/// as their respective special floating point values. It ignores case when matching.
pub fn parse_float(s: impl AsRef<[u8]>, bit_size: i64) -> Result<f64, NumError> {
    let s = s.as_ref();
    match internal::parse_float(s, bit_size) {
        (_, Some(err)) => Err(to_error("ParseFloat", s, 0, bit_size, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:ParseUint
/// ParseUint is like [`parse_int`] but for unsigned numbers.
///
/// A sign prefix is not permitted.
pub fn parse_uint(s: impl AsRef<[u8]>, base: i64, bit_size: i64) -> Result<u64, NumError> {
    let s = s.as_ref();
    match internal::parse_uint(s, base, bit_size) {
        (_, Some(err)) => Err(to_error("ParseUint", s, base, bit_size, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:ParseInt
/// ParseInt interprets a string s in the given base (0, 2 to 36) and
/// bit size (0 to 64) and returns the corresponding value i.
///
/// The string may begin with a leading sign: "+" or "-".
///
/// If the base argument is 0, the true base is implied by the string's
/// prefix following the sign (if present): 2 for "0b", 8 for "0" or "0o",
/// 16 for "0x", and 10 otherwise. Also, for argument base 0 only,
/// underscore characters are permitted as defined by the Go syntax for
/// integer literals.
///
/// The bitSize argument specifies the integer type
/// that the result must fit into. Bit sizes 0, 8, 16, 32, and 64
/// correspond to int, int8, int16, int32, and int64.
/// If bitSize is below 0 or above 64, an error is returned.
///
/// On ErrRange Go also returns the maximum magnitude integer of the
/// appropriate bitSize and sign; use [`internal::parse_int`] to get it.
pub fn parse_int(s: impl AsRef<[u8]>, base: i64, bit_size: i64) -> Result<i64, NumError> {
    let s = s.as_ref();
    match internal::parse_int(s, base, bit_size) {
        (_, Some(err)) => Err(to_error("ParseInt", s, base, bit_size, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:Atoi
/// Atoi is equivalent to ParseInt(s, 10, 0), converted to type int.
pub fn atoi(s: impl AsRef<[u8]>) -> Result<i64, NumError> {
    let s = s.as_ref();
    match internal::atoi(s) {
        (_, Some(err)) => Err(to_error("Atoi", s, 0, 0, err)),
        (x, None) => Ok(x),
    }
}

// Go: strconv/number.go:FormatComplex
/// FormatComplex converts the complex number c = `(real, imag)` to a string of the
/// form (a+bi) where a and b are the real and imaginary parts,
/// formatted according to the format fmt and precision prec.
///
/// Panics (like Go) if bitSize is not 64 or 128, and (deviation, see
/// [`format_float`]) for a non-ASCII `fmt` byte with a finite component.
pub fn format_complex(c: (f64, f64), fmt: u8, prec: i64, bit_size: i64) -> String {
    internal::format_complex(c, fmt, prec, bit_size)
}

// Go: strconv/number.go:FormatFloat
/// FormatFloat converts the floating-point number f to a string,
/// according to the format fmt and precision prec. It rounds the
/// result assuming that the original was obtained from a floating-point
/// value of bitSize bits (32 for float32, 64 for float64).
///
/// The format fmt is one of
///   - 'b' (-ddddp±ddd, a binary exponent),
///   - 'e' (-d.dddde±dd, a decimal exponent),
///   - 'E' (-d.ddddE±dd, a decimal exponent),
///   - 'f' (-ddd.dddd, no exponent),
///   - 'g' ('e' for large exponents, 'f' otherwise),
///   - 'G' ('E' for large exponents, 'f' otherwise),
///   - 'x' (-0xd.ddddp±ddd, a hexadecimal fraction and binary exponent), or
///   - 'X' (-0Xd.ddddP±ddd, a hexadecimal fraction and binary exponent).
///
/// The precision prec controls the number of digits (excluding the exponent)
/// printed by the 'e', 'E', 'f', 'g', 'G', 'x', and 'X' formats.
/// For 'e', 'E', 'f', 'x', and 'X', it is the number of digits after the decimal point.
/// For 'g' and 'G' it is the maximum number of significant digits (trailing
/// zeros are removed).
/// The special precision -1 uses the smallest number of digits
/// necessary such that ParseFloat will return f exactly.
/// The exponent is written as a decimal integer;
/// for all formats other than 'b', it will be at least two digits.
///
/// Panics (like Go) if bitSize is not 32 or 64. Also panics (deviation: Go
/// returns the invalid-UTF-8 string `"%" + fmt`) for a finite value with a
/// non-ASCII `fmt` byte; [`append_float`] gives Go's bytes for those.
pub fn format_float(f: f64, fmt: u8, prec: i64, bit_size: i64) -> String {
    internal::format_float(f, fmt, prec, bit_size)
}

// Go: strconv/number.go:AppendFloat
/// AppendFloat appends the string form of the floating-point number f,
/// as generated by [`format_float`], to dst.
pub fn append_float(dst: &mut Vec<u8>, f: f64, fmt: u8, prec: i64, bit_size: i64) {
    internal::append_float(dst, f, fmt, prec, bit_size)
}

// Go: strconv/number.go:FormatUint
/// FormatUint returns the string representation of i in the given base,
/// for 2 <= base <= 36. The result uses the lower-case letters 'a' to 'z'
/// for digit values >= 10. Panics (like Go) on an illegal base.
pub fn format_uint(i: u64, base: i64) -> String {
    internal::format_uint(i, base)
}

// Go: strconv/number.go:FormatInt
/// FormatInt returns the string representation of i in the given base,
/// for 2 <= base <= 36. The result uses the lower-case letters 'a' to 'z'
/// for digit values >= 10. Panics (like Go) on an illegal base.
pub fn format_int(i: i64, base: i64) -> String {
    internal::format_int(i, base)
}

// Go: strconv/number.go:Itoa
/// Itoa is equivalent to FormatInt(int64(i), 10).
pub fn itoa(i: i64) -> String {
    internal::itoa(i)
}

// Go: strconv/number.go:AppendInt
/// AppendInt appends the string form of the integer i,
/// as generated by [`format_int`], to dst.
pub fn append_int(dst: &mut Vec<u8>, i: i64, base: i64) {
    internal::append_int(dst, i, base)
}

// Go: strconv/number.go:AppendUint
/// AppendUint appends the string form of the unsigned integer i,
/// as generated by [`format_uint`], to dst.
pub fn append_uint(dst: &mut Vec<u8>, i: u64, base: i64) {
    internal::append_uint(dst, i, base)
}
