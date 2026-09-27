//! The small pieces of the Go standard library (go1.27.1) that yaml.v2
//! decoding and the metadecoders layer depend on:
//!
//! - `strconv.ParseInt`/`ParseUint`/`ParseFloat`/`FormatFloat`/`Quote`
//!   (thin wrappers over the `go-strconv` crate),
//! - `time.Parse` for the four layouts of yaml.v2's `allowedTimestampFormats`
//!   (only success/failure is needed),
//! - `encoding/base64.StdEncoding.DecodeString`.
//!
//! The time and base64 pieces are local ports (only success/failure of the
//! four yaml.v2 layouts is needed); see PORTING.md.

// ---------------------------------------------------------------------------
// strconv, via the go-strconv crate (go1.27.1 strconv port)

/// Error kinds of `strconv` parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumError {
    Syntax,
    Range,
}

fn num_err(e: go_strconv::NumError) -> NumError {
    match e.err {
        go_strconv::Error::Range => NumError::Range,
        _ => NumError::Syntax,
    }
}

/// Go: `strconv.ParseUint(s, base, 64)`.
pub(crate) fn parse_uint(s: &[u8], base: u32) -> Result<u64, NumError> {
    go_strconv::parse_uint(s, base as i64, 64).map_err(num_err)
}

/// Go: `strconv.ParseInt(s, base, 64)`.
pub(crate) fn parse_int(s: &[u8], base: u32) -> Result<i64, NumError> {
    go_strconv::parse_int(s, base as i64, 64).map_err(num_err)
}

/// Go: `strconv.ParseFloat(s, 64)`.
pub(crate) fn parse_float(s: &[u8]) -> Result<f64, NumError> {
    go_strconv::parse_float(s, 64).map_err(num_err)
}

/// Go: `strconv.FormatFloat(f, 'f', -1, 64)`.
pub(crate) fn format_float_f(f: f64) -> String {
    go_strconv::format_float(f, b'f', -1, 64)
}

/// Go: `strconv.FormatFloat(f, 'g', -1, 64)` (also what `%v` prints).
pub(crate) fn format_float_g(f: f64) -> String {
    go_strconv::format_float(f, b'g', -1, 64)
}

/// Go: `strconv.Quote`.
pub(crate) fn quote(s: &[u8]) -> String {
    go_strconv::quote(s)
}

// ---------------------------------------------------------------------------
// time.Parse for yaml.v2's allowedTimestampFormats
// Go: $GOROOT/src/time/format.go:parse (the std chunks used by the layouts)

/// A chunk of a pre-tokenised layout (Go: nextStdChunk output).
#[derive(Clone, Copy)]
enum Chunk {
    Lit(&'static [u8]),
    LongYear,
    NumMonth,
    Day,
    Hour,
    Minute,
    Second,
    FracSecond9,
    Iso8601ColonTz,
}

/// The four layouts of yaml.v2 resolve.go:allowedTimestampFormats, as Go's
/// nextStdChunk splits them.
const TIMESTAMP_LAYOUTS: [&[Chunk]; 4] = [
    // "2006-1-2T15:4:5.999999999Z07:00"
    &[
        Chunk::LongYear,
        Chunk::Lit(b"-"),
        Chunk::NumMonth,
        Chunk::Lit(b"-"),
        Chunk::Day,
        Chunk::Lit(b"T"),
        Chunk::Hour,
        Chunk::Lit(b":"),
        Chunk::Minute,
        Chunk::Lit(b":"),
        Chunk::Second,
        Chunk::FracSecond9,
        Chunk::Iso8601ColonTz,
    ],
    // "2006-1-2t15:4:5.999999999Z07:00"
    &[
        Chunk::LongYear,
        Chunk::Lit(b"-"),
        Chunk::NumMonth,
        Chunk::Lit(b"-"),
        Chunk::Day,
        Chunk::Lit(b"t"),
        Chunk::Hour,
        Chunk::Lit(b":"),
        Chunk::Minute,
        Chunk::Lit(b":"),
        Chunk::Second,
        Chunk::FracSecond9,
        Chunk::Iso8601ColonTz,
    ],
    // "2006-1-2 15:4:5.999999999"
    &[
        Chunk::LongYear,
        Chunk::Lit(b"-"),
        Chunk::NumMonth,
        Chunk::Lit(b"-"),
        Chunk::Day,
        Chunk::Lit(b" "),
        Chunk::Hour,
        Chunk::Lit(b":"),
        Chunk::Minute,
        Chunk::Lit(b":"),
        Chunk::Second,
        Chunk::FracSecond9,
    ],
    // "2006-1-2"
    &[
        Chunk::LongYear,
        Chunk::Lit(b"-"),
        Chunk::NumMonth,
        Chunk::Lit(b"-"),
        Chunk::Day,
    ],
];

// Go: time/format.go:isDigit
fn t_is_digit(s: &[u8], i: usize) -> bool {
    s.len() > i && s[i].is_ascii_digit()
}

// Go: time/format.go:getnum
fn getnum(s: &[u8], fixed: bool) -> Option<(i64, &[u8])> {
    if !t_is_digit(s, 0) {
        return None;
    }
    if !t_is_digit(s, 1) {
        if fixed {
            return None;
        }
        return Some(((s[0] - b'0') as i64, &s[1..]));
    }
    Some((((s[0] - b'0') as i64) * 10 + (s[1] - b'0') as i64, &s[2..]))
}

// Go: time/format.go:cutspace
fn cutspace(mut s: &[u8]) -> &[u8] {
    while !s.is_empty() && s[0] == b' ' {
        s = &s[1..];
    }
    s
}

// Go: time/format.go:skip
fn tskip<'a>(value: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let mut value = value;
    let mut prefix = prefix;
    while !prefix.is_empty() {
        if prefix[0] == b' ' {
            if !value.is_empty() && value[0] != b' ' {
                return None;
            }
            prefix = cutspace(prefix);
            value = cutspace(value);
            continue;
        }
        if value.is_empty() || value[0] != prefix[0] {
            return None;
        }
        prefix = &prefix[1..];
        value = &value[1..];
    }
    Some(value)
}

