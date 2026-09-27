//! Go: minify/common.go — byte-level helpers shared by the minifiers (and by
//! the js minifier in `tdewolff-minify-js`).

use tdewolff_parse::strconv::{len_int, parse_int};
use tdewolff_parse::{
    DATA_URI_ENCODING_TABLE, GoBytes, GoError, Input, encode_url, equal_fold, is_whitespace,
    to_lower,
};

use crate::M;

static TEXT_MIME_BYTES: &[u8] = b"text/plain";
static CHARSET_ASCII_BYTES: &[u8] = b"charset=us-ascii";
static DATA_BYTES: &[u8] = b"data:";
static BASE64_BYTES: &[u8] = b";base64";

/// Go: common.go:Epsilon — the closest number to zero that is not
/// considered to be zero. (A package `var` in Go; never reassigned.)
pub const EPSILON: f64 = 0.00001;

/// Go: common.go:MaxInt (64-bit `int`).
pub const MAX_INT: i64 = i64::MAX;

/// Go: common.go:MinInt (64-bit `int`).
pub const MIN_INT: i64 = i64::MIN;

// Go: common.go:Mediatype
/// Minifies a given mediatype by removing all whitespace and lowercasing all
/// parts except strings (which may be case sensitive). Works in place.
pub fn mediatype(b: GoBytes) -> GoBytes {
    let mut j: usize = 0;
    let mut in_string = false;
    let (mut start, mut last_string) = (0usize, 0usize);
    for i in 0..b.len() {
        let c = b.at(i);
        if !in_string && is_whitespace(c) {
            if start != 0 {
                j += b.slice_from(j).copy_from(&b.slice(start, i));
            } else {
                j += i;
            }
            start = i + 1;
        } else if c == b'"' {
            in_string = !in_string;
            if in_string {
                if i - last_string < 1024 {
                    // ToLower may otherwise slow down minification greatly
                    to_lower(b.slice(last_string, i));
                }
            } else {
                last_string = j + (i + 1 - start);
            }
        }
    }
    if start != 0 {
        j += b.slice_from(j).copy_from(&b.slice_from(start));
        to_lower(b.slice(last_string, j));
        return b.slice_to(j);
    }
    to_lower(b.slice_from(last_string));
    b
}

const BASE64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Go `base64.StdEncoding.EncodedLen(n)` (padded).
pub fn base64_std_encoded_len(n: usize) -> usize {
    n.div_ceil(3) * 4
}

// Go: encoding/base64/base64.go:Encoding.Encode (StdEncoding, padded)
/// Base64-encodes `src` with Go's `StdEncoding`.
pub fn base64_std_encode(src: &[u8]) -> Vec<u8> {
    let mut dst = Vec::with_capacity(base64_std_encoded_len(src.len()));
    let n = (src.len() / 3) * 3;
    let mut si = 0;
    while si < n {
        // Convert 3x 8bit source bytes into 4 bytes
        let val = ((src[si] as u32) << 16) | ((src[si + 1] as u32) << 8) | (src[si + 2] as u32);
        dst.push(BASE64_STD[(val >> 18 & 0x3F) as usize]);
        dst.push(BASE64_STD[(val >> 12 & 0x3F) as usize]);
        dst.push(BASE64_STD[(val >> 6 & 0x3F) as usize]);
        dst.push(BASE64_STD[(val & 0x3F) as usize]);
        si += 3;
    }
    let remain = src.len() - si;
    if remain == 0 {
        return dst;
    }
    // Add the remaining small block
    let mut val = (src[si] as u32) << 16;
    if remain == 2 {
        val |= (src[si + 1] as u32) << 8;
    }
    dst.push(BASE64_STD[(val >> 18 & 0x3F) as usize]);
    dst.push(BASE64_STD[(val >> 12 & 0x3F) as usize]);
    if remain == 2 {
        dst.push(BASE64_STD[(val >> 6 & 0x3F) as usize]);
    } else {
        dst.push(b'=');
    }
    dst.push(b'=');
    dst
}

