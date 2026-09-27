//! Data model of Go's `time.Time` and `time.Location`.
//!
//! Only the representation lives here; formatting, parsing, zone lookup and
//! calendar arithmetic are implemented in the `go-time` crate on top of these
//! types, so that every crate can carry times inside [`crate::Value`].

use std::fmt;
use std::sync::Arc;

/// Seconds from 0001-01-01T00:00:00Z (Go's internal epoch) to the Unix epoch.
/// Go: `unixToInternal`.
pub const UNIX_TO_INTERNAL: i64 = (1969 * 365 + 1969 / 4 - 1969 / 100 + 1969 / 400) * 86400;

/// Unix seconds of the zero `time.Time` (0001-01-01T00:00:00Z).
pub const ZERO_TIME_UNIX: i64 = -UNIX_TO_INTERNAL;

/// A Go `time.Location`. Mirrors the Go struct (`$GOROOT/src/time/zoneinfo.go`)
/// minus the lookup cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub name: String,
    pub zone: Vec<Zone>,
    pub tx: Vec<ZoneTrans>,
    /// The TZ string used for instants after the last transition.
    pub extend: String,
}

/// Go: `time.zone`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    /// Abbreviated name, e.g. "CET".
    pub name: String,
    /// Seconds east of UTC.
    pub offset: i32,
    pub is_dst: bool,
}

/// Go: `time.zoneTrans`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneTrans {
    /// Transition time, in Unix seconds.
    pub when: i64,
    /// Index into `Location::zone`.
    pub index: u8,
    pub is_std: bool,
    pub is_utc: bool,
}

impl Location {
    /// Go's `time.UTC`: one shared location, so pointer comparisons behave
    /// like Go's `&utcLoc`.
    pub fn utc() -> Arc<Location> {
        static UTC: std::sync::OnceLock<Arc<Location>> = std::sync::OnceLock::new();
        UTC.get_or_init(|| {
            Arc::new(Location {
                name: "UTC".into(),
                zone: Vec::new(),
                tx: Vec::new(),
                extend: String::new(),
            })
        })
        .clone()
    }

    /// Go's `time.FixedZone(name, offset)`.
    pub fn fixed(name: impl Into<String>, offset: i32) -> Arc<Location> {
        let name = name.into();
        Arc::new(Location {
            name: name.clone(),
            zone: vec![Zone {
                name,
                offset,
                is_dst: false,
            }],
            tx: vec![ZoneTrans {
                when: i64::MIN,
                index: 0,
                is_std: false,
                is_utc: false,
            }],
            extend: String::new(),
        })
    }

    /// True for a location that behaves as UTC (Go's `&utcLoc`, or a location
    /// without zones).
    pub fn is_utc(&self) -> bool {
        self.zone.is_empty()
    }
}

/// A Go `time.Time`: an instant with nanosecond precision plus a location.
///
/// The instant is stored as seconds since the Unix epoch and nanoseconds in
/// `[0, 1e9)`. `loc == None` is Go's nil location, which behaves as UTC (the
/// zero `time.Time{}` has it). The monotonic clock reading is not modelled.
#[derive(Clone)]
pub struct Time {
    pub unix_sec: i64,
    pub nsec: u32,
    pub loc: Option<Arc<Location>>,
}

impl Time {
    /// The zero `time.Time{}`: 0001-01-01 00:00:00 +0000 UTC.
    pub fn zero() -> Time {
        Time {
            unix_sec: ZERO_TIME_UNIX,
            nsec: 0,
            loc: None,
        }
    }

    /// Go's `time.Unix(sec, nsec).In(loc)`, with `nsec` normalised into range
    /// (wrapping like Go on overflow). Note that Go's `time.Unix` uses the
    /// Local location; use `go_time` constructors for Go-exact behaviour.
    pub fn from_unix(sec: i64, nsec: i64, loc: Option<Arc<Location>>) -> Time {
        let mut sec = sec;
        let mut nsec = nsec;
        if !(0..1_000_000_000).contains(&nsec) {
            let n = nsec.div_euclid(1_000_000_000);
            sec = sec.wrapping_add(n);
            nsec = nsec.wrapping_sub(n.wrapping_mul(1_000_000_000));
        }
        Time {
            unix_sec: sec,
            nsec: nsec as u32,
            loc,
        }
    }

    /// Seconds since 0001-01-01T00:00:00Z, wrapping like Go's internal
    /// representation (Go compares instants on this value).
    fn internal_sec(&self) -> i64 {
        self.unix_sec.wrapping_add(UNIX_TO_INTERNAL)
    }

    /// Go's `Time.IsZero`: the instant is 0001-01-01T00:00:00Z, whatever the location.
    pub fn is_zero(&self) -> bool {
        self.unix_sec == ZERO_TIME_UNIX && self.nsec == 0
    }

    /// Go's `Time.Unix`.
    pub fn unix(&self) -> i64 {
        self.unix_sec
    }

    /// Go's `Time.UnixNano` (wraps like Go for out-of-range instants).
    pub fn unix_nano(&self) -> i64 {
        self.unix_sec
            .wrapping_mul(1_000_000_000)
            .wrapping_add(self.nsec as i64)
    }

    /// Go's `Time.Equal`: same instant, location ignored.
    pub fn equal(&self, other: &Time) -> bool {
        self.unix_sec == other.unix_sec && self.nsec == other.nsec
    }

    /// Go's `Time.Before`.
    pub fn before(&self, other: &Time) -> bool {
        (self.internal_sec(), self.nsec) < (other.internal_sec(), other.nsec)
    }

    /// Go's `Time.After`.
    pub fn after(&self, other: &Time) -> bool {
        (self.internal_sec(), self.nsec) > (other.internal_sec(), other.nsec)
    }

    /// The location, with nil mapped to UTC (Go's `Time.Location`).
    pub fn location(&self) -> Arc<Location> {
        match &self.loc {
            Some(l) => l.clone(),
            None => Location::utc(),
        }
    }
}

/// Go's `==` on `time.Time`: same instant and same location pointer. Locations
/// compare by pointer, or structurally when both are UTC-like.
impl PartialEq for Time {
    fn eq(&self, other: &Time) -> bool {
        if self.unix_sec != other.unix_sec || self.nsec != other.nsec {
            return false;
        }
        match (&self.loc, &other.loc) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loc = self.loc.as_ref().map(|l| l.name.as_str()).unwrap_or("UTC");
        write!(
            f,
            "Time{{unix: {}, nsec: {}, loc: {}}}",
            self.unix_sec, self.nsec, loc
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_time() {
        assert!(Time::zero().is_zero());
        assert_eq!(ZERO_TIME_UNIX, -62135596800);
        assert!(!Time::from_unix(0, 0, None).is_zero());
    }

    #[test]
    fn nsec_normalisation() {
        let t = Time::from_unix(10, -1, None);
        assert_eq!((t.unix_sec, t.nsec), (9, 999_999_999));
        let t = Time::from_unix(10, 2_500_000_000, None);
        assert_eq!((t.unix_sec, t.nsec), (12, 500_000_000));
    }
}
