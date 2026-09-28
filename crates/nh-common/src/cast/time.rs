//! Module `cast::time`.
//!
//! PORT spf13/cast@v1.9.2 time.go + internal/time.go (ToTimeInDefaultLocationE, 24 layouts)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).

//! Upstream: `github.com/spf13/cast v1.9.2`, `time.go` (`ToTimeE`, `ToTimeInDefaultLocationE`,
//! `StringToDate`, `StringToDateInDefaultLocation`; `ToDurationE` is in [`super::caste`]) and
//! `internal/time.go` (`TimeFormats`, `ParseDateWith`).
//!
//! Layouts are tried with `time.Parse`, so a zone abbreviation is resolved against `time.Local`
//! (go-time's `local()`), and `time.Unix` results are in `time.Local`, as in Go. A nil location
//! (`time.Local` in Go) is written `None` in [`parse_date_with`].

use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{Location, Time, UintKind, Value};

use super::caste::{cast_error, indirect, json_number, trim_zero_decimal};
use crate::herrors::{Error, Result};

/// The 24 layouts of spf13/cast `internal.TimeFormats`, tried in order with `time.Parse`
/// (`internal.ParseDateWith`); zone-less layouts are re-attached to `location`.
pub const TIME_FORMATS: &[&str] = &[
    "2006-01-02",
    "2006-01-02T15:04:05Z07:00", // RFC3339
    "2006-01-02T15:04:05",
    "Mon, 02 Jan 2006 15:04:05 -0700", // RFC1123Z
    "Mon, 02 Jan 2006 15:04:05 MST",   // RFC1123
    "02 Jan 06 15:04 -0700",           // RFC822Z
    "02 Jan 06 15:04 MST",             // RFC822
    "Monday, 02-Jan-06 15:04:05 MST",  // RFC850
    "2006-01-02 15:04:05.999999999 -0700 MST",
    "2006-01-02T15:04:05-0700",
    "2006-01-02 15:04:05Z0700",
    "2006-01-02 15:04:05",
    "Mon Jan _2 15:04:05 2006",       // ANSIC
    "Mon Jan _2 15:04:05 MST 2006",   // UnixDate
    "Mon Jan 02 15:04:05 -0700 2006", // RubyDate
    "2006-01-02 15:04:05Z07:00",
    "02 Jan 2006",
    "2006-01-02 15:04:05 -07:00",
    "2006-01-02 15:04:05 -0700",
    "3:04PM",                    // Kitchen
    "Jan _2 15:04:05",           // Stamp
    "Jan _2 15:04:05.000",       // StampMilli
    "Jan _2 15:04:05.000000",    // StampMicro
    "Jan _2 15:04:05.000000000", // StampNano
];

// Go: spf13/cast internal/time.go:TimeFormatType
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeFormatType {
    NoTimezone = 0,
    NamedTimezone = 1,
    NumericTimezone = 2,
    NumericAndNamedTimezone = 3,
    TimeOnly = 4,
}

// Go: spf13/cast internal/time.go:TimeFormat
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeFormat {
    pub format: &'static str,
    pub typ: TimeFormatType,
}

impl TimeFormat {
    // Go: spf13/cast internal/time.go:TimeFormat.HasTimezone
    /// We don't include the formats with only named timezones, see
    /// <https://github.com/golang/go/issues/19694#issuecomment-289103522>.
    pub fn has_timezone(&self) -> bool {
        self.typ >= TimeFormatType::NumericTimezone
            && self.typ <= TimeFormatType::NumericAndNamedTimezone
    }
}

/// The types of [`TIME_FORMATS`], index by index (Go: `internal.TimeFormats`).
const TIME_FORMAT_TYPES: [TimeFormatType; 24] = {
    use TimeFormatType::*;
    [
        NoTimezone,
        NumericTimezone,
        NoTimezone,
        NumericTimezone,
        NamedTimezone,
        NumericTimezone,
        NamedTimezone,
        NamedTimezone,
        NumericAndNamedTimezone,
        NumericTimezone,
        NumericTimezone,
        NoTimezone,
        NoTimezone,
        NamedTimezone,
        NumericTimezone,
        NumericTimezone,
        NoTimezone,
        NumericTimezone,
        NumericTimezone,
        TimeOnly,
        TimeOnly,
        TimeOnly,
        TimeOnly,
        TimeOnly,
    ]
};

/// Go: `internal.TimeFormats`.
pub fn time_formats() -> Vec<TimeFormat> {
    TIME_FORMATS
        .iter()
        .zip(TIME_FORMAT_TYPES)
        .map(|(format, typ)| TimeFormat { format, typ })
        .collect()
}

