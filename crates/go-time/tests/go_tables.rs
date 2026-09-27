//! Ports of Go's own `time` test tables (format_test.go, time_test.go).
//! Like Go's tests (ForceUSPacificForTesting), Local is America/Los_Angeles
//! named "Local"; this is its own test binary so it does not clash with the
//! oracle tests' Local.

#![allow(clippy::type_complexity)]

mod common;

use std::sync::{Arc, Once};

use go_time::{Duration, GoTimeExt, Location, Month, Time, Weekday};

static SETUP: Once = Once::new();

fn setup() {
    SETUP.call_once(|| {
        let data = common::zone_data("America/Los_Angeles");
        go_time::set_local(go_time::load_location_from_tz_data("Local", &data).unwrap());
    });
}

fn utc() -> Arc<Location> {
    go_time::utc()
}

fn local() -> Arc<Location> {
    setup();
    go_time::local()
}

#[allow(clippy::too_many_arguments)]
fn date(y: i64, m: i64, d: i64, h: i64, mi: i64, s: i64, ns: i64, l: &Arc<Location>) -> Time {
    go_time::date(y, Month(m), d, h, mi, s, ns, l)
}

const SECOND: i64 = 1_000_000_000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;

// Go: format_test.go:TestRFC3339Conversion
#[test]
fn rfc3339_conversion() {
    let cases = [
        (
            date(2008, 9, 17, 20, 4, 26, 0, &utc()),
            "2008-09-17T20:04:26Z",
        ),
        (
            date(
                1994,
                9,
                17,
                20,
                4,
                26,
                0,
                &go_time::fixed_zone("EST", -18000),
            ),
            "1994-09-17T20:04:26-05:00",
        ),
        (
            date(
                2000,
                12,
                26,
                1,
                15,
                6,
                0,
                &go_time::fixed_zone("OTO", 15600),
            ),
            "2000-12-26T01:15:06+04:20",
        ),
    ];
    for (t, want) in cases {
        assert_eq!(t.format(go_time::RFC3339), want);
    }
}

// Go: format_test.go:TestFormat
#[test]
fn format() {
    setup();
    let tests: &[(&str, &str)] = &[
        (go_time::ANSIC, "Wed Feb  4 21:00:57 2009"),
        (go_time::UNIX_DATE, "Wed Feb  4 21:00:57 PST 2009"),
        (go_time::RUBY_DATE, "Wed Feb 04 21:00:57 -0800 2009"),
        (go_time::RFC822, "04 Feb 09 21:00 PST"),
        (go_time::RFC850, "Wednesday, 04-Feb-09 21:00:57 PST"),
        (go_time::RFC1123, "Wed, 04 Feb 2009 21:00:57 PST"),
        (go_time::RFC1123Z, "Wed, 04 Feb 2009 21:00:57 -0800"),
        (go_time::RFC3339, "2009-02-04T21:00:57-08:00"),
        (go_time::RFC3339_NANO, "2009-02-04T21:00:57.0123456-08:00"),
        (go_time::KITCHEN, "9:00PM"),
        ("3pm", "9pm"),
        ("3PM", "9PM"),
        ("06 01 02", "09 02 04"),
        (
            "Hi Janet, the Month is January",
            "Hi Janet, the Month is February",
        ),
        (go_time::STAMP, "Feb  4 21:00:57"),
        (go_time::STAMP_MILLI, "Feb  4 21:00:57.012"),
        (go_time::STAMP_MICRO, "Feb  4 21:00:57.012345"),
        (go_time::STAMP_NANO, "Feb  4 21:00:57.012345600"),
        (go_time::DATE_TIME, "2009-02-04 21:00:57"),
        (go_time::DATE_ONLY, "2009-02-04"),
        (go_time::TIME_ONLY, "21:00:57"),
        ("Jan  2 002 __2 2", "Feb  4 035  35 4"),
        ("2006 6 06 _6 __6 ___6", "2009 6 09 _6 __6 ___6"),
        ("Jan January 1 01 _1", "Feb February 2 02 _2"),
        ("2 02 _2 __2", "4 04  4  35"),
        ("Mon Monday", "Wed Wednesday"),
        ("15 3 03 _3", "21 9 09 _9"),
        ("4 04 _4", "0 00 _0"),
        ("5 05 _5", "57 57 _57"),
    ];
    // The numeric time represents Thu Feb  4 21:00:57.012345600 PST 2009
    let t = go_time::unix(0, 1233810057012345600);
    for &(layout, want) in tests {
        assert_eq!(t.format(layout), want, "layout {:?}", layout);
    }
    // Go: TestFormatFractionalSecondSeparators
    for (layout, want) in [
        ("15:04:05.000", "21:00:57.012"),
        ("15:04:05.999", "21:00:57.012"),
        ("15:04:05,000", "21:00:57,012"),
        ("15:04:05,999", "21:00:57,012"),
    ] {
        assert_eq!(t.format(layout), want);
    }
}

// Go: format_test.go:TestGoString
#[test]
fn go_string() {
    let l = local();
    let cases = [
        (
            date(2009, 2, 5, 5, 0, 57, 12345600, &utc()),
            "time.Date(2009, time.February, 5, 5, 0, 57, 12345600, time.UTC)",
        ),
        (
            date(2009, 2, 5, 5, 0, 57, 12345600, &l),
            "time.Date(2009, time.February, 5, 5, 0, 57, 12345600, time.Local)",
        ),
        (
            date(
                2009,
                2,
                5,
                5,
                0,
                57,
                12345600,
                &go_time::fixed_zone("Europe/Berlin", 3 * 60 * 60),
            ),
            r#"time.Date(2009, time.February, 5, 5, 0, 57, 12345600, time.Location("Europe/Berlin"))"#,
        ),
        (
            date(
                2009,
                2,
                5,
                5,
                0,
                57,
                12345600,
                &go_time::fixed_zone("Non-ASCII character ⏰", 3 * 60 * 60),
            ),
            r#"time.Date(2009, time.February, 5, 5, 0, 57, 12345600, time.Location("Non-ASCII character \xe2\x8f\xb0"))"#,
        ),
    ];
    for (t, want) in cases {
        assert_eq!(t.go_string(), want);
    }
}

// Go: format_test.go:TestFormatSingleDigits, TestFormatShortYear
#[test]
fn format_single_digits_and_short_year() {
    let t = date(2001, 2, 3, 4, 5, 6, 700000000, &utc());
    assert_eq!(t.format("3:4:5"), "4:5:6");
    let years = [
        -100001, -100000, -99999, -10001, -10000, -9999, -1001, -1000, -999, -101, -100, -99, -11,
        -10, -9, -1, 0, 1, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10000, 10001, 99999,
        100000, 100001,
    ];
    for y in years {
        let t = date(y, 1, 1, 0, 0, 0, 0, &utc());
        let want = if y < 0 {
            format!("-{:04}.{:02}.{:02}", -y, 1, 1)
        } else {
            format!("{:04}.{:02}.{:02}", y, 1, 1)
        };
        assert_eq!(t.format("2006.01.02"), want);
    }
}

/// Go: format_test.go:checkTime (the time should be Thu Feb  4 21:00:57 PST 2010).
fn check_time(
    t: &Time,
    name: &str,
    has_tz: bool,
    has_wd: bool,
    year_sign: i64,
    frac_digits: usize,
) {
    if year_sign >= 0 {
        assert_eq!(year_sign * t.year(), 2010, "{}: bad year", name);
    }
    assert_eq!(t.month(), Month::FEBRUARY, "{}: bad month", name);
    assert_eq!(t.day(), 4, "{}: bad day", name);
    assert_eq!(t.hour(), 21, "{}: bad hour", name);
    assert_eq!(t.minute(), 0, "{}: bad minute", name);
    assert_eq!(t.second(), 57, "{}: bad second", name);
    let ns: i64 = format!(
        "{}{}",
        &"012345678"[..frac_digits],
        &"000000000"[..9 - frac_digits]
    )
    .parse()
    .unwrap();
    assert_eq!(t.nanosecond(), ns, "{}: bad nanosecond", name);
    let (_, off) = t.zone();
    if has_tz {
        assert_eq!(off, -28800, "{}: bad tz offset", name);
    }
    if has_wd {
        assert_eq!(t.weekday(), Weekday::THURSDAY, "{}: bad weekday", name);
    }
}

