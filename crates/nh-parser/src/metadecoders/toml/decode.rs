//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/decode.go` (scalar parsing).

use std::sync::Arc;

use go_value::{Location, Time};

use super::parser::{ParserError, new_parser_error};
use super::{LocalDate, LocalDateTime, LocalTime};

type PResult<'a, T> = Result<T, ParserError<'a>>;

/// Go's `math.NaN()` bits.
const GO_NAN_BITS: u64 = 0x7FF8_0000_0000_0001;

// Go: decode.go:parseInteger
pub(crate) fn parse_integer(b: &[u8]) -> PResult<'_, i64> {
    if b.len() > 2 && b[0] == b'0' {
        match b[1] {
            b'x' => return parse_int_hex(b),
            b'b' => return parse_int_bin(b),
            b'o' => return parse_int_oct(b),
            c => panic!(
                "invalid base '{}', should have been checked by scanIntOrFloat",
                c as char
            ),
        }
    }

    parse_int_dec(b)
}

// Go: decode.go:parseLocalDate
pub(crate) fn parse_local_date(b: &[u8]) -> PResult<'_, LocalDate> {
    // full-date      = date-fullyear "-" date-month "-" date-mday
    // date-fullyear  = 4DIGIT
    // date-month     = 2DIGIT  ; 01-12
    // date-mday      = 2DIGIT  ; 01-28, 01-29, 01-30, 01-31 based on month/year
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return Err(new_parser_error(
            b,
            "dates are expected to have the format YYYY-MM-DD",
        ));
    }

    let year = parse_decimal_digits(&b[0..4])?;
    let month = parse_decimal_digits(&b[5..7])?;
    let day = parse_decimal_digits(&b[8..10])?;

    if !is_valid_date(year, month, day) {
        return Err(new_parser_error(b, "impossible date"));
    }

    Ok(LocalDate { year, month, day })
}

// Go: decode.go:parseDecimalDigits
fn parse_decimal_digits(b: &[u8]) -> PResult<'_, i64> {
    let mut v: i64 = 0;

    for (i, &c) in b.iter().enumerate() {
        if !c.is_ascii_digit() {
            return Err(new_parser_error(&b[i..i + 1], "expected digit (0-9)"));
        }
        v *= 10;
        v += (c - b'0') as i64;
    }

    Ok(v)
}

// Go: decode.go:parseDateTime
pub(crate) fn parse_date_time(b: &[u8]) -> PResult<'_, Time> {
    // offset-date-time = full-date time-delim full-time
    // full-time      = partial-time time-offset
    // time-offset    = "Z" / time-numoffset
    // time-numoffset = ( "+" / "-" ) time-hour ":" time-minute

    let (dt, mut b) = parse_local_date_time(b)?;

    let zone: Arc<Location>;

    if b.is_empty() {
        // parser should have checked that when assigning the date time node
        panic!("date time should have a timezone");
    }

    if b[0] == b'Z' || b[0] == b'z' {
        b = &b[1..];
        zone = go_time::utc();
    } else {
        const DATE_TIME_BYTE_LEN: usize = 6;
        if b.len() != DATE_TIME_BYTE_LEN {
            return Err(new_parser_error(b, "invalid date-time timezone"));
        }
        let direction: i64 = match b[0] {
            b'-' => -1,
            b'+' => 1,
            _ => {
                return Err(new_parser_error(
                    &b[..1],
                    "invalid timezone offset character",
                ));
            }
        };

        if b[3] != b':' {
            return Err(new_parser_error(&b[3..4], "expected a : separator"));
        }

        let hours = parse_decimal_digits(&b[1..3])?;
        if hours > 23 {
            return Err(new_parser_error(&b[..1], "invalid timezone offset hours"));
        }

        let minutes = parse_decimal_digits(&b[4..6])?;
        if minutes > 59 {
            return Err(new_parser_error(&b[..1], "invalid timezone offset minutes"));
        }

        let seconds = direction * (hours * 3600 + minutes * 60);
        if seconds == 0 {
            zone = go_time::utc();
        } else {
            zone = go_time::fixed_zone("", seconds);
        }
        b = &b[DATE_TIME_BYTE_LEN..];
    }

    if !b.is_empty() {
        return Err(new_parser_error(
            b,
            "extra bytes at the end of the timezone",
        ));
    }

    let t = go_time::date(
        dt.date.year,
        go_time::Month(dt.date.month),
        dt.date.day,
        dt.time.hour,
        dt.time.minute,
        dt.time.second,
        dt.time.nanosecond,
        &zone,
    );

    Ok(t)
}