// Go: common.go:DataURI
/// Minifies a data URI and calls a minifier by the specified mediatype.
/// Specifications: <https://www.ietf.org/rfc/rfc2397.txt>. Percent-decoding
/// happens in place in `data_uri` (as in Go).
pub fn data_uri(m: &M, data_uri: GoBytes) -> GoBytes {
    let orig_data = tdewolff_parse::copy(&data_uri);
    let (mut mediatype, mut data) = match tdewolff_parse::data_uri(&data_uri) {
        Ok(v) => v,
        Err(_) => return data_uri,
    };

    (data, _) = m.bytes(mediatype.to_vec(), data);
    let base64_len = b";base64".len() + base64_std_encoded_len(data.len());
    let mut ascii_len = data.len();
    for c in data.iter() {
        if DATA_URI_ENCODING_TABLE[c as usize] {
            ascii_len += 2;
        }
        if ascii_len > base64_len {
            break;
        }
    }
    if orig_data.len() < base64_len && orig_data.len() < ascii_len {
        return orig_data;
    }
    if base64_len < ascii_len {
        let encoded = base64_std_encode(&data.to_vec());
        data = GoBytes::from_slice(&encoded); // make([]byte, base64Len-len(";base64"))
        mediatype = mediatype.append(BASE64_BYTES);
    } else {
        data = encode_url(data, &DATA_URI_ENCODING_TABLE);
    }
    if b"text/plain".len() <= mediatype.len()
        && equal_fold(&mediatype.slice_to(b"text/plain".len()), TEXT_MIME_BYTES)
    {
        mediatype = mediatype.slice_from(b"text/plain".len());
    }
    const N: usize = 17; // len(";charset=us-ascii")
    let mut i = 0;
    while i + N <= mediatype.len() {
        // must start with semicolon and be followed by end of mediatype or semicolon
        if mediatype.at(i) == b';'
            && equal_fold(&mediatype.slice(i + 1, i + N), CHARSET_ASCII_BYTES)
            && (i + N >= mediatype.len() || mediatype.at(i + N) == b';')
        {
            mediatype = mediatype
                .slice_to(i)
                .append_bytes(&mediatype.slice_from(i + N));
            break;
        }
        i += 1;
    }
    GoBytes::from_static(DATA_BYTES)
        .append_bytes(&mediatype)
        .append_byte(b',')
        .append_bytes(&data)
}

/// Index helpers for the Go-`int` arithmetic in [`decimal`] / [`number`].
#[inline]
fn at(num: &GoBytes, i: isize) -> u8 {
    assert!(i >= 0, "index out of range [{}]", i);
    num.at(i as usize)
}

#[inline]
fn set(num: &GoBytes, i: isize, c: u8) {
    assert!(i >= 0, "index out of range [{}]", i);
    num.set(i as usize, c)
}

#[inline]
fn sub(num: &GoBytes, lo: isize, hi: isize) -> GoBytes {
    assert!(
        lo >= 0 && hi >= 0,
        "slice bounds out of range [{}:{}]",
        lo,
        hi
    );
    num.slice(lo as usize, hi as usize)
}

#[inline]
fn sub_from(num: &GoBytes, lo: isize) -> GoBytes {
    assert!(lo >= 0, "slice bounds out of range [{}:]", lo);
    num.slice_from(lo as usize)
}

/// Go `copy(dst, src)` on two windows of the same slice.
#[inline]
fn gocopy(dst: GoBytes, src: GoBytes) -> isize {
    dst.copy_from(&src) as isize
}