// Go: format_test.go:TestParse, TestRubyParse
#[test]
fn parse() {
    setup();
    let tests: &[(&str, &str, bool, bool, i64, usize)] = &[
        (
            go_time::ANSIC,
            "Thu Feb  4 21:00:57 2010",
            false,
            true,
            1,
            0,
        ),
        (
            go_time::UNIX_DATE,
            "Thu Feb  4 21:00:57 PST 2010",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RUBY_DATE,
            "Thu Feb 04 21:00:57 -0800 2010",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RFC850,
            "Thursday, 04-Feb-10 21:00:57 PST",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RFC1123,
            "Thu, 04 Feb 2010 21:00:57 PST",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RFC1123,
            "Thu, 04 Feb 2010 22:00:57 PDT",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RFC1123Z,
            "Thu, 04 Feb 2010 21:00:57 -0800",
            true,
            true,
            1,
            0,
        ),
        (
            go_time::RFC3339,
            "2010-02-04T21:00:57-08:00",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05-07",
            "2010-02-04 21:00:57-08",
            true,
            false,
            1,
            0,
        ),
        (
            go_time::ANSIC,
            "Thu Feb  4 21:00:57.0 2010",
            false,
            true,
            1,
            1,
        ),
        (
            go_time::UNIX_DATE,
            "Thu Feb  4 21:00:57.01 PST 2010",
            true,
            true,
            1,
            2,
        ),
        (
            go_time::RUBY_DATE,
            "Thu Feb 04 21:00:57.012 -0800 2010",
            true,
            true,
            1,
            3,
        ),
        (
            go_time::RFC850,
            "Thursday, 04-Feb-10 21:00:57.0123 PST",
            true,
            true,
            1,
            4,
        ),
        (
            go_time::RFC1123,
            "Thu, 04 Feb 2010 21:00:57.01234 PST",
            true,
            true,
            1,
            5,
        ),
        (
            go_time::RFC1123Z,
            "Thu, 04 Feb 2010 21:00:57.01234 -0800",
            true,
            true,
            1,
            5,
        ),
        (
            go_time::RFC3339,
            "2010-02-04T21:00:57.012345678-08:00",
            true,
            false,
            1,
            9,
        ),
        (
            "2006-01-02 15:04:05",
            "2010-02-04 21:00:57.0",
            false,
            false,
            1,
            0,
        ),
        (go_time::ANSIC, "Thu Feb 4 21:00:57 2010", false, true, 1, 0),
        (
            go_time::ANSIC,
            "Thu      Feb     4     21:00:57     2010",
            false,
            true,
            1,
            0,
        ),
        (go_time::ANSIC, "THU FEB 4 21:00:57 2010", false, true, 1, 0),
        (go_time::ANSIC, "thu feb 4 21:00:57 2010", false, true, 1, 0),
        (
            "Mon Jan _2 15:04:05.000 2006",
            "Thu Feb  4 21:00:57.012 2010",
            false,
            true,
            1,
            3,
        ),
        (
            "Mon Jan _2 15:04:05.000000 2006",
            "Thu Feb  4 21:00:57.012345 2010",
            false,
            true,
            1,
            6,
        ),
        (
            "Mon Jan _2 15:04:05.000000000 2006",
            "Thu Feb  4 21:00:57.012345678 2010",
            false,
            true,
            1,
            9,
        ),
        (
            "Mon Jan _2 15:04:05,000 2006",
            "Thu Feb  4 21:00:57.012 2010",
            false,
            true,
            1,
            3,
        ),
        (
            "Mon Jan _2 15:04:05,000000 2006",
            "Thu Feb  4 21:00:57.012345 2010",
            false,
            true,
            1,
            6,
        ),
        (
            "Mon Jan _2 15:04:05,000000000 2006",
            "Thu Feb  4 21:00:57.012345678 2010",
            false,
            true,
            1,
            9,
        ),
        (
            "2006.01.02.15.04.05.0",
            "2010.02.04.21.00.57.0",
            false,
            false,
            1,
            1,
        ),
        (
            "2006.01.02.15.04.05.00",
            "2010.02.04.21.00.57.01",
            false,
            false,
            1,
            2,
        ),
        (
            "Hi Janet, the Month is January: Jan _2 15:04:05 2006",
            "Hi Janet, the Month is February: Feb  4 21:00:57 2010",
            false,
            true,
            1,
            0,
        ),
        (
            go_time::UNIX_DATE,
            "Fri Feb  5 05:00:57 GMT-8 2010",
            true,
            true,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05.9999 -0700 MST",
            "2010-02-04 21:00:57 -0800 PST",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05.999999999 -0700 MST",
            "2010-02-04 21:00:57 -0800 PST",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05.9999 -0700 MST",
            "2010-02-04 21:00:57.0123 -0800 PST",
            true,
            false,
            1,
            4,
        ),
        (
            "2006-01-02 15:04:05.999999999 -0700 MST",
            "2010-02-04 21:00:57.0123 -0800 PST",
            true,
            false,
            1,
            4,
        ),
        (
            "2006-01-02 15:04:05.9999 -0700 MST",
            "2010-02-04 21:00:57.012345678 -0800 PST",
            true,
            false,
            1,
            9,
        ),
        (
            "2006-01-02 15:04:05.999999999 -0700 MST",
            "2010-02-04 21:00:57.012345678 -0800 PST",
            true,
            false,
            1,
            9,
        ),
        (
            "2006-01-02 15:04:05,9999 -0700 MST",
            "2010-02-04 21:00:57 -0800 PST",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05,999999999 -0700 MST",
            "2010-02-04 21:00:57 -0800 PST",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02 15:04:05,9999 -0700 MST",
            "2010-02-04 21:00:57.0123 -0800 PST",
            true,
            false,
            1,
            4,
        ),
        (
            "2006-01-02 15:04:05,999999999 -0700 MST",
            "2010-02-04 21:00:57.0123 -0800 PST",
            true,
            false,
            1,
            4,
        ),
        (
            "2006-01-02 15:04:05,9999 -0700 MST",
            "2010-02-04 21:00:57.012345678 -0800 PST",
            true,
            false,
            1,
            9,
        ),
        (
            "2006-01-02 15:04:05,999999999 -0700 MST",
            "2010-02-04 21:00:57.012345678 -0800 PST",
            true,
            false,
            1,
            9,
        ),
        (
            go_time::STAMP_NANO,
            "Feb  4 21:00:57.012345678",
            false,
            false,
            -1,
            9,
        ),
        (
            "Jan _2 15:04:05.999",
            "Feb  4 21:00:57.012300000",
            false,
            false,
            -1,
            4,
        ),
        (
            "Jan _2 15:04:05.999",
            "Feb  4 21:00:57.012345678",
            false,
            false,
            -1,
            9,
        ),
        (
            "Jan _2 15:04:05.999999999",
            "Feb  4 21:00:57.0123",
            false,
            false,
            -1,
            4,
        ),
        (
            "Jan _2 15:04:05.999999999",
            "Feb  4 21:00:57.012345678",
            false,
            false,
            -1,
            9,
        ),
        (
            "2006-01-02 002 15:04:05",
            "2010-02-04 035 21:00:57",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01 002 15:04:05",
            "2010-02 035 21:00:57",
            false,
            false,
            1,
            0,
        ),
        ("2006-002 15:04:05", "2010-035 21:00:57", false, false, 1, 0),
        (
            "200600201 15:04:05",
            "201003502 21:00:57",
            false,
            false,
            1,
            0,
        ),
        (
            "200600204 15:04:05",
            "201003504 21:00:57",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07",
            "2010-02-04T21:00:57Z",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07",
            "2010-02-04T21:00:57+08",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07",
            "2010-02-04T21:00:57-08",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z0700",
            "2010-02-04T21:00:57Z",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z0700",
            "2010-02-04T21:00:57+0800",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z0700",
            "2010-02-04T21:00:57-0800",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00",
            "2010-02-04T21:00:57Z",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00",
            "2010-02-04T21:00:57+08:00",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00",
            "2010-02-04T21:00:57-08:00",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z070000",
            "2010-02-04T21:00:57Z",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z070000",
            "2010-02-04T21:00:57+080000",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z070000",
            "2010-02-04T21:00:57-080000",
            true,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00:00",
            "2010-02-04T21:00:57Z",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00:00",
            "2010-02-04T21:00:57+08:00:00",
            false,
            false,
            1,
            0,
        ),
        (
            "2006-01-02T15:04:05Z07:00:00",
            "2010-02-04T21:00:57-08:00:00",
            true,
            false,
            1,
            0,
        ),
        (
            go_time::RUBY_DATE,
            "Thu Feb 04 21:00:57 -0000 2010",
            false,
            true,
            1,
            0,
        ),
        (
            go_time::RUBY_DATE,
            "Thu Feb 04 21:00:57 +0000 2010",
            false,
            true,
            1,
            0,
        ),
        (
            go_time::RUBY_DATE,
            "Thu Feb 04 21:00:57 +1130 2010",
            false,
            true,
            1,
            0,
        ),
    ];
    for &(layout, value, has_tz, has_wd, year_sign, frac) in tests {
        let t = go_time::parse(layout, value).unwrap_or_else(|e| panic!("{} error: {}", layout, e));
        check_time(&t, layout, has_tz, has_wd, year_sign, frac);
    }
}

// Go: format_test.go:TestParseDayOutOfRange, TestParseMonthOutOfRange
#[test]
fn parse_out_of_range() {
    setup();
    let day_tests: &[(&str, bool)] = &[
        ("Thu Jan 99 21:00:57 2010", false),
        ("Thu Jan 31 21:00:57 2010", true),
        ("Thu Jan 32 21:00:57 2010", false),
        ("Thu Feb 28 21:00:57 2012", true),
        ("Thu Feb 29 21:00:57 2012", true),
        ("Thu Feb 29 21:00:57 2010", false),
        ("Thu Mar 31 21:00:57 2010", true),
        ("Thu Mar 32 21:00:57 2010", false),
        ("Thu Apr 30 21:00:57 2010", true),
        ("Thu Apr 31 21:00:57 2010", false),
        ("Thu May 31 21:00:57 2010", true),
        ("Thu May 32 21:00:57 2010", false),
        ("Thu Jun 30 21:00:57 2010", true),
        ("Thu Jun 31 21:00:57 2010", false),
        ("Thu Jul 31 21:00:57 2010", true),
        ("Thu Jul 32 21:00:57 2010", false),
        ("Thu Aug 31 21:00:57 2010", true),
        ("Thu Aug 32 21:00:57 2010", false),
        ("Thu Sep 30 21:00:57 2010", true),
        ("Thu Sep 31 21:00:57 2010", false),
        ("Thu Oct 31 21:00:57 2010", true),
        ("Thu Oct 32 21:00:57 2010", false),
        ("Thu Nov 30 21:00:57 2010", true),
        ("Thu Nov 31 21:00:57 2010", false),
        ("Thu Dec 31 21:00:57 2010", true),
        ("Thu Dec 32 21:00:57 2010", false),
        ("Thu Dec 00 21:00:57 2010", false),
    ];
    for &(d, ok) in day_tests {
        match go_time::parse(go_time::ANSIC, d) {
            Ok(_) => assert!(ok, "{:?}: expected 'day' error", d),
            Err(e) => {
                assert!(!ok, "{:?}: unexpected error {}", d, e);
                assert!(e.error().contains("day out of range"), "{:?}: {}", d, e);
            }
        }
    }
    for (v, ok) in [("00-01", false), ("13-01", false), ("01-01", true)] {
        match go_time::parse("01-02", v) {
            Ok(_) => assert!(ok),
            Err(e) => assert!(!ok && e.error().contains("month out of range"), "{}", e),
        }
    }
}