// Go: decode.go:parseLocalDateTime
pub(crate) fn parse_local_date_time(b: &[u8]) -> PResult<'_, (LocalDateTime, &[u8])> {
    const LOCAL_DATE_TIME_BYTE_MIN_LEN: usize = 11;
    if b.len() < LOCAL_DATE_TIME_BYTE_MIN_LEN {
        return Err(new_parser_error(
            b,
            "local datetimes are expected to have the format YYYY-MM-DDTHH:MM:SS[.NNNNNNNNN]",
        ));
    }

    let date = parse_local_date(&b[..10])?;

    let sep = b[10];
    if sep != b'T' && sep != b' ' && sep != b't' {
        return Err(new_parser_error(
            &b[10..11],
            "datetime separator is expected to be T or a space",
        ));
    }

    let (time, rest) = parse_local_time(&b[11..])?;

    Ok((LocalDateTime { date, time }, rest))
}

/// parseLocalTime is a bit different because it also returns the remaining
/// []byte that is didn't need. This is to allow parseDateTime to parse those
/// remaining bytes as a timezone.
// Go: decode.go:parseLocalTime
pub(crate) fn parse_local_time(b: &[u8]) -> PResult<'_, (LocalTime, &[u8])> {
    const NSPOW: [i64; 10] = [
        0,
        100_000_000,
        10_000_000,
        1_000_000,
        100_000,
        10_000,
        1_000,
        100,
        10,
        1,
    ];
    let mut t = LocalTime::default();

    // check if b matches to have expected format HH:MM:SS[.NNNNNN]
    const LOCAL_TIME_BYTE_LEN: usize = 8;
    if b.len() < LOCAL_TIME_BYTE_LEN {
        return Err(new_parser_error(
            b,
            "times are expected to have the format HH:MM:SS[.NNNNNN]",
        ));
    }

    t.hour = parse_decimal_digits(&b[0..2])?;

    if t.hour > 23 {
        return Err(new_parser_error(&b[0..2], "hour cannot be greater 23"));
    }
    if b[2] != b':' {
        return Err(new_parser_error(
            &b[2..3],
            "expecting colon between hours and minutes",
        ));
    }

    t.minute = parse_decimal_digits(&b[3..5])?;
    if t.minute > 59 {
        return Err(new_parser_error(&b[3..5], "minutes cannot be greater 59"));
    }
    if b[5] != b':' {
        return Err(new_parser_error(
            &b[5..6],
            "expecting colon between minutes and seconds",
        ));
    }

    t.second = parse_decimal_digits(&b[6..8])?;

    if t.second > 60 {
        return Err(new_parser_error(&b[6..8], "seconds cannot be greater 60"));
    }

    let b = &b[8..];

    if !b.is_empty() && b[0] == b'.' {
        let mut frac: i64 = 0;
        let mut precision: usize = 0;
        let mut digits = 0;

        for (i, &c) in b[1..].iter().enumerate() {
            if !c.is_ascii_digit() {
                if i == 0 {
                    return Err(new_parser_error(
                        &b[0..1],
                        "need at least one digit after fraction point",
                    ));
                }
                break;
            }
            digits += 1;

            const MAX_FRAC_PRECISION: usize = 9;
            if i >= MAX_FRAC_PRECISION {
                // go-toml allows decoding fractional seconds
                // beyond the supported precision of 9
                // digits. It truncates the fractional component
                // to the supported precision and ignores the
                // remaining digits.
                //
                // https://github.com/pelletier/go-toml/discussions/707
                continue;
            }

            frac *= 10;
            frac += (c - b'0') as i64;
            precision += 1;
        }

        if precision == 0 {
            return Err(new_parser_error(
                &b[..1],
                "nanoseconds need at least one digit",
            ));
        }

        t.nanosecond = frac * NSPOW[precision];
        t.precision = precision as i64;

        return Ok((t, &b[1 + digits..]));
    }
    Ok((t, b))
}

