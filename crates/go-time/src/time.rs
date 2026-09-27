//! Port of `$GOROOT/src/time/time.go` (go1.27.1).
//!
//! `go_value::Time` stores Unix seconds (`unix_sec`) where Go stores the
//! seconds since January 1, year 1 (`ext`, "internal" seconds). The two are
//! related by a wrapping add of `unixToInternal`, which is a bijection on
//! `i64`, so every Go computation on `sec()` is reproduced exactly through
//! [`sec`] / [`set_sec`]. The monotonic clock reading is not modelled (see
//! PORTING.md): every time behaves like one with `hasMonotonic == 0`.

use std::sync::Arc;

use go_value::{Location, Time};

use crate::TimeError;
use crate::format::{LONG_DAY_NAMES, LONG_MONTH_NAMES, append_int, quote};
use crate::format_rfc3339::{append_format_rfc3339, append_strict_rfc3339, parse_strict_rfc3339};
use crate::zoneinfo::{
    ALPHA, OMEGA, fixed_zone, is_local_loc, is_utc_loc, local, lookup, lookup_opt, utc,
};

// ---------------------------------------------------------------------------
// Internal representation helpers (Go: the unexported methods on *Time).
// ---------------------------------------------------------------------------

// Go: time.go:(*Time).sec
/// Seconds since January 1, year 1 (Go's internal seconds).
#[inline]
pub(crate) fn sec(t: &Time) -> i64 {
    t.unix_sec.wrapping_add(UNIX_TO_INTERNAL)
}

/// Stores Go internal seconds back into the Unix-seconds representation.
#[inline]
pub(crate) fn set_sec(t: &mut Time, internal: i64) {
    t.unix_sec = internal.wrapping_add(INTERNAL_TO_UNIX);
}

// Go: time.go:(*Time).nsec
#[inline]
pub(crate) fn nsec(t: &Time) -> i32 {
    t.nsec as i32
}

// Go: time.go:(*Time).unixSec
#[inline]
pub(crate) fn unix_sec(t: &Time) -> i64 {
    // sec() + internalToUnix == unix_sec exactly (wrapping bijection).
    t.unix_sec
}

// Go: time.go:(*Time).addSec
/// Adds `d` seconds, saturating at ±(2^63-1) internal seconds like Go.
pub(crate) fn add_sec(t: &mut Time, d: i64) {
    // (monotonic branch not modelled)
    // Check if the sum of t.ext and d overflows and handle it properly.
    let ext = sec(t);
    let sum = ext.wrapping_add(d);
    let new = if (sum > ext) == (d > 0) {
        sum
    } else if d > 0 {
        i64::MAX
    } else {
        -i64::MAX
    };
    set_sec(t, new);
}

// Go: time.go:(*Time).setLoc
/// Sets the location; Go's `&utcLoc` is normalised to nil (`None`).
pub(crate) fn set_loc(t: &mut Time, loc: &Arc<Location>) {
    if is_utc_loc(loc) {
        t.loc = None;
    } else {
        t.loc = Some(loc.clone());
    }
}

// ---------------------------------------------------------------------------
// Comparisons (Go compares internal seconds, then nanoseconds).
// ---------------------------------------------------------------------------

// Go: time.go:Time.After
pub fn after(t: &Time, u: &Time) -> bool {
    let ts = sec(t);
    let us = sec(u);
    ts > us || ts == us && nsec(t) > nsec(u)
}

// Go: time.go:Time.Before
pub fn before(t: &Time, u: &Time) -> bool {
    let ts = sec(t);
    let us = sec(u);
    ts < us || ts == us && nsec(t) < nsec(u)
}

// Go: time.go:Time.Compare
pub fn compare(t: &Time, u: &Time) -> i64 {
    let (mut tc, mut uc) = (sec(t), sec(u));
    if tc == uc {
        tc = nsec(t) as i64;
        uc = nsec(u) as i64;
    }
    if tc < uc {
        return -1;
    }
    if tc > uc {
        return 1;
    }
    0
}

// Go: time.go:Time.Equal
pub fn equal(t: &Time, u: &Time) -> bool {
    sec(t) == sec(u) && nsec(t) == nsec(u)
}

// Go: time.go:Time.IsZero
pub fn is_zero(t: &Time) -> bool {
    sec(t) == 0 && nsec(t) == 0
}

// ---------------------------------------------------------------------------
// Month and Weekday.
// ---------------------------------------------------------------------------

/// Go: `time.Month` (January = 1, ...). Wraps a Go `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Month(pub i64);

impl Month {
    pub const JANUARY: Month = Month(1);
    pub const FEBRUARY: Month = Month(2);
    pub const MARCH: Month = Month(3);
    pub const APRIL: Month = Month(4);
    pub const MAY: Month = Month(5);
    pub const JUNE: Month = Month(6);
    pub const JULY: Month = Month(7);
    pub const AUGUST: Month = Month(8);
    pub const SEPTEMBER: Month = Month(9);
    pub const OCTOBER: Month = Month(10);
    pub const NOVEMBER: Month = Month(11);
    pub const DECEMBER: Month = Month(12);

    // Go: time.go:Month.String
    /// The English name of the month ("January", "February", ...).
    pub fn string(self) -> String {
        if Month::JANUARY <= self && self <= Month::DECEMBER {
            return LONG_MONTH_NAMES[(self.0 - 1) as usize].to_string();
        }
        let mut buf = [0u8; 20];
        let n = fmt_int(&mut buf, self.0 as u64);
        format!("%!Month({})", String::from_utf8_lossy(&buf[n..]))
    }
}

impl std::fmt::Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string())
    }
}

/// Go: `time.Weekday` (Sunday = 0, ...). Wraps a Go `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Weekday(pub i64);

impl Weekday {
    pub const SUNDAY: Weekday = Weekday(0);
    pub const MONDAY: Weekday = Weekday(1);
    pub const TUESDAY: Weekday = Weekday(2);
    pub const WEDNESDAY: Weekday = Weekday(3);
    pub const THURSDAY: Weekday = Weekday(4);
    pub const FRIDAY: Weekday = Weekday(5);
    pub const SATURDAY: Weekday = Weekday(6);