// Go: time/format.go:leadingInt
fn leading_int(s: &[u8]) -> Option<(u64, &[u8])> {
    let mut x: u64 = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        if x > (1u64 << 63) / 10 {
            return None;
        }
        x = x * 10 + (c - b'0') as u64;
        if x > 1u64 << 63 {
            return None;
        }
        i += 1;
    }
    Some((x, &s[i..]))
}

// Go: time/format.go:atoi
fn tatoi(s: &[u8]) -> Option<i64> {
    let mut s = s;
    let mut neg = false;
    if !s.is_empty() && (s[0] == b'-' || s[0] == b'+') {
        neg = s[0] == b'-';
        s = &s[1..];
    }
    let (q, rem) = leading_int(s)?;
    let mut x = q as i64;
    if !rem.is_empty() {
        return None;
    }
    if neg {
        x = -x;
    }
    Some(x)
}

// Go: time/format.go:parseNanoseconds — only validity matters here.
fn parse_nanoseconds_ok(value: &[u8], nbytes: usize) -> bool {
    if !(value[0] == b'.' || value[0] == b',') {
        return false;
    }
    let mut nbytes = nbytes;
    let mut value = value;
    if nbytes > 10 {
        value = &value[..10];
        nbytes = 10;
    }
    match tatoi(&value[1..nbytes]) {
        Some(ns) => ns >= 0,
        None => false,
    }
}

// Go: time/time.go:isLeap
fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// Go: time/time.go:daysIn
fn days_in(month: i64, year: i64) -> i64 {
    if month == 2 && is_leap(year) {
        return 29;
    }
    const DAYS_BEFORE: [i64; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];
    DAYS_BEFORE[month as usize] - DAYS_BEFORE[month as usize - 1]
}