// Go: decode.go:parseFloat
pub(crate) fn parse_float(b: &[u8]) -> PResult<'_, f64> {
    if b.len() == 4
        && (b[0] == b'+' || b[0] == b'-')
        && b[1] == b'n'
        && b[2] == b'a'
        && b[3] == b'n'
    {
        return Ok(f64::from_bits(GO_NAN_BITS));
    }

    let cleaned = check_and_remove_underscores_floats(b)?;

    if cleaned[0] == b'.' {
        return Err(new_parser_error(b, "float cannot start with a dot"));
    }

    if cleaned[cleaned.len() - 1] == b'.' {
        return Err(new_parser_error(b, "float cannot end with a dot"));
    }

    let mut dot_already_seen = false;
    for (i, &c) in cleaned.iter().enumerate() {
        if c == b'.' {
            if dot_already_seen {
                return Err(new_parser_error(
                    &b[i..i + 1],
                    "float can have at most one decimal point",
                ));
            }
            if !cleaned[i - 1].is_ascii_digit() {
                return Err(new_parser_error(
                    &b[i - 1..i + 1],
                    "float decimal point must be preceded by a digit",
                ));
            }
            if !cleaned[i + 1].is_ascii_digit() {
                return Err(new_parser_error(
                    &b[i..i + 2],
                    "float decimal point must be followed by a digit",
                ));
            }
            dot_already_seen = true;
        }
    }

    let mut start = 0;
    if cleaned[0] == b'+' || cleaned[0] == b'-' {
        start = 1;
    }
    if cleaned[start] == b'0' && cleaned.len() > start + 1 && cleaned[start + 1].is_ascii_digit() {
        return Err(new_parser_error(
            b,
            "float integer part cannot have leading zeroes",
        ));
    }

    match go_strconv::parse_float(&cleaned, 64) {
        Ok(f) => Ok(f),
        Err(err) => Err(new_parser_error(b, format!("unable to parse float: {err}"))),
    }
}

// Go: decode.go:parseIntHex
fn parse_int_hex(b: &[u8]) -> PResult<'_, i64> {
    let cleaned = check_and_remove_underscores_integers(&b[2..])?;

    match go_strconv::parse_int(&cleaned, 16, 64) {
        Ok(i) => Ok(i),
        Err(err) => Err(new_parser_error(
            b,
            format!("couldn't parse hexadecimal number: {err}"),
        )),
    }
}

// Go: decode.go:parseIntOct
fn parse_int_oct(b: &[u8]) -> PResult<'_, i64> {
    let cleaned = check_and_remove_underscores_integers(&b[2..])?;

    match go_strconv::parse_int(&cleaned, 8, 64) {
        Ok(i) => Ok(i),
        Err(err) => Err(new_parser_error(
            b,
            format!("couldn't parse octal number: {err}"),
        )),
    }
}

// Go: decode.go:parseIntBin
fn parse_int_bin(b: &[u8]) -> PResult<'_, i64> {
    let cleaned = check_and_remove_underscores_integers(&b[2..])?;

    match go_strconv::parse_int(&cleaned, 2, 64) {
        Ok(i) => Ok(i),
        Err(err) => Err(new_parser_error(
            b,
            format!("couldn't parse binary number: {err}"),
        )),
    }
}

// Go: decode.go:isSign
fn is_sign(b: u8) -> bool {
    b == b'+' || b == b'-'
}

// Go: decode.go:parseIntDec
fn parse_int_dec(b: &[u8]) -> PResult<'_, i64> {
    let cleaned = check_and_remove_underscores_integers(b)?;

    let mut start_idx = 0;

    if is_sign(cleaned[0]) {
        start_idx += 1;
    }

    if cleaned.len() > start_idx + 1 && cleaned[start_idx] == b'0' {
        return Err(new_parser_error(
            b,
            "leading zero not allowed on decimal number",
        ));
    }

    match go_strconv::parse_int(&cleaned, 10, 64) {
        Ok(i) => Ok(i),
        Err(err) => Err(new_parser_error(
            b,
            format!("couldn't parse decimal number: {err}"),
        )),
    }
}