    // Go: time.go:Weekday.String
    /// The English name of the day ("Sunday", "Monday", ...).
    pub fn string(self) -> String {
        if Weekday::SUNDAY <= self && self <= Weekday::SATURDAY {
            return LONG_DAY_NAMES[self.0 as usize].to_string();
        }
        let mut buf = [0u8; 20];
        let n = fmt_int(&mut buf, self.0 as u64);
        format!("%!Weekday({})", String::from_utf8_lossy(&buf[n..]))
    }
}

impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string())
    }
}

// ---------------------------------------------------------------------------
// Calendar computations (absolute times).
// ---------------------------------------------------------------------------

pub(crate) const SECONDS_PER_MINUTE: i64 = 60;
pub(crate) const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
pub(crate) const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
#[allow(dead_code)]
pub(crate) const SECONDS_PER_WEEK: i64 = 7 * SECONDS_PER_DAY;
#[allow(dead_code)]
pub(crate) const DAYS_PER_400_YEARS: i64 = 365 * 400 + 97;

/// Days from March 1 through end of year.
pub(crate) const MARCH_THRU_DECEMBER: i64 = 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30 + 31;

/// The number of years subtracted from internal time to get absolute time.
pub(crate) const ABSOLUTE_YEARS: i64 = 292277022400;

/// Go: `absoluteToInternal = -(absoluteYears*365.2425 + marchThruDecember) * secondsPerDay`
/// (`absoluteYears*365.2425` is exact: `absoluteYears` is a multiple of 400).
pub(crate) const ABSOLUTE_TO_INTERNAL: i64 =
    -((ABSOLUTE_YEARS * 365 + ABSOLUTE_YEARS / 400 * 97 + MARCH_THRU_DECEMBER) * SECONDS_PER_DAY);
pub(crate) const INTERNAL_TO_ABSOLUTE: i64 = -ABSOLUTE_TO_INTERNAL;

pub(crate) const UNIX_TO_INTERNAL: i64 =
    (1969 * 365 + 1969 / 4 - 1969 / 100 + 1969 / 400) * SECONDS_PER_DAY;
pub(crate) const INTERNAL_TO_UNIX: i64 = -UNIX_TO_INTERNAL;

pub(crate) const ABSOLUTE_TO_UNIX: i64 = ABSOLUTE_TO_INTERNAL + INTERNAL_TO_UNIX;
#[allow(dead_code)]
pub(crate) const UNIX_TO_ABSOLUTE: i64 = UNIX_TO_INTERNAL + INTERNAL_TO_ABSOLUTE;

const _: () = assert!(UNIX_TO_INTERNAL == go_value::UNIX_TO_INTERNAL);

// Go: time.go:dateToAbsDays
/// Number of days from the absolute epoch to year/month/day (day may be out of range).
pub(crate) fn date_to_abs_days(year: i64, month: Month, day: i64) -> u64 {
    let mut amonth = month.0 as u32;
    let mut jan_feb: u32 = 0;
    if amonth < 3 {
        jan_feb = 1;
    }
    amonth = amonth.wrapping_add(12u32.wrapping_mul(jan_feb));
    let y = (year as u64)
        .wrapping_sub(jan_feb as u64)
        .wrapping_add(ABSOLUTE_YEARS as u64);

    // ayday := (153*amonth - 457) / 5, computed as (979*amonth - 2919) >> 5.
    let ayday = 979u32.wrapping_mul(amonth).wrapping_sub(2919) >> 5;

    let century = y / 100;
    let cyear = (y % 100) as u32;
    let cday = 1461u32.wrapping_mul(cyear) / 4;
    let centurydays = 146097u64.wrapping_mul(century) / 4;

    centurydays.wrapping_add(
        ((cday.wrapping_add(ayday) as i64)
            .wrapping_add(day)
            .wrapping_sub(1)) as u64,
    )
}

// Go: time.go:absSeconds.days
#[inline]
pub(crate) fn abs_days(abs: u64) -> u64 {
    abs / SECONDS_PER_DAY as u64
}

// Go: time.go:absDays.split
/// Splits days into (century, cyear, ayday).
pub(crate) fn days_split(days: u64) -> (u64, i64, i64) {
    let d = 4u64.wrapping_mul(days).wrapping_add(3);
    let century = d / 146097;

    // cd := uint32(d%146097) | 3
    let cd = ((d % 146097) as u32) | 3;

    // hi, lo := bits.Mul32(2939745, cd)
    let prod = 2939745u64 * cd as u64;
    let hi = (prod >> 32) as u32;
    let lo = prod as u32;
    let cyear = hi as i64;
    let ayday = (lo / 2939745 / 4) as i64;
    (century, cyear, ayday)
}

// Go: time.go:absYday.split
/// Splits ayday into absolute month and standard (1-based) day-in-month.
pub(crate) fn ayday_split(ayday: i64) -> (i64, i64) {
    let d = 2141u32.wrapping_mul(ayday as u32).wrapping_add(197913);
    ((d >> 16) as i64, 1 + ((d & 0xFFFF) / 2141) as i64)
}

// Go: time.go:absYday.janFeb
pub(crate) fn ayday_jan_feb(ayday: i64) -> i64 {
    let mut jf = 0;
    if ayday >= MARCH_THRU_DECEMBER {
        jf = 1;
    }
    jf
}

// Go: time.go:absMonth.month
pub(crate) fn amonth_month(m: i64, jan_feb: i64) -> Month {
    Month(m.wrapping_sub(jan_feb.wrapping_mul(12)))
}

// Go: time.go:absCentury.leap
pub(crate) fn century_leap(century: u64, cyear: i64) -> i64 {
    let mut y4ok = 0;
    if cyear % 4 == 0 {
        y4ok = 1;
    }
    let mut y100ok = 0;
    if cyear != 0 {
        y100ok = 1;
    }
    let mut y400ok = 0;
    if century % 4 == 0 {
        y400ok = 1;
    }
    y4ok & (y100ok | y400ok)
}

// Go: time.go:absCentury.year
pub(crate) fn century_year(century: u64, cyear: i64, jan_feb: i64) -> i64 {
    (century
        .wrapping_mul(100)
        .wrapping_sub(ABSOLUTE_YEARS as u64) as i64)
        .wrapping_add(cyear)
        .wrapping_add(jan_feb)
}

