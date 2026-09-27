//! Port of `$GOROOT/src/time/format_rfc3339.go` (go1.27.1): the RFC 3339
//! fast paths used by `Format`, `Parse` and the Marshal/Unmarshal methods.

use std::sync::Arc;

use go_value::{Location, Time};

use crate::format::{
    ParseError, RFC3339, STD_FRAC_SECOND9, append_int, append_nano, is_digit, parse,
    parse_nanoseconds,
};
use crate::time::{
    Month, abs_clock, abs_days, add_sec, date, days_date, days_in, locabs, nanosecond, set_loc,
    unix_sec,
};
use crate::zoneinfo::{fixed_zone, local, lookup, utc};

// Go: format_rfc3339.go:Time.appendFormatRFC3339
pub(crate) fn append_format_rfc3339(t: &Time, b: &mut Vec<u8>, nanos: bool) {
    let (_, offset, abs) = locabs(t);

    // Format date.
    let (year, month, day) = days_date(abs_days(abs));
    append_int(b, year, 4);
    b.push(b'-');
    append_int(b, month.0, 2);
    b.push(b'-');
    append_int(b, day, 2);

    b.push(b'T');

    // Format time.
    let (hour, min, sec) = abs_clock(abs);
    append_int(b, hour, 2);
    b.push(b':');
    append_int(b, min, 2);
    b.push(b':');
    append_int(b, sec, 2);

    if nanos {
        // stdFracSecond(stdFracSecond9, 9, '.')
        let std = STD_FRAC_SECOND9 | (9 << 16);
        append_nano(b, nanosecond(t), std);
    }

    if offset == 0 {
        b.push(b'Z');
        return;
    }

    // Format zone.
    let mut zone = offset / 60; // convert to minutes
    if zone < 0 {
        b.push(b'-');
        zone = -zone;
    } else {
        b.push(b'+');
    }
    append_int(b, zone / 60, 2);
    b.push(b':');
    append_int(b, zone % 60, 2);
}

// Go: format_rfc3339.go:Time.appendStrictRFC3339
/// Appends RFC 3339 with nanoseconds; the error (if any) is returned with the
/// buffer, as in Go.
pub(crate) fn append_strict_rfc3339(t: &Time, mut b: Vec<u8>) -> (Vec<u8>, Option<&'static str>) {
    let n0 = b.len();
    append_format_rfc3339(t, &mut b, true);

    // Not all valid Go timestamps can be serialized as valid RFC 3339.
    // Explicitly check for these edge cases.
    // See https://go.dev/issue/4556 and https://go.dev/issue/54580.
    let num2 = |b: &[u8]| -> u8 {
        10u8.wrapping_mul(b[0].wrapping_sub(b'0'))
            .wrapping_add(b[1].wrapping_sub(b'0'))
    };
    if b[n0 + "9999".len()] != b'-' {
        // year must be exactly 4 digits wide
        return (b, Some("year outside of range [0,9999]"));
    }
    if b[b.len() - 1] != b'Z' {
        let c = b[b.len() - "Z07:00".len()];
        if c.is_ascii_digit() || num2(&b[b.len() - "07:00".len()..]) >= 24 {
            return (b, Some("timezone hour outside of range [0,23]"));
        }
    }
    (b, None)
}

// Go: format_rfc3339.go:parseRFC3339
/// The RFC 3339 fast parser; `None` when it does not apply (the caller then
/// falls back to the general parser).
pub(crate) fn parse_rfc3339(s: &[u8], local: &Arc<Location>) -> Option<Time> {
    // parseUint parses s as an unsigned decimal integer and
    // verifies that it is within some range.
    // If it is invalid or out-of-range,
    // it sets ok to false and returns the min value.
    let mut ok = true;
    fn parse_uint(ok: &mut bool, s: &[u8], min: i64, max: i64) -> i64 {
        let mut x: i64 = 0;
        for &c in s {
            if !c.is_ascii_digit() {
                *ok = false;
                return min;
            }
            x = x * 10 + c as i64 - b'0' as i64;
        }
        if x < min || max < x {
            *ok = false;
            return min;
        }
        x
    }

    // Parse the date and time.
    if s.len() < "2006-01-02T15:04:05".len() {
        return None;
    }
    let year = parse_uint(&mut ok, &s[0..4], 0, 9999); // e.g., 2006
    let month = parse_uint(&mut ok, &s[5..7], 1, 12); // e.g., 01
    let day = parse_uint(&mut ok, &s[8..10], 1, days_in(Month(month), year)); // e.g., 02
    let hour = parse_uint(&mut ok, &s[11..13], 0, 23); // e.g., 15
    let min = parse_uint(&mut ok, &s[14..16], 0, 59); // e.g., 04
    let sec = parse_uint(&mut ok, &s[17..19], 0, 59); // e.g., 05
    if !ok || !(s[4] == b'-' && s[7] == b'-' && s[10] == b'T' && s[13] == b':' && s[16] == b':') {
        return None;
    }
    let mut s = &s[19..];

    // Parse the fractional second.
    let mut nsec: i64 = 0;
    if s.len() >= 2 && s[0] == b'.' && is_digit(s, 1) {
        let mut n = 2;
        while n < s.len() && is_digit(s, n) {
            n += 1;
        }
        (nsec, _, _) = parse_nanoseconds(s, n);
        s = &s[n..];
    }

    // Parse the time zone.
    let mut t = date(year, Month(month), day, hour, min, sec, nsec, &utc());
    if s.len() != 1 || s[0] != b'Z' {
        if s.len() != "-07:00".len() {
            return None;
        }
        let hr = parse_uint(&mut ok, &s[1..3], 0, 23); // e.g., 07
        let mm = parse_uint(&mut ok, &s[4..6], 0, 59); // e.g., 00
        if !ok || !((s[0] == b'-' || s[0] == b'+') && s[3] == b':') {
            return None;
        }
        let mut zone_offset = (hr * 60 + mm) * 60;
        if s[0] == b'-' {
            zone_offset *= -1;
        }
        add_sec(&mut t, -zone_offset);

        // Use local zone with the given offset if possible.
        if lookup(local, unix_sec(&t)).offset == zone_offset {
            set_loc(&mut t, local);
        } else {
            set_loc(&mut t, &fixed_zone("", zone_offset));
        }
    }
    Some(t)
}

// Go: format_rfc3339.go:parseStrictRFC3339
pub(crate) fn parse_strict_rfc3339(b: &[u8]) -> Result<Time, ParseError> {
    match parse_rfc3339(b, &local()) {
        Some(t) => Ok(t),
        None => {
            // The parse template syntax cannot correctly validate RFC 3339.
            // TODO(https://go.dev/issue/54580): Strict parsing is disabled for now
            // (`case true: return t, nil`); the remaining checks are unreachable.
            parse(RFC3339, b)
        }
    }
}
