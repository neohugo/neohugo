// Port of go1.27.1 src/internal/strconv/ctoa.go.

use super::ftoa::{append_float, float_string};

// Go: internal/strconv/ctoa.go:FormatComplex
/// FormatComplex converts the complex number c = (real, imag) to a string of the
/// form (a+bi) where a and b are the real and imaginary parts,
/// formatted according to the format fmt and precision prec.
///
/// The format fmt and precision prec have the same meaning as in FormatFloat.
/// It rounds the result assuming that the original was obtained from a complex
/// value of bitSize bits, which must be 64 for complex64 and 128 for complex128.
///
/// Panics (like Go) if bitSize is not 64 or 128, and (deviation, see
/// `format_float`) for a non-ASCII `fmt` byte with a finite component;
/// `append_complex` returns Go's bytes then.
pub fn format_complex(c: (f64, f64), fmt: u8, prec: i64, bit_size: i64) -> String {
    let mut buf = Vec::with_capacity(64);
    append_complex(&mut buf, c, fmt, prec, bit_size);
    float_string(buf)
}

// Go: internal/strconv/ctoa.go:AppendComplex
/// AppendComplex appends the result of FormatComplex to dst.
pub fn append_complex(dst: &mut Vec<u8>, c: (f64, f64), fmt: u8, prec: i64, bit_size: i64) {
    if bit_size != 64 && bit_size != 128 {
        panic!("invalid bitSize");
    }
    let bit_size = bit_size >> 1; // complex64 uses float32 internally

    dst.push(b'(');
    append_float(dst, c.0, fmt, prec, bit_size);
    let i = dst.len();
    append_float(dst, c.1, fmt, prec, bit_size);
    // Check if imaginary part has a sign. If not, add one.
    if dst[i] != b'+' && dst[i] != b'-' {
        dst.insert(i, b'+');
    }
    dst.extend_from_slice(b"i)");
}