// Go: common.go:Decimal
/// Minifies a given byte slice containing a decimal and removes superfluous
/// characters. It differs from [`number`] in that it does not parse
/// exponents. It does not parse or output exponents. `prec` is the number of
/// significant digits. When `prec` is zero it will keep all digits. Only
/// digits after the dot can be removed to reach the number of significant
/// digits. Very large number may thus have more significant digits. Works in
/// place.
pub fn decimal(num: GoBytes, prec: i64) -> GoBytes {
    if num.len() <= 1 {
        return num;
    }

    // omit first + and register mantissa start and end, whether it's negative and the exponent
    let mut neg = false;
    let mut start: isize = 0;
    let mut dot: isize = -1;
    let mut end: isize = num.len() as isize;
    if 0 < end && (num.at(0) == b'+' || num.at(0) == b'-') {
        if num.at(0) == b'-' {
            neg = true;
        }
        start += 1;
    }
    for i in start..num.len() as isize {
        if at(&num, i) == b'.' {
            dot = i;
            break;
        }
    }
    if dot == -1 {
        dot = end;
    }

    // trim leading zeros but leave at least one digit
    while start < end - 1 && at(&num, start) == b'0' {
        start += 1;
    }
    // trim trailing zeros
    let mut i = end - 1;
    while dot < i {
        if at(&num, i) != b'0' {
            end = i + 1;
            break;
        }
        i -= 1;
    }
    if i == dot {
        end = dot;
        if start == end {
            set(&num, start, b'0');
            return sub(&num, start, start + 1);
        }
    } else if start == end - 1 && at(&num, start) == b'0' {
        return sub(&num, start, end);
    }

    // apply precision
    let prec = prec as isize;
    if 0 < prec && dot <= start.wrapping_add(prec) {
        let mut prec_end = start.wrapping_add(prec).wrapping_add(1); // include dot
        if dot == start {
            // for numbers like .012
            let mut digit = start + 1;
            while digit < end && at(&num, digit) == b'0' {
                digit += 1;
            }
            prec_end = digit.wrapping_add(prec);
        }
        if prec_end < end {
            end = prec_end;

            // process either an increase from a lesser significant decimal (>= 5)
            // or remove trailing zeros after the dot, or both
            let mut i = end - 1;
            let mut inc = b'5' <= at(&num, end);
            while start < i {
                if i == dot {
                    // no-op
                } else if inc && at(&num, i) != b'9' {
                    set(&num, i, at(&num, i).wrapping_add(1));
                    inc = false;
                    break;
                } else if inc && i < dot {
                    // end inc for integer
                    set(&num, i, b'0');
                } else if !inc && (i < dot || at(&num, i) != b'0') {
                    break;
                }
                i -= 1;
            }
            if i < dot {
                end = dot;
            } else {
                end = i + 1;
            }

            if inc {
                if dot == start && end == start + 1 {
                    set(&num, start, b'1');
                } else if at(&num, start) == b'9' {
                    set(&num, start, b'1');
                    set(&num, start + 1, b'0');
                    end += 1;
                } else {
                    set(&num, start, at(&num, start).wrapping_add(1));
                }
            }
        }
    }

    if neg {
        start -= 1;
        set(&num, start, b'-');
    }
    sub(&num, start, end)
}

