//! Go `time` package (go1.27.1) behaviour over [`go_value::Time`] /
//! [`go_value::Location`].
//!
//! The data types live in `go-value`; this crate ports the functionality:
//! layout formatting and parsing (`format.go`, `format_rfc3339.go`), the
//! calendar, arithmetic, `Duration` and binary/text/JSON marshalling
//! (`time.go`), and zone handling (`zoneinfo*.go`: `LoadLocation`, `Local`
//! from `$TZ`, `FixedZone`, TZ-string rules).
//!
//! Go methods on `time.Time` are available through [`GoTimeExt`]; package
//! functions are free functions (`date`, `unix`, `parse`, `load_location`...).
//! See `PORTING.md` for the Go file → module map and deviations.

// Faithful-port lints: Go control flow is kept as-is.
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::manual_range_contains,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_late_init,
    clippy::manual_is_multiple_of
)]

mod format;
mod format_rfc3339;
mod sys_unix;
mod time;
mod utf8;
mod zoneinfo;
mod zoneinfo_read;
mod zoneinfo_unix;

use std::sync::Arc;

pub use go_value::{Location, Time, Zone, ZoneTrans};

pub use crate::format::{
    ANSIC, DATE_ONLY, DATE_TIME, KITCHEN, LAYOUT, ParseError, RFC822, RFC822Z, RFC850, RFC1123,
    RFC1123Z, RFC3339, RFC3339_NANO, RUBY_DATE, STAMP, STAMP_MICRO, STAMP_MILLI, STAMP_NANO,
    TIME_ONLY, UNIX_DATE, parse, parse_duration, parse_in_location,
};
pub use crate::time::{
    Duration, Month, Weekday, date, now, since, unix, unix_micro, unix_milli, until,
};
pub use crate::zoneinfo::{
    ZoneLookup, fixed_zone, is_local_loc, is_utc_loc, local, set_local, utc,
};
pub use crate::zoneinfo_read::load_location_from_tz_data;
pub use crate::zoneinfo_unix::{init_local_from, init_local_from_env};

/// A Go `error` produced with `errors.New` (its `Error()` text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeError {
    msg: String,
}

impl TimeError {
    pub(crate) fn new(msg: impl Into<String>) -> TimeError {
        TimeError { msg: msg.into() }
    }

    /// Go: `err.Error()`.
    pub fn error(&self) -> &str {
        &self.msg
    }
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for TimeError {}

impl From<ParseError> for TimeError {
    fn from(e: ParseError) -> TimeError {
        TimeError::new(e.error())
    }
}

// Go: zoneinfo.go:LoadLocation
/// The Location with the given IANA name ("" and "UTC" give [`utc`], "Local"
/// gives [`local`]), read from `$ZONEINFO`, the system zoneinfo directories,
/// or `$GOROOT/lib/time/zoneinfo.zip`.
pub fn load_location(name: &str) -> Result<Arc<Location>, TimeError> {
    zoneinfo::load_location_named(name)
}

/// [`load_location`] with explicit `$ZONEINFO` and `runtime.GOROOT()` values
/// (Go reads both from the environment once; used by tests).
pub fn load_location_env(
    name: &str,
    zoneinfo: &str,
    goroot: Option<&str>,
) -> Result<Arc<Location>, TimeError> {
    zoneinfo::load_location_env(name, zoneinfo, goroot)
}

/// Go: `(*Location).String()` (nil is UTC).
pub fn location_string(l: Option<&Arc<Location>>) -> String {
    zoneinfo::location_string(l)
}

/// Go: `(*Location).lookup(sec)` — the zone in effect at Unix second `sec`.
pub fn location_lookup(l: &Location, sec: i64) -> ZoneLookup<'_> {
    zoneinfo::lookup(l, sec)
}

// Go: time.go:(*Time).UnmarshalBinary
/// Decodes `MarshalBinary` output into `t` (versions 1 and 2).
pub fn unmarshal_binary(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    time::unmarshal_binary(t, data)
}

// Go: time.go:(*Time).UnmarshalJSON
/// `null` leaves `t` unchanged; otherwise a quoted RFC 3339 string.
pub fn unmarshal_json(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    time::unmarshal_json(t, data)
}

