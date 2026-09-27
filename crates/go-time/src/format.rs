//! Port of `$GOROOT/src/time/format.go` (go1.27.1): layouts, `Format`,
//! `Parse`/`ParseInLocation`, `ParseError`, `ParseDuration`.
//!
//! Go strings are byte strings: layouts and values are handled as `&[u8]`.

use std::sync::Arc;

use go_value::{Location, Time};

use crate::TimeError;
use crate::format_rfc3339::{append_format_rfc3339, parse_rfc3339};
use crate::time::{
    Duration, Month, abs_clock, abs_days, add_sec, date, days_before, days_date, days_in,
    days_weekday, days_year_yday, is_leap, locabs, nanosecond, set_loc, unix_sec,
};
use crate::utf8::{RUNE_ERROR, RUNE_SELF, decode_rune};
use crate::zoneinfo::{fixed_zone, local, lookup, lookup_name, utc};

// These are predefined layouts for use in Time.Format and time.Parse.
/// The reference time, in numerical order.
pub const LAYOUT: &str = "01/02 03:04:05PM '06 -0700";
pub const ANSIC: &str = "Mon Jan _2 15:04:05 2006";
pub const UNIX_DATE: &str = "Mon Jan _2 15:04:05 MST 2006";
pub const RUBY_DATE: &str = "Mon Jan 02 15:04:05 -0700 2006";
pub const RFC822: &str = "02 Jan 06 15:04 MST";
/// RFC822 with numeric zone.
pub const RFC822Z: &str = "02 Jan 06 15:04 -0700";
pub const RFC850: &str = "Monday, 02-Jan-06 15:04:05 MST";
pub const RFC1123: &str = "Mon, 02 Jan 2006 15:04:05 MST";
/// RFC1123 with numeric zone.
pub const RFC1123Z: &str = "Mon, 02 Jan 2006 15:04:05 -0700";
pub const RFC3339: &str = "2006-01-02T15:04:05Z07:00";
pub const RFC3339_NANO: &str = "2006-01-02T15:04:05.999999999Z07:00";
pub const KITCHEN: &str = "3:04PM";
// Handy time stamps.
pub const STAMP: &str = "Jan _2 15:04:05";
pub const STAMP_MILLI: &str = "Jan _2 15:04:05.000";
pub const STAMP_MICRO: &str = "Jan _2 15:04:05.000000";
pub const STAMP_NANO: &str = "Jan _2 15:04:05.000000000";
pub const DATE_TIME: &str = "2006-01-02 15:04:05";
pub const DATE_ONLY: &str = "2006-01-02";
pub const TIME_ONLY: &str = "15:04:05";

// The std chunk codes (Go's iota values).
const STD_NEED_DATE: i64 = 1 << 8; // need month, day, year
const STD_NEED_YDAY: i64 = 1 << 9; // need yday
const STD_NEED_CLOCK: i64 = 1 << 10; // need hour, minute, second
const STD_ARG_SHIFT: i64 = 16; // extra argument in high bits, above low stdArgShift
const STD_SEPARATOR_SHIFT: i64 = 28; // extra argument in high 4 bits for fractional second separators
pub(crate) const STD_MASK: i64 = (1 << STD_ARG_SHIFT) - 1; // mask out argument

pub(crate) const STD_LONG_MONTH: i64 = 1 + STD_NEED_DATE; // "January"
pub(crate) const STD_MONTH: i64 = 2 + STD_NEED_DATE; // "Jan"
pub(crate) const STD_NUM_MONTH: i64 = 3 + STD_NEED_DATE; // "1"
pub(crate) const STD_ZERO_MONTH: i64 = 4 + STD_NEED_DATE; // "01"
pub(crate) const STD_LONG_WEEK_DAY: i64 = 5 + STD_NEED_DATE; // "Monday"
pub(crate) const STD_WEEK_DAY: i64 = 6 + STD_NEED_DATE; // "Mon"
pub(crate) const STD_DAY: i64 = 7 + STD_NEED_DATE; // "2"
pub(crate) const STD_UNDER_DAY: i64 = 8 + STD_NEED_DATE; // "_2"
pub(crate) const STD_ZERO_DAY: i64 = 9 + STD_NEED_DATE; // "02"
pub(crate) const STD_UNDER_YEAR_DAY: i64 = 10 + STD_NEED_YDAY; // "__2"
pub(crate) const STD_ZERO_YEAR_DAY: i64 = 11 + STD_NEED_YDAY; // "002"
pub(crate) const STD_HOUR: i64 = 12 + STD_NEED_CLOCK; // "15"
pub(crate) const STD_HOUR12: i64 = 13 + STD_NEED_CLOCK; // "3"
pub(crate) const STD_ZERO_HOUR12: i64 = 14 + STD_NEED_CLOCK; // "03"
pub(crate) const STD_MINUTE: i64 = 15 + STD_NEED_CLOCK; // "4"
pub(crate) const STD_ZERO_MINUTE: i64 = 16 + STD_NEED_CLOCK; // "04"
pub(crate) const STD_SECOND: i64 = 17 + STD_NEED_CLOCK; // "5"
pub(crate) const STD_ZERO_SECOND: i64 = 18 + STD_NEED_CLOCK; // "05"
pub(crate) const STD_LONG_YEAR: i64 = 19 + STD_NEED_DATE; // "2006"
pub(crate) const STD_YEAR: i64 = 20 + STD_NEED_DATE; // "06"
pub(crate) const STD_PM: i64 = 21 + STD_NEED_CLOCK; // "PM"
pub(crate) const STD_PM_LOWER: i64 = 22 + STD_NEED_CLOCK; // "pm"
pub(crate) const STD_TZ: i64 = 23; // "MST"
pub(crate) const STD_ISO8601_TZ: i64 = 24; // "Z0700"  // prints Z for UTC
pub(crate) const STD_ISO8601_SECONDS_TZ: i64 = 25; // "Z070000"
pub(crate) const STD_ISO8601_SHORT_TZ: i64 = 26; // "Z07"
pub(crate) const STD_ISO8601_COLON_TZ: i64 = 27; // "Z07:00" // prints Z for UTC
pub(crate) const STD_ISO8601_COLON_SECONDS_TZ: i64 = 28; // "Z07:00:00"
pub(crate) const STD_NUM_TZ: i64 = 29; // "-0700"  // always numeric
pub(crate) const STD_NUM_SECONDS_TZ: i64 = 30; // "-070000"
pub(crate) const STD_NUM_SHORT_TZ: i64 = 31; // "-07"    // always numeric
pub(crate) const STD_NUM_COLON_TZ: i64 = 32; // "-07:00" // always numeric
pub(crate) const STD_NUM_COLON_SECONDS_TZ: i64 = 33; // "-07:00:00"
pub(crate) const STD_FRAC_SECOND0: i64 = 34; // ".0", ".00", ... , trailing zeros included
pub(crate) const STD_FRAC_SECOND9: i64 = 35; // ".9", ".99", ..., trailing zeros omitted

/// Go: `std0x`, the std values for "01", "02", ..., "06".
const STD0X: [i64; 6] = [
    STD_ZERO_MONTH,
    STD_ZERO_DAY,
    STD_ZERO_HOUR12,
    STD_ZERO_MINUTE,
    STD_ZERO_SECOND,
    STD_YEAR,
];

// Go: format.go:startsWithLowerCase
/// Whether the string has a lower-case letter at the beginning.
fn starts_with_lower_case(s: &[u8]) -> bool {
    if s.is_empty() {
        return false;
    }
    let c = s[0];
    c.is_ascii_lowercase()
}