// Go: common.go:Number
/// Minifies a given byte slice containing a number and removes superfluous
/// characters. Works in place (it may write past the returned window inside
/// `num`).
pub fn number(num: GoBytes, prec: i64) -> GoBytes {
    if num.len() <= 1 {
        return num;
    }

    // omit first + and register mantissa start and end, whether it's negative and the exponent
    let mut neg = false;
    let mut start: isize = 0;
    let mut dot: isize = -1;
    let mut end: isize = num.len() as isize;
    let mut orig_exp: i64 = 0;
    if num.at(0) == b'+' || num.at(0) == b'-' {
        if num.at(0) == b'-' {
            neg = true;
        }
        start += 1;
    }
    {
        let mut k = start;
        while k < num.len() as isize {
            let c = at(&num, k);
            if c == b'.' {
                dot = k;
            } else if c == b'e' || c == b'E' {
                end = k;
                let mut i = k + 1;
                if i < num.len() as isize && at(&num, i) == b'+' {
                    i += 1;
                }
                let (tmp_orig_exp, n) = parse_int(&sub_from(&num, i));
                if 0 < n && MIN_INT <= tmp_orig_exp && tmp_orig_exp <= MAX_INT {
                    // range checks for when int is 32 bit
                    orig_exp = tmp_orig_exp;
                } else {
                    return num;
                }
                break;
            }
            k += 1;
        }
    }
    if dot == -1 {
        dot = end;
    }

    // trim leading zeros but leave at least one digit
    while start < end - 1 && at(&num, start) == b'0' {
        start += 1;
    }
    // trim trailing zeros
    let mut i = end - 1;
    while dot < i {
        if at(&num, i) != b'0' {
            end = i + 1;
            break;
        }
        i -= 1;
    }
    if i == dot {
        end = dot;
        if start == end {
            set(&num, start, b'0');
            return sub(&num, start, start + 1);
        }
    } else if start == end - 1 && at(&num, start) == b'0' {
        return sub(&num, start, end);
    }

    // apply precision
    let prec = prec as isize;
    if 0 < prec {
        //&& (dot <= start+prec || start+prec+1 < dot || 0 < origExp) { // don't minify 9 to 10, but do 999 to 1e3 and 99e1 to 1e3
        let mut prec_end = start.wrapping_add(prec);
        if dot == start {
            // for numbers like .012
            let mut digit = start + 1;
            while digit < end && at(&num, digit) == b'0' {
                digit += 1;
            }
            prec_end = digit.wrapping_add(prec);
        } else if dot < prec_end {
            // for numbers where precision will include the dot
            prec_end += 1;
        }
        if prec_end < end && (dot < end || 1 < ((dot - prec_end) as i64).wrapping_add(orig_exp)) {
            // do not minify 9=>10 or 99=>100 or 9e1=>1e2 (but 90), but 999=>1e3 and 99e1=>1e3
            end = prec_end;
            let mut inc = b'5' <= at(&num, end);
            if dot == end {
                inc = end + 1 < num.len() as isize && b'5' <= at(&num, end + 1);
            }
            if prec_end < dot {
                orig_exp = orig_exp.wrapping_add((dot - prec_end) as i64);
                dot = prec_end;
            }
            // process either an increase from a lesser significant decimal (>= 5)
            // and remove trailing zeros
            let mut i = end - 1;
            while start < i {
                if i == dot {
                    // no-op
                } else if inc && at(&num, i) != b'9' {
                    set(&num, i, at(&num, i).wrapping_add(1));
                    inc = false;
                    break;
                } else if !inc && at(&num, i) != b'0' {
                    break;
                }
                i -= 1;
            }
            end = i + 1;
            if end < dot {
                orig_exp = orig_exp.wrapping_add((dot - end) as i64);
                dot = end;
            }
            if inc {
                // single digit left
                if dot == start {
                    set(&num, start, b'1');
                    dot = start + 1;
                } else if at(&num, start) == b'9' {
                    set(&num, start, b'1');
                    orig_exp = orig_exp.wrapping_add(1);
                } else {
                    set(&num, start, at(&num, start).wrapping_add(1));
                }
            }
        }
    }

    // n is the number of significant digits
    // normExp would be the exponent if it were normalised (0.1 <= f < 1)
    let mut n: isize = 0;
    let mut norm_exp: i64 = 0;
    if start == end {
        return num; // no number before exponent
    } else if dot == start {
        let mut i = dot + 1;
        while i < end {
            if at(&num, i) != b'0' {
                n = end - i;
                norm_exp = (dot - i + 1) as i64;
                break;
            }
            i += 1;
        }
    } else if dot == end {
        norm_exp = (end - start) as i64;
        let mut i = end - 1;
        while start <= i {
            if at(&num, i) != b'0' {
                n = i + 1 - start;
                end = i + 1;
                break;
            }
            i -= 1;
        }
    } else {
        n = end - start - 1;
        norm_exp = (dot - start) as i64;
    }

    let n64 = n as i64;
    if orig_exp < 0
        && (norm_exp < MIN_INT.wrapping_sub(orig_exp)
            || norm_exp.wrapping_sub(n64) < MIN_INT.wrapping_sub(orig_exp))
        || 0 < orig_exp
            && (MAX_INT.wrapping_sub(orig_exp) < norm_exp
                || MAX_INT.wrapping_sub(orig_exp) < norm_exp.wrapping_sub(n64))
    {
        return num; // exponent overflow
    }
    norm_exp = norm_exp.wrapping_add(orig_exp);

    // intExp would be the exponent if it were an integer
    let mut int_exp: i64 = norm_exp.wrapping_sub(n64);
    let len_int_exp = len_int(int_exp) as isize;
    let len_norm_exp = len_int(norm_exp) as isize;

    // there are three cases to consider when printing the number
    // case 1: without decimals and with a positive exponent (large numbers: 5e4)
    // case 2: with decimals and with a negative exponent (small numbers with many digits: .123456e-4)
    // case 3: with decimals and without an exponent (around zero: 5.6)
    // case 4: without decimals and with a negative exponent (small numbers: 123456e-9)
    if n64 <= norm_exp {
        // case 1: print number with positive exponent
        if dot < end {
            // remove dot, either from the front or copy the smallest part
            if dot == start {
                start = end - n;
            } else if dot - start < end - dot - 1 {
                gocopy(sub_from(&num, start + 1), sub(&num, start, dot));
                start += 1;
            } else {
                gocopy(sub_from(&num, dot), sub(&num, dot + 1, end));
                end -= 1;
            }
        }
        if n64 + 3 <= norm_exp {
            set(&num, end, b'e');
            end += 1;
            let mut i = end + len_int_exp - 1;
            while end <= i {
                set(&num, i, ((int_exp % 10) as u8).wrapping_add(b'0'));
                int_exp /= 10;
                i -= 1;
            }
            end += len_int_exp;
        } else if n64 + 2 == norm_exp {
            set(&num, end, b'0');
            set(&num, end + 1, b'0');
            end += 2;
        } else if n64 + 1 == norm_exp {
            set(&num, end, b'0');
            end += 1;
        }
    } else if norm_exp < -3 && len_norm_exp < len_int_exp && dot < end {
        // case 2: print normalized number (0.1 <= f < 1)
        let zeroes = norm_exp.wrapping_neg().wrapping_add(orig_exp);
        if 0 < zeroes {
            let z = zeroes as isize;
            gocopy(sub_from(&num, start + 1), sub(&num, start + 1 + z, end));
            end -= z;
        } else if zeroes < 0 {
            gocopy(sub_from(&num, start + 1), sub(&num, start, dot));
            set(&num, start, b'.');
        } else {
            return num;
        }
        set(&num, end, b'e');
        set(&num, end + 1, b'-');
        end += 2;
        let mut i = end + len_norm_exp - 2;
        while end <= i {
            set(
                &num,
                i,
                ((norm_exp % 10) as u8).wrapping_neg().wrapping_add(b'0'),
            );
            norm_exp /= 10;
            i -= 1;
        }
        end += len_norm_exp - 1;
    } else if -(len_int_exp as i64) <= norm_exp {
        // case 3: print number without exponent
        let zeroes = norm_exp.wrapping_neg();
        if 0 < zeroes {
            // place dot at the front, adding zeroes after the dot
            let zeroes = zeroes as isize;
            let mut new_dot = end - n - zeroes - 1;
            if new_dot != dot {
                let d = start - new_dot;
                if 0 < d {
                    if dot < end {
                        // copy original digits after the dot towards the end
                        gocopy(sub_from(&num, dot + 1 + d), sub(&num, dot + 1, end));
                        if start < dot {
                            // copy original digits before the dot towards the end
                            gocopy(sub_from(&num, start + d + 1), sub(&num, start, dot));
                        }
                    } else if start < dot {
                        // copy original digits before the dot towards the end
                        gocopy(sub_from(&num, start + d), sub(&num, start, dot));
                    }
                    new_dot = start;
                    end += d;
                } else {
                    start += -d;
                }
                set(&num, new_dot, b'.');
                for i in 0..zeroes {
                    set(&num, new_dot + 1 + i, b'0');
                }
            }
        } else {
            // place dot in the middle of the number
            if end <= dot {
                // when input has no dot in it
                dot = end;
                end += 1;
            } else if dot == start {
                // when there are zeroes after the dot
                dot = end - n - 1;
                start = dot;
            }
            // move digits between dot and newDot towards the end
            let new_dot = start + norm_exp as isize;
            if dot < new_dot {
                gocopy(sub_from(&num, dot), sub(&num, dot + 1, new_dot + 1));
            } else if new_dot < dot {
                gocopy(sub_from(&num, new_dot + 1), sub(&num, new_dot, dot));
            }
            set(&num, new_dot, b'.');
        }
    } else {
        // case 4: print number with negative exponent
        // find new end, considering moving numbers to the front, removing the dot and increasing the length of the exponent
        let mut new_end = end;
        if dot == start {
            new_end = dot + n;
        } else {
            new_end -= 1;
        }
        new_end += 1 + len_int_exp;

        let mut exp = int_exp;
        let mut len_exp = len_int_exp;
        if new_end < num.len() as isize {
            // it saves space to convert the decimal to an integer and decrease the exponent
            if dot < end {
                if dot == start {
                    gocopy(sub_from(&num, start), sub(&num, end - n, end));
                    end = start + n;
                } else {
                    gocopy(sub_from(&num, dot), sub(&num, dot + 1, end));
                    end -= 1;
                }
            }
        } else {
            // it does not save space and will panic, so we revert to the original representation
            exp = orig_exp;
            len_exp = len_int(orig_exp) as isize;
        }
        set(&num, end, b'e');
        set(&num, end + 1, b'-');
        end += 2;
        let mut i = end + len_exp - 2;
        while end <= i {
            set(
                &num,
                i,
                ((exp % 10) as u8).wrapping_neg().wrapping_add(b'0'),
            );
            exp /= 10;
            i -= 1;
        }
        end += len_exp - 1;
    }

    if neg {
        start -= 1;
        set(&num, start, b'-');
    }
    sub(&num, start, end)
}

// Go: common.go:UpdateErrorPosition
/// Offsets the line and column of a `*parse.Error` returned by a nested
/// minifier by the position of `offset` in `input`; other errors are
/// returned unchanged.
pub fn update_error_position(err: GoError, input: &Input, offset: usize) -> GoError {
    if let GoError::Parse(mut perr) = err {
        let mut r = tdewolff_parse::buffer::Reader::new(input.bytes());
        let (line, column, _) = tdewolff_parse::position(Some(&mut r), offset as isize);
        perr.line += line - 1;
        perr.column += column - 1;
        return GoError::Parse(perr);
    }
    err
}