// Go: time.go:absYday.yday
pub(crate) fn ayday_yday(ayday: i64, jan_feb: i64, leap: i64) -> i64 {
    ayday + (1 + 31 + 28) + (leap & !jan_feb) - 365 * jan_feb
}

// Go: time.go:absDays.date
/// Converts days into standard year, month, day.
pub(crate) fn days_date(days: u64) -> (i64, Month, i64) {
    let (century, cyear, ayday) = days_split(days);
    let (amonth, day) = ayday_split(ayday);
    let jan_feb = ayday_jan_feb(ayday);
    let year = century_year(century, cyear, jan_feb);
    let month = amonth_month(amonth, jan_feb);
    (year, month, day)
}

// Go: time.go:absDays.yearYday
/// Converts days into the standard year and 1-based yday.
pub(crate) fn days_year_yday(days: u64) -> (i64, i64) {
    let (century, cyear, ayday) = days_split(days);
    let jan_feb = ayday_jan_feb(ayday);
    let year = century_year(century, cyear, jan_feb);
    let yday = ayday_yday(ayday, jan_feb, century_leap(century, cyear));
    (year, yday)
}

// Go: time.go:Time.absSec
/// The time as absolute seconds, adjusted by the zone offset.
pub(crate) fn abs_sec(t: &Time) -> u64 {
    let mut sec = unix_sec(t);
    if let Some(l) = &t.loc {
        // l != &utcLoc (a zone-less location looks up as offset 0).
        let offset = lookup(l, sec).offset;
        sec = sec.wrapping_add(offset);
    }
    sec.wrapping_add(UNIX_TO_INTERNAL + INTERNAL_TO_ABSOLUTE) as u64
}

// Go: time.go:Time.locabs
/// Zone name, offset and absolute seconds from a single zone lookup.
pub(crate) fn locabs(t: &Time) -> (&str, i64, u64) {
    let mut sec = unix_sec(t);
    let name;
    match &t.loc {
        Some(l) => {
            let z = lookup(l, sec);
            name = z.name;
            sec = sec.wrapping_add(z.offset);
            let abs = sec.wrapping_add(UNIX_TO_INTERNAL + INTERNAL_TO_ABSOLUTE) as u64;
            return (name, z.offset, abs);
        }
        None => {
            name = "UTC";
        }
    }
    let abs = sec.wrapping_add(UNIX_TO_INTERNAL + INTERNAL_TO_ABSOLUTE) as u64;
    (name, 0, abs)
}

// Go: time.go:Time.Date
pub fn date_of(t: &Time) -> (i64, Month, i64) {
    days_date(abs_days(abs_sec(t)))
}

// Go: time.go:Time.Year
pub fn year(t: &Time) -> i64 {
    let (century, cyear, ayday) = days_split(abs_days(abs_sec(t)));
    let jan_feb = ayday_jan_feb(ayday);
    century_year(century, cyear, jan_feb)
}

// Go: time.go:Time.Month
pub fn month(t: &Time) -> Month {
    let (_, _, ayday) = days_split(abs_days(abs_sec(t)));
    let (amonth, _) = ayday_split(ayday);
    amonth_month(amonth, ayday_jan_feb(ayday))
}

// Go: time.go:Time.Day
pub fn day(t: &Time) -> i64 {
    let (_, _, ayday) = days_split(abs_days(abs_sec(t)));
    let (_, day) = ayday_split(ayday);
    day
}

// Go: time.go:Time.Weekday
pub fn weekday(t: &Time) -> Weekday {
    days_weekday(abs_days(abs_sec(t)))
}

// Go: time.go:absDays.weekday
pub(crate) fn days_weekday(days: u64) -> Weekday {
    // March 1 of the absolute year, like March 1 of 2000, was a Wednesday.
    Weekday((days.wrapping_add(Weekday::WEDNESDAY.0 as u64) % 7) as i64)
}

// Go: time.go:Time.ISOWeek
pub fn iso_week(t: &Time) -> (i64, i64) {
    let days = abs_days(abs_sec(t));
    let delta = Weekday::THURSDAY.0 - (days_weekday(days.wrapping_sub(1)).0 + 1);
    let thu = days.wrapping_add(delta as u64);
    let (year, yday) = days_year_yday(thu);
    (year, (yday - 1) / 7 + 1)
}

// Go: time.go:Time.Clock
pub fn clock(t: &Time) -> (i64, i64, i64) {
    abs_clock(abs_sec(t))
}

// Go: time.go:absSeconds.clock
pub(crate) fn abs_clock(abs: u64) -> (i64, i64, i64) {
    let mut sec = (abs % SECONDS_PER_DAY as u64) as i64;
    let hour = sec / SECONDS_PER_HOUR;
    sec -= hour * SECONDS_PER_HOUR;
    let min = sec / SECONDS_PER_MINUTE;
    sec -= min * SECONDS_PER_MINUTE;
    (hour, min, sec)
}

// Go: time.go:Time.Hour
pub fn hour(t: &Time) -> i64 {
    (abs_sec(t) % SECONDS_PER_DAY as u64) as i64 / SECONDS_PER_HOUR
}

// Go: time.go:Time.Minute
pub fn minute(t: &Time) -> i64 {
    (abs_sec(t) % SECONDS_PER_HOUR as u64) as i64 / SECONDS_PER_MINUTE
}

// Go: time.go:Time.Second
pub fn second(t: &Time) -> i64 {
    (abs_sec(t) % SECONDS_PER_MINUTE as u64) as i64
}

// Go: time.go:Time.Nanosecond
pub fn nanosecond(t: &Time) -> i64 {
    nsec(t) as i64
}

// Go: time.go:Time.YearDay
pub fn year_day(t: &Time) -> i64 {
    let (_, yday) = days_year_yday(abs_days(abs_sec(t)));
    yday
}

// ---------------------------------------------------------------------------
// Duration.
// ---------------------------------------------------------------------------

/// Go: `time.Duration`, an `int64` nanosecond count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration(pub i64);

const MIN_DURATION: Duration = Duration(i64::MIN);
const MAX_DURATION: Duration = Duration(i64::MAX);

