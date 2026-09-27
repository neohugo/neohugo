//! Go: parse/strconv/number.go

use crate::gobytes::{ByteView, GoBytes};
use crate::strconv::int::len_int;
use crate::utf8::{append_rune, decode_rune, rune_len};

// Go: parse/strconv/number.go:ParseNumber
/// Parses a byte-slice and returns the number it represents, the amount of
/// decimals and the number of bytes consumed.
pub fn parse_number<B: ByteView + ?Sized>(
    b: &B,
    group_sym: i32,
    dec_sym: i32,
) -> (i64, isize, usize) {
    let (mut n, mut dec) = (0usize, 0isize);
    let mut sign: i64 = 1;
    let mut num: i64 = 0;
    let mut has_decimals = false;
    if 0 < b.len() && b.at(0) == b'-' {
        sign = -1;
        n += 1;
    }
    while n < b.len() {
        if b.at(n).is_ascii_digit() {
            let digit = sign * (b.at(n) - b'0') as i64;
            if sign == 1 && (i64::MAX / 10 < num || i64::MAX - digit < num * 10) {
                break;
            } else if sign == -1 && (num < i64::MIN / 10 || num * 10 < i64::MIN - digit) {
                break;
            }
            num *= 10;
            num += digit;
            if has_decimals {
                dec += 1;
            }
            n += 1;
        } else {
            let (r, size) = decode_rune(b, n);
            if !has_decimals && (r == group_sym || r == dec_sym) {
                if r == dec_sym {
                    has_decimals = true;
                }
                n += size;
            } else {
                break;
            }
        }
    }
    (num, dec, n)
}

/// Go `utf8.EncodeRune(b[i:], r)` into a GoBytes.
fn encode_rune_at(b: &GoBytes, i: usize, r: i32) {
    let mut tmp = Vec::with_capacity(4);
    append_rune(&mut tmp, r);
    for (k, c) in tmp.into_iter().enumerate() {
        b.set(i + k, c);
    }
}

// Go: parse/strconv/number.go:AppendNumber
/// Appends an int64 formatted as a number with the given number of decimal
/// digits, digit grouping and decimal symbol.
pub fn append_number(
    b: GoBytes,
    mut num: i64,
    mut dec: isize,
    group_size: isize,
    mut group_sym: i32,
    mut dec_sym: i32,
) -> GoBytes {
    if dec < 0 {
        dec = 0;
    }
    if rune_len(group_sym) == -1 {
        group_sym = b'.' as i32;
    }
    if rune_len(dec_sym) == -1 {
        dec_sym = b',' as i32;
    }

    let mut sign: i64 = 1;
    if num < 0 {
        sign = -1;
    }

    // calculate size
    let mut n = len_int(num) as isize;
    if sign == -1 {
        n -= 1; // ignore minux sign, add later
    }
    if dec < n && 0 < group_size && group_sym != 0 {
        n += rune_len(group_sym) * (n - dec - 1) / group_size;
    }
    if 0 < dec {
        if n <= dec {
            n = 1 + dec; // zero and decimals
        }
        n += rune_len(dec_sym);
    }
    if sign == -1 {
        n += 1;
    }

    // resize byte slice
    let mut i = b.len() as isize;
    let b = if (b.cap() as isize) < i + n {
        b.append(&vec![0u8; n as usize])
    } else {
        b.slice_to((i + n) as usize)
    };

    // print fractional-part
    i += n - 1;
    if 0 < dec {
        while 0 < dec {
            let c = ((sign * (num % 10)) as u8).wrapping_add(b'0');
            num /= 10;
            b.set(i as usize, c);
            dec -= 1;
            i -= 1;
        }
        i -= rune_len(dec_sym);
        encode_rune_at(&b, (i + 1) as usize, dec_sym);
    }

    // print integer-part
    if num == 0 {
        b.set(i as usize, b'0');
        if sign == -1 {
            b.set((i - 1) as usize, b'-');
        }
        return b;
    }
    let mut j: isize = 0;
    while num != 0 {
        if 0 < group_size && group_sym != 0 && 0 < j && j % group_size == 0 {
            i -= rune_len(group_sym);
            encode_rune_at(&b, (i + 1) as usize, group_sym);
        }

        let c = ((sign * (num % 10)) as u8).wrapping_add(b'0');
        num /= 10;
        b.set(i as usize, c);
        i -= 1;
        j += 1;
    }

    if sign == -1 {
        b.set(i as usize, b'-');
    }
    b
}