// Go: time.go:(*Time).UnmarshalText
pub fn unmarshal_text(t: &mut Time, data: &[u8]) -> Result<(), TimeError> {
    time::unmarshal_text(t, data)
}

/// Go's `time.Time` methods.
///
/// Methods that `go_value::Time` already provides as inherent methods
/// (`unix`, `unix_nano`, `is_zero`, `equal`, `before`, `after`, `location`)
/// are exposed here under `go_*` names with Go's exact semantics (comparisons
/// on Go's internal seconds, `UTC` returned as the shared [`utc`] Arc).
pub trait GoTimeExt {
    /// Go: `t.Format(layout)`.
    fn format(&self, layout: &str) -> String;
    /// Go: `t.Format(layout)` for a byte-string layout.
    fn format_bytes(&self, layout: &[u8]) -> Vec<u8>;
    /// Go: `t.AppendFormat(b, layout)`.
    fn append_format(&self, b: &mut Vec<u8>, layout: &[u8]);
    /// Go: `t.String()` = `Format("2006-01-02 15:04:05.999999999 -0700 MST")`.
    fn string(&self) -> String;
    /// Go: `t.GoString()`.
    fn go_string(&self) -> String;

    /// Go: `t.Date()`.
    fn date(&self) -> (i64, Month, i64);
    fn year(&self) -> i64;
    fn month(&self) -> Month;
    fn day(&self) -> i64;
    fn weekday(&self) -> Weekday;
    /// Go: `t.ISOWeek()` = (year, week).
    fn iso_week(&self) -> (i64, i64);
    /// Go: `t.Clock()` = (hour, min, sec).
    fn clock(&self) -> (i64, i64, i64);
    fn hour(&self) -> i64;
    fn minute(&self) -> i64;
    fn second(&self) -> i64;
    fn nanosecond(&self) -> i64;
    fn year_day(&self) -> i64;

    fn add(&self, d: Duration) -> Time;
    fn sub(&self, u: &Time) -> Duration;
    fn add_date(&self, years: i64, months: i64, days: i64) -> Time;
    fn truncate(&self, d: Duration) -> Time;
    fn round(&self, d: Duration) -> Time;

    /// Go: `t.UTC()`.
    fn utc(&self) -> Time;
    /// Go: `t.Local()`.
    fn local(&self) -> Time;
    /// Go: `t.In(loc)`.
    fn in_loc(&self, loc: &Arc<Location>) -> Time;
    /// Go: `t.Location()` (nil is the shared UTC).
    fn go_location(&self) -> Arc<Location>;
    /// Go: `t.Zone()` = (abbreviation, seconds east of UTC).
    fn zone(&self) -> (String, i64);
    /// Go: `t.ZoneBounds()`.
    fn zone_bounds(&self) -> (Time, Time);
    /// Go: `t.IsDST()`.
    fn is_dst(&self) -> bool;

    /// Go: `t.Unix()`.
    fn go_unix(&self) -> i64;
    fn unix_milli(&self) -> i64;
    fn unix_micro(&self) -> i64;
    /// Go: `t.UnixNano()`.
    fn go_unix_nano(&self) -> i64;
    /// Go: `t.IsZero()`.
    fn go_is_zero(&self) -> bool;
    /// Go: `t.Equal(u)`.
    fn go_equal(&self, u: &Time) -> bool;
    /// Go: `t.Before(u)`.
    fn go_before(&self, u: &Time) -> bool;
    /// Go: `t.After(u)`.
    fn go_after(&self, u: &Time) -> bool;
    /// Go: `t.Compare(u)` (-1, 0, +1).
    fn compare(&self, u: &Time) -> i64;

    /// Go: `t.MarshalBinary()`.
    fn marshal_binary(&self) -> Result<Vec<u8>, TimeError>;
    /// Go: `t.AppendBinary(b)` (b unchanged on error).
    fn append_binary(&self, b: &mut Vec<u8>) -> Result<(), TimeError>;
    /// Go: `t.MarshalJSON()`.
    fn marshal_json(&self) -> Result<Vec<u8>, TimeError>;
    /// Go: `t.MarshalText()`.
    fn marshal_text(&self) -> Result<Vec<u8>, TimeError>;
    /// Go: `t.AppendText(b)`.
    fn append_text(&self, b: Vec<u8>) -> Result<Vec<u8>, TimeError>;
}