// Go: format_test.go:TestParseErrors
#[test]
fn parse_errors() {
    setup();
    let tests: &[(&str, &str, &str)] = &[
        (
            go_time::ANSIC,
            "Feb  4 21:00:60 2010",
            r#"cannot parse "Feb  4 21:00:60 2010" as "Mon""#,
        ),
        (
            go_time::ANSIC,
            "Thu Feb  4 21:00:57 @2010",
            r#"cannot parse "@2010" as "2006""#,
        ),
        (
            go_time::ANSIC,
            "Thu Feb  4 21:00:60 2010",
            "second out of range",
        ),
        (
            go_time::ANSIC,
            "Thu Feb  4 21:61:57 2010",
            "minute out of range",
        ),
        (
            go_time::ANSIC,
            "Thu Feb  4 24:00:60 2010",
            "hour out of range",
        ),
        (
            "Mon Jan _2 15:04:05.000 2006",
            "Thu Feb  4 23:00:59x01 2010",
            r#"cannot parse "x01 2010" as ".000""#,
        ),
        (
            "Mon Jan _2 15:04:05.000 2006",
            "Thu Feb  4 23:00:59.xxx 2010",
            r#"cannot parse ".xxx 2010" as ".000""#,
        ),
        (
            "Mon Jan _2 15:04:05.000 2006",
            "Thu Feb  4 23:00:59.-123 2010",
            "fractional second out of range",
        ),
        (
            go_time::STAMP_NANO,
            "Dec  7 11:22:01.000000",
            r#"cannot parse ".000000" as ".000000000""#,
        ),
        (
            go_time::STAMP_NANO,
            "Dec  7 11:22:01.0000000000",
            r#"extra text: "0""#,
        ),
        (
            go_time::RFC3339,
            "2006-01-02T15:04:05Z07:00",
            r#"parsing time "2006-01-02T15:04:05Z07:00": extra text: "07:00""#,
        ),
        (
            go_time::RFC3339,
            "2006-01-02T15:04_abc",
            r#"parsing time "2006-01-02T15:04_abc" as "2006-01-02T15:04:05Z07:00": cannot parse "_abc" as ":""#,
        ),
        (
            go_time::RFC3339,
            "2006-01-02T15:04:05_abc",
            r#"parsing time "2006-01-02T15:04:05_abc" as "2006-01-02T15:04:05Z07:00": cannot parse "_abc" as "Z07:00""#,
        ),
        (
            go_time::RFC3339,
            "2006-01-02T15:04:05Z_abc",
            r#"parsing time "2006-01-02T15:04:05Z_abc": extra text: "_abc""#,
        ),
        (
            go_time::RFC3339,
            "2010-02-04T21:00:67.012345678-08:00",
            "second out of range",
        ),
        (
            go_time::RFC3339,
            "0000-01-01T00:00:.0+00:00",
            r#"parsing time "0000-01-01T00:00:.0+00:00" as "2006-01-02T15:04:05Z07:00": cannot parse ".0+00:00" as "05""#,
        ),
        (
            "_2 Jan 06 15:04 MST",
            "4 --- 00 00:00 GMT",
            r#"cannot parse "--- 00 00:00 GMT" as "Jan""#,
        ),
        (
            "_2 January 06 15:04 MST",
            "4 --- 00 00:00 GMT",
            r#"cannot parse "--- 00 00:00 GMT" as "January""#,
        ),
        (
            "Jan _2 002 2006",
            "Feb  4 034 2006",
            "day-of-year does not match day",
        ),
        (
            "Jan _2 002 2006",
            "Feb  4 004 2006",
            "day-of-year does not match month",
        ),
        (
            r#""2006-01-02T15:04:05Z07:00""#,
            "0",
            r#"parsing time "0" as "\"2006-01-02T15:04:05Z07:00\"": cannot parse "0" as "\"""#,
        ),
        (
            go_time::RFC3339,
            "\"",
            r#"parsing time "\"" as "2006-01-02T15:04:05Z07:00": cannot parse "\"" as "2006""#,
        ),
        (
            go_time::RFC3339,
            "0000-01-01T00:00:00+00:+0",
            r#"parsing time "0000-01-01T00:00:00+00:+0" as "2006-01-02T15:04:05Z07:00": cannot parse "+00:+0" as "Z07:00""#,
        ),
        (
            go_time::RFC3339,
            "0000-01-01T00:00:00+-0:00",
            r#"parsing time "0000-01-01T00:00:00+-0:00" as "2006-01-02T15:04:05Z07:00": cannot parse "+-0:00" as "Z07:00""#,
        ),
        (
            "2006-01-02",
            "22-10-25",
            r#"parsing time "22-10-25" as "2006-01-02": cannot parse "22-10-25" as "2006""#,
        ),
        (
            "06-01-02",
            "a2-10-25",
            r#"parsing time "a2-10-25" as "06-01-02": cannot parse "a2-10-25" as "06""#,
        ),
        (
            "03:04PM",
            "12:03pM",
            r#"parsing time "12:03pM" as "03:04PM": cannot parse "pM" as "PM""#,
        ),
        (
            "03:04pm",
            "12:03pM",
            r#"parsing time "12:03pM" as "03:04pm": cannot parse "pM" as "pm""#,
        ),
        ("-07", "-25", "time zone offset hour out of range"),
        ("-07:00", "+25:00", "time zone offset hour out of range"),
        ("-07:00", "-23:61", "time zone offset minute out of range"),
        (
            "-07:00:00",
            "+23:59:61",
            "time zone offset second out of range",
        ),
        ("Z07", "-25", "time zone offset hour out of range"),
        ("Z07:00", "+25:00", "time zone offset hour out of range"),
        ("Z07:00", "-23:61", "time zone offset minute out of range"),
        (
            "Z07:00:00",
            "+23:59:61",
            "time zone offset second out of range",
        ),
    ];
    for &(layout, value, expect) in tests {
        let e = go_time::parse(layout, value).expect_err(value);
        assert!(
            e.error().contains(expect),
            "{:?} {:?}: got {}",
            layout,
            value,
            e
        );
    }
    // Go: TestStd0xParseError
    for (layout, value, prefix) in [
        ("01 MST", "0 MST", "0"),
        ("01 MST", "1 MST", "1"),
        (go_time::RFC850, "Thursday, 04-Feb-1 21:00:57 PST", "1"),
    ] {
        let e = go_time::parse(layout, value).expect_err(value);
        assert!(
            e.error().contains("cannot parse") && e.value_elem.starts_with(prefix.as_bytes()),
            "{}",
            e
        );
    }
}

// Go: format_test.go: 12PM/12AM tests, TestMissingZone, TestMinutesInTimeZone,
// TestParseSecondsInTimeZone, TestFormatSecondsInTimeZone, TestUnderscoreTwoThousand
#[test]
fn parse_misc() {
    setup();
    let noon = date(0, 1, 1, 12, 0, 0, 0, &utc());
    assert_eq!(noon.format("3:04PM"), "12:00PM");
    assert_eq!(noon.format("03:04PM"), "12:00PM");
    let midnight = date(0, 1, 1, 0, 0, 0, 0, &utc());
    assert_eq!(midnight.format("3:04PM"), "12:00AM");
    assert_eq!(midnight.format("03:04PM"), "12:00AM");
    assert_eq!(go_time::parse("3:04PM", "12:00PM").unwrap().hour(), 12);
    assert_eq!(go_time::parse("03:04PM", "12:00PM").unwrap().hour(), 12);
    assert_eq!(go_time::parse("3:04PM", "12:00AM").unwrap().hour(), 0);
    assert_eq!(go_time::parse("03:04PM", "12:00AM").unwrap().hour(), 0);

    let t = go_time::parse(go_time::RUBY_DATE, "Thu Feb 02 16:10:03 -0500 2006").unwrap();
    assert_eq!(
        t.format(go_time::UNIX_DATE),
        "Thu Feb  2 16:10:03 -0500 2006"
    );
    let t = go_time::parse(go_time::RUBY_DATE, "Mon Jan 02 15:04:05 +0123 2006").unwrap();
    assert_eq!(t.zone().1, (60 + 23) * 60);

    let secs: &[(&str, &str, i64)] = &[
        (
            "2006-01-02T15:04:05-070000",
            "1871-01-01T05:33:02-003408",
            -(34 * 60 + 8),
        ),
        (
            "2006-01-02T15:04:05-07:00:00",
            "1871-01-01T05:33:02-00:34:08",
            -(34 * 60 + 8),
        ),
        (
            "2006-01-02T15:04:05-070000",
            "1871-01-01T05:33:02+003408",
            34 * 60 + 8,
        ),
        (
            "2006-01-02T15:04:05-07:00:00",
            "1871-01-01T05:33:02+00:34:08",
            34 * 60 + 8,
        ),
        (
            "2006-01-02T15:04:05Z070000",
            "1871-01-01T05:33:02-003408",
            -(34 * 60 + 8),
        ),
        (
            "2006-01-02T15:04:05Z07:00:00",
            "1871-01-01T05:33:02+00:34:08",
            34 * 60 + 8,
        ),
        ("2006-01-02T15:04:05-07", "1871-01-01T05:33:02+01", 60 * 60),
        (
            "2006-01-02T15:04:05-07",
            "1871-01-01T05:33:02-02",
            -2 * 60 * 60,
        ),
        (
            "2006-01-02T15:04:05Z07",
            "1871-01-01T05:33:02-02",
            -2 * 60 * 60,
        ),
    ];
    for &(layout, value, off) in secs {
        assert_eq!(
            go_time::parse(layout, value).unwrap().zone().1,
            off,
            "{}",
            value
        );
        let d = date(1871, 1, 1, 5, 33, 2, 0, &go_time::fixed_zone("LMT", off));
        assert_eq!(d.format(layout), value);
    }

    let t = go_time::parse("15:04_20060102", "14:38_20150618").unwrap();
    assert_eq!(t.date(), (2015, Month(6), 18));
    assert_eq!((t.hour(), t.minute()), (14, 38));

    // Go: TestParseYday
    for i in 1..=365 {
        let d = format!("2020-{:03}", i);
        let tm = go_time::parse("2006-002", &d).unwrap();
        assert_eq!((tm.year(), tm.year_day()), (2020, i));
    }

    // Go: TestParseFractionalSecondsLongerThanNineDigits
    let long: &[(&str, i64)] = &[
        ("2021-09-29T16:04:33.000000000Z", 0),
        ("2021-09-29T16:04:33.000000001Z", 1),
        ("2021-09-29T16:04:33.100000000Z", 100_000_000),
        ("2021-09-29T16:04:33.100000001Z", 100_000_001),
        ("2021-09-29T16:04:33.999999999Z", 999_999_999),
        ("2021-09-29T16:04:33.012345678Z", 12_345_678),
        ("2021-09-29T16:04:33.0000000000Z", 0),
        ("2021-09-29T16:04:33.0000000001Z", 0),
        ("2021-09-29T16:04:33.1000000000Z", 100_000_000),
        ("2021-09-29T16:04:33.1000000009Z", 100_000_000),
        ("2021-09-29T16:04:33.9999999999Z", 999_999_999),
        ("2021-09-29T16:04:33.0123456789Z", 12_345_678),
        ("2021-09-29T16:04:33.10000000000Z", 100_000_000),
        ("2021-09-29T16:04:33.00123456789Z", 1_234_567),
        ("2021-09-29T16:04:33.000123456789Z", 123_456),
        ("2021-09-29T16:04:33.9999999999999999Z", 999_999_999),
    ];
    for &(v, want) in long {
        for layout in [go_time::RFC3339, go_time::RFC3339_NANO] {
            assert_eq!(
                go_time::parse(layout, v).unwrap().nanosecond(),
                want,
                "{}",
                v
            );
        }
    }
}