// Go: format.go:nextStdChunk
/// Finds the first std string in layout: (text before, std code, text after).
pub(crate) fn next_std_chunk(layout: &[u8]) -> (&[u8], i64, &[u8]) {
    let n = layout.len();
    let mut i = 0;
    while i < n {
        let c = layout[i];
        match c {
            b'J' => {
                // January, Jan
                if n >= i + 3 && &layout[i..i + 3] == b"Jan" {
                    if n >= i + 7 && &layout[i..i + 7] == b"January" {
                        return (&layout[0..i], STD_LONG_MONTH, &layout[i + 7..]);
                    }
                    if !starts_with_lower_case(&layout[i + 3..]) {
                        return (&layout[0..i], STD_MONTH, &layout[i + 3..]);
                    }
                }
            }
            b'M' => {
                // Monday, Mon, MST
                if n >= i + 3 {
                    if &layout[i..i + 3] == b"Mon" {
                        if n >= i + 6 && &layout[i..i + 6] == b"Monday" {
                            return (&layout[0..i], STD_LONG_WEEK_DAY, &layout[i + 6..]);
                        }
                        if !starts_with_lower_case(&layout[i + 3..]) {
                            return (&layout[0..i], STD_WEEK_DAY, &layout[i + 3..]);
                        }
                    }
                    if &layout[i..i + 3] == b"MST" {
                        return (&layout[0..i], STD_TZ, &layout[i + 3..]);
                    }
                }
            }
            b'0' => {
                // 01, 02, 03, 04, 05, 06, 002
                if n >= i + 2 && b'1' <= layout[i + 1] && layout[i + 1] <= b'6' {
                    return (
                        &layout[0..i],
                        STD0X[(layout[i + 1] - b'1') as usize],
                        &layout[i + 2..],
                    );
                }
                if n >= i + 3 && layout[i + 1] == b'0' && layout[i + 2] == b'2' {
                    return (&layout[0..i], STD_ZERO_YEAR_DAY, &layout[i + 3..]);
                }
            }
            b'1' => {
                // 15, 1
                if n >= i + 2 && layout[i + 1] == b'5' {
                    return (&layout[0..i], STD_HOUR, &layout[i + 2..]);
                }
                return (&layout[0..i], STD_NUM_MONTH, &layout[i + 1..]);
            }
            b'2' => {
                // 2006, 2
                if n >= i + 4 && &layout[i..i + 4] == b"2006" {
                    return (&layout[0..i], STD_LONG_YEAR, &layout[i + 4..]);
                }
                return (&layout[0..i], STD_DAY, &layout[i + 1..]);
            }
            b'_' => {
                // _2, _2006, __2
                if n >= i + 2 && layout[i + 1] == b'2' {
                    //_2006 is really a literal _, followed by stdLongYear
                    if n >= i + 5 && &layout[i + 1..i + 5] == b"2006" {
                        return (&layout[0..i + 1], STD_LONG_YEAR, &layout[i + 5..]);
                    }
                    return (&layout[0..i], STD_UNDER_DAY, &layout[i + 2..]);
                }
                if n >= i + 3 && layout[i + 1] == b'_' && layout[i + 2] == b'2' {
                    return (&layout[0..i], STD_UNDER_YEAR_DAY, &layout[i + 3..]);
                }
            }
            b'3' => return (&layout[0..i], STD_HOUR12, &layout[i + 1..]),
            b'4' => return (&layout[0..i], STD_MINUTE, &layout[i + 1..]),
            b'5' => return (&layout[0..i], STD_SECOND, &layout[i + 1..]),
            b'P' => {
                // PM
                if n >= i + 2 && layout[i + 1] == b'M' {
                    return (&layout[0..i], STD_PM, &layout[i + 2..]);
                }
            }
            b'p' => {
                // pm
                if n >= i + 2 && layout[i + 1] == b'm' {
                    return (&layout[0..i], STD_PM_LOWER, &layout[i + 2..]);
                }
            }
            b'-' => {
                // -070000, -07:00:00, -0700, -07:00, -07
                if n >= i + 7 && &layout[i..i + 7] == b"-070000" {
                    return (&layout[0..i], STD_NUM_SECONDS_TZ, &layout[i + 7..]);
                }
                if n >= i + 9 && &layout[i..i + 9] == b"-07:00:00" {
                    return (&layout[0..i], STD_NUM_COLON_SECONDS_TZ, &layout[i + 9..]);
                }
                if n >= i + 5 && &layout[i..i + 5] == b"-0700" {
                    return (&layout[0..i], STD_NUM_TZ, &layout[i + 5..]);
                }
                if n >= i + 6 && &layout[i..i + 6] == b"-07:00" {
                    return (&layout[0..i], STD_NUM_COLON_TZ, &layout[i + 6..]);
                }
                if n >= i + 3 && &layout[i..i + 3] == b"-07" {
                    return (&layout[0..i], STD_NUM_SHORT_TZ, &layout[i + 3..]);
                }
            }
            b'Z' => {
                // Z070000, Z07:00:00, Z0700, Z07:00,
                if n >= i + 7 && &layout[i..i + 7] == b"Z070000" {
                    return (&layout[0..i], STD_ISO8601_SECONDS_TZ, &layout[i + 7..]);
                }
                if n >= i + 9 && &layout[i..i + 9] == b"Z07:00:00" {
                    return (
                        &layout[0..i],
                        STD_ISO8601_COLON_SECONDS_TZ,
                        &layout[i + 9..],
                    );
                }
                if n >= i + 5 && &layout[i..i + 5] == b"Z0700" {
                    return (&layout[0..i], STD_ISO8601_TZ, &layout[i + 5..]);
                }
                if n >= i + 6 && &layout[i..i + 6] == b"Z07:00" {
                    return (&layout[0..i], STD_ISO8601_COLON_TZ, &layout[i + 6..]);
                }
                if n >= i + 3 && &layout[i..i + 3] == b"Z07" {
                    return (&layout[0..i], STD_ISO8601_SHORT_TZ, &layout[i + 3..]);
                }
            }
            b'.' | b',' => {
                // ,000, or .000, or ,999, or .999 - repeated digits for fractional seconds.
                if i + 1 < n && (layout[i + 1] == b'0' || layout[i + 1] == b'9') {
                    let ch = layout[i + 1];
                    let mut j = i + 1;
                    while j < n && layout[j] == ch {
                        j += 1;
                    }
                    // String of digits must end here - only fractional second is all digits.
                    if !is_digit(layout, j) {
                        let mut code = STD_FRAC_SECOND0;
                        if layout[i + 1] == b'9' {
                            code = STD_FRAC_SECOND9;
                        }
                        let std = std_frac_second(code, (j - (i + 1)) as i64, c);
                        return (&layout[0..i], std, &layout[j..]);
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    (layout, 0, &[])
}

pub(crate) const LONG_DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

pub(crate) const SHORT_DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub(crate) const SHORT_MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub(crate) const LONG_MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// Go: format.go:match
/// Whether s1 and s2 match ignoring case (same length assumed).
fn match_(s1: &[u8], s2: &[u8]) -> bool {
    for i in 0..s1.len() {
        let mut c1 = s1[i];
        let mut c2 = s2[i];
        if c1 != c2 {
            // Switch to lower-case; 'a'-'A' is known to be a single bit.
            c1 |= b'a' - b'A';
            c2 |= b'a' - b'A';
            if c1 != c2 || !c1.is_ascii_lowercase() {
                return false;
            }
        }
    }
    true
}

// Go: format.go:lookup
/// Index of the table entry that prefixes val (ignoring case), and the rest.
fn lookup_tab<'a>(tab: &[&str], val: &'a [u8]) -> (i64, &'a [u8], bool) {
    for (i, v) in tab.iter().enumerate() {
        let v = v.as_bytes();
        if val.len() >= v.len() && match_(&val[..v.len()], v) {
            return (i as i64, &val[v.len()..], false);
        }
    }
    (-1, val, true)
}

// Go: format.go:appendInt
/// Appends the decimal form of x, zero-padded to width digits (sign excluded).
pub(crate) fn append_int(b: &mut Vec<u8>, x: i64, width: i64) {
    let mut u = x as u64;
    if x < 0 {
        b.push(b'-');
        u = x.wrapping_neg() as u64;
    }

    // 2-digit and 4-digit fields are the most common in time formats.
    let utod = |u: u64| b'0' + u as u8;
    if width == 2 && u < 100 {
        b.push(utod(u / 10));
        b.push(utod(u % 10));
        return;
    }
    if width == 4 && u < 10000 {
        b.push(utod(u / 1000));
        b.push(utod(u / 100 % 10));
        b.push(utod(u / 10 % 10));
        b.push(utod(u % 10));
        return;
    }

    // Compute the number of decimal digits.
    let mut n: i64 = 0;
    if u == 0 {
        n = 1;
    }
    let mut u2 = u;
    while u2 > 0 {
        n += 1;
        u2 /= 10;
    }

    // Add 0-padding.
    let mut pad = width - n;
    while pad > 0 {
        b.push(b'0');
        pad -= 1;
    }

    // Ensure capacity.
    let start = b.len();
    b.resize(start + n as usize, 0);

    // Assemble decimal in reverse order.
    let mut i = b.len() - 1;
    while u >= 10 && i > 0 {
        let q = u / 10;
        b[i] = utod(u - q * 10);
        u = q;
        i -= 1;
    }
    b[i] = utod(u);
}

// Go: format.go:atoi
/// Duplicates strconv.Atoi for time's purposes (accepts a leading sign).
pub(crate) fn atoi(s: &[u8]) -> Result<i64, ()> {
    let mut s = s;
    let mut neg = false;
    if !s.is_empty() && (s[0] == b'-' || s[0] == b'+') {
        neg = s[0] == b'-';
        s = &s[1..];
    }
    let (q, rem, err) = leading_int(s);
    let mut x = q as i64;
    if err || !rem.is_empty() {
        return Err(());
    }
    if neg {
        x = x.wrapping_neg();
    }
    Ok(x)
}

// Go: format.go:stdFracSecond
/// Packs the fractional-second std code, digit count and separator.
fn std_frac_second(code: i64, n: i64, c: u8) -> i64 {
    // Use 0xfff to make the failure case even more absurd.
    if c == b'.' {
        return code | ((n & 0xfff) << STD_ARG_SHIFT);
    }
    code | ((n & 0xfff) << STD_ARG_SHIFT) | 1 << STD_SEPARATOR_SHIFT
}

// Go: format.go:digitsLen
fn digits_len(std: i64) -> i64 {
    (std >> STD_ARG_SHIFT) & 0xfff
}

// Go: format.go:separator
fn separator(std: i64) -> u8 {
    if (std >> STD_SEPARATOR_SHIFT) == 0 {
        return b'.';
    }
    b','
}

// Go: format.go:appendNano
/// Appends a fractional second (nanosec in [0, 999999999]) per std.
pub(crate) fn append_nano(b: &mut Vec<u8>, nanosec: i64, std: i64) {
    let trim = std & STD_MASK == STD_FRAC_SECOND9;
    let n = digits_len(std);
    if trim && (n == 0 || nanosec == 0) {
        return;
    }
    let dot = separator(std);
    b.push(dot);
    append_int(b, nanosec, 9);
    if n < 9 {
        let l = b.len();
        b.truncate((l as i64 - 9 + n) as usize);
    }
    if trim {
        while !b.is_empty() && b[b.len() - 1] == b'0' {
            b.pop();
        }
        if !b.is_empty() && b[b.len() - 1] == dot {
            b.pop();
        }
    }
}

// Go: format.go:Time.Format
/// The textual representation of t formatted according to layout.
pub fn format(t: &Time, layout: &str) -> String {
    let b = format_bytes(t, layout.as_bytes());
    match String::from_utf8(b) {
        Ok(s) => s,
        // Unreachable for a UTF-8 layout (chunks split at ASCII bytes and
        // zone names are Rust strings); kept total.
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

/// [`format`] for a byte-string layout (Go strings may hold any bytes).
pub fn format_bytes(t: &Time, layout: &[u8]) -> Vec<u8> {
    const BUF_SIZE: usize = 64;
    let max = layout.len() + 10;
    let mut b = Vec::with_capacity(if max < BUF_SIZE { BUF_SIZE } else { max });
    append_format(t, &mut b, layout);
    b
}

// Go: format.go:Time.AppendFormat
/// Like [`format`] but appends the textual representation to b.
pub fn append_format(t: &Time, b: &mut Vec<u8>, layout: &[u8]) {
    // Optimize for RFC3339 as it accounts for over half of all representations.
    if layout == RFC3339.as_bytes() {
        append_format_rfc3339(t, b, false);
    } else if layout == RFC3339_NANO.as_bytes() {
        append_format_rfc3339(t, b, true);
    } else {
        append_format_(t, b, layout);
    }
}

// Go: format.go:Time.appendFormat
fn append_format_(t: &Time, b: &mut Vec<u8>, layout: &[u8]) {
    let (name, offset, abs) = locabs(t);
    let days = abs_days(abs);

    let mut year: i64 = -1;
    let mut month = Month(0);
    let mut day: i64 = 0;
    let mut yday: i64 = -1;
    let mut hour: i64 = -1;
    let mut min: i64 = 0;
    let mut sec: i64 = 0;

    // Each iteration generates one std value.
    let mut layout = layout;
    while !layout.is_empty() {
        let (prefix, std, suffix) = next_std_chunk(layout);
        if !prefix.is_empty() {
            b.extend_from_slice(prefix);
        }
        if std == 0 {
            break;
        }
        layout = suffix;

        // Compute year, month, day if needed.
        if year < 0 && std & STD_NEED_DATE != 0 {
            (year, month, day) = days_date(days);
        }
        if yday < 0 && std & STD_NEED_YDAY != 0 {
            (_, yday) = days_year_yday(days);
        }

        // Compute hour, minute, second if needed.
        if hour < 0 && std & STD_NEED_CLOCK != 0 {
            (hour, min, sec) = abs_clock(abs);
        }

        match std & STD_MASK {
            STD_YEAR => {
                let mut y = year;
                if y < 0 {
                    y = y.wrapping_neg();
                }
                append_int(b, y % 100, 2);
            }
            STD_LONG_YEAR => append_int(b, year, 4),
            STD_MONTH => b.extend_from_slice(&month.string().as_bytes()[..3]),
            STD_LONG_MONTH => b.extend_from_slice(month.string().as_bytes()),
            STD_NUM_MONTH => append_int(b, month.0, 0),
            STD_ZERO_MONTH => append_int(b, month.0, 2),
            STD_WEEK_DAY => b.extend_from_slice(&days_weekday(days).string().as_bytes()[..3]),
            STD_LONG_WEEK_DAY => b.extend_from_slice(days_weekday(days).string().as_bytes()),
            STD_DAY => append_int(b, day, 0),
            STD_UNDER_DAY => {
                if day < 10 {
                    b.push(b' ');
                }
                append_int(b, day, 0);
            }
            STD_ZERO_DAY => append_int(b, day, 2),
            STD_UNDER_YEAR_DAY => {
                if yday < 100 {
                    b.push(b' ');
                    if yday < 10 {
                        b.push(b' ');
                    }
                }
                append_int(b, yday, 0);
            }
            STD_ZERO_YEAR_DAY => append_int(b, yday, 3),
            STD_HOUR => append_int(b, hour, 2),
            STD_HOUR12 => {
                // Noon is 12PM, midnight is 12AM.
                let mut hr = hour % 12;
                if hr == 0 {
                    hr = 12;
                }
                append_int(b, hr, 0);
            }
            STD_ZERO_HOUR12 => {
                // Noon is 12PM, midnight is 12AM.
                let mut hr = hour % 12;
                if hr == 0 {
                    hr = 12;
                }
                append_int(b, hr, 2);
            }
            STD_MINUTE => append_int(b, min, 0),
            STD_ZERO_MINUTE => append_int(b, min, 2),
            STD_SECOND => append_int(b, sec, 0),
            STD_ZERO_SECOND => append_int(b, sec, 2),
            STD_PM => {
                if hour >= 12 {
                    b.extend_from_slice(b"PM");
                } else {
                    b.extend_from_slice(b"AM");
                }
            }
            STD_PM_LOWER => {
                if hour >= 12 {
                    b.extend_from_slice(b"pm");
                } else {
                    b.extend_from_slice(b"am");
                }
            }
            STD_ISO8601_TZ
            | STD_ISO8601_COLON_TZ
            | STD_ISO8601_SECONDS_TZ
            | STD_ISO8601_SHORT_TZ
            | STD_ISO8601_COLON_SECONDS_TZ
            | STD_NUM_TZ
            | STD_NUM_COLON_TZ
            | STD_NUM_SECONDS_TZ
            | STD_NUM_SHORT_TZ
            | STD_NUM_COLON_SECONDS_TZ => {
                // Ugly special case. We cheat and take the "Z" variants
                // to mean "the time zone as formatted for ISO 8601".
                if offset == 0
                    && (std == STD_ISO8601_TZ
                        || std == STD_ISO8601_COLON_TZ
                        || std == STD_ISO8601_SECONDS_TZ
                        || std == STD_ISO8601_SHORT_TZ
                        || std == STD_ISO8601_COLON_SECONDS_TZ)
                {
                    b.push(b'Z');
                    continue;
                }
                let mut zone = offset / 60; // convert to minutes
                let mut absoffset = offset;
                if zone < 0 {
                    b.push(b'-');
                    zone = -zone;
                    absoffset = -absoffset;
                } else {
                    b.push(b'+');
                }
                append_int(b, zone / 60, 2);
                if std == STD_ISO8601_COLON_TZ
                    || std == STD_NUM_COLON_TZ
                    || std == STD_ISO8601_COLON_SECONDS_TZ
                    || std == STD_NUM_COLON_SECONDS_TZ
                {
                    b.push(b':');
                }
                if std != STD_NUM_SHORT_TZ && std != STD_ISO8601_SHORT_TZ {
                    append_int(b, zone % 60, 2);
                }

                // append seconds if appropriate
                if std == STD_ISO8601_SECONDS_TZ
                    || std == STD_NUM_SECONDS_TZ
                    || std == STD_NUM_COLON_SECONDS_TZ
                    || std == STD_ISO8601_COLON_SECONDS_TZ
                {
                    if std == STD_NUM_COLON_SECONDS_TZ || std == STD_ISO8601_COLON_SECONDS_TZ {
                        b.push(b':');
                    }
                    append_int(b, absoffset % 60, 2);
                }
            }
            STD_TZ => {
                if !name.is_empty() {
                    b.extend_from_slice(name.as_bytes());
                    continue;
                }
                // No time zone known for this time, but we must print one.
                // Use the -0700 format.
                let mut zone = offset / 60; // convert to minutes
                if zone < 0 {
                    b.push(b'-');
                    zone = -zone;
                } else {
                    b.push(b'+');
                }
                append_int(b, zone / 60, 2);
                append_int(b, zone % 60, 2);
            }
            STD_FRAC_SECOND0 | STD_FRAC_SECOND9 => append_nano(b, nanosecond(t), std),
            _ => {}
        }
    }
}

/// Go: `time.ParseError`, describing a problem parsing a time string.
/// Fields are Go strings (bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub layout: Vec<u8>,
    pub value: Vec<u8>,
    pub layout_elem: Vec<u8>,
    pub value_elem: Vec<u8>,
    pub message: Vec<u8>,
}

// Go: format.go:newParseError
fn new_parse_error(
    layout: &[u8],
    value: &[u8],
    layout_elem: &[u8],
    value_elem: &[u8],
    message: Vec<u8>,
) -> ParseError {
    ParseError {
        layout: layout.to_vec(),
        value: value.to_vec(),
        layout_elem: layout_elem.to_vec(),
        value_elem: value_elem.to_vec(),
        message,
    }
}

const LOWERHEX: &[u8; 16] = b"0123456789abcdef";

// Go: format.go:quote
/// Go-quotes s, escaping non-ASCII and control bytes as `\xNN`.
pub(crate) fn quote(s: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(s.len() + 2); // slice will be at least len(s) + quotes
    buf.push(b'"');
    let mut i = 0;
    while i < s.len() {
        let (c, size) = decode_rune(&s[i..]);
        if c >= RUNE_SELF || c < ' ' as u32 {
            // This means you are asking us to parse a time.Duration or
            // time.Location with unprintable or non-ASCII characters in it.
            let width;
            if c == RUNE_ERROR {
                let mut w = 1;
                if i + 2 < s.len() && &s[i..i + 3] == "\u{FFFD}".as_bytes() {
                    w = 3;
                }
                width = w;
            } else {
                width = size; // len(string(c))
            }
            for j in 0..width {
                buf.extend_from_slice(b"\\x");
                buf.push(LOWERHEX[(s[i + j] >> 4) as usize]);
                buf.push(LOWERHEX[(s[i + j] & 0xF) as usize]);
            }
        } else {
            if c == '"' as u32 || c == '\\' as u32 {
                buf.push(b'\\');
            }
            buf.push(c as u8);
        }
        i += size;
    }
    buf.push(b'"');
    buf
}

impl ParseError {
    // Go: format.go:(*ParseError).Error
    /// The string representation of the error (always ASCII).
    pub fn error(&self) -> String {
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"parsing time ");
        b.extend_from_slice(&quote(&self.value));
        if self.message.is_empty() {
            b.extend_from_slice(b" as ");
            b.extend_from_slice(&quote(&self.layout));
            b.extend_from_slice(b": cannot parse ");
            b.extend_from_slice(&quote(&self.value_elem));
            b.extend_from_slice(b" as ");
            b.extend_from_slice(&quote(&self.layout_elem));
        } else {
            b.extend_from_slice(&self.message);
        }
        String::from_utf8_lossy(&b).into_owned()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error())
    }
}

impl std::error::Error for ParseError {}

// Go: format.go:isDigit
/// Whether s[i] is in range and is a decimal digit.
pub(crate) fn is_digit(s: &[u8], i: usize) -> bool {
    if s.len() <= i {
        return false;
    }
    s[i].is_ascii_digit()
}

// Go: format.go:getnum
/// Parses s[0:1] or s[0:2] (fixed forces s[0:2]) as a decimal integer.
fn getnum(s: &[u8], fixed: bool) -> (i64, &[u8], bool) {
    if !is_digit(s, 0) {
        return (0, s, true);
    }
    if !is_digit(s, 1) {
        if fixed {
            return (0, s, true);
        }
        return ((s[0] - b'0') as i64, &s[1..], false);
    }
    (
        ((s[0] - b'0') as i64) * 10 + (s[1] - b'0') as i64,
        &s[2..],
        false,
    )
}

// Go: format.go:getnum3
/// Parses s[0:1], s[0:2], or s[0:3] (fixed forces s[0:3]) as a decimal integer.
fn getnum3(s: &[u8], fixed: bool) -> (i64, &[u8], bool) {
    let mut n: i64 = 0;
    let mut i = 0;
    while i < 3 && is_digit(s, i) {
        n = n * 10 + (s[i] - b'0') as i64;
        i += 1;
    }
    if i == 0 || fixed && i != 3 {
        return (0, s, true);
    }
    (n, &s[i..], false)
}

// Go: format.go:cutspace
fn cutspace(s: &[u8]) -> &[u8] {
    let mut s = s;
    while !s.is_empty() && s[0] == b' ' {
        s = &s[1..];
    }
    s
}

// Go: format.go:skip
/// Removes the given prefix from value, treating runs of spaces as equivalent.
/// Returns the (possibly partially consumed) value and whether it failed.
fn skip<'a>(value: &'a [u8], prefix: &[u8]) -> (&'a [u8], bool) {
    let mut value = value;
    let mut prefix = prefix;
    while !prefix.is_empty() {
        if prefix[0] == b' ' {
            if !value.is_empty() && value[0] != b' ' {
                return (value, true);
            }
            prefix = cutspace(prefix);
            value = cutspace(value);
            continue;
        }
        if value.is_empty() || value[0] != prefix[0] {
            return (value, true);
        }
        prefix = &prefix[1..];
        value = &value[1..];
    }
    (value, false)
}

// Go: format.go:Parse
/// Parses a formatted string (UTC unless the value carries a zone; zone
/// offsets/abbreviations are matched against `Local`).
pub fn parse(layout: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<Time, ParseError> {
    let layout = layout.as_ref();
    let value = value.as_ref();
    let local = local();
    // Optimize for RFC3339 as it accounts for over half of all representations.
    if layout == RFC3339.as_bytes() || layout == RFC3339_NANO.as_bytes() {
        if let Some(t) = parse_rfc3339(value, &local) {
            return Ok(t);
        }
    }
    parse_(layout, value, &utc(), &local)
}

// Go: format.go:ParseInLocation
/// Like [`parse`], but without zone information the time is in `loc`, and
/// zone offsets/abbreviations are matched against `loc`.
pub fn parse_in_location(
    layout: impl AsRef<[u8]>,
    value: impl AsRef<[u8]>,
    loc: &Arc<Location>,
) -> Result<Time, ParseError> {
    let layout = layout.as_ref();
    let value = value.as_ref();
    // Optimize for RFC3339 as it accounts for over half of all representations.
    if layout == RFC3339.as_bytes() || layout == RFC3339_NANO.as_bytes() {
        if let Some(t) = parse_rfc3339(value, loc) {
            return Ok(t);
        }
    }
    parse_(layout, value, loc, loc)
}

// Go: format.go:parse
pub(crate) fn parse_(
    layout: &[u8],
    value: &[u8],
    default_location: &Arc<Location>,
    local: &Arc<Location>,
) -> Result<Time, ParseError> {
    let (alayout, avalue) = (layout, value);
    let mut layout = layout;
    let mut value = value;
    let mut range_err_string: &'static str = ""; // set if a value is out of range
    let mut am_set = false; // do we need to subtract 12 from the hour for midnight?
    let mut pm_set = false; // do we need to add 12 to the hour?

    // Time being constructed.
    let mut year: i64 = 0;
    let mut month: i64 = -1;
    let mut day: i64 = -1;
    let mut yday: i64 = -1;
    let mut hour: i64 = 0;
    let mut min: i64 = 0;
    let mut sec: i64 = 0;
    let mut nsec: i64 = 0;
    let mut z_utc = false; // z != nil (only ever UTC)
    let mut zone_offset: i64 = -1;
    let mut zone_name: &[u8] = b"";

    // Each iteration processes one std value.
    loop {
        let mut err; // errBad / errAtoi (never printed)
        let (prefix, std, suffix) = next_std_chunk(layout);
        let stdstr = &layout[prefix.len()..layout.len() - suffix.len()];
        (value, err) = skip(value, prefix);
        if err {
            return Err(new_parse_error(alayout, avalue, prefix, value, Vec::new()));
        }
        if std == 0 {
            if !value.is_empty() {
                let mut msg = b": extra text: ".to_vec();
                msg.extend_from_slice(&quote(value));
                return Err(new_parse_error(alayout, avalue, b"", value, msg));
            }
            break;
        }
        layout = suffix;
        let p: &[u8];
        let hold = value;
        match std & STD_MASK {
            STD_YEAR => {
                if value.len() < 2 {
                    err = true;
                } else {
                    (p, value) = (&value[0..2], &value[2..]);
                    match atoi(p) {
                        Ok(y) => {
                            year = y;
                            if year >= 69 {
                                // Unix time starts Dec 31 1969 in some time zones
                                year += 1900;
                            } else {
                                year += 2000;
                            }
                        }
                        Err(()) => {
                            year = 0;
                            err = true;
                        }
                    }
                }
            }
            STD_LONG_YEAR => {
                if value.len() < 4 || !is_digit(value, 0) {
                    err = true;
                } else {
                    (p, value) = (&value[0..4], &value[4..]);
                    match atoi(p) {
                        Ok(y) => year = y,
                        Err(()) => {
                            year = 0;
                            err = true;
                        }
                    }
                }
            }
            STD_MONTH => {
                (month, value, err) = lookup_tab(&SHORT_MONTH_NAMES, value);
                month += 1;
            }
            STD_LONG_MONTH => {
                (month, value, err) = lookup_tab(&LONG_MONTH_NAMES, value);
                month += 1;
            }
            STD_NUM_MONTH | STD_ZERO_MONTH => {
                (month, value, err) = getnum(value, std == STD_ZERO_MONTH);
                if !err && (month <= 0 || 12 < month) {
                    range_err_string = "month";
                }
            }
            STD_WEEK_DAY => {
                // Ignore weekday except for error checking.
                (_, value, err) = lookup_tab(&SHORT_DAY_NAMES, value);
            }
            STD_LONG_WEEK_DAY => {
                (_, value, err) = lookup_tab(&LONG_DAY_NAMES, value);
            }
            STD_DAY | STD_UNDER_DAY | STD_ZERO_DAY => {
                if std == STD_UNDER_DAY && !value.is_empty() && value[0] == b' ' {
                    value = &value[1..];
                }
                (day, value, err) = getnum(value, std == STD_ZERO_DAY);
                // Note that we allow any one- or two-digit day here.
                // The month, day, year combination is validated after we've completed parsing.
            }
            STD_UNDER_YEAR_DAY | STD_ZERO_YEAR_DAY => {
                for _ in 0..2 {
                    if std == STD_UNDER_YEAR_DAY && !value.is_empty() && value[0] == b' ' {
                        value = &value[1..];
                    }
                }
                (yday, value, err) = getnum3(value, std == STD_ZERO_YEAR_DAY);
                // Note that we allow any one-, two-, or three-digit year-day here.
                // The year-day, year combination is validated after we've completed parsing.
            }
            STD_HOUR => {
                (hour, value, err) = getnum(value, false);
                if !(0..24).contains(&hour) {
                    range_err_string = "hour";
                }
            }
            STD_HOUR12 | STD_ZERO_HOUR12 => {
                (hour, value, err) = getnum(value, std == STD_ZERO_HOUR12);
                if !(0..=12).contains(&hour) {
                    range_err_string = "hour";
                }
            }
            STD_MINUTE | STD_ZERO_MINUTE => {
                (min, value, err) = getnum(value, std == STD_ZERO_MINUTE);
                if !(0..60).contains(&min) {
                    range_err_string = "minute";
                }
            }
            STD_SECOND | STD_ZERO_SECOND => {
                (sec, value, err) = getnum(value, std == STD_ZERO_SECOND);
                if !err {
                    if !(0..60).contains(&sec) {
                        range_err_string = "second";
                    } else if value.len() >= 2 && comma_or_period(value[0]) && is_digit(value, 1) {
                        // Special case: do we have a fractional second but no
                        // fractional second in the format?
                        let (_, std2, _) = next_std_chunk(layout);
                        let std2 = std2 & STD_MASK;
                        if std2 == STD_FRAC_SECOND0 || std2 == STD_FRAC_SECOND9 {
                            // Fractional second in the layout; proceed normally
                        } else {
                            // No fractional second in the layout but we have one in the input.
                            let mut n = 2;
                            while n < value.len() && is_digit(value, n) {
                                n += 1;
                            }
                            (nsec, range_err_string, err) = parse_nanoseconds(value, n);
                            value = &value[n..];
                        }
                    }
                }
            }
            STD_PM => {
                if value.len() < 2 {
                    err = true;
                } else {
                    (p, value) = (&value[0..2], &value[2..]);
                    match p {
                        b"PM" => pm_set = true,
                        b"AM" => am_set = true,
                        _ => err = true,
                    }
                }
            }
            STD_PM_LOWER => {
                if value.len() < 2 {
                    err = true;
                } else {
                    (p, value) = (&value[0..2], &value[2..]);
                    match p {
                        b"pm" => pm_set = true,
                        b"am" => am_set = true,
                        _ => err = true,
                    }
                }
            }
            STD_ISO8601_TZ
            | STD_ISO8601_SHORT_TZ
            | STD_ISO8601_COLON_TZ
            | STD_ISO8601_SECONDS_TZ
            | STD_ISO8601_COLON_SECONDS_TZ
            | STD_NUM_TZ
            | STD_NUM_SHORT_TZ
            | STD_NUM_COLON_TZ
            | STD_NUM_SECONDS_TZ
            | STD_NUM_COLON_SECONDS_TZ => {
                let iso = matches!(
                    std,
                    STD_ISO8601_TZ
                        | STD_ISO8601_SHORT_TZ
                        | STD_ISO8601_COLON_TZ
                        | STD_ISO8601_SECONDS_TZ
                        | STD_ISO8601_COLON_SECONDS_TZ
                );
                if iso && !value.is_empty() && value[0] == b'Z' {
                    value = &value[1..];
                    z_utc = true;
                } else {
                    // fallthrough into the numeric zone case
                    (value, err) =
                        parse_num_tz(std, value, &mut zone_offset, &mut range_err_string);
                }
            }
            STD_TZ => {
                // Does it look like a time zone?
                if value.len() >= 3 && &value[0..3] == b"UTC" {
                    z_utc = true;
                    value = &value[3..];
                } else {
                    match parse_time_zone(value) {
                        (n, true) => {
                            (zone_name, value) = (&value[..n], &value[n..]);
                            err = false;
                        }
                        _ => err = true,
                    }
                }
            }
            STD_FRAC_SECOND0 => {
                // stdFracSecond0 requires the exact number of digits as specified in
                // the layout.
                let ndigit = 1 + digits_len(std) as usize;
                if value.len() < ndigit {
                    err = true;
                } else {
                    (nsec, range_err_string, err) = parse_nanoseconds(value, ndigit);
                    value = &value[ndigit..];
                }
            }
            STD_FRAC_SECOND9 => {
                if value.len() < 2
                    || !comma_or_period(value[0])
                    || value[1] < b'0'
                    || b'9' < value[1]
                {
                    // Fractional second omitted.
                } else {
                    // Take any number of digits, even more than asked for,
                    // because it is what the stdSecond case would do.
                    let mut i = 0;
                    while i + 1 < value.len() && b'0' <= value[i + 1] && value[i + 1] <= b'9' {
                        i += 1;
                    }
                    (nsec, range_err_string, err) = parse_nanoseconds(value, 1 + i);
                    value = &value[1 + i..];
                }
            }
            _ => {}
        }
        if !range_err_string.is_empty() {
            let msg = format!(": {} out of range", range_err_string).into_bytes();
            return Err(new_parse_error(alayout, avalue, stdstr, value, msg));
        }
        if err {
            return Err(new_parse_error(alayout, avalue, stdstr, hold, Vec::new()));
        }
    }
    if pm_set && hour < 12 {
        hour += 12;
    } else if am_set && hour == 12 {
        hour = 0;
    }

    // Convert yday to day, month.
    if yday >= 0 {
        let mut d: i64 = 0;
        let mut m: i64 = 0;
        if is_leap(year) {
            if yday == 31 + 29 {
                m = Month::FEBRUARY.0;
                d = 29;
            } else if yday > 31 + 29 {
                yday -= 1;
            }
        }
        if !(1..=365).contains(&yday) {
            return Err(new_parse_error(
                alayout,
                avalue,
                b"",
                value,
                b": day-of-year out of range".to_vec(),
            ));
        }
        if m == 0 {
            m = (yday - 1) / 31 + 1;
            if days_before(Month(m + 1)) < yday {
                m += 1;
            }
            d = yday - days_before(Month(m));
        }
        // If month, day already seen, yday's m, d must match.
        // Otherwise, set them from m, d.
        if month >= 0 && month != m {
            return Err(new_parse_error(
                alayout,
                avalue,
                b"",
                value,
                b": day-of-year does not match month".to_vec(),
            ));
        }
        month = m;
        if day >= 0 && day != d {
            return Err(new_parse_error(
                alayout,
                avalue,
                b"",
                value,
                b": day-of-year does not match day".to_vec(),
            ));
        }
        day = d;
    } else {
        if month < 0 {
            month = Month::JANUARY.0;
        }
        if day < 0 {
            day = 1;
        }
    }

    // Validate the day of the month.
    if day < 1 || day > days_in(Month(month), year) {
        return Err(new_parse_error(
            alayout,
            avalue,
            b"",
            value,
            b": day out of range".to_vec(),
        ));
    }

    let utc_loc = utc();
    if z_utc {
        return Ok(date(
            year,
            Month(month),
            day,
            hour,
            min,
            sec,
            nsec,
            &utc_loc,
        ));
    }

    if zone_offset != -1 {
        let mut t = date(year, Month(month), day, hour, min, sec, nsec, &utc_loc);
        add_sec(&mut t, zone_offset.wrapping_neg());

        // Look for local zone with the given offset.
        // If that zone was in effect at the given time, use it.
        let z = lookup(local, unix_sec(&t));
        if z.offset == zone_offset && (zone_name.is_empty() || z.name.as_bytes() == zone_name) {
            set_loc(&mut t, local);
            return Ok(t);
        }

        // Otherwise create fake zone to record offset.
        let zone_name_copy = String::from_utf8_lossy(zone_name); // ASCII (see parse_time_zone)
        set_loc(&mut t, &fixed_zone(&zone_name_copy, zone_offset));
        return Ok(t);
    }

    if !zone_name.is_empty() {
        let mut t = date(year, Month(month), day, hour, min, sec, nsec, &utc_loc);
        // Look for local zone with the given offset.
        // If that zone was in effect at the given time, use it.
        let (mut offset, ok) = match lookup_name(local, zone_name, unix_sec(&t)) {
            Some(o) => (o, true),
            None => (0, false),
        };
        if ok {
            add_sec(&mut t, offset.wrapping_neg());
            set_loc(&mut t, local);
            return Ok(t);
        }

        // Otherwise, create fake zone with unknown offset.
        if zone_name.len() > 3 && &zone_name[..3] == b"GMT" {
            offset = atoi(&zone_name[3..]).unwrap_or(0); // Guaranteed OK by parseGMT.
            offset = offset.wrapping_mul(3600);
        }
        let zone_name_copy = String::from_utf8_lossy(zone_name); // ASCII (see parse_time_zone)
        set_loc(&mut t, &fixed_zone(&zone_name_copy, offset));
        return Ok(t);
    }

    // Otherwise, fall back to default.
    Ok(date(
        year,
        Month(month),
        day,
        hour,
        min,
        sec,
        nsec,
        default_location,
    ))
}

/// The `stdNumTZ...` arm of `parse` (Go: the `case` body reached directly or
/// by `fallthrough` from the ISO 8601 arm). Returns the new value and err.
fn parse_num_tz<'a>(
    std: i64,
    value: &'a [u8],
    zone_offset: &mut i64,
    range_err_string: &mut &'static str,
) -> (&'a [u8], bool) {
    let sign: &[u8];
    let hour: &[u8];
    let min: &[u8];
    let seconds: &[u8];
    let mut value = value;
    if std == STD_ISO8601_COLON_TZ || std == STD_NUM_COLON_TZ {
        if value.len() < 6 {
            return (value, true);
        }
        if value[3] != b':' {
            return (value, true);
        }
        (sign, hour, min, seconds, value) =
            (&value[0..1], &value[1..3], &value[4..6], b"00", &value[6..]);
    } else if std == STD_NUM_SHORT_TZ || std == STD_ISO8601_SHORT_TZ {
        if value.len() < 3 {
            return (value, true);
        }
        (sign, hour, min, seconds, value) = (&value[0..1], &value[1..3], b"00", b"00", &value[3..]);
    } else if std == STD_ISO8601_COLON_SECONDS_TZ || std == STD_NUM_COLON_SECONDS_TZ {
        if value.len() < 9 {
            return (value, true);
        }
        if value[3] != b':' || value[6] != b':' {
            return (value, true);
        }
        (sign, hour, min, seconds, value) = (
            &value[0..1],
            &value[1..3],
            &value[4..6],
            &value[7..9],
            &value[9..],
        );
    } else if std == STD_ISO8601_SECONDS_TZ || std == STD_NUM_SECONDS_TZ {
        if value.len() < 7 {
            return (value, true);
        }
        (sign, hour, min, seconds, value) = (
            &value[0..1],
            &value[1..3],
            &value[3..5],
            &value[5..7],
            &value[7..],
        );
    } else {
        if value.len() < 5 {
            return (value, true);
        }
        (sign, hour, min, seconds, value) =
            (&value[0..1], &value[1..3], &value[3..5], b"00", &value[5..]);
    }
    let mut mm: i64 = 0;
    let mut ss: i64 = 0;
    let (hr, _, mut err) = getnum(hour, true);
    if !err {
        (mm, _, err) = getnum(min, true);
        if !err {
            (ss, _, err) = getnum(seconds, true);
        }
    }

    // The range test use > rather than >=,
    // as some people do write offsets of 24 hours
    // or 60 minutes or 60 seconds.
    if hr > 24 {
        *range_err_string = "time zone offset hour";
    }
    if mm > 60 {
        *range_err_string = "time zone offset minute";
    }
    if ss > 60 {
        *range_err_string = "time zone offset second";
    }

    *zone_offset = (hr * 60 + mm) * 60 + ss; // offset is in seconds
    match sign[0] {
        b'+' => {}
        b'-' => *zone_offset = -*zone_offset,
        _ => err = true,
    }
    (value, err)
}

// Go: format.go:parseTimeZone
/// Parses a time zone string and returns its length.
pub(crate) fn parse_time_zone(value: &[u8]) -> (usize, bool) {
    if value.len() < 3 {
        return (0, false);
    }
    // Special case 1: ChST and MeST are the only zones with a lower-case letter.
    if value.len() >= 4 && (&value[..4] == b"ChST" || &value[..4] == b"MeST") {
        return (4, true);
    }
    // Special case 2: GMT may have an hour offset; treat it specially.
    if &value[..3] == b"GMT" {
        let length = parse_gmt(value);
        return (length, true);
    }
    // Special Case 3: Some time zones are not named, but have +/-00 format
    if value[0] == b'+' || value[0] == b'-' {
        let length = parse_signed_offset(value);
        let ok = length > 0; // parseSignedOffset returns 0 in case of bad input
        return (length, ok);
    }
    // How many upper-case letters are there? Need at least three, at most five.
    let mut n_upper = 0;
    while n_upper < 6 {
        if n_upper >= value.len() {
            break;
        }
        let c = value[n_upper];
        if !c.is_ascii_uppercase() {
            break;
        }
        n_upper += 1;
    }
    match n_upper {
        0 | 1 | 2 | 6 => return (0, false),
        5 => {
            // Must end in T to match.
            if value[4] == b'T' {
                return (5, true);
            }
        }
        4 => {
            // Must end in T, except one special case.
            if value[3] == b'T' || &value[..4] == b"WITA" {
                return (4, true);
            }
        }
        3 => return (3, true),
        _ => {}
    }
    (0, false)
}

// Go: format.go:parseGMT
/// Parses a GMT time zone (the input is known to start "GMT").
fn parse_gmt(value: &[u8]) -> usize {
    let value = &value[3..];
    if value.is_empty() {
        return 3;
    }

    3 + parse_signed_offset(value)
}

// Go: format.go:parseSignedOffset
/// Parses a signed timezone offset (e.g. "+03" or "-04") in [-23, +23];
/// returns its length or 0.
fn parse_signed_offset(value: &[u8]) -> usize {
    let sign = value[0];
    if sign != b'-' && sign != b'+' {
        return 0;
    }
    let (x, rem, err) = leading_int(&value[1..]);

    // fail if nothing consumed by leadingInt
    if err || value[1..].len() == rem.len() {
        return 0;
    }
    if x > 23 {
        return 0;
    }
    value.len() - rem.len()
}

// Go: format.go:commaOrPeriod
fn comma_or_period(b: u8) -> bool {
    b == b'.' || b == b','
}

// Go: format.go:parseNanoseconds
pub(crate) fn parse_nanoseconds(value: &[u8], nbytes: usize) -> (i64, &'static str, bool) {
    if !comma_or_period(value[0]) {
        return (0, "", true);
    }
    let mut value = value;
    let mut nbytes = nbytes;
    if nbytes > 10 {
        value = &value[..10];
        nbytes = 10;
    }
    let mut ns = match atoi(&value[1..nbytes]) {
        Ok(v) => v,
        Err(()) => return (0, "", true),
    };
    if ns < 0 {
        return (ns, "fractional second", false);
    }
    // We need nanoseconds, which means scaling by the number
    // of missing digits in the format, maximum length 10.
    let scale_digits = 10 - nbytes;
    for _ in 0..scale_digits {
        ns *= 10;
    }
    (ns, "", false)
}

// Go: format.go:leadingInt
/// Consumes the leading [0-9]* from s: (value, rest, overflow error).
pub(crate) fn leading_int(s: &[u8]) -> (u64, &[u8], bool) {
    let mut x: u64 = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        if x > (1u64 << 63) / 10 {
            // overflow
            return (0, &[], true);
        }
        x = x * 10 + c as u64 - b'0' as u64;
        if x > 1u64 << 63 {
            // overflow
            return (0, &[], true);
        }
        i += 1;
    }
    (x, &s[i..], false)
}

// Go: format.go:leadingFraction
/// Consumes the leading [0-9]* from s for a fraction: (value, scale, rest).
fn leading_fraction(s: &[u8]) -> (u64, f64, &[u8]) {
    let mut x: u64 = 0;
    let mut i = 0;
    let mut scale: f64 = 1.0;
    let mut overflow = false;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        i += 1;
        if overflow {
            continue;
        }
        if x > ((1u64 << 63) - 1) / 10 {
            // It's possible for overflow to give a positive number, so take care.
            overflow = true;
            continue;
        }
        let y = x * 10 + c as u64 - b'0' as u64;
        if y > 1u64 << 63 {
            overflow = true;
            continue;
        }
        x = y;
        scale *= 10.0;
    }
    (x, scale, &s[i..])
}