// Go: spf13/cast internal/time.go:ParseDateWith
/// Tries each format with `time.Parse`; formats without a numeric zone are re-created in
/// `location` (`None` = `time.Local`).
pub fn parse_date_with(
    s: &[u8],
    location: Option<&Arc<Location>>,
    formats: &[TimeFormat],
) -> Result<Time> {
    for format in formats {
        if let Ok(d) = go_time::parse(format.format, s) {
            // Some time formats have a zone name, but no offset, so it gets
            // put in that zone name (not the default one passed in to us), but
            // without that zone's offset. So set the location manually.
            if format.typ <= TimeFormatType::NamedTimezone {
                let location = match location {
                    Some(l) => l.clone(),
                    None => go_time::local(),
                };
                let (year, month, day) = d.date();
                let (hour, min, sec) = d.clock();
                return Ok(go_time::date(
                    year,
                    month,
                    day,
                    hour,
                    min,
                    sec,
                    d.nanosecond(),
                    &location,
                ));
            }

            return Ok(d);
        }
    }
    Err(Error::new(format!(
        "unable to parse date: {}",
        String::from_utf8_lossy(s)
    )))
}

// Go: spf13/cast time.go:StringToDate
/// Parses a string into a `time.Time` using the predefined list of formats (UTC default).
pub fn string_to_date(s: &[u8]) -> Result<Time> {
    parse_date_with(s, Some(&go_time::utc()), &time_formats())
}

// Go: spf13/cast time.go:StringToDateInDefaultLocation
/// Parses a string into a `time.Time`, interpreting inputs without a timezone to be in the given
/// location (`None` = `time.Local`).
pub fn string_to_date_in_default_location(
    s: &[u8],
    location: Option<&Arc<Location>>,
) -> Result<Time> {
    parse_date_with(s, location, &time_formats())
}

// Go: spf13/cast time.go:ToTimeE
/// ToTimeE casts any value to a `time.Time` (UTC default location).
pub fn to_time_e(v: &Value) -> Result<Time> {
    to_time_in_default_location_e(v, &go_time::utc())
}

/// `cast.ToTime` (errors -> the zero time).
pub fn to_time(v: &Value) -> Time {
    to_time_e(v).unwrap_or_else(|_| Time::zero())
}

/// Go: `cast.ToTimeInDefaultLocationE(i, location)` (spf13/cast v1.9.2 time.go).
/// Verify the layout list and the zone re-attachment rules against the Go module source.
// Go: spf13/cast time.go:ToTimeInDefaultLocationE
pub fn to_time_in_default_location_e(v: &Value, loc: &Arc<Location>) -> Result<Time> {
    let (i, _) = indirect(v);

    match &i {
        Value::Time(t) => Ok(t.clone()),
        Value::String(s) => string_to_date_in_default_location(s.as_bytes(), Some(loc)),
        Value::Int(
            n,
            go_value::IntKind::Int | go_value::IntKind::Int32 | go_value::IntKind::Int64,
        ) => Ok(go_time::unix(*n, 0)),
        Value::Uint(n, UintKind::Uint | UintKind::Uint32 | UintKind::Uint64) => {
            Ok(go_time::unix(*n as i64, 0))
        }
        Value::Invalid => Ok(Time::zero()),
        _ => {
            if let Some(s) = json_number(&i) {
                // Originally this used ToInt64E, but adding string float conversion broke ToTime.
                // the behavior of ToTime would have changed if we continued using it.
                // For now, using json.Number's own Int64 method should be good enough to preserve
                // backwards compatibility.
                let v = trim_zero_decimal(s.as_bytes());
                return match go_strconv::parse_int(v, 10, 64) {
                    Ok(s) => Ok(go_time::unix(s, 0)),
                    Err(_) => Err(cast_error(&i, "time.Time")),
                };
            }
            Err(cast_error(&i, "time.Time"))
        }
    }
}

/// `cast.ToTimeInDefaultLocation` (errors -> the zero time).
pub fn to_time_in_default_location(v: &Value, loc: &Arc<Location>) -> Time {
    to_time_in_default_location_e(v, loc).unwrap_or_else(|_| Time::zero())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (third-party: spf13/cast v1.9.2; written by T26, not generated).
// Source: time.go
// OK L19-21: ToTimeE(i any) (time.Time, error)
// OK L26-61: ToTimeInDefaultLocationE(i any, location *time.Location) (tim time.Time, err error)
// OK L64-101: ToDurationE(i any) (time.Duration, error) (cast/caste.rs)
// OK L107-109: StringToDate(s string) (time.Time, error)
// OK L114-116: StringToDateInDefaultLocation(s string, location *time.Location) (time.Time, error)
// Source: internal/time.go
//   types: TimeFormatType, TimeFormat
// OK L13-17: TimeFormatType constants
// OK L25-29: (f TimeFormat) HasTimezone() bool
// OK L31-57: TimeFormats
// OK L59-79: ParseDateWith(s string, location *time.Location, formats []TimeFormat) (d time.Time, e error)
// Source: internal/timeformattype_string.go — not used by neohugo
//    L22-27: (i TimeFormatType) String() string
// ---------------------------------------------------------------------------