// Go: decode.go:checkAndRemoveUnderscoresIntegers
fn check_and_remove_underscores_integers(b: &[u8]) -> PResult<'_, Vec<u8>> {
    let mut start = 0;
    if b[start] == b'+' || b[start] == b'-' {
        start += 1;
    }

    if b.len() == start {
        return Ok(b.to_vec());
    }

    if b[start] == b'_' {
        return Err(new_parser_error(
            &b[start..start + 1],
            "number cannot start with underscore",
        ));
    }

    if b[b.len() - 1] == b'_' {
        return Err(new_parser_error(
            &b[b.len() - 1..],
            "number cannot end with underscore",
        ));
    }

    // fast path
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'_' {
            break;
        }
        i += 1;
    }
    if i == b.len() {
        return Ok(b.to_vec());
    }

    let mut before = false;
    let mut cleaned = b[..i].to_vec();

    i += 1;
    while i < b.len() {
        let c = b[i];
        if c == b'_' {
            if !before {
                return Err(new_parser_error(
                    &b[i - 1..i + 1],
                    "number must have at least one digit between underscores",
                ));
            }
            before = false;
        } else {
            before = true;
            cleaned.push(c);
        }
        i += 1;
    }

    Ok(cleaned)
}

// Go: decode.go:checkAndRemoveUnderscoresFloats
fn check_and_remove_underscores_floats(b: &[u8]) -> PResult<'_, Vec<u8>> {
    if b[0] == b'_' {
        return Err(new_parser_error(
            &b[0..1],
            "number cannot start with underscore",
        ));
    }

    if b[b.len() - 1] == b'_' {
        return Err(new_parser_error(
            &b[b.len() - 1..],
            "number cannot end with underscore",
        ));
    }

    // fast path
    if !b.contains(&b'_') {
        return Ok(b.to_vec());
    }

    let mut before = false;
    let mut cleaned = Vec::with_capacity(b.len());

    for i in 0..b.len() {
        let c = b[i];

        match c {
            b'_' => {
                if !before {
                    return Err(new_parser_error(
                        &b[i - 1..i + 1],
                        "number must have at least one digit between underscores",
                    ));
                }
                if i < b.len() - 1 && (b[i + 1] == b'e' || b[i + 1] == b'E') {
                    return Err(new_parser_error(
                        &b[i + 1..i + 2],
                        "cannot have underscore before exponent",
                    ));
                }
                before = false;
            }
            b'+' | b'-' => {
                // signed exponents
                cleaned.push(c);
                before = false;
            }
            b'e' | b'E' => {
                if i < b.len() - 1 && b[i + 1] == b'_' {
                    return Err(new_parser_error(
                        &b[i + 1..i + 2],
                        "cannot have underscore after exponent",
                    ));
                }
                cleaned.push(c);
            }
            b'.' => {
                if i < b.len() - 1 && b[i + 1] == b'_' {
                    return Err(new_parser_error(
                        &b[i + 1..i + 2],
                        "cannot have underscore after decimal point",
                    ));
                }
                if i > 0 && b[i - 1] == b'_' {
                    return Err(new_parser_error(
                        &b[i - 1..i],
                        "cannot have underscore before decimal point",
                    ));
                }
                cleaned.push(c);
            }
            _ => {
                before = true;
                cleaned.push(c);
            }
        }
    }

    Ok(cleaned)
}

/// isValidDate checks if a provided date is a date that exists.
// Go: decode.go:isValidDate
fn is_valid_date(year: i64, month: i64, day: i64) -> bool {
    month > 0 && month < 13 && day > 0 && day <= days_in(month, year)
}

/// daysBefore[m] counts the number of days in a non-leap year
/// before month m begins. There is an entry for m=12, counting
/// the number of days before January of next year (365).
const DAYS_BEFORE: [i32; 13] = [
    0,
    31,
    31 + 28,
    31 + 28 + 31,
    31 + 28 + 31 + 30,
    31 + 28 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30 + 31,
];

// Go: decode.go:daysIn
fn days_in(m: i64, year: i64) -> i64 {
    if m == 2 && is_leap(year) {
        return 29;
    }
    (DAYS_BEFORE[m as usize] - DAYS_BEFORE[m as usize - 1]) as i64
}

// Go: decode.go:isLeap
fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