impl Duration {
    pub const NANOSECOND: Duration = Duration(1);
    pub const MICROSECOND: Duration = Duration(1000);
    pub const MILLISECOND: Duration = Duration(1000 * 1000);
    pub const SECOND: Duration = Duration(1000 * 1000 * 1000);
    pub const MINUTE: Duration = Duration(60 * 1000 * 1000 * 1000);
    pub const HOUR: Duration = Duration(60 * 60 * 1000 * 1000 * 1000);

    // Go: time.go:Duration.String
    /// Formats the duration as e.g. "72h3m0.5s".
    pub fn string(self) -> String {
        let mut arr = [0u8; 32];
        let n = self.format(&mut arr);
        String::from_utf8_lossy(&arr[n..]).into_owned()
    }

    // Go: time.go:Duration.format
    fn format(self, buf: &mut [u8; 32]) -> usize {
        // Largest time is 2540400h10m10.000000000s
        let mut w = buf.len();

        let mut u = self.0 as u64;
        let neg = self.0 < 0;
        if neg {
            u = u.wrapping_neg();
        }

        if u < Duration::SECOND.0 as u64 {
            // Special case: if duration is smaller than a second,
            // use smaller units, like 1.2ms
            let prec;
            w -= 1;
            buf[w] = b's';
            w -= 1;
            if u == 0 {
                buf[w] = b'0';
                return w;
            } else if u < Duration::MICROSECOND.0 as u64 {
                // print nanoseconds
                prec = 0;
                buf[w] = b'n';
            } else if u < Duration::MILLISECOND.0 as u64 {
                // print microseconds
                prec = 3;
                // U+00B5 'µ' micro sign == 0xC2 0xB5
                w -= 1; // Need room for two bytes.
                buf[w..w + 2].copy_from_slice("µ".as_bytes());
            } else {
                // print milliseconds
                prec = 6;
                buf[w] = b'm';
            }
            let (nw, nu) = fmt_frac(&mut buf[..w], u, prec);
            w = nw;
            u = nu;
            w = fmt_int(&mut buf[..w], u);
        } else {
            w -= 1;
            buf[w] = b's';

            let (nw, nu) = fmt_frac(&mut buf[..w], u, 9);
            w = nw;
            u = nu;

            // u is now integer seconds
            w = fmt_int(&mut buf[..w], u % 60);
            u /= 60;

            // u is now integer minutes
            if u > 0 {
                w -= 1;
                buf[w] = b'm';
                w = fmt_int(&mut buf[..w], u % 60);
                u /= 60;

                // u is now integer hours
                // Stop at hours because days can be different lengths.
                if u > 0 {
                    w -= 1;
                    buf[w] = b'h';
                    w = fmt_int(&mut buf[..w], u);
                }
            }
        }

        if neg {
            w -= 1;
            buf[w] = b'-';
        }

        w
    }

    // Go: time.go:Duration.Nanoseconds
    pub fn nanoseconds(self) -> i64 {
        self.0
    }

    // Go: time.go:Duration.Microseconds
    pub fn microseconds(self) -> i64 {
        self.0 / 1000
    }

    // Go: time.go:Duration.Milliseconds
    pub fn milliseconds(self) -> i64 {
        self.0 / 1_000_000
    }

    // Go: time.go:Duration.Seconds
    pub fn seconds(self) -> f64 {
        let sec = self.0 / Duration::SECOND.0;
        let nsec = self.0 % Duration::SECOND.0;
        sec as f64 + nsec as f64 / 1e9
    }

    // Go: time.go:Duration.Minutes
    pub fn minutes(self) -> f64 {
        let min = self.0 / Duration::MINUTE.0;
        let nsec = self.0 % Duration::MINUTE.0;
        min as f64 + nsec as f64 / (60.0 * 1e9)
    }

    // Go: time.go:Duration.Hours
    pub fn hours(self) -> f64 {
        let hour = self.0 / Duration::HOUR.0;
        let nsec = self.0 % Duration::HOUR.0;
        hour as f64 + nsec as f64 / (60.0 * 60.0 * 1e9)
    }

    // Go: time.go:Duration.Truncate
    pub fn truncate(self, m: Duration) -> Duration {
        if m.0 <= 0 {
            return self;
        }
        Duration(self.0 - self.0 % m.0)
    }

    // Go: time.go:Duration.Round
    pub fn round(self, m: Duration) -> Duration {
        if m.0 <= 0 {
            return self;
        }
        let d = self.0;
        let m = m.0;
        let mut r = d % m;
        if d < 0 {
            r = -r;
            if less_than_half(Duration(r), Duration(m)) {
                return Duration(d + r);
            }
            let d1 = d.wrapping_sub(m).wrapping_add(r);
            if d1 < d {
                return Duration(d1);
            }
            return MIN_DURATION; // overflow
        }
        if less_than_half(Duration(r), Duration(m)) {
            return Duration(d - r);
        }
        let d1 = d.wrapping_add(m).wrapping_sub(r);
        if d1 > d {
            return Duration(d1);
        }
        MAX_DURATION // overflow
    }

    // Go: time.go:Duration.Abs
    pub fn abs(self) -> Duration {
        if self.0 >= 0 {
            self
        } else if self == MIN_DURATION {
            MAX_DURATION
        } else {
            Duration(-self.0)
        }
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string())
    }
}

// Go: time.go:fmtFrac
/// Formats the fraction of v/10**prec (e.g., ".12345") into the tail of buf,
/// omitting trailing zeros (and the point when the fraction is 0).
fn fmt_frac(buf: &mut [u8], mut v: u64, prec: i64) -> (usize, u64) {
    // Omit trailing zeros up to and including decimal point.
    let mut w = buf.len();
    let mut print = false;
    for _ in 0..prec {
        let digit = v % 10;
        print = print || digit != 0;
        if print {
            w -= 1;
            buf[w] = digit as u8 + b'0';
        }
        v /= 10;
    }
    if print {
        w -= 1;
        buf[w] = b'.';
    }
    (w, v)
}

// Go: time.go:fmtInt
/// Formats v into the tail of buf, returning the index where the output begins.
pub(crate) fn fmt_int(buf: &mut [u8], mut v: u64) -> usize {
    let mut w = buf.len();
    if v == 0 {
        w -= 1;
        buf[w] = b'0';
    } else {
        while v > 0 {
            w -= 1;
            buf[w] = (v % 10) as u8 + b'0';
            v /= 10;
        }
    }
    w
}