// Go: time_test.go:TestZeroTime, TestUnixUTC/TestUnix (utctests, localtests)
#[test]
fn zero_and_unix() {
    let zero = Time::zero();
    assert_eq!(zero.date(), (1, Month::JANUARY, 1));
    assert_eq!(zero.clock(), (0, 0, 0));
    assert_eq!(
        (zero.nanosecond(), zero.year_day(), zero.weekday()),
        (0, 1, Weekday::MONDAY)
    );

    type Golden = (i64, i64, i64, i64, i64, i64, i64, i64, i64, &'static str);
    let utctests: &[(i64, Golden)] = &[
        (0, (1970, 1, 1, 0, 0, 0, 0, 4, 0, "UTC")),
        (1221681866, (2008, 9, 17, 20, 4, 26, 0, 3, 0, "UTC")),
        (-1221681866, (1931, 4, 16, 3, 55, 34, 0, 4, 0, "UTC")),
        (-11644473600, (1601, 1, 1, 0, 0, 0, 0, 1, 0, "UTC")),
        (599529660, (1988, 12, 31, 0, 1, 0, 0, 6, 0, "UTC")),
        (978220860, (2000, 12, 31, 0, 1, 0, 0, 0, 0, "UTC")),
    ];
    let localtests: &[(i64, Golden)] = &[
        (0, (1969, 12, 31, 16, 0, 0, 0, 3, -8 * 3600, "PST")),
        (1221681866, (2008, 9, 17, 13, 4, 26, 0, 3, -7 * 3600, "PDT")),
        (2159200800, (2038, 6, 3, 11, 0, 0, 0, 4, -7 * 3600, "PDT")),
        (2152173599, (2038, 3, 14, 1, 59, 59, 0, 0, -8 * 3600, "PST")),
        (2152173600, (2038, 3, 14, 3, 0, 0, 0, 0, -7 * 3600, "PDT")),
        (2152173601, (2038, 3, 14, 3, 0, 1, 0, 0, -7 * 3600, "PDT")),
        (2172733199, (2038, 11, 7, 1, 59, 59, 0, 0, -7 * 3600, "PDT")),
        (2172733200, (2038, 11, 7, 1, 0, 0, 0, 0, -8 * 3600, "PST")),
        (2172733201, (2038, 11, 7, 1, 0, 1, 0, 0, -8 * 3600, "PST")),
    ];
    let same = |t: &Time, g: &Golden| {
        let (name, off) = t.zone();
        assert_eq!(
            (
                t.year(),
                t.month().0,
                t.day(),
                t.hour(),
                t.minute(),
                t.second(),
                t.nanosecond(),
                t.weekday().0,
                off
            ),
            (g.0, g.1, g.2, g.3, g.4, g.5, g.6, g.7, g.8)
        );
        assert_eq!(name, g.9);
        let (y, m, d) = t.date();
        assert_eq!((y, m.0, d), (g.0, g.1, g.2));
    };
    for (sec, g) in utctests {
        let t = go_time::unix(*sec, 0).utc();
        same(&t, g);
        assert_eq!(t.go_unix(), *sec);
    }
    setup();
    for (sec, g) in localtests {
        let t = go_time::unix(*sec, 0);
        same(&t, g);
        assert_eq!(t.go_unix(), *sec);
    }
    // nano variants
    let t = go_time::unix(0, 100_000_000).utc();
    assert_eq!(t.nanosecond(), 100_000_000);
    let t = go_time::unix(1221681866, 300_000_000);
    assert_eq!(
        (t.hour(), t.nanosecond(), t.zone().0),
        (13, 300_000_000, "PDT".to_string())
    );
}

// Go: time_test.go:TestUnixMilli/TestUnixMicro (quick.Check replaced by a
// deterministic sweep), TestTruncateRound (big-int reference via i128).
#[test]
fn unix_milli_micro_truncate_round() {
    let mut x: u64 = 0x9E3779B97F4A7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..20000 {
        let v = next() as i64;
        assert_eq!(go_time::unix_milli(v).unix_milli(), v);
        assert_eq!(go_time::unix_micro(v).unix_micro(), v);
    }

    // Go: unixToZero = -978307200 + 63113904000
    const UNIX_TO_ZERO: i128 = -978307200 + 63113904000;
    let test_one = |ti: i64, tns: i64, di: i64| {
        let t0 = go_time::unix(ti, tns).utc();
        let mut d = di;
        if d < 0 {
            d = d.wrapping_neg();
        }
        if d <= 0 {
            d = 1;
        }
        let bt = (t0.go_unix() as i128 + UNIX_TO_ZERO) * 1_000_000_000 + t0.nanosecond() as i128;
        let r = bt.rem_euclid(d as i128) as i64;
        let mut t1 = t0.add(Duration(-r));
        assert_eq!(t0.truncate(Duration(d)), t1, "Truncate({:?}, {})", t0, d);
        if r > d / 2 || r + r == d {
            t1 = t1.add(Duration(d));
        }
        assert_eq!(t0.round(Duration(d)), t1, "Round({:?}, {})", t0, d);
    };
    // truncateRoundTests
    let tt = date(-1, 1, 1, 12, 15, 30, 500_000_000, &utc());
    test_one(tt.go_unix(), tt.nanosecond(), 3);
    let tt = date(-1, 1, 1, 12, 15, 31, 500_000_000, &utc());
    test_one(tt.go_unix(), tt.nanosecond(), 3);
    let tt = date(2012, 1, 1, 12, 15, 30, 500_000_000, &utc());
    test_one(tt.go_unix(), tt.nanosecond(), SECOND);
    let tt = date(2012, 1, 1, 12, 15, 31, 500_000_000, &utc());
    test_one(tt.go_unix(), tt.nanosecond(), SECOND);
    test_one(-19012425939, 649146258, 7435029458905025217);
    // exhaustive small cases and random ones, as in the Go test
    for i in 0..=40 {
        for j in 0..=40 {
            test_one(i, j, 1 + (i + j) % 17);
        }
    }
    for _ in 0..10000 {
        let ti = (next() as i64) >> 4;
        let tns = (next() % 1_000_000_000) as i64;
        let di = match next() % 4 {
            0 => (next() % 1000) as i64,
            1 => (next() % 1_000_000_000_000) as i64,
            2 => (next() >> 1) as i64,
            _ => SECOND * (next() % 100000) as i64,
        };
        test_one(ti, tns, di);
    }
}

// Go: time_test.go:TestISOWeek, TestYearDay, TestDaysIn(via Date)
#[test]
fn iso_week_and_year_day() {
    let iso: &[(i64, i64, i64, i64, i64)] = &[
        (1981, 1, 1, 1981, 1),
        (1982, 1, 1, 1981, 53),
        (1983, 1, 1, 1982, 52),
        (1984, 1, 1, 1983, 52),
        (1985, 1, 1, 1985, 1),
        (1986, 1, 1, 1986, 1),
        (1987, 1, 1, 1987, 1),
        (1988, 1, 1, 1987, 53),
        (1989, 1, 1, 1988, 52),
        (1990, 1, 1, 1990, 1),
        (1991, 1, 1, 1991, 1),
        (1992, 1, 1, 1992, 1),
        (1993, 1, 1, 1992, 53),
        (1994, 1, 1, 1993, 52),
        (1995, 1, 2, 1995, 1),
        (1996, 1, 1, 1996, 1),
        (1996, 1, 7, 1996, 1),
        (1996, 1, 8, 1996, 2),
        (1997, 1, 1, 1997, 1),
        (1998, 1, 1, 1998, 1),
        (1999, 1, 1, 1998, 53),
        (2000, 1, 1, 1999, 52),
        (2001, 1, 1, 2001, 1),
        (2002, 1, 1, 2002, 1),
        (2003, 1, 1, 2003, 1),
        (2004, 1, 1, 2004, 1),
        (2005, 1, 1, 2004, 53),
        (2006, 1, 1, 2005, 52),
        (2007, 1, 1, 2007, 1),
        (2008, 1, 1, 2008, 1),
        (2009, 1, 1, 2009, 1),
        (2010, 1, 1, 2009, 53),
        (2011, 1, 1, 2010, 52),
        (2011, 1, 2, 2010, 52),
        (2011, 1, 3, 2011, 1),
        (2011, 1, 10, 2011, 2),
        (2011, 6, 12, 2011, 23),
        (2011, 6, 13, 2011, 24),
        (2011, 12, 25, 2011, 51),
        (2011, 12, 26, 2011, 52),
        (2011, 12, 31, 2011, 52),
        (1995, 1, 1, 1994, 52),
        (2012, 1, 1, 2011, 52),
        (2012, 1, 2, 2012, 1),
        (2012, 12, 31, 2013, 1),
        (2013, 12, 30, 2014, 1),
        (2014, 1, 6, 2014, 2),
        (2016, 1, 1, 2015, 53),
        (2021, 1, 1, 2020, 53),
        (2027, 1, 1, 2026, 53),
        (2033, 1, 1, 2032, 53),
        (2038, 1, 1, 2037, 53),
        (2040, 1, 1, 2039, 52),
    ];
    for &(y, m, d, yex, wex) in iso {
        assert_eq!(
            date(y, m, d, 0, 0, 0, 0, &utc()).iso_week(),
            (yex, wex),
            "{}-{}-{}",
            y,
            m,
            d
        );
    }
    for year in 1950..2100 {
        assert_eq!(date(year, 1, 4, 0, 0, 0, 0, &utc()).iso_week(), (year, 1));
    }

    let ydt: &[(i64, i64, i64, i64)] = &[
        (2007, 1, 1, 1),
        (2007, 1, 15, 15),
        (2007, 2, 1, 32),
        (2007, 2, 15, 46),
        (2007, 3, 1, 60),
        (2007, 3, 15, 74),
        (2007, 4, 1, 91),
        (2007, 12, 31, 365),
        (2008, 1, 1, 1),
        (2008, 1, 15, 15),
        (2008, 2, 1, 32),
        (2008, 2, 15, 46),
        (2008, 3, 1, 61),
        (2008, 3, 15, 75),
        (2008, 4, 1, 92),
        (2008, 12, 31, 366),
        (1900, 1, 1, 1),
        (1900, 1, 15, 15),
        (1900, 2, 1, 32),
        (1900, 2, 15, 46),
        (1900, 3, 1, 60),
        (1900, 3, 15, 74),
        (1900, 4, 1, 91),
        (1900, 12, 31, 365),
        (1, 1, 1, 1),
        (1, 1, 15, 15),
        (1, 2, 1, 32),
        (1, 2, 15, 46),
        (1, 3, 1, 60),
        (1, 3, 15, 74),
        (1, 4, 1, 91),
        (1, 12, 31, 365),
        (-1, 1, 1, 1),
        (-1, 1, 15, 15),
        (-1, 2, 1, 32),
        (-1, 2, 15, 46),
        (-1, 3, 1, 60),
        (-1, 3, 15, 74),
        (-1, 4, 1, 91),
        (-1, 12, 31, 365),
        (-400, 1, 1, 1),
        (-400, 1, 15, 15),
        (-400, 2, 1, 32),
        (-400, 2, 15, 46),
        (-400, 3, 1, 61),
        (-400, 3, 15, 75),
        (-400, 4, 1, 92),
        (-400, 12, 31, 366),
        (1582, 10, 4, 277),
        (1582, 10, 15, 288),
    ];
    let locs = [
        go_time::fixed_zone("UTC-8", -8 * 3600),
        go_time::fixed_zone("UTC-4", -4 * 3600),
        utc(),
        go_time::fixed_zone("UTC+4", 4 * 3600),
        go_time::fixed_zone("UTC+8", 8 * 3600),
    ];
    for (i, l) in locs.iter().enumerate() {
        for &(y, m, d, yday) in ydt {
            let dt = date(y, m, d, 0, 0, 0, 0, l);
            assert_eq!(dt.year_day(), yday);
            if !(0..=9999).contains(&y) {
                continue;
            }
            let off = (i as i64 - 2) * 4;
            let f = format!(
                "{:04}-{:02}-{:02} {:03} {}{:02}00",
                y,
                m,
                d,
                yday,
                if off < 0 { '-' } else { '+' },
                off.abs()
            );
            let dt1 = go_time::parse("2006-01-02 002 -0700", &f).unwrap();
            assert!(dt1.go_equal(&dt), "{}", f);
        }
    }
    for (y, m, want) in [
        (2011, 1, 31),
        (2011, 2, 28),
        (2012, 2, 29),
        (2011, 6, 30),
        (2011, 12, 31),
    ] {
        // daysIn(m, y) == the day before the 1st of the next month
        assert_eq!(date(y, m + 1, 0, 0, 0, 0, 0, &utc()).day(), want);
    }
}

// Go: time_test.go:TestDurationString and friends
#[test]
fn durations() {
    let tests: &[(&str, i64)] = &[
        ("0s", 0),
        ("1ns", 1),
        ("1.1µs", 1100),
        ("2.2ms", 2200 * 1000),
        ("3.3s", 3300 * 1_000_000),
        ("4m5s", 4 * MINUTE + 5 * SECOND),
        ("4m5.001s", 4 * MINUTE + 5001 * 1_000_000),
        ("5h6m7.001s", 5 * HOUR + 6 * MINUTE + 7001 * 1_000_000),
        ("8m0.000000001s", 8 * MINUTE + 1),
        ("2562047h47m16.854775807s", i64::MAX),
        ("-2562047h47m16.854775808s", i64::MIN),
    ];
    for &(s, d) in tests {
        assert_eq!(Duration(d).string(), s);
        if d > 0 {
            assert_eq!(Duration(-d).string(), format!("-{}", s));
        }
    }
    assert_eq!(Duration(-1000).nanoseconds(), -1000);
    assert_eq!(Duration(-1000).microseconds(), -1);
    assert_eq!(Duration(1_000_000).milliseconds(), 1);
    assert_eq!(Duration(300000000).seconds(), 0.3);
    assert_eq!(Duration(-60000000000).minutes(), -1.0);
    assert_eq!(Duration(-1).minutes(), -1.0 / 60e9);
    assert_eq!(Duration(1).minutes(), 1.0 / 60e9);
    assert_eq!(Duration(3000).minutes(), 5e-8);
    assert_eq!(Duration(-3600000000000).hours(), -1.0);
    assert_eq!(Duration(-1).hours(), -1.0 / 3600e9);
    assert_eq!(Duration(36).hours(), 1e-11);

    let trunc: &[(i64, i64, i64)] = &[
        (0, SECOND, 0),
        (MINUTE, -7 * SECOND, MINUTE),
        (MINUTE, 0, MINUTE),
        (MINUTE, 1, MINUTE),
        (MINUTE + 10 * SECOND, 10 * SECOND, MINUTE + 10 * SECOND),
        (2 * MINUTE + 10 * SECOND, MINUTE, 2 * MINUTE),
        (10 * MINUTE + 10 * SECOND, 3 * MINUTE, 9 * MINUTE),
        (MINUTE + 10 * SECOND, MINUTE + 10 * SECOND + 1, 0),
        (MINUTE + 10 * SECOND, HOUR, 0),
        (-MINUTE, SECOND, -MINUTE),
        (-10 * MINUTE, 3 * MINUTE, -9 * MINUTE),
        (-10 * MINUTE, HOUR, 0),
    ];
    for &(d, m, want) in trunc {
        assert_eq!(Duration(d).truncate(Duration(m)), Duration(want));
    }
    let round: &[(i64, i64, i64)] = &[
        (0, SECOND, 0),
        (MINUTE, -11 * SECOND, MINUTE),
        (MINUTE, 0, MINUTE),
        (MINUTE, 1, MINUTE),
        (2 * MINUTE, MINUTE, 2 * MINUTE),
        (2 * MINUTE + 10 * SECOND, MINUTE, 2 * MINUTE),
        (2 * MINUTE + 30 * SECOND, MINUTE, 3 * MINUTE),
        (2 * MINUTE + 50 * SECOND, MINUTE, 3 * MINUTE),
        (-MINUTE, 1, -MINUTE),
        (-2 * MINUTE, MINUTE, -2 * MINUTE),
        (-2 * MINUTE - 10 * SECOND, MINUTE, -2 * MINUTE),
        (-2 * MINUTE - 30 * SECOND, MINUTE, -3 * MINUTE),
        (-2 * MINUTE - 50 * SECOND, MINUTE, -3 * MINUTE),
        (
            8_000_000_000_000_000_000,
            3_000_000_000_000_000_000,
            9_000_000_000_000_000_000,
        ),
        (
            9_000_000_000_000_000_000,
            5_000_000_000_000_000_000,
            i64::MAX,
        ),
        (
            -8_000_000_000_000_000_000,
            3_000_000_000_000_000_000,
            -9_000_000_000_000_000_000,
        ),
        (
            -9_000_000_000_000_000_000,
            5_000_000_000_000_000_000,
            i64::MIN,
        ),
        ((3 << 61) - 1, 3 << 61, 3 << 61),
    ];
    for &(d, m, want) in round {
        assert_eq!(
            Duration(d).round(Duration(m)),
            Duration(want),
            "{} {}",
            d,
            m
        );
    }
    for (d, want) in [
        (0, 0),
        (1, 1),
        (-1, 1),
        (MINUTE, MINUTE),
        (-MINUTE, MINUTE),
        (i64::MIN, i64::MAX),
        (i64::MIN + 1, i64::MAX),
        (i64::MIN + 2, i64::MAX - 1),
        (i64::MAX, i64::MAX),
        (i64::MAX - 1, i64::MAX - 1),
    ] {
        assert_eq!(Duration(d).abs(), Duration(want));
    }
}

// Go: time_test.go:TestParseDuration, TestParseDurationErrors, TestParseDurationRoundTrip
#[test]
fn parse_duration() {
    let ok: &[(&str, i64)] = &[
        ("0", 0),
        ("5s", 5 * SECOND),
        ("30s", 30 * SECOND),
        ("1478s", 1478 * SECOND),
        ("-5s", -5 * SECOND),
        ("+5s", 5 * SECOND),
        ("-0", 0),
        ("+0", 0),
        ("5.0s", 5 * SECOND),
        ("5.6s", 5 * SECOND + 600 * 1_000_000),
        ("5.s", 5 * SECOND),
        (".5s", 500 * 1_000_000),
        ("1.0s", SECOND),
        ("1.00s", SECOND),
        ("1.004s", SECOND + 4 * 1_000_000),
        ("1.0040s", SECOND + 4 * 1_000_000),
        ("100.00100s", 100 * SECOND + 1_000_000),
        ("10ns", 10),
        ("11us", 11 * 1000),
        ("12µs", 12 * 1000),
        ("12μs", 12 * 1000),
        ("13ms", 13 * 1_000_000),
        ("14s", 14 * SECOND),
        ("15m", 15 * MINUTE),
        ("16h", 16 * HOUR),
        ("3h30m", 3 * HOUR + 30 * MINUTE),
        ("10.5s4m", 4 * MINUTE + 10 * SECOND + 500 * 1_000_000),
        ("-2m3.4s", -(2 * MINUTE + 3 * SECOND + 400 * 1_000_000)),
        (
            "1h2m3s4ms5us6ns",
            HOUR + 2 * MINUTE + 3 * SECOND + 4 * 1_000_000 + 5 * 1000 + 6,
        ),
        (
            "39h9m14.425s",
            39 * HOUR + 9 * MINUTE + 14 * SECOND + 425 * 1_000_000,
        ),
        ("52763797000ns", 52763797000),
        ("0.3333333333333333333h", 20 * MINUTE),
        ("9007199254740993ns", (1 << 53) + 1),
        ("9223372036854775807ns", i64::MAX),
        ("9223372036854775.807us", i64::MAX),
        ("9223372036s854ms775us807ns", i64::MAX),
        ("-9223372036854775808ns", i64::MIN),
        ("-9223372036854775.808us", i64::MIN),
        ("-9223372036s854ms775us808ns", i64::MIN),
        ("-2562047h47m16.854775808s", i64::MIN),
        ("0.100000000000000000000h", 6 * MINUTE),
        (
            "0.830103483285477580700h",
            49 * MINUTE + 48 * SECOND + 372539827,
        ),
    ];
    for &(s, want) in ok {
        assert_eq!(go_time::parse_duration(s), Ok(Duration(want)), "{:?}", s);
    }
    let errs: &[(&[u8], &str)] = &[
        (b"", r#""""#),
        (b"3", r#""3""#),
        (b"-", r#""-""#),
        (b"s", r#""s""#),
        (b".", r#"".""#),
        (b"-.", r#""-.""#),
        (b".s", r#"".s""#),
        (b"+.s", r#""+.s""#),
        (b"1d", r#""1d""#),
        (b"\x85\x85", r#""\x85\x85""#),
        (b"\xffff", r#""\xffff""#),
        (b"hello \xffff world", r#""hello \xffff world""#),
        ("\u{FFFD}".as_bytes(), r#""\xef\xbf\xbd""#),
        (
            "\u{FFFD} hello \u{FFFD} world".as_bytes(),
            r#""\xef\xbf\xbd hello \xef\xbf\xbd world""#,
        ),
        (b"9223372036854775810ns", r#""9223372036854775810ns""#),
        (b"9223372036854775808ns", r#""9223372036854775808ns""#),
        (b"-9223372036854775809ns", r#""-9223372036854775809ns""#),
        (b"9223372036854776us", r#""9223372036854776us""#),
        (b"3000000h", r#""3000000h""#),
        (b"9223372036854775.808us", r#""9223372036854775.808us""#),
        (
            b"9223372036854ms775us808ns",
            r#""9223372036854ms775us808ns""#,
        ),
    ];
    for &(s, expect) in errs {
        let e = go_time::parse_duration(s).expect_err("should fail");
        assert!(e.error().contains(expect), "{:?}: {}", s, e);
    }
    for d in [Duration(i64::MAX), Duration(i64::MIN)] {
        assert_eq!(go_time::parse_duration(d.string()), Ok(d));
    }
}

// Go: time_test.go:TestDate, TestAddDate, TestSub
#[test]
fn date_add_sub() {
    let l = local();
    let date_tests: &[(i64, i64, i64, i64, i64, i64, i64, i64)] = &[
        (2011, 11, 6, 1, 0, 0, 0, 1320566400),
        (2011, 11, 6, 1, 59, 59, 0, 1320569999),
        (2011, 11, 6, 2, 0, 0, 0, 1320573600),
        (2011, 3, 13, 1, 0, 0, 0, 1300006800),
        (2011, 3, 13, 1, 59, 59, 0, 1300010399),
        (2011, 3, 13, 3, 0, 0, 0, 1300010400),
        (2011, 3, 13, 2, 30, 0, 0, 1300008600),
        (2012, 12, 24, 0, 0, 0, 0, 1356336000),
        (2011, 11, 18, 7, 56, 35, 0, 1321631795),
        (2011, 11, 19, -17, 56, 35, 0, 1321631795),
        (2011, 11, 17, 31, 56, 35, 0, 1321631795),
        (2011, 11, 18, 6, 116, 35, 0, 1321631795),
        (2011, 10, 49, 7, 56, 35, 0, 1321631795),
        (2011, 11, 18, 7, 55, 95, 0, 1321631795),
        (2011, 11, 18, 7, 56, 34, 1_000_000_000, 1321631795),
        (2011, 12, -12, 7, 56, 35, 0, 1321631795),
        (2012, 1, -43, 7, 56, 35, 0, 1321631795),
        (2012, -1, 18, 7, 56, 35, 0, 1321631795),
        (2010, 23, 18, 7, 56, 35, 0, 1321631795),
        (1970, 1, 15297, 7, 56, 35, 0, 1321631795),
        (1970, 1, -25508, 0, 0, 0, 0, -2203948800),
    ];
    for &(y, m, d, h, mi, s, ns, unix) in date_tests {
        let t = date(y, m, d, h, mi, s, ns, &l);
        assert!(
            t.go_equal(&go_time::unix(unix, 0)),
            "Date({} {} {}) = {}",
            y,
            m,
            d,
            t.string()
        );
    }

    let t0 = date(2011, 11, 18, 7, 56, 35, 0, &utc());
    let t1 = date(2016, 3, 19, 7, 56, 35, 0, &utc());
    for (y, m, d) in [(4, 4, 1), (3, 16, 1), (3, 15, 30), (5, -6, -18 - 30 - 12)] {
        assert!(t0.add_date(y, m, d).go_equal(&t1));
    }
    let t2 = date(1899, 12, 31, 0, 0, 0, 0, &utc());
    let days = t2.go_unix() / (24 * 60 * 60);
    assert!(go_time::unix(0, 0).add_date(0, 0, days).go_equal(&t2));

    let z = Time::zero;
    let u = |y, m, d, h, mi, s, ns| date(y, m, d, h, mi, s, ns, &utc());
    let sub_tests: Vec<(Time, Time, i64)> = vec![
        (z(), z(), 0),
        (u(2009, 11, 23, 0, 0, 0, 1), u(2009, 11, 23, 0, 0, 0, 0), 1),
        (
            u(2009, 11, 23, 0, 0, 0, 0),
            u(2009, 11, 24, 0, 0, 0, 0),
            -24 * HOUR,
        ),
        (
            u(2009, 11, 24, 0, 0, 0, 0),
            u(2009, 11, 23, 0, 0, 0, 0),
            24 * HOUR,
        ),
        (
            u(-2009, 11, 24, 0, 0, 0, 0),
            u(-2009, 11, 23, 0, 0, 0, 0),
            24 * HOUR,
        ),
        (z(), u(2109, 11, 23, 0, 0, 0, 0), i64::MIN),
        (u(2109, 11, 23, 0, 0, 0, 0), z(), i64::MAX),
        (z(), u(-2109, 11, 23, 0, 0, 0, 0), i64::MAX),
        (u(-2109, 11, 23, 0, 0, 0, 0), z(), i64::MIN),
        (
            u(2290, 1, 1, 0, 0, 0, 0),
            u(2000, 1, 1, 0, 0, 0, 0),
            290 * 365 * 24 * HOUR + 71 * 24 * HOUR,
        ),
        (
            u(2300, 1, 1, 0, 0, 0, 0),
            u(2000, 1, 1, 0, 0, 0, 0),
            i64::MAX,
        ),
        (
            u(2000, 1, 1, 0, 0, 0, 0),
            u(2290, 1, 1, 0, 0, 0, 0),
            -290 * 365 * 24 * HOUR - 71 * 24 * HOUR,
        ),
        (
            u(2000, 1, 1, 0, 0, 0, 0),
            u(2300, 1, 1, 0, 0, 0, 0),
            i64::MIN,
        ),
        (
            u(2311, 11, 26, 2, 16, 47, 63535996),
            u(2019, 8, 16, 2, 29, 30, 268436582),
            9223372036795099414,
        ),
    ];
    for (i, (t, uu, d)) in sub_tests.iter().enumerate() {
        assert_eq!(t.sub(uu), Duration(*d), "#{}", i);
    }
}

// Go: time_test.go gob/binary/JSON tests
#[test]
fn marshal() {
    let l = local();
    // TestTimeGob (round trip through MarshalBinary)
    let gob = vec![
        date(0, 1, 2, 3, 4, 5, 6, &utc()),
        date(7, 8, 9, 10, 11, 12, 13, &go_time::fixed_zone("", 0)),
        go_time::unix(81985467080890095, 0x76543210),
        Time::zero(),
        date(1, 2, 3, 4, 5, 6, 7, &go_time::fixed_zone("", 32767 * 60)),
        date(1, 2, 3, 4, 5, 6, 7, &go_time::fixed_zone("", -32768 * 60)),
    ];
    for tt in gob {
        let b = tt.marshal_binary().unwrap();
        let mut back = Time::zero();
        go_time::unmarshal_binary(&mut back, &b).unwrap();
        assert!(
            back.go_equal(&tt) && back.zone() == tt.zone(),
            "{}",
            tt.string()
        );
    }
    for (b, want) in [
        (&[][..], "Time.UnmarshalBinary: no data"),
        (&[0, 2, 3][..], "Time.UnmarshalBinary: unsupported version"),
        (&[1, 2, 3][..], "Time.UnmarshalBinary: invalid length"),
    ] {
        let mut t = Time::zero();
        assert_eq!(
            go_time::unmarshal_binary(&mut t, b).unwrap_err().error(),
            want
        );
    }
    for off in [-60, -32769 * 60, 32768 * 60] {
        let t = date(0, 1, 2, 3, 4, 5, 6, &go_time::fixed_zone("", off));
        assert_eq!(
            t.marshal_binary().unwrap_err().error(),
            "Time.MarshalBinary: unexpected zone offset"
        );
    }

    // TestTimeJSON
    let json = [
        (
            date(9999, 4, 12, 23, 20, 50, 520_000_000, &utc()),
            r#""9999-04-12T23:20:50.52Z""#,
        ),
        (
            date(1996, 12, 19, 16, 39, 57, 0, &l),
            r#""1996-12-19T16:39:57-08:00""#,
        ),
        (
            date(0, 1, 1, 0, 0, 0, 1, &go_time::fixed_zone("", 60)),
            r#""0000-01-01T00:00:00.000000001+00:01""#,
        ),
        (
            date(
                2020,
                1,
                1,
                0,
                0,
                0,
                0,
                &go_time::fixed_zone("", 23 * 3600 + 59 * 60),
            ),
            r#""2020-01-01T00:00:00+23:59""#,
        ),
    ];
    for (t, want) in json {
        let b = t.marshal_json().unwrap();
        assert_eq!(b, want.as_bytes());
        let mut back = Time::zero();
        go_time::unmarshal_json(&mut back, &b).unwrap();
        assert!(back.go_equal(&t) && back.zone().1 == t.zone().1);
    }

    // TestUnmarshalInvalidTimes (encoding/json v1 semantics for the first two)
    let inv: &[(&str, &str)] = &[
        (r#"{}"#, "Time.UnmarshalJSON: input is not a JSON string"),
        (r#"[]"#, "Time.UnmarshalJSON: input is not a JSON string"),
        (r#""2000-01-01T1:12:34Z""#, "<nil>"),
        (r#""2000-01-01T00:00:00,000Z""#, "<nil>"),
        (r#""2000-01-01T00:00:00+24:00""#, "<nil>"),
        (r#""2000-01-01T00:00:00+00:60""#, "<nil>"),
        (
            r#""2000-01-01T00:00:00+123:45""#,
            r#"parsing time "2000-01-01T00:00:00+123:45" as "2006-01-02T15:04:05Z07:00": cannot parse "+123:45" as "Z07:00""#,
        ),
    ];
    for &(input, want) in inv {
        let mut ts = Time::zero();
        let got = match go_time::unmarshal_json(&mut ts, input.as_bytes()) {
            Ok(()) => "<nil>".to_string(),
            Err(e) => e.error().to_string(),
        };
        assert_eq!(got, want, "{}", input);
        if input.starts_with('"') {
            let got = match go_time::unmarshal_text(&mut ts, input.trim_matches('"').as_bytes()) {
                Ok(()) => "<nil>".to_string(),
                Err(e) => e.error().to_string(),
            };
            assert_eq!(got, want, "{}", input);
        }
    }

    // TestMarshalInvalidTimes
    let bad = [
        (
            date(10000, 1, 1, 0, 0, 0, 0, &utc()),
            "Time.MarshalJSON: year outside of range [0,9999]",
        ),
        (
            date(-998, 1, 1, 0, 0, 0, 0, &utc()).add(Duration(-SECOND)),
            "Time.MarshalJSON: year outside of range [0,9999]",
        ),
        (
            date(0, 1, 1, 0, 0, 0, 0, &utc()).add(Duration(-1)),
            "Time.MarshalJSON: year outside of range [0,9999]",
        ),
        (
            date(2020, 1, 1, 0, 0, 0, 0, &go_time::fixed_zone("", 24 * 3600)),
            "Time.MarshalJSON: timezone hour outside of range [0,23]",
        ),
        (
            date(2020, 1, 1, 0, 0, 0, 0, &go_time::fixed_zone("", 123 * 3600)),
            "Time.MarshalJSON: timezone hour outside of range [0,23]",
        ),
    ];
    for (t, want) in bad {
        assert_eq!(t.marshal_json().unwrap_err().error(), want);
        assert_eq!(
            t.marshal_text().unwrap_err().error(),
            want.replace("JSON", "Text")
        );
        assert_eq!(
            t.append_text(Vec::new()).unwrap_err().error(),
            want.replace("MarshalJSON", "AppendText")
        );
    }

    // TestMarshalBinaryZeroTime
    let t0 = Time::zero();
    let enc = t0.marshal_binary().unwrap();
    let mut t1 = go_time::now();
    go_time::unmarshal_binary(&mut t1, &enc).unwrap();
    assert_eq!(t1, t0);
    assert!(t1.loc.is_none());

    // TestMarshalBinaryVersion2 (US/Eastern -> America/New_York fixture)
    let t0 = go_time::parse(go_time::RFC3339, "1880-01-01T00:00:00Z").unwrap();
    let ny =
        go_time::load_location_from_tz_data("US/Eastern", &common::zone_data("America/New_York"))
            .unwrap();
    let t1 = t0.in_loc(&ny);
    let b = t1.marshal_binary().unwrap();
    assert_eq!(b[0], 2);
    let mut t2 = Time::zero();
    go_time::unmarshal_binary(&mut t2, &b).unwrap();
    assert!(t0.go_equal(&t1) && t1.go_equal(&t2));

    // TestMarshalBinaryVersion2Bugfix
    let off = -5 * 3600 - 30 * 60 - 45;
    let t1 = date(2024, 6, 15, 12, 0, 0, 0, &go_time::fixed_zone("LMT", off));
    let b = t1.marshal_binary().unwrap();
    let mut t2 = Time::zero();
    go_time::unmarshal_binary(&mut t2, &b).unwrap();
    assert_eq!(t1.zone().1, t2.zone().1);

    // TestZeroMonthString, TestWeekdayString
    assert_eq!(Month(0).string(), "%!Month(0)");
    assert_eq!(Weekday::TUESDAY.string(), "Tuesday");
    assert_eq!(Weekday(14).string(), "%!Weekday(14)");
}

// Local-dependent parse (Go: "a numeric offset equal to Local's gives Local").
#[test]
fn parse_uses_local_zone() {
    let l = local();
    let t = go_time::parse(go_time::RFC3339, "2011-07-01T12:00:00-07:00").unwrap();
    assert!(go_time::is_local_loc(t.loc.as_ref().unwrap()));
    assert_eq!(t.string(), "2011-07-01 12:00:00 -0700 PDT");
    let t = go_time::parse(go_time::RFC3339, "2011-07-01T12:00:00-08:00").unwrap();
    assert_eq!(t.string(), "2011-07-01 12:00:00 -0800 -0800");
    let t = go_time::parse_in_location("2006-01-02 15:04", "2011-03-13 02:30", &l).unwrap();
    assert_eq!(t.string(), "2011-03-13 01:30:00 -0800 PST"); // verified against go1.27.1
    // the local() Arc is what times carry
    assert!(Arc::ptr_eq(&go_time::unix(0, 0).go_location(), &l));
    assert_eq!(go_time::location_string(Some(&l)), "Local");
}

/// A zone checked in under tests/fixtures/zoneinfo, loaded under its IANA name.
fn zone(name: &str) -> Arc<Location> {
    go_time::load_location_from_tz_data(name, &common::zone_data(name)).unwrap()
}

// Go: time_test.go:TestTimeAddSecOverflow
#[test]
fn time_add_sec_overflow() {
    const UNIX_TO_INTERNAL: i64 = go_value::UNIX_TO_INTERNAL;
    // Test it with positive delta.
    let mut max_int64: i64 = i64::MAX;
    let time_ext = max_int64 - UNIX_TO_INTERNAL - 50;
    let mut t = go_time::unix(time_ext, 0);
    for i in 0..100i64 {
        let sec = t.go_unix();
        t = t.add(Duration(i * 1_000_000_000));
        let new_sec = t.go_unix();
        assert!(
            new_sec == sec.wrapping_add(i) || new_sec.wrapping_add(UNIX_TO_INTERNAL) == max_int64,
            "positive delta: {} {}",
            i,
            new_sec
        );
    }
    // Test it with negative delta (NotMonoNegativeTime: ext = -1<<63 + 50).
    max_int64 = -max_int64;
    let mut t = Time {
        unix_sec: (i64::MIN + 50).wrapping_sub(UNIX_TO_INTERNAL),
        nsec: 0,
        loc: None,
    };
    for i in (-99..=0i64).rev() {
        let sec = t.go_unix();
        t = t.add(Duration(i * 1_000_000_000));
        let new_sec = t.go_unix();
        assert!(
            new_sec == sec.wrapping_add(i) || new_sec.wrapping_add(UNIX_TO_INTERNAL) == max_int64,
            "negative delta: {} {}",
            i,
            new_sec
        );
    }
}

// Go: time_test.go:TestTimeWithZoneTransition
#[test]
fn time_with_zone_transition() {
    let loc = zone("Asia/Shanghai");
    let u = utc();
    let tests = [
        // 14 Apr 1991 - Daylight Saving Time Started
        (
            date(1991, 4, 13, 17, 50, 0, 0, &loc),
            date(1991, 4, 13, 9, 50, 0, 0, &u),
        ),
        (
            date(1991, 4, 13, 18, 0, 0, 0, &loc),
            date(1991, 4, 13, 10, 0, 0, 0, &u),
        ),
        (
            date(1991, 4, 14, 1, 50, 0, 0, &loc),
            date(1991, 4, 13, 17, 50, 0, 0, &u),
        ),
        (
            date(1991, 4, 14, 3, 0, 0, 0, &loc),
            date(1991, 4, 13, 18, 0, 0, 0, &u),
        ),
        // 15 Sep 1991 - Daylight Saving Time Ended
        (
            date(1991, 9, 14, 16, 50, 0, 0, &loc),
            date(1991, 9, 14, 7, 50, 0, 0, &u),
        ),
        (
            date(1991, 9, 14, 17, 0, 0, 0, &loc),
            date(1991, 9, 14, 8, 0, 0, 0, &u),
        ),
        (
            date(1991, 9, 15, 0, 50, 0, 0, &loc),
            date(1991, 9, 14, 15, 50, 0, 0, &u),
        ),
        (
            date(1991, 9, 15, 2, 0, 0, 0, &loc),
            date(1991, 9, 14, 18, 0, 0, 0, &u),
        ),
    ];
    for (i, (give, want)) in tests.iter().enumerate() {
        assert!(
            give.go_equal(want),
            "#{}: {} is not equal to {}",
            i,
            give.format(go_time::RFC3339),
            want.format(go_time::RFC3339)
        );
    }
}

// Go: time_test.go:TestZoneBounds
#[test]
fn zone_bounds() {
    let _ = local();
    let loc = zone("Asia/Shanghai");

    // The ZoneBounds of a UTC location would just return two zero Time.
    for sec in [
        0i64,
        1221681866,
        -1221681866,
        -11644473600,
        599529660,
        978220860,
    ] {
        let (start, end) = go_time::unix(sec, 0).utc().zone_bounds();
        assert!(start.go_is_zero() && end.go_is_zero(), "UTC {}", sec);
    }

    // If the zone begins at the beginning of time, start will be returned as a zero Time.
    let begin = date(i32::MIN as i64, 1, 1, 0, 0, 0, 0, &loc);
    let (start, end) = begin.zone_bounds();
    assert!(start.go_is_zero() && !end.go_is_zero());

    // If the zone goes on forever, end will be returned as a zero Time.
    let forever = date(i32::MAX as i64, 1, 1, 0, 0, 0, 0, &loc);
    let (start, end) = forever.zone_bounds();
    assert!(!start.go_is_zero() && end.go_is_zero());

    // Check some real-world cases to make sure we're getting the right bounds.
    let one = date(1990, 9, 16, 1, 0, 0, 0, &loc);
    let two = date(1991, 4, 14, 3, 0, 0, 0, &loc);
    let three = date(1991, 9, 15, 1, 0, 0, 0, &loc);
    let lt = |sec: i64| go_time::unix(sec, 0);
    let zero = Time::zero();
    let tests = [
        (
            date(1991, 4, 13, 17, 50, 0, 0, &loc),
            one.clone(),
            two.clone(),
        ),
        (
            date(1991, 4, 13, 18, 0, 0, 0, &loc),
            one.clone(),
            two.clone(),
        ),
        (
            date(1991, 4, 14, 1, 50, 0, 0, &loc),
            one.clone(),
            two.clone(),
        ),
        (two.clone(), two.clone(), three.clone()),
        (
            date(1991, 9, 14, 16, 50, 0, 0, &loc),
            two.clone(),
            three.clone(),
        ),
        (
            date(1991, 9, 14, 17, 0, 0, 0, &loc),
            two.clone(),
            three.clone(),
        ),
        (
            date(1991, 9, 15, 0, 50, 0, 0, &loc),
            two.clone(),
            three.clone(),
        ),
        // after the last transition (Standard Time)
        (three.clone(), three.clone(), zero.clone()),
        (
            date(1991, 12, 15, 1, 50, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 4, 13, 17, 50, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 4, 13, 18, 0, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 4, 14, 1, 50, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 9, 14, 16, 50, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 9, 14, 17, 0, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        (
            date(1992, 9, 15, 0, 50, 0, 0, &loc),
            three.clone(),
            zero.clone(),
        ),
        // local times (Local is America/Los_Angeles)
        (lt(0), lt(-5756400), lt(9972000)),
        (lt(1221681866), lt(1205056800), lt(1225616400)),
        (lt(2152173599), lt(2145916800), lt(2152173600)),
        (lt(2152173600), lt(2152173600), lt(2172733200)),
        (lt(2152173601), lt(2152173600), lt(2172733200)),
        (lt(2159200800), lt(2152173600), lt(2172733200)),
        (lt(2172733199), lt(2152173600), lt(2172733200)),
        (lt(2172733200), lt(2172733200), lt(2177452800)),
    ];
    for (i, (give, want_start, want_end)) in tests.iter().enumerate() {
        let (start, end) = give.zone_bounds();
        assert!(
            start.go_equal(want_start) && end.go_equal(want_end),
            "#{}: ZoneBounds of {} got {} {} want {} {}",
            i,
            give.string(),
            start.string(),
            end.string(),
            want_start.string(),
            want_end.string()
        );
    }
}

// Go: time_test.go:TestTimeIsDST
#[test]
fn time_is_dst() {
    let with_dst = zone("Australia/Sydney");
    let without_dst = zone("Australia/Brisbane");
    let fixed = go_time::fixed_zone("FIXED_TIME", 12345);
    let tests = [
        (date(2009, 1, 1, 12, 0, 0, 0, &utc()), false),
        (date(2009, 6, 1, 12, 0, 0, 0, &utc()), false),
        (date(2009, 1, 1, 12, 0, 0, 0, &with_dst), true),
        (date(2009, 6, 1, 12, 0, 0, 0, &with_dst), false),
        (date(2009, 1, 1, 12, 0, 0, 0, &without_dst), false),
        (date(2009, 6, 1, 12, 0, 0, 0, &without_dst), false),
        (date(2009, 1, 1, 12, 0, 0, 0, &fixed), false),
        (date(2009, 6, 1, 12, 0, 0, 0, &fixed), false),
    ];
    for (i, (t, want)) in tests.iter().enumerate() {
        assert_eq!(t.is_dst(), *want, "#{}", i);
    }
}

// Go: time_test.go:TestLoadFixed
#[test]
fn load_fixed() {
    // Issue 4064: handle locations without any zone transitions.
    let loc = zone("Etc/GMT+1");
    // The tzdata name Etc/GMT+1 uses "east is negative".
    let (name, offset) = go_time::now().in_loc(&loc).zone();
    assert!(name == "GMT+1" || name == "-01", "{}", name);
    assert_eq!(offset, -3600);
}

// Go: time_test.go:TestAddToExactSecond
#[test]
fn add_to_exact_second() {
    let t1 = go_time::now();
    let t2 = t1.add(Duration(SECOND - t1.nanosecond()));
    let sec = (t1.second() + 1) % 60;
    assert!(t2.second() == sec && t2.nanosecond() == 0);
}

// Go: time_test.go:TestDefaultLoc
#[test]
fn default_loc() {
    let l = local();
    let t1 = Time::zero();
    let t2 = Time::zero().utc();
    assert_eq!(t1.go_after(&t2), t2.go_after(&t1));
    assert_eq!(t1.go_before(&t2), t2.go_before(&t1));
    assert_eq!(t1.go_equal(&t2), t2.go_equal(&t1));
    assert_eq!(t1.compare(&t2), t2.compare(&t1));
    assert_eq!(t1.go_is_zero(), t2.go_is_zero());
    assert_eq!(t1.date(), t2.date());
    assert_eq!(t1.year(), t2.year());
    assert_eq!(t1.month(), t2.month());
    assert_eq!(t1.day(), t2.day());
    assert_eq!(t1.weekday(), t2.weekday());
    assert_eq!(t1.iso_week(), t2.iso_week());
    assert_eq!(t1.clock(), t2.clock());
    assert_eq!(t1.hour(), t2.hour());
    assert_eq!(t1.minute(), t2.minute());
    assert_eq!(t1.second(), t2.second());
    assert_eq!(t1.nanosecond(), t2.nanosecond());
    assert_eq!(t1.year_day(), t2.year_day());
    assert!(t1.add(Duration(HOUR)).go_equal(&t2.add(Duration(HOUR))));
    assert_eq!(t1.sub(&t2), t2.sub(&t1));
    assert!(t1.add_date(1991, 9, 3) == t2.add_date(1991, 9, 3));
    assert!(t1.utc() == t2.utc());
    assert!(t1.local() == t2.local());
    assert!(t1.in_loc(&utc()) == t2.in_loc(&utc()));
    assert!(t1.in_loc(&l) == t2.in_loc(&l));
    assert_eq!(t1.zone(), t2.zone());
    assert_eq!(t1.zone_bounds().0, t2.zone_bounds().0);
    assert_eq!(t1.zone_bounds().1, t2.zone_bounds().1);
    assert_eq!(t1.go_unix(), t2.go_unix());
    assert_eq!(t1.go_unix_nano(), t2.go_unix_nano());
    assert_eq!(t1.unix_milli(), t2.unix_milli());
    assert_eq!(t1.unix_micro(), t2.unix_micro());
    assert_eq!(t1.marshal_binary(), t2.marshal_binary());
    assert_eq!(t1.is_dst(), t2.is_dst());
    assert_eq!(t1.marshal_json(), t2.marshal_json());
    assert_eq!(t1.marshal_text(), t2.marshal_text());
    assert!(t1.truncate(Duration(HOUR)) == t2.truncate(Duration(HOUR)));
    assert!(t1.round(Duration(HOUR)) == t2.round(Duration(HOUR)));
    assert!(Arc::ptr_eq(&t1.go_location(), &t2.go_location()));
    assert_eq!(t1.string(), t2.string());
    assert_eq!(t1.go_string(), t2.go_string());
}

// Go: zoneinfo_test.go:TestVersion3
#[test]
fn version3() {
    let data = common::zone_data("Asia/Jerusalem");
    assert_eq!(data[4], b'3');
    go_time::load_location_from_tz_data("Asia/Jerusalem", &data).unwrap();
}

// Go: zoneinfo_test.go:TestFirstZone
#[test]
fn first_zone() {
    const FORMAT: &str = "Mon, 02 Jan 2006 15:04:05 -0700 (MST)";
    let tests = [
        (
            "PST8PDT",
            -1633269601i64,
            "Sun, 31 Mar 1918 01:59:59 -0800 (PST)",
            "Sun, 31 Mar 1918 03:00:00 -0700 (PDT)",
        ),
        (
            "Pacific/Fakaofo",
            1325242799,
            "Thu, 29 Dec 2011 23:59:59 -1100 (-11)",
            "Sat, 31 Dec 2011 00:00:00 +1300 (+13)",
        ),
    ];
    for (z, unix, want1, want2) in tests {
        let l = zone(z);
        assert_eq!(go_time::unix(unix, 0).in_loc(&l).format(FORMAT), want1);
        assert_eq!(go_time::unix(unix + 1, 0).in_loc(&l).format(FORMAT), want2);
    }
}

// Go: zoneinfo_test.go:TestLocationNames
#[test]
fn location_names() {
    let l = local();
    assert_eq!(go_time::location_string(Some(&l)), "Local");
    assert_eq!(go_time::location_string(Some(&utc())), "UTC");
    assert_eq!(go_time::location_string(None), "UTC");
}

// Go: zoneinfo_test.go:TestEarlyLocation
#[test]
fn early_location() {
    let l = zone("America/New_York");
    let (name, off) = date(1900, 1, 1, 0, 0, 0, 0, &l).zone();
    assert_eq!((name.as_str(), off), ("EST", -18000));
}

// Go: zoneinfo_test.go:TestMalformedTZData
#[test]
fn malformed_tz_data() {
    // The goal here is just that malformed tzdata results in an error, not a panic.
    let issue29437 = b"TZif\x00000000000000000\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x0000";
    assert!(go_time::load_location_from_tz_data("abc", issue29437).is_err());
}

// Go: zoneinfo_test.go:TestLoadLocationFromTZDataSlim
#[test]
fn load_location_from_tz_data_slim() {
    let tests: [(&str, &str, (i64, i64, i64, i64, i64, i64), &str, i64); 4] = [
        (
            "Europe/Berlin",
            "2020b_Europe_Berlin",
            (2020, 10, 29, 15, 30, 0),
            "CET",
            3600,
        ),
        (
            "America/Nuuk",
            "2021a_America_Nuuk",
            (2020, 10, 29, 15, 30, 0),
            "-03",
            -10800,
        ),
        (
            "Asia/Gaza",
            "2021a_Asia_Gaza",
            (2020, 10, 29, 15, 30, 0),
            "EET",
            7200,
        ),
        (
            "Europe/Dublin",
            "2021a_Europe_Dublin",
            (2021, 4, 2, 11, 12, 13),
            "IST",
            3600,
        ),
    ];
    for (zone_name, file, (y, m, d, h, mi, s), want_name, want_off) in tests {
        let data = common::zone_data(&format!("testdata/{}", file));
        let l = go_time::load_location_from_tz_data(zone_name, &data).unwrap();
        let (name, off) = date(y, m, d, h, mi, s, 0, &l).zone();
        assert_eq!((name.as_str(), off), (want_name, want_off), "{}", file);
    }
}

// Go: zoneinfo_test.go:TestLoadLocationValidatesNames, TestBadLocationErrMsg
#[test]
fn load_location_validates_names() {
    for v in ["/usr/foo/Foo", "\\UNC\x0coo", "..", "a.."] {
        let err = go_time::load_location_env(v, "", None).unwrap_err();
        assert_eq!(err.error(), "time: invalid location name", "{:?}", v);
    }
    let loc = "Asia/SomethingNotExist";
    let err = go_time::load_location_env(loc, "", Some("/nonexistent-goroot")).unwrap_err();
    assert_eq!(err.error(), format!("unknown time zone {}", loc));
}