// Go: format.go:parseDurationError.Error
fn parse_duration_error(message: &[u8], value: &[u8]) -> TimeError {
    let mut b = b"time: ".to_vec();
    b.extend_from_slice(message);
    b.push(b' ');
    b.extend_from_slice(&quote(value));
    TimeError::new(String::from_utf8_lossy(&b).into_owned())
}

// Go: format.go:unitMap
fn unit_map(u: &[u8]) -> Option<u64> {
    Some(match u {
        b"ns" => Duration::NANOSECOND.0 as u64,
        b"us" => Duration::MICROSECOND.0 as u64,
        b"\xc2\xb5s" => Duration::MICROSECOND.0 as u64, // U+00B5 = micro symbol
        b"\xce\xbcs" => Duration::MICROSECOND.0 as u64, // U+03BC = Greek letter mu
        b"ms" => Duration::MILLISECOND.0 as u64,
        b"s" => Duration::SECOND.0 as u64,
        b"m" => Duration::MINUTE.0 as u64,
        b"h" => Duration::HOUR.0 as u64,
        _ => return None,
    })
}

// Go: format.go:ParseDuration
/// Parses a duration string such as "300ms", "-1.5h" or "2h45m".
pub fn parse_duration(s: impl AsRef<[u8]>) -> Result<Duration, TimeError> {
    // [-+]?([0-9]*(\.[0-9]*)?[a-z]+)+
    let orig = s.as_ref();
    let mut s = orig;
    let mut d: u64 = 0;
    let mut neg = false;

    // Consume [-+]?
    if !s.is_empty() {
        let c = s[0];
        if c == b'-' || c == b'+' {
            neg = c == b'-';
            s = &s[1..];
        }
    }
    // Special case: if all that is left is "0", this is zero.
    if s == b"0" {
        return Ok(Duration(0));
    }
    if s.is_empty() {
        return Err(parse_duration_error(b"invalid duration", orig));
    }
    while !s.is_empty() {
        let mut v: u64; // integers before, after decimal point
        let mut f: u64 = 0;
        let mut scale: f64 = 1.0; // value = v + f/scale

        // The next character must be [0-9.]
        if !(s[0] == b'.' || s[0].is_ascii_digit()) {
            return Err(parse_duration_error(b"invalid duration", orig));
        }
        // Consume [0-9]*
        let pl = s.len();
        let (lv, rest, err) = leading_int(s);
        if err {
            return Err(parse_duration_error(b"invalid duration", orig));
        }
        v = lv;
        s = rest;
        let pre = pl != s.len(); // whether we consumed anything before a period

        // Consume (\.[0-9]*)?
        let mut post = false;
        if !s.is_empty() && s[0] == b'.' {
            s = &s[1..];
            let pl = s.len();
            (f, scale, s) = leading_fraction(s);
            post = pl != s.len();
        }
        if !pre && !post {
            // no digits (e.g. ".s" or "-.s")
            return Err(parse_duration_error(b"invalid duration", orig));
        }

        // Consume unit.
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            if c == b'.' || c.is_ascii_digit() {
                break;
            }
            i += 1;
        }
        if i == 0 {
            return Err(parse_duration_error(b"missing unit in duration", orig));
        }
        let u = &s[..i];
        s = &s[i..];
        let unit = match unit_map(u) {
            Some(unit) => unit,
            None => {
                let mut msg = b"unknown unit ".to_vec();
                msg.extend_from_slice(&quote(u));
                msg.extend_from_slice(b" in duration");
                return Err(parse_duration_error(&msg, orig));
            }
        };
        if v > (1u64 << 63) / unit {
            // overflow
            return Err(parse_duration_error(b"invalid duration", orig));
        }
        v *= unit;
        if f > 0 {
            // float64 is needed to be nanosecond accurate for fractions of hours.
            // v >= 0 && (f*unit/scale) <= 3.6e+12 (ns/h, h is the largest unit)
            // (no FMA: the product is converted to uint64 before the add)
            v = v.wrapping_add((f as f64 * (unit as f64 / scale)) as u64);
            if v > 1u64 << 63 {
                // overflow
                return Err(parse_duration_error(b"invalid duration", orig));
            }
        }
        d = d.wrapping_add(v);
        if d > 1u64 << 63 {
            return Err(parse_duration_error(b"invalid duration", orig));
        }
    }
    if neg {
        return Ok(Duration((d as i64).wrapping_neg()));
    }
    if d > (1u64 << 63) - 1 {
        return Err(parse_duration_error(b"invalid duration", orig));
    }
    Ok(Duration(d as i64))
}