// Go: time.go:lessThanHalf
fn less_than_half(x: Duration, y: Duration) -> bool {
    (x.0 as u64).wrapping_add(x.0 as u64) < y.0 as u64
}

// ---------------------------------------------------------------------------
// Arithmetic.
// ---------------------------------------------------------------------------

// Go: time.go:Time.Add
pub fn add(t: &Time, d: Duration) -> Time {
    let mut dsec = d.0 / 1_000_000_000;
    let mut nsec = nsec(t) + (d.0 % 1_000_000_000) as i32;
    if nsec >= 1_000_000_000 {
        dsec += 1;
        nsec -= 1_000_000_000;
    } else if nsec < 0 {
        dsec -= 1;
        nsec += 1_000_000_000;
    }
    let mut t = t.clone();
    t.nsec = nsec as u32; // update nsec
    add_sec(&mut t, dsec);
    t
}

// Go: time.go:Time.Sub
pub fn sub(t: &Time, u: &Time) -> Duration {
    let d = Duration(
        sec(t)
            .wrapping_sub(sec(u))
            .wrapping_mul(Duration::SECOND.0)
            .wrapping_add((nsec(t) - nsec(u)) as i64),
    );
    // Check for overflow or underflow.
    if equal(&add(u, d), t) {
        d // d is correct
    } else if before(t, u) {
        MIN_DURATION // t - u is negative out of range
    } else {
        MAX_DURATION // t - u is positive out of range
    }
}

// Go: time.go:Since
/// Time elapsed since t (wall clock; no monotonic reading is modelled).
pub fn since(t: &Time) -> Duration {
    sub(&now(), t)
}

// Go: time.go:Until
pub fn until(t: &Time) -> Duration {
    sub(t, &now())
}

// Go: time.go:Time.AddDate
pub fn add_date(t: &Time, years: i64, months: i64, days: i64) -> Time {
    let (year, month, day) = date_of(t);
    let (hour, min, sec) = clock(t);
    date(
        year.wrapping_add(years),
        Month(month.0.wrapping_add(months)),
        day.wrapping_add(days),
        hour,
        min,
        sec,
        nsec(t) as i64,
        &location(t),
    )
}

// Go: time.go:daysBefore
/// Number of days in a non-leap year before month m.
pub(crate) fn days_before(m: Month) -> i64 {
    let mut adj = 0;
    if m >= Month::MARCH {
        adj = -2;
    }
    (214i64.wrapping_mul(m.0).wrapping_sub(211)) / 7 + adj
}

// Go: time.go:daysIn
pub(crate) fn days_in(m: Month, year: i64) -> i64 {
    if m == Month::FEBRUARY {
        if is_leap(year) {
            return 29;
        }
        return 28;
    }
    30 + (m.0.wrapping_add(m.0 >> 3) & 1)
}

// Go: time.go:Now
/// The current local time (no monotonic reading).
pub fn now() -> Time {
    let (sec, nsec) = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos() as i64),
        Err(e) => {
            let d = e.duration();
            (-(d.as_secs() as i64), -(d.subsec_nanos() as i64))
        }
    };
    let t = unix(sec, nsec);
    Time {
        unix_sec: t.unix_sec,
        nsec: t.nsec,
        loc: Some(local()),
    }
}

// Go: time.go:unixTime
/// `Time{uint64(nsec), sec + unixToInternal, Local}`.
pub(crate) fn unix_time(sec: i64, nsec: i32) -> Time {
    Time {
        unix_sec: sec,
        nsec: nsec as u32,
        loc: Some(local()),
    }
}

// Go: time.go:Time.UTC
pub fn to_utc(t: &Time) -> Time {
    let mut t = t.clone();
    t.loc = None;
    t
}

// Go: time.go:Time.Local
pub fn to_local(t: &Time) -> Time {
    let mut t = t.clone();
    set_loc(&mut t, &local());
    t
}

// Go: time.go:Time.In
pub fn in_location(t: &Time, loc: &Arc<Location>) -> Time {
    let mut t = t.clone();
    set_loc(&mut t, loc);
    t
}

// Go: time.go:Time.Location
/// The time zone information associated with t (nil maps to UTC).
pub fn location(t: &Time) -> Arc<Location> {
    match &t.loc {
        Some(l) => l.clone(),
        None => utc(),
    }
}

// Go: time.go:Time.Zone
/// The zone abbreviation and offset (seconds east of UTC) in effect at t.
pub fn zone(t: &Time) -> (String, i64) {
    let z = lookup_opt(t.loc.as_ref(), unix_sec(t));
    (z.name.to_string(), z.offset)
}

// Go: time.go:Time.ZoneBounds
/// The bounds of the zone in effect at t; zero Times for open ends.
pub fn zone_bounds(t: &Time) -> (Time, Time) {
    let z = lookup_opt(t.loc.as_ref(), unix_sec(t));
    let (start_sec, end_sec) = (z.start, z.end);
    let mut start = Time::zero();
    let mut end = Time::zero();
    if start_sec != ALPHA {
        start = Time {
            unix_sec: start_sec,
            nsec: 0,
            loc: t.loc.clone(),
        };
    }
    if end_sec != OMEGA {
        end = Time {
            unix_sec: end_sec,
            nsec: 0,
            loc: t.loc.clone(),
        };
    }
    (start, end)
}

// Go: time.go:Time.Unix
pub fn unix_of(t: &Time) -> i64 {
    unix_sec(t)
}

// Go: time.go:Time.UnixMilli
pub fn unix_milli_of(t: &Time) -> i64 {
    unix_sec(t)
        .wrapping_mul(1000)
        .wrapping_add(nsec(t) as i64 / 1_000_000)
}

// Go: time.go:Time.UnixMicro
pub fn unix_micro_of(t: &Time) -> i64 {
    unix_sec(t)
        .wrapping_mul(1_000_000)
        .wrapping_add(nsec(t) as i64 / 1000)
}

// Go: time.go:Time.UnixNano
pub fn unix_nano_of(t: &Time) -> i64 {
    unix_sec(t)
        .wrapping_mul(1_000_000_000)
        .wrapping_add(nsec(t) as i64)
}