/// Go: `time.Parse(layout, value)` succeeds, for one of yaml.v2's layouts.
fn time_parse_ok(layout: &[Chunk], value: &[u8]) -> bool {
    let mut value = value;
    let mut year: i64 = 0;
    let mut month: i64 = -1;
    let mut day: i64 = -1;
    let mut i = 0;
    while i < layout.len() {
        let chunk = layout[i];
        i += 1;
        match chunk {
            Chunk::Lit(p) => match tskip(value, p) {
                Some(v) => value = v,
                None => return false,
            },
            Chunk::LongYear => {
                if value.len() < 4 || !t_is_digit(value, 0) {
                    return false;
                }
                let p = &value[..4];
                value = &value[4..];
                match tatoi(p) {
                    Some(y) => year = y,
                    None => return false,
                }
            }
            Chunk::NumMonth => match getnum(value, false) {
                Some((m, rest)) => {
                    month = m;
                    value = rest;
                    if month <= 0 || 12 < month {
                        return false; // month out of range
                    }
                }
                None => return false,
            },
            Chunk::Day => match getnum(value, false) {
                Some((d, rest)) => {
                    day = d;
                    value = rest;
                }
                None => return false,
            },
            Chunk::Hour => match getnum(value, false) {
                Some((h, rest)) => {
                    value = rest;
                    if !(0..24).contains(&h) {
                        return false;
                    }
                }
                None => return false,
            },
            Chunk::Minute => match getnum(value, false) {
                Some((m, rest)) => {
                    value = rest;
                    if !(0..60).contains(&m) {
                        return false;
                    }
                }
                None => return false,
            },
            Chunk::Second => {
                match getnum(value, false) {
                    Some((sec, rest)) => {
                        value = rest;
                        if !(0..60).contains(&sec) {
                            return false;
                        }
                    }
                    None => return false,
                }
                // Special case: do we have a fractional second but no
                // fractional second in the format?
                if value.len() >= 2
                    && (value[0] == b'.' || value[0] == b',')
                    && t_is_digit(value, 1)
                {
                    let next_is_frac = matches!(layout.get(i), Some(Chunk::FracSecond9));
                    if !next_is_frac {
                        let mut n = 2;
                        while n < value.len() && t_is_digit(value, n) {
                            n += 1;
                        }
                        if !parse_nanoseconds_ok(value, n) {
                            return false;
                        }
                        value = &value[n..];
                    }
                }
            }
            Chunk::FracSecond9 => {
                if value.len() < 2
                    || !(value[0] == b'.' || value[0] == b',')
                    || !value[1].is_ascii_digit()
                {
                    // Fractional second omitted.
                    continue;
                }
                // Take any number of digits, even more than asked for,
                // because it is what the stdSecond case would do.
                let mut k = 0;
                while k + 1 < value.len() && value[k + 1].is_ascii_digit() {
                    k += 1;
                }
                if !parse_nanoseconds_ok(value, 1 + k) {
                    return false;
                }
                value = &value[1 + k..];
            }
            Chunk::Iso8601ColonTz => {
                if !value.is_empty() && value[0] == b'Z' {
                    value = &value[1..];
                    continue;
                }
                if value.len() < 6 {
                    return false;
                }
                if value[3] != b':' {
                    return false;
                }
                let sign = value[0];
                let hour = &value[1..3];
                let min = &value[4..6];
                value = &value[6..];
                let (hr, mm) = match (getnum(hour, true), getnum(min, true)) {
                    (Some((h, _)), Some((m, _))) => (h, m),
                    _ => return false,
                };
                if hr > 24 || mm > 60 {
                    return false;
                }
                if sign != b'+' && sign != b'-' {
                    return false;
                }
            }
        }
    }
    if !value.is_empty() {
        return false; // extra text
    }
    if month < 0 {
        month = 1;
    }
    if day < 0 {
        day = 1;
    }
    // Validate the day of the month.
    if day < 1 || day > days_in(month, year) {
        return false;
    }
    true
}

// Go: yaml.v2 resolve.go:parseTimestamp (only whether it succeeds).
pub(crate) fn parse_timestamp_ok(s: &[u8]) -> bool {
    // Quick check: all date formats start with YYYY-.
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        i += 1;
    }
    if i != 4 || i == s.len() || s[i] != b'-' {
        return false;
    }
    TIMESTAMP_LAYOUTS
        .iter()
        .any(|layout| time_parse_ok(layout, s))
}

// ---------------------------------------------------------------------------
// encoding/base64.StdEncoding.DecodeString
// Go: $GOROOT/src/encoding/base64/base64.go (decodeQuantum; the assemble
// fast paths produce identical results)

fn b64_decode_map(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => 0xFF,
    }
}

