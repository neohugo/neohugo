// Port of go1.27.1 src/internal/strconv/atoc.go.

use super::Error;
use super::atof::parse_float_prefix;

// Go: internal/strconv/atoc.go:ParseComplex
/// ParseComplex converts the string s to a complex number
/// with the precision specified by bitSize: 64 for complex64, or 128 for complex128.
/// When bitSize=64, the result still has type complex128, but it will be
/// convertible to complex64 without changing its value.
///
/// The complex value is returned as `(real, imag)`.
///
/// The number represented by s must be of the form N, Ni, or N±Ni, where N stands
/// for a floating-point number as recognized by ParseFloat, and i is the imaginary
/// component. If the second N is unsigned, a + sign is required between the two components
/// as indicated by the ±. If the second N is NaN, only a + sign is accepted.
/// The form may be parenthesized and cannot contain any spaces.
/// The resulting complex number consists of the two components converted by ParseFloat.
pub fn parse_complex(s: &[u8], bit_size: i64) -> ((f64, f64), Option<Error>) {
    let mut size = 64;
    if bit_size == 64 {
        size = 32; // complex64 uses float32 parts
    }

    let mut s = s;

    // Remove parentheses, if any.
    if s.len() >= 2 && s[0] == b'(' && s[s.len() - 1] == b')' {
        s = &s[1..s.len() - 1];
    }

    let mut pending: Option<Error> = None; // pending range error, or nil

    // Read real part (possibly imaginary part if followed by 'i').
    let (re, n, err) = parse_float_prefix(s, size);
    if let Some(e) = err {
        if e != Error::Range {
            return ((0.0, 0.0), err);
        }
        pending = err;
    }
    s = &s[n..];

    // If we have nothing left, we're done.
    if s.is_empty() {
        return ((re, 0.0), pending);
    }

    // Otherwise, look at the next character.
    match s[0] {
        b'+' => {
            // Consume the '+' to avoid an error if we have "+NaNi", but
            // do this only if we don't have a "++" (don't hide that error).
            if s.len() > 1 && s[1] != b'+' {
                s = &s[1..];
            }
        }
        b'-' => {
            // ok
        }
        b'i' => {
            // If 'i' is the last character, we only have an imaginary part.
            if s.len() == 1 {
                return ((0.0, re), pending);
            }
            return ((0.0, 0.0), Some(Error::Syntax));
        }
        _ => {
            return ((0.0, 0.0), Some(Error::Syntax));
        }
    }

    // Read imaginary part.
    let (im, n, err) = parse_float_prefix(s, size);
    if let Some(e) = err {
        if e != Error::Range {
            return ((0.0, 0.0), err);
        }
        pending = err;
    }
    s = &s[n..];
    if s != b"i" {
        return ((0.0, 0.0), Some(Error::Syntax));
    }
    ((re, im), pending)
}