impl GoTimeExt for Time {
    fn format(&self, layout: &str) -> String {
        format::format(self, layout)
    }
    fn format_bytes(&self, layout: &[u8]) -> Vec<u8> {
        format::format_bytes(self, layout)
    }
    fn append_format(&self, b: &mut Vec<u8>, layout: &[u8]) {
        format::append_format(self, b, layout)
    }
    fn string(&self) -> String {
        time::string(self)
    }
    fn go_string(&self) -> String {
        time::go_string(self)
    }

    fn date(&self) -> (i64, Month, i64) {
        time::date_of(self)
    }
    fn year(&self) -> i64 {
        time::year(self)
    }
    fn month(&self) -> Month {
        time::month(self)
    }
    fn day(&self) -> i64 {
        time::day(self)
    }
    fn weekday(&self) -> Weekday {
        time::weekday(self)
    }
    fn iso_week(&self) -> (i64, i64) {
        time::iso_week(self)
    }
    fn clock(&self) -> (i64, i64, i64) {
        time::clock(self)
    }
    fn hour(&self) -> i64 {
        time::hour(self)
    }
    fn minute(&self) -> i64 {
        time::minute(self)
    }
    fn second(&self) -> i64 {
        time::second(self)
    }
    fn nanosecond(&self) -> i64 {
        time::nanosecond(self)
    }
    fn year_day(&self) -> i64 {
        time::year_day(self)
    }

    fn add(&self, d: Duration) -> Time {
        time::add(self, d)
    }
    fn sub(&self, u: &Time) -> Duration {
        time::sub(self, u)
    }
    fn add_date(&self, years: i64, months: i64, days: i64) -> Time {
        time::add_date(self, years, months, days)
    }
    fn truncate(&self, d: Duration) -> Time {
        time::truncate(self, d)
    }
    fn round(&self, d: Duration) -> Time {
        time::round(self, d)
    }

    fn utc(&self) -> Time {
        time::to_utc(self)
    }
    fn local(&self) -> Time {
        time::to_local(self)
    }
    fn in_loc(&self, loc: &Arc<Location>) -> Time {
        time::in_location(self, loc)
    }
    fn go_location(&self) -> Arc<Location> {
        time::location(self)
    }
    fn zone(&self) -> (String, i64) {
        time::zone(self)
    }
    fn zone_bounds(&self) -> (Time, Time) {
        time::zone_bounds(self)
    }
    fn is_dst(&self) -> bool {
        time::is_dst(self)
    }

    fn go_unix(&self) -> i64 {
        time::unix_of(self)
    }
    fn unix_milli(&self) -> i64 {
        time::unix_milli_of(self)
    }
    fn unix_micro(&self) -> i64 {
        time::unix_micro_of(self)
    }
    fn go_unix_nano(&self) -> i64 {
        time::unix_nano_of(self)
    }
    fn go_is_zero(&self) -> bool {
        time::is_zero(self)
    }
    fn go_equal(&self, u: &Time) -> bool {
        time::equal(self, u)
    }
    fn go_before(&self, u: &Time) -> bool {
        time::before(self, u)
    }
    fn go_after(&self, u: &Time) -> bool {
        time::after(self, u)
    }
    fn compare(&self, u: &Time) -> i64 {
        time::compare(self, u)
    }

    fn marshal_binary(&self) -> Result<Vec<u8>, TimeError> {
        time::marshal_binary(self)
    }
    fn append_binary(&self, b: &mut Vec<u8>) -> Result<(), TimeError> {
        time::append_binary(self, b)
    }
    fn marshal_json(&self) -> Result<Vec<u8>, TimeError> {
        time::marshal_json(self)
    }
    fn marshal_text(&self) -> Result<Vec<u8>, TimeError> {
        time::marshal_text(self)
    }
    fn append_text(&self, b: Vec<u8>) -> Result<Vec<u8>, TimeError> {
        time::append_text(self, b)
    }
}