/// Go: `base64.StdEncoding.DecodeString(s)`; `None` on CorruptInputError.
pub(crate) fn base64_std_decode(src: &[u8]) -> Option<Vec<u8>> {
    let mut dst: Vec<u8> = Vec::with_capacity(src.len() / 4 * 3);
    let mut si = 0usize;
    // Go: Decode loop over decodeQuantum.
    while si < src.len() {
        // decodeQuantum
        let mut dbuf = [0u8; 4];
        let mut dlen = 4usize;
        let mut j = 0usize;
        let mut err = false;
        while j < 4 {
            if src.len() == si {
                if j == 0 {
                    return Some(dst);
                }
                // j == 1 or padChar != NoPadding (StdEncoding has padding)
                return None;
            }
            let inp = src[si];
            si += 1;

            let out = b64_decode_map(inp);
            if out != 0xFF {
                dbuf[j] = out;
                j += 1;
                continue;
            }

            if inp == b'\n' || inp == b'\r' {
                continue;
            }

            if inp != b'=' {
                return None;
            }

            // We've reached the end and there's padding
            match j {
                0 | 1 => {
                    // incorrect padding
                    return None;
                }
                2 => {
                    // "==" is expected, the first "=" is already consumed.
                    // skip over newlines
                    while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
                        si += 1;
                    }
                    if si == src.len() {
                        // not enough padding
                        return None;
                    }
                    if src[si] != b'=' {
                        // incorrect padding
                        return None;
                    }
                    si += 1;
                }
                _ => {}
            }

            // skip over newlines
            while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
                si += 1;
            }
            if si < src.len() {
                // trailing garbage
                err = true;
            }
            dlen = j;
            break;
        }

        // Convert 4x 6bit source bytes into 3 bytes
        let val: u32 = (dbuf[0] as u32) << 18
            | (dbuf[1] as u32) << 12
            | (dbuf[2] as u32) << 6
            | dbuf[3] as u32;
        let (b0, b1, b2) = ((val >> 16) as u8, (val >> 8) as u8, val as u8);
        match dlen {
            4 => {
                dst.push(b0);
                dst.push(b1);
                dst.push(b2);
            }
            3 => {
                dst.push(b0);
                dst.push(b1);
            }
            2 => {
                dst.push(b0);
            }
            _ => {}
        }
        if err {
            return None;
        }
    }
    Some(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_cases() {
        assert_eq!(parse_int(b"0x1F", 0), Ok(31));
        assert_eq!(parse_int(b"-0b101", 0), Ok(-5));
        assert_eq!(parse_int(b"0o17", 0), Ok(15));
        assert_eq!(parse_int(b"017", 0), Ok(15));
        assert_eq!(parse_int(b"08", 0), Err(NumError::Syntax));
        assert_eq!(parse_int(b"9223372036854775808", 0), Err(NumError::Range));
        assert_eq!(parse_int(b"-9223372036854775808", 0), Ok(i64::MIN));
        assert_eq!(parse_uint(b"18446744073709551615", 0), Ok(u64::MAX));
        assert_eq!(parse_uint(b"18446744073709551616", 0), Err(NumError::Range));
        assert_eq!(parse_int(b"-101", 2), Ok(-5));
        assert_eq!(parse_int(b"1_000", 0), Ok(1000));
        assert_eq!(parse_int(b"1__000", 0), Err(NumError::Syntax));
    }

    #[test]
    fn parse_float_cases() {
        assert_eq!(parse_float(b".5"), Ok(0.5));
        assert_eq!(parse_float(b".5_5"), Ok(0.55));
        assert_eq!(parse_float(b"._5"), Err(NumError::Syntax));
        assert_eq!(parse_float(b".5e999"), Err(NumError::Range));
        assert_eq!(parse_float(b".5e-999"), Ok(0.0));
        assert_eq!(parse_float(b"."), Err(NumError::Syntax));
    }

    #[test]
    fn format_float_cases() {
        assert_eq!(format_float_f(1.5), "1.5");
        assert_eq!(format_float_f(1e20), "100000000000000000000");
        assert_eq!(format_float_f(1e-7), "0.0000001");
        assert_eq!(format_float_f(-0.0), "-0");
        assert_eq!(format_float_f(123.0), "123");
        assert_eq!(format_float_g(1e6), "1e+06");
        assert_eq!(format_float_g(123456.0), "123456");
        assert_eq!(format_float_g(1234567.0), "1.234567e+06");
        assert_eq!(format_float_g(0.0001), "0.0001");
        assert_eq!(format_float_g(0.00001), "1e-05");
        assert_eq!(format_float_g(1e100), "1e+100");
    }

    #[test]
    fn timestamps() {
        assert!(parse_timestamp_ok(b"2001-12-14"));
        assert!(parse_timestamp_ok(b"2001-12-14t21:59:43.10-05:00"));
        assert!(parse_timestamp_ok(b"2001-12-14 21:59:43.10"));
        assert!(parse_timestamp_ok(b"2001-12-14   21:59:43"));
        assert!(parse_timestamp_ok(b"2015-02-24T18:19:39.123456789-03:00"));
        assert!(!parse_timestamp_ok(b"2001-12-14 21:59:43.10 -5"));
        assert!(!parse_timestamp_ok(b"2001-02-29"));
        assert!(parse_timestamp_ok(b"2000-02-29"));
        assert!(!parse_timestamp_ok(b"2001-13-01"));
        assert!(!parse_timestamp_ok(b"2001-12-14T21:59:43"));
    }

    #[test]
    fn base64() {
        assert_eq!(base64_std_decode(b"aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(base64_std_decode(b"aGVs\nbG8="), Some(b"hello".to_vec()));
        assert_eq!(base64_std_decode(b"aGVsbG8"), None);
        assert_eq!(base64_std_decode(b"=="), None);
        assert_eq!(base64_std_decode(b""), Some(vec![]));
    }
}
