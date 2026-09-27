// Port of go1.27.1 src/internal/strconv/atoi.go.

use super::Error;
use super::ftoa::lower;

/// IntSize is the size in bits of an int or uint value (Go int is 64-bit).
pub const INT_SIZE: i64 = 64;

// Go: internal/strconv/atoi.go:ParseUint
/// ParseUint is like ParseInt but for unsigned numbers.
///
/// A sign prefix is not permitted.
pub fn parse_uint(s: &[u8], base: i64, bit_size: i64) -> (u64, Option<Error>) {
    if s.is_empty() {
        return (0, Some(Error::Syntax));
    }

    let base0 = base == 0;
    let mut base = base;
    let mut bit_size = bit_size;

    let s0 = s;
    let mut s = s;
    if (2..=36).contains(&base) {
        // valid base; nothing to do
    } else if base == 0 {
        // Look for octal, hex prefix.
        base = 10;
        if s[0] == b'0' {
            if s.len() >= 3 && lower(s[1]) == b'b' {
                base = 2;
                s = &s[2..];
            } else if s.len() >= 3 && lower(s[1]) == b'o' {
                base = 8;
                s = &s[2..];
            } else if s.len() >= 3 && lower(s[1]) == b'x' {
                base = 16;
                s = &s[2..];
            } else {
                base = 8;
                s = &s[1..];
            }
        }
    } else {
        return (0, Some(Error::Base));
    }

    if bit_size == 0 {
        bit_size = INT_SIZE;
    } else if !(0..=64).contains(&bit_size) {
        return (0, Some(Error::BitSize));
    }

    // Cutoff is the smallest number such that cutoff*base > maxUint64.
    // Use compile-time constants for common cases.
    const MAX_UINT64: u64 = u64::MAX;
    let cutoff: u64 = match base {
        10 => MAX_UINT64 / 10 + 1,
        16 => MAX_UINT64 / 16 + 1,
        _ => MAX_UINT64 / base as u64 + 1,
    };

    let max_val: u64 = 1u64
        .checked_shl(bit_size as u32)
        .unwrap_or(0)
        .wrapping_sub(1);

    let mut underscores = false;
    let mut n: u64 = 0;
    for &c in s {
        let d: u8 = if c == b'_' && base0 {
            underscores = true;
            continue;
        } else if c.is_ascii_digit() {
            c - b'0'
        } else if b'a' <= lower(c) && lower(c) <= b'z' {
            lower(c) - b'a' + 10
        } else {
            return (0, Some(Error::Syntax));
        };

        if d as i64 >= base {
            return (0, Some(Error::Syntax));
        }

        if n >= cutoff {
            // n*base overflows
            return (max_val, Some(Error::Range));
        }
        n = n.wrapping_mul(base as u64);

        let n1 = n.wrapping_add(d as u64);
        if n1 < n || n1 > max_val {
            // n+d overflows
            return (max_val, Some(Error::Range));
        }
        n = n1;
    }

    if underscores && !underscore_ok(s0) {
        return (0, Some(Error::Syntax));
    }

    (n, None)
}

// Go: internal/strconv/atoi.go:ParseInt
/// ParseInt interprets a string s in the given base (0, 2 to 36) and
/// bit size (0 to 64) and returns the corresponding value i.
///
/// Returns Go's `(int64, error)` pair: on `Some(Error::Range)` the value is
/// the maximum magnitude integer of the appropriate bitSize and sign.
pub fn parse_int(s: &[u8], base: i64, bit_size: i64) -> (i64, Option<Error>) {
    if s.is_empty() {
        return (0, Some(Error::Syntax));
    }

    let mut s = s;
    let mut bit_size = bit_size;

    // Pick off leading sign.
    let mut neg = false;
    match s[0] {
        b'+' => s = &s[1..],
        b'-' => {
            s = &s[1..];
            neg = true;
        }
        _ => {}
    }

    // Convert unsigned and check range.
    let (un, err) = parse_uint(s, base, bit_size);
    if err.is_some() && err != Some(Error::Range) {
        return (0, err);
    }

    if bit_size == 0 {
        bit_size = INT_SIZE;
    }

    let cutoff: u64 = 1u64 << (bit_size - 1) as u32;
    if !neg && un >= cutoff {
        return ((cutoff - 1) as i64, Some(Error::Range));
    }
    if neg && un > cutoff {
        return ((cutoff as i64).wrapping_neg(), Some(Error::Range));
    }
    let mut n = un as i64;
    if neg {
        n = n.wrapping_neg();
    }
    (n, None)
}

// Go: internal/strconv/atoi.go:Atoi
/// Atoi is equivalent to ParseInt(s, 10, 0), converted to type int.
pub fn atoi(s: &[u8]) -> (i64, Option<Error>) {
    let s_len = s.len();
    if INT_SIZE == 64 && (0 < s_len && s_len < 19) {
        // Fast path for small integers that fit int type.
        let s0 = s;
        let mut s = s;
        if s[0] == b'-' || s[0] == b'+' {
            s = &s[1..];
            if s.is_empty() {
                return (0, Some(Error::Syntax));
            }
        }

        let mut n: i64 = 0;
        for &ch in s {
            let ch = ch.wrapping_sub(b'0');
            if ch > 9 {
                return (0, Some(Error::Syntax));
            }
            n = n * 10 + ch as i64;
        }
        if s0[0] == b'-' {
            n = -n;
        }
        return (n, None);
    }

    // Slow path for invalid, big, or underscored integers.
    parse_int(s, 10, 0)
}

// Go: internal/strconv/atoi.go:underscoreOK
/// underscoreOK reports whether the underscores in s are allowed.
/// Checking them in this one function lets all the parsers skip over them simply.
/// Underscore must appear only between digits or between a base prefix and a digit.
pub(crate) fn underscore_ok(s: &[u8]) -> bool {
    // saw tracks the last character (class) we saw:
    // ^ for beginning of number,
    // 0 for a digit or base prefix,
    // _ for an underscore,
    // ! for none of the above.
    let mut saw = b'^';
    let mut i = 0usize;
    let mut s = s;

    // Optional sign.
    if !s.is_empty() && (s[0] == b'-' || s[0] == b'+') {
        s = &s[1..];
    }

    // Optional base prefix.
    let mut hex = false;
    if s.len() >= 2
        && s[0] == b'0'
        && (lower(s[1]) == b'b' || lower(s[1]) == b'o' || lower(s[1]) == b'x')
    {
        i = 2;
        saw = b'0'; // base prefix counts as a digit for "underscore as digit separator"
        hex = lower(s[1]) == b'x';
    }

    // Number proper.
    while i < s.len() {
        // Digits are always okay.
        if s[i].is_ascii_digit() || hex && b'a' <= lower(s[i]) && lower(s[i]) <= b'f' {
            saw = b'0';
            i += 1;
            continue;
        }
        // Underscore must follow digit.
        if s[i] == b'_' {
            if saw != b'0' {
                return false;
            }
            saw = b'_';
            i += 1;
            continue;
        }
        // Underscore must also be followed by digit.
        if saw == b'_' {
            return false;
        }
        // Saw non-digit, non-underscore.
        saw = b'!';
        i += 1;
    }
    saw != b'_'
}
