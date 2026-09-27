//! Module `cast::time`.
//!
//! PORT spf13/cast@v1.9.2 time.go + internal/time.go (ToTimeInDefaultLocationE, 24 layouts)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).


use std::sync::Arc;

use go_value::{Location, Time, Value};

use crate::herrors::Result;

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
    "Mon Jan _2 15:04:05 2006",        // ANSIC
    "Mon Jan _2 15:04:05 MST 2006",    // UnixDate
    "Mon Jan 02 15:04:05 -0700 2006",  // RubyDate
    "2006-01-02 15:04:05Z07:00",
    "02 Jan 2006",
    "2006-01-02 15:04:05 -07:00",
    "2006-01-02 15:04:05 -0700",
    "3:04PM",                          // Kitchen
    "Jan _2 15:04:05",                 // Stamp
    "Jan _2 15:04:05.000",             // StampMilli
    "Jan _2 15:04:05.000000",          // StampMicro
    "Jan _2 15:04:05.000000000",       // StampNano
];

/// Go: `cast.ToTimeInDefaultLocationE(i, location)` (spf13/cast v1.9.2 time.go).
/// Verify the layout list and the zone re-attachment rules against the Go module source.
// Go: spf13/cast time.go:ToTimeInDefaultLocationE
pub fn to_time_in_default_location_e(v: &Value, loc: &Arc<Location>) -> Result<Time> {
    todo!()
}
