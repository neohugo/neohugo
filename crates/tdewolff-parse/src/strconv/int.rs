//! Go: parse/strconv/int.go

use crate::gobytes::{ByteView, GoBytes};

// Go: parse/strconv/int.go:ParseInt
/// Parses a byte-slice and returns the integer it represents and the number
/// of bytes consumed (0 on overflow or when there are no digits).
pub fn parse_int<B: ByteView + ?Sized>(b: &B) -> (i64, usize) {
    let mut i = 0;
    let mut neg = false;
    if b.len() > 0 && (b.at(0) == b'+' || b.at(0) == b'-') {
        neg = b.at(0) == b'-';
        i += 1;
    }
    let start = i;
    let mut n: u64 = 0;
    const LIM: u64 = 9223372036854775808; // uint64(-math.MinInt64)
    while i < b.len() {
        let c = b.at(i);
        if c.is_ascii_digit() {
            if LIM / 10 < n || LIM - ((c - b'0') as u64) < n.wrapping_mul(10) {
                return (0, 0);
            }
            n *= 10;
            n += (c - b'0') as u64;
        } else {
            break;
        }
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    if !neg && (i64::MAX as u64) < n {
        return (0, 0);
    } else if neg {
        return ((n as i64).wrapping_neg(), i);
    }
    (n as i64, i)
}

// Go: parse/strconv/int.go:ParseUint
/// Parses a byte-slice and returns the unsigned integer it represents.
pub fn parse_uint<B: ByteView + ?Sized>(b: &B) -> (u64, usize) {
    let mut i = 0;
    let mut n: u64 = 0;
    while i < b.len() {
        let c = b.at(i);
        if c.is_ascii_digit() {
            if u64::MAX / 10 < n || u64::MAX - ((c - b'0') as u64) < n.wrapping_mul(10) {
                return (0, 0);
            }
            n *= 10;
            n += (c - b'0') as u64;
        } else {
            break;
        }
        i += 1;
    }
    (n, i)
}

// Go: parse/strconv/int.go:AppendInt
/// Appends an int64 in decimal.
pub fn append_int(b: GoBytes, mut num: i64) -> GoBytes {
    if num == 0 {
        return b.append(b"0");
    } else if num == i64::MIN {
        return b.append(b"-9223372036854775808");
    }

    // resize byte slice
    let (mut i, n) = (b.len(), len_int(num));
    let b = if b.cap() < i + n {
        b.append(&vec![0u8; n])
    } else {
        b.slice_to(i + n)
    };

    // print sign
    if num < 0 {
        num = -num;
        b.set(i, b'-');
    }
    i += n - 1;

    // print number
    while num != 0 {
        b.set(i, (num % 10) as u8 + b'0');
        num /= 10;
        i = i.wrapping_sub(1);
    }
    b
}

// Go: parse/strconv/int.go:LenInt
/// Returns the written length of an integer.
pub fn len_int(i: i64) -> usize {
    if i < 0 {
        if i == i64::MIN {
            return 20;
        }
        return 1 + len_uint((-i) as u64);
    }
    len_uint(i as u64)
}

// Go: parse/strconv/int.go:LenUint
/// Returns the written length of an unsigned integer.
pub fn len_uint(i: u64) -> usize {
    match i {
        _ if i < 10 => 1,
        _ if i < 100 => 2,
        _ if i < 1000 => 3,
        _ if i < 10000 => 4,
        _ if i < 100000 => 5,
        _ if i < 1000000 => 6,
        _ if i < 10000000 => 7,
        _ if i < 100000000 => 8,
        _ if i < 1000000000 => 9,
        _ if i < 10000000000 => 10,
        _ if i < 100000000000 => 11,
        _ if i < 1000000000000 => 12,
        _ if i < 10000000000000 => 13,
        _ if i < 100000000000000 => 14,
        _ if i < 1000000000000000 => 15,
        _ if i < 10000000000000000 => 16,
        _ if i < 100000000000000000 => 17,
        _ if i < 1000000000000000000 => 18,
        _ if i < 10000000000000000000 => 19,
        _ => 20,
    }
}

pub(crate) static INT64_POW10: [i64; 19] = [
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
];