const HAS_MONOTONIC: u64 = 1 << 63;
const NSEC_MASK: u64 = (1 << 30) - 1;
const NSEC_SHIFT: u64 = 30;
/// Go: `wallToInternal`, seconds from year 1 to year 1885.
const WALL_TO_INTERNAL: i64 = (1884 * 365 + 1884 / 4 - 1884 / 100 + 1884 / 400) * SECONDS_PER_DAY;

const TIME_BINARY_VERSION_V1: u8 = 1; // For general situation
const TIME_BINARY_VERSION_V2: u8 = 2; // For LMT only

// Go: time.go:Time.AppendBinary
/// Appends the binary form of t to b; on error b is left unchanged.
pub fn append_binary(t: &Time, b: &mut Vec<u8>) -> Result<(), TimeError> {
    let offset_min: i16; // minutes east of UTC. -1 is UTC.
    let mut offset_sec: i8 = 0;
    let mut version = TIME_BINARY_VERSION_V1;

    if location_is_utc(t) {
        offset_min = -1;
    } else {
        let (_, mut offset) = zone(t);
        if offset % 60 != 0 {
            version = TIME_BINARY_VERSION_V2;
            offset_sec = (offset % 60) as i8;
        }

        offset /= 60;
        if !(-32768..=32767).contains(&offset) || offset == -1 {
            return Err(TimeError::new("Time.MarshalBinary: unexpected zone offset"));
        }
        offset_min = offset as i16;
    }

    let sec = sec(t);
    let nsec = nsec(t);
    b.extend_from_slice(&[
        version,           // byte 0 : version
        (sec >> 56) as u8, // bytes 1-8: seconds
        (sec >> 48) as u8,
        (sec >> 40) as u8,
        (sec >> 32) as u8,
        (sec >> 24) as u8,
        (sec >> 16) as u8,
        (sec >> 8) as u8,
        sec as u8,
        (nsec >> 24) as u8, // bytes 9-12: nanoseconds
        (nsec >> 16) as u8,
        (nsec >> 8) as u8,
        nsec as u8,
        (offset_min >> 8) as u8, // bytes 13-14: zone offset in minutes
        offset_min as u8,
    ]);
    if version == TIME_BINARY_VERSION_V2 {
        b.push(offset_sec as u8);
    }
    Ok(())
}

/// Go: `t.Location() == UTC`.
fn location_is_utc(t: &Time) -> bool {
    match &t.loc {
        None => true,
        Some(l) => is_utc_loc(l),
    }
}

// Go: time.go:Time.MarshalBinary
pub fn marshal_binary(t: &Time) -> Result<Vec<u8>, TimeError> {
    let mut b = Vec::with_capacity(16);
    append_binary(t, &mut b)?;
    Ok(b)
}

// Go: time.go:(*Time).UnmarshalBinary
pub fn unmarshal_binary(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    let mut buf = data;
    if buf.is_empty() {
        return Err(TimeError::new("Time.UnmarshalBinary: no data"));
    }

    let version = buf[0];
    if version != TIME_BINARY_VERSION_V1 && version != TIME_BINARY_VERSION_V2 {
        return Err(TimeError::new("Time.UnmarshalBinary: unsupported version"));
    }

    let mut want_len = /*version*/ 1 + /*sec*/ 8 + /*nsec*/ 4 + /*zone offset*/ 2;
    if version == TIME_BINARY_VERSION_V2 {
        want_len += 1;
    }
    if buf.len() != want_len {
        return Err(TimeError::new("Time.UnmarshalBinary: invalid length"));
    }

    buf = &buf[1..];
    let sec = (buf[7] as i64)
        | (buf[6] as i64) << 8
        | (buf[5] as i64) << 16
        | (buf[4] as i64) << 24
        | (buf[3] as i64) << 32
        | (buf[2] as i64) << 40
        | (buf[1] as i64) << 48
        | (buf[0] as i64) << 56;

    buf = &buf[8..];
    let nsec =
        (buf[3] as i32) | (buf[2] as i32) << 8 | (buf[1] as i32) << 16 | (buf[0] as i32) << 24;

    buf = &buf[4..];
    let mut offset = (((buf[1] as i16) | (buf[0] as i16) << 8) as i64) * 60;
    if version == TIME_BINARY_VERSION_V2 {
        offset += (buf[2] as i8) as i64;
    }

    *t = Time::zero();
    // t.wall = uint64(nsec); t.ext = sec. uint64 of a negative int32
    // sign-extends, which sets Go's hasMonotonic bit; the setLoc below then
    // runs stripMono, replacing ext by the (garbage) 33-bit wall seconds.
    // Emulated here up front (the Local lookup reads the same sec()).
    let wall = nsec as i64 as u64;
    let mut ext = sec;
    if wall & HAS_MONOTONIC != 0 {
        ext = WALL_TO_INTERNAL.wrapping_add((wall << 1 >> (NSEC_SHIFT + 1)) as i64);
    }
    t.nsec = (wall & NSEC_MASK) as u32;
    set_sec(t, ext);

    if offset == -60 {
        t.loc = None;
    } else {
        let l = local();
        let localoff = lookup(&l, unix_sec(t)).offset;
        if offset == localoff {
            set_loc(t, &l);
        } else {
            set_loc(t, &fixed_zone("", offset));
        }
    }

    Ok(())
}

// Go: time.go:Time.MarshalJSON
pub fn marshal_json(t: &Time) -> Result<Vec<u8>, TimeError> {
    let mut b = Vec::with_capacity(crate::format::RFC3339_NANO.len() + 2);
    b.push(b'"');
    let (mut b, err) = append_strict_rfc3339(t, b);
    b.push(b'"');
    if let Some(err) = err {
        return Err(TimeError::new(format!("Time.MarshalJSON: {}", err)));
    }
    Ok(b)
}