/// Go: `time.Date` month helper used by callers that hold a Go `int`.
#[allow(dead_code)]
pub(crate) fn month_of(m: i64) -> Month {
    Month(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn std_chunk_name(std: i64) -> &'static str {
        match std & STD_MASK {
            STD_LONG_MONTH => "January",
            STD_MONTH => "Jan",
            STD_NUM_MONTH => "1",
            STD_ZERO_MONTH => "01",
            STD_LONG_WEEK_DAY => "Monday",
            STD_WEEK_DAY => "Mon",
            STD_DAY => "2",
            STD_UNDER_DAY => "_2",
            STD_ZERO_DAY => "02",
            STD_UNDER_YEAR_DAY => "__2",
            STD_ZERO_YEAR_DAY => "002",
            STD_HOUR => "15",
            STD_HOUR12 => "3",
            STD_ZERO_HOUR12 => "03",
            STD_MINUTE => "4",
            STD_ZERO_MINUTE => "04",
            STD_SECOND => "5",
            STD_ZERO_SECOND => "05",
            STD_LONG_YEAR => "2006",
            STD_YEAR => "06",
            STD_PM => "PM",
            STD_PM_LOWER => "pm",
            STD_TZ => "MST",
            STD_ISO8601_TZ => "Z0700",
            STD_ISO8601_SECONDS_TZ => "Z070000",
            STD_ISO8601_SHORT_TZ => "Z07",
            STD_ISO8601_COLON_TZ => "Z07:00",
            STD_ISO8601_COLON_SECONDS_TZ => "Z07:00:00",
            STD_NUM_TZ => "-0700",
            STD_NUM_SECONDS_TZ => "-070000",
            STD_NUM_SHORT_TZ => "-07",
            STD_NUM_COLON_TZ => "-07:00",
            STD_NUM_COLON_SECONDS_TZ => "-07:00:00",
            _ => "?",
        }
    }

    // Go: format_test.go:TestNextStdChunk
    #[test]
    fn go_next_std_chunk() {
        let tests = [
            "(2006)-(01)-(02)T(15):(04):(05)(Z07:00)",
            "(2006)-(01)-(02) (002) (15):(04):(05)",
            "(2006)-(01) (002) (15):(04):(05)",
            "(2006)-(002) (15):(04):(05)",
            "(2006)(002)(01) (15):(04):(05)",
            "(2006)(002)(04) (15):(04):(05)",
        ];
        for marked in tests {
            let format: String = marked.chars().filter(|&c| c != '(' && c != ')').collect();
            let mut out = String::new();
            let mut s = format.as_bytes();
            while !s.is_empty() {
                let (prefix, std, suffix) = next_std_chunk(s);
                out.push_str(std::str::from_utf8(prefix).unwrap());
                if std > 0 {
                    out.push('(');
                    out.push_str(std_chunk_name(std));
                    out.push(')');
                }
                s = suffix;
            }
            assert_eq!(out, marked, "nextStdChunk parses {:?}", format);
        }
    }

    // Go: format_test.go:TestAppendInt
    #[test]
    fn go_append_int() {
        let tests: &[(i64, i64, &str)] = &[
            (0, 0, "0"),
            (0, 1, "0"),
            (0, 2, "00"),
            (0, 3, "000"),
            (1, 0, "1"),
            (1, 1, "1"),
            (1, 2, "01"),
            (1, 3, "001"),
            (-1, 0, "-1"),
            (-1, 1, "-1"),
            (-1, 2, "-01"),
            (-1, 3, "-001"),
            (99, 2, "99"),
            (100, 2, "100"),
            (1, 4, "0001"),
            (12, 4, "0012"),
            (123, 4, "0123"),
            (1234, 4, "1234"),
            (12345, 4, "12345"),
            (1, 5, "00001"),
            (12, 5, "00012"),
            (123, 5, "00123"),
            (1234, 5, "01234"),
            (12345, 5, "12345"),
            (123456, 5, "123456"),
            (0, 9, "000000000"),
            (123, 9, "000000123"),
            (123456, 9, "000123456"),
            (123456789, 9, "123456789"),
            (i64::MIN, 0, "-9223372036854775808"),
            (i64::MAX, 25, "0000009223372036854775807"),
        ];
        for &(x, width, want) in tests {
            let mut b = b"pre".to_vec();
            append_int(&mut b, x, width);
            assert_eq!(&b[3..], want.as_bytes(), "appendInt({}, {})", x, width);
        }
    }

    // Go: format_test.go:TestQuote
    #[test]
    fn go_quote() {
        let tests: &[(&[u8], &str)] = &[
            (b"\"", r#""\"""#),
            (b"abc\"xyz\"", r#""abc\"xyz\"""#),
            (b"", r#""""#),
            (b"abc", r#""abc""#),
            ("☺".as_bytes(), r#""\xe2\x98\xba""#),
            (
                "☺ hello ☺ hello".as_bytes(),
                r#""\xe2\x98\xba hello \xe2\x98\xba hello""#,
            ),
            (b"\x04", r#""\x04""#),
            (b"\xff\\", r#""\xff\\""#),
        ];
        for &(s, want) in tests {
            assert_eq!(quote(s), want.as_bytes(), "quote({:?})", s);
        }
    }

    // Go: format_test.go:TestParseTimeZone
    #[test]
    fn go_parse_time_zone() {
        let tests: &[(&str, usize, bool)] = &[
            ("gmt hi there", 0, false),
            ("GMT hi there", 3, true),
            ("GMT+12 hi there", 6, true),
            ("GMT+00 hi there", 6, true),
            ("GMT+", 3, true),
            ("GMT+3", 5, true),
            ("GMT+a", 3, true),
            ("GMT+3a", 5, true),
            ("GMT-5 hi there", 5, true),
            ("GMT-51 hi there", 3, true),
            ("ChST hi there", 4, true),
            ("MeST hi there", 4, true),
            ("MSDx", 3, true),
            ("MSDY", 0, false),
            ("ESAST hi", 5, true),
            ("ESASTT hi", 0, false),
            ("ESATY hi", 0, false),
            ("WITA hi", 4, true),
            ("+03 hi", 3, true),
            ("-04 hi", 3, true),
            ("+00", 3, true),
            ("-11", 3, true),
            ("-12", 3, true),
            ("-23", 3, true),
            ("-24", 0, false),
            ("+13", 3, true),
            ("+14", 3, true),
            ("+23", 3, true),
            ("+24", 0, false),
        ];
        for &(value, length, ok) in tests {
            let (l, o) = parse_time_zone(value.as_bytes());
            assert_eq!(o, ok, "ok for {:?}", value);
            if ok {
                assert_eq!(l, length, "length for {:?}", value);
            }
        }
    }

    #[test]
    fn std_codes_match_go_iota() {
        // The iota values of format.go's std constants.
        assert_eq!(STD_LONG_MONTH, 257);
        assert_eq!(STD_ZERO_DAY, 265);
        assert_eq!(STD_UNDER_YEAR_DAY, 522);
        assert_eq!(STD_ZERO_YEAR_DAY, 523);
        assert_eq!(STD_HOUR, 1036);
        assert_eq!(STD_ZERO_SECOND, 1042);
        assert_eq!(STD_LONG_YEAR, 275);
        assert_eq!(STD_YEAR, 276);
        assert_eq!(STD_PM, 1045);
        assert_eq!(STD_PM_LOWER, 1046);
        assert_eq!(STD_TZ, 23);
        assert_eq!(STD_FRAC_SECOND9, 35);
    }
}