// Go: time.go:(*Time).UnmarshalJSON
pub fn unmarshal_json(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    if data == b"null" {
        return Ok(());
    }
    // TODO(https://go.dev/issue/47353): Properly unescape a JSON string.
    if data.len() < 2 || data[0] != b'"' || data[data.len() - 1] != b'"' {
        return Err(TimeError::new(
            "Time.UnmarshalJSON: input is not a JSON string",
        ));
    }
    let data = &data[1..data.len() - 1];
    match parse_strict_rfc3339(data) {
        Ok(v) => {
            *t = v;
            Ok(())
        }
        Err(e) => {
            *t = Time::zero();
            Err(TimeError::from(e))
        }
    }
}

// Go: time.go:Time.appendTo
fn append_to(t: &Time, b: Vec<u8>, err_prefix: &str) -> Result<Vec<u8>, TimeError> {
    let (b, err) = append_strict_rfc3339(t, b);
    if let Some(err) = err {
        return Err(TimeError::new(format!("{}{}", err_prefix, err)));
    }
    Ok(b)
}

// Go: time.go:Time.AppendText
pub fn append_text(t: &Time, b: Vec<u8>) -> Result<Vec<u8>, TimeError> {
    append_to(t, b, "Time.AppendText: ")
}

// Go: time.go:Time.MarshalText
pub fn marshal_text(t: &Time) -> Result<Vec<u8>, TimeError> {
    append_to(
        t,
        Vec::with_capacity(crate::format::RFC3339_NANO.len()),
        "Time.MarshalText: ",
    )
}

// Go: time.go:(*Time).UnmarshalText
pub fn unmarshal_text(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    match parse_strict_rfc3339(data) {
        Ok(v) => {
            *t = v;
            Ok(())
        }
        Err(e) => {
            *t = Time::zero();
            Err(TimeError::from(e))
        }
    }
}

// Go: time.go:Unix
/// The local Time for the given Unix time (`nsec` may be out of range).
pub fn unix(sec: i64, nsec: i64) -> Time {
    let mut sec = sec;
    let mut nsec = nsec;
    if !(0..1_000_000_000).contains(&nsec) {
        let n = nsec / 1_000_000_000;
        sec = sec.wrapping_add(n);
        nsec -= n * 1_000_000_000;
        if nsec < 0 {
            nsec += 1_000_000_000;
            sec = sec.wrapping_sub(1);
        }
    }
    unix_time(sec, nsec as i32)
}

// Go: time.go:UnixMilli
pub fn unix_milli(msec: i64) -> Time {
    unix(msec / 1000, (msec % 1000) * 1_000_000)
}

// Go: time.go:UnixMicro
pub fn unix_micro(usec: i64) -> Time {
    unix(usec / 1_000_000, (usec % 1_000_000) * 1000)
}

// Go: time.go:Time.IsDST
pub fn is_dst(t: &Time) -> bool {
    lookup_opt(t.loc.as_ref(), unix_sec(t)).is_dst
}

// Go: time.go:isLeap
pub(crate) fn is_leap(year: i64) -> bool {
    // year%4 == 0 && (year%100 != 0 || year%400 == 0)
    // Bottom 2 bits must be clear.
    // For multiples of 25, bottom 4 bits must be clear.
    let mut mask = 0xf;
    if year % 25 != 0 {
        mask = 3;
    }
    year & mask == 0
}

// Go: time.go:norm
/// Returns nhi, nlo such that hi*base + lo == nhi*base + nlo, 0 <= nlo < base.
fn norm(hi: i64, lo: i64, base: i64) -> (i64, i64) {
    let mut hi = hi;
    let mut lo = lo;
    if lo < 0 {
        let n = (lo.wrapping_neg().wrapping_sub(1)) / base + 1;
        hi = hi.wrapping_sub(n);
        lo = lo.wrapping_add(n.wrapping_mul(base));
    }
    if lo >= base {
        let n = lo / base;
        hi = hi.wrapping_add(n);
        lo = lo.wrapping_sub(n.wrapping_mul(base));
    }
    (hi, lo)
}

// Go: time.go:Date
/// The Time for yyyy-mm-dd hh:mm:ss + nsec in the given location; out-of-range
/// values are normalised (October 32 is November 1).
#[allow(clippy::too_many_arguments)]
pub fn date(
    year: i64,
    month: Month,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
    nsec: i64,
    loc: &Arc<Location>,
) -> Time {
    // Normalize month, overflowing into year.
    let m = month.0.wrapping_sub(1);
    let (year, m) = norm(year, m, 12);
    let month = Month(m + 1);

    // Normalize nsec, sec, min, hour, overflowing into day.
    let (sec, nsec) = norm(sec, nsec, 1_000_000_000);
    let (min, sec) = norm(min, sec, 60);
    let (hour, min) = norm(hour, min, 60);
    let (day, hour) = norm(day, hour, 24);

    // Convert to absolute time and then Unix time.
    let mut unix = (date_to_abs_days(year, month, day) as i64)
        .wrapping_mul(SECONDS_PER_DAY)
        .wrapping_add(
            hour.wrapping_mul(SECONDS_PER_HOUR)
                .wrapping_add(min.wrapping_mul(SECONDS_PER_MINUTE))
                .wrapping_add(sec),
        )
        .wrapping_add(ABSOLUTE_TO_UNIX);

    // Look for zone offset for expected time, so we can adjust to UTC.
    // The lookup function expects UTC, so first we pass unix in the
    // hope that it will not be too close to a zone transition,
    // and then adjust if it is.
    let z = lookup(loc, unix);
    let mut offset = z.offset;
    if offset != 0 {
        let utc = unix.wrapping_sub(offset);
        // If utc is valid for the time zone we found, then we have the right offset.
        // If not, we get the correct offset by looking up utc in the location.
        if utc < z.start || utc >= z.end {
            offset = lookup(loc, utc).offset;
        }
        unix = unix.wrapping_sub(offset);
    }

    // t := unixTime(unix, int32(nsec)); t.setLoc(loc)
    let mut t = Time {
        unix_sec: unix,
        nsec: nsec as i32 as u32,
        loc: None,
    };
    set_loc(&mut t, loc);
    t
}

// Go: time.go:Time.Truncate
/// Rounds t down to a multiple of d (since the zero time).
pub fn truncate(t: &Time, d: Duration) -> Time {
    if d.0 <= 0 {
        return t.clone();
    }
    let (_, r) = div(t, d);
    add(t, Duration(r.0.wrapping_neg()))
}

// Go: time.go:Time.Round
/// Rounds t to the nearest multiple of d (since the zero time), halfway up.
pub fn round(t: &Time, d: Duration) -> Time {
    if d.0 <= 0 {
        return t.clone();
    }
    let (_, r) = div(t, d);
    if less_than_half(r, d) {
        return add(t, Duration(r.0.wrapping_neg()));
    }
    add(t, Duration(d.0.wrapping_sub(r.0)))
}

// Go: time.go:div
/// Divides t by d and returns the quotient parity and remainder.
fn div(t: &Time, d: Duration) -> (i64, Duration) {
    let mut neg = false;
    let mut nsec = nsec(t);
    let mut sec = sec(t);
    if sec < 0 {
        // Operate on absolute value.
        neg = true;
        sec = sec.wrapping_neg();
        nsec = -nsec;
        if nsec < 0 {
            nsec += 1_000_000_000;
            sec = sec.wrapping_sub(1); // sec >= 1 before the -- so safe
        }
    }

    let d = d.0;
    let mut qmod2: i64;
    let mut r: i64;
    if d < Duration::SECOND.0 && Duration::SECOND.0 % (d + d) == 0 {
        // Special case: 2d divides 1 second.
        qmod2 = ((nsec / d as i32) & 1) as i64;
        r = (nsec % d as i32) as i64;
    } else if d % Duration::SECOND.0 == 0 {
        // Special case: d is a multiple of 1 second.
        let d1 = d / Duration::SECOND.0;
        qmod2 = sec.wrapping_div(d1) & 1;
        r = sec
            .wrapping_rem(d1)
            .wrapping_mul(Duration::SECOND.0)
            .wrapping_add(nsec as i64);
    } else {
        // General case.
        // Compute nanoseconds as 128-bit number.
        let sec = sec as u64;
        let mut tmp = (sec >> 32).wrapping_mul(1_000_000_000);
        let mut u1 = tmp >> 32;
        let mut u0 = tmp << 32;
        tmp = (sec & 0xFFFF_FFFF).wrapping_mul(1_000_000_000);
        let mut u0x = u0;
        u0 = u0.wrapping_add(tmp);
        if u0 < u0x {
            u1 = u1.wrapping_add(1);
        }
        u0x = u0;
        u0 = u0.wrapping_add(nsec as u64);
        if u0 < u0x {
            u1 = u1.wrapping_add(1);
        }

        // Compute remainder by subtracting r<<k for decreasing k.
        // Quotient parity is whether we subtract on last round.
        let mut d1 = d as u64;
        while d1 >> 63 != 1 {
            d1 <<= 1;
        }
        let mut d0: u64 = 0;
        loop {
            qmod2 = 0;
            if u1 > d1 || u1 == d1 && u0 >= d0 {
                // subtract
                qmod2 = 1;
                u0x = u0;
                u0 = u0.wrapping_sub(d0);
                if u0 > u0x {
                    u1 = u1.wrapping_sub(1);
                }
                u1 = u1.wrapping_sub(d1);
            }
            if d1 == 0 && d0 == d as u64 {
                break;
            }
            d0 >>= 1;
            d0 |= (d1 & 1) << 63;
            d1 >>= 1;
        }
        r = u0 as i64;
    }

    if neg && r != 0 {
        // If input was negative and not an exact multiple of d, we computed q, r such that
        //	q*d + r = -t
        // But the right answers are given by -(q-1), d-r:
        qmod2 ^= 1;
        // Wrapping like Go: r is negative when sec() == MinInt64 (the
        // negation above wraps), and d - r can overflow.
        r = d.wrapping_sub(r);
    }
    (qmod2, Duration(r))
}

// ---------------------------------------------------------------------------
// String forms that live in format.go but are Time methods.
// ---------------------------------------------------------------------------

// Go: format.go:Time.String
/// `t.Format("2006-01-02 15:04:05.999999999 -0700 MST")` (no monotonic suffix).
pub fn string(t: &Time) -> String {
    crate::format::format(t, "2006-01-02 15:04:05.999999999 -0700 MST")
}

// Go: format.go:Time.GoString
pub fn go_string(t: &Time) -> String {
    let abs = abs_sec(t);
    let (year, month, day) = days_date(abs_days(abs));
    let (hour, minute, second) = abs_clock(abs);

    let mut buf: Vec<u8> = Vec::with_capacity(
        "time.Date(9999, time.September, 31, 23, 59, 59, 999999999, time.Local)".len(),
    );
    buf.extend_from_slice(b"time.Date(");
    append_int(&mut buf, year, 0);
    if Month::JANUARY <= month && month <= Month::DECEMBER {
        buf.extend_from_slice(b", time.");
        buf.extend_from_slice(LONG_MONTH_NAMES[(month.0 - 1) as usize].as_bytes());
    } else {
        // It's difficult to construct a time.Time with a date outside the
        // standard range but we might as well try to handle the case.
        append_int(&mut buf, month.0, 0);
    }
    buf.extend_from_slice(b", ");
    append_int(&mut buf, day, 0);
    buf.extend_from_slice(b", ");
    append_int(&mut buf, hour, 0);
    buf.extend_from_slice(b", ");
    append_int(&mut buf, minute, 0);
    buf.extend_from_slice(b", ");
    append_int(&mut buf, second, 0);
    buf.extend_from_slice(b", ");
    append_int(&mut buf, nanosecond(t), 0);
    buf.extend_from_slice(b", ");
    match &t.loc {
        None => buf.extend_from_slice(b"time.UTC"),
        Some(l) if is_utc_loc(l) => buf.extend_from_slice(b"time.UTC"),
        Some(l) if is_local_loc(l) => buf.extend_from_slice(b"time.Local"),
        Some(l) => {
            buf.extend_from_slice(b"time.Location(");
            buf.extend_from_slice(&quote(l.name.as_bytes()));
            buf.push(b')');
        }
    }
    buf.push(b')');
    String::from_utf8_lossy(&buf).into_owned()
}

/// Go: `t.appendFormatRFC3339(b, nanos)`; exposed for the format fast path.
#[allow(dead_code)]
pub(crate) fn append_rfc3339(t: &Time, b: &mut Vec<u8>, nanos: bool) {
    append_format_rfc3339(t, b, nanos);
}
