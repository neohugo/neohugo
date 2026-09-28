//! Port of `tpl/time/time.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_time::{Duration, GoTimeExt, Location};
use go_value::{HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::dynacache::{OptionsPartition, Partition, get_or_create_partition};
use nh_common::htime::{self, TimeFormatter};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_tplimpl::template_funcs::DurationValue;

// Parity notes: `dateFormat` = htime.ToTimeInDefaultLocationE (RFC3339 round trip drops sub-seconds) + TimeFormatter (locale month/day substitution); `now` = htime.Now() (--clock).

/// Go: `time.Namespace` (template value `*time.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    time_formatter: TimeFormatter,
    location: Arc<Location>,
    cache_in: Arc<Partition<String, Arc<Location>>>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn time_err(e: go_time::TimeError) -> go_value::Error {
    gerr(e.to_string())
}

/// Go: `durationUnits`.
fn duration_unit(unit: &[u8]) -> Option<i64> {
    const NANOSECOND: i64 = 1;
    const MICROSECOND: i64 = 1000 * NANOSECOND;
    const MILLISECOND: i64 = 1000 * MICROSECOND;
    const SECOND: i64 = 1000 * MILLISECOND;
    const MINUTE: i64 = 60 * SECOND;
    const HOUR: i64 = 60 * MINUTE;
    Some(match unit {
        b"nanosecond" | b"ns" => NANOSECOND,
        b"microsecond" | b"us" => MICROSECOND,
        _ if unit == "µs".as_bytes() => MICROSECOND,
        b"millisecond" | b"ms" => MILLISECOND,
        b"second" | b"s" => SECOND,
        b"minute" | b"m" => MINUTE,
        b"hour" | b"h" => HOUR,
        _ => return None,
    })
}

impl Namespace {
    /// New returns a new instance of the time-namespaced template functions, with the time
    /// formatter and location of the site's language (Go's `init` passes
    /// `langs.GetTimeFormatter(d.Conf.Language())` and `langs.GetLocation(...)`).
    // Go: tpl/time/time.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let lang = d.conf.language();
        Namespace::new_with(lang.time_formatter().clone(), lang.location(), d)
    }

    /// Go: `New(timeFormatter, location, deps)`.
    // Go: tpl/time/time.go:New
    pub fn new_with(
        time_formatter: TimeFormatter,
        location: Arc<Location>,
        d: Arc<Deps>,
    ) -> Namespace {
        let cache_in = get_or_create_partition::<String, Arc<Location>>(
            &d.mem_cache,
            "/tmpl/time/in",
            OptionsPartition {
                weight: 30,
                clear_when: nh_common::dynacache::ClearWhen::Never,
            },
        );
        Namespace {
            d,
            time_formatter,
            location,
            cache_in,
        }
    }

    /// AsTime converts the textual representation of the datetime string into a time.Time
    /// interface.
    // Go: tpl/time/time.go:AsTime
    pub fn as_time(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "AsTime")?;
        self.as_time_impl(&a[0], &a[1..])
    }

    // Go: tpl/time/time.go:AsTime
    pub(crate) fn as_time_impl(&self, v: &Value, a: &[Value]) -> GoResult<Value> {
        let mut loc = self.location.clone();
        if let Some(l) = a.first() {
            let loc_str = caste::to_string_e(l)?;
            loc = go_time::load_location(&loc_str.to_str_lossy()).map_err(time_err)?;
        }

        Ok(Value::Time(htime::to_time_in_default_location_e(v, &loc)?))
    }

    /// Duration converts the given number to a time.Duration. Unit is one of nanosecond/ns,
    /// microsecond/us/µs, millisecond/ms, second/s, minute/m or hour/h.
    // Go: tpl/time/time.go:Duration
    pub fn duration(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Duration")?;
        let unit_str = caste::to_string_e(&a[0])?;
        let Some(unit_duration) = duration_unit(unit_str.as_bytes()) else {
            return Err(gerr(format!(
                "{} is not a valid duration unit",
                String::from_utf8_lossy(&go_fmt::sprintf("%q", &[a[0].clone()]))
            )));
        };
        let n = caste::to_int64_e(&a[1])?;
        Ok(Value::object(DurationValue(Duration(
            n.wrapping_mul(unit_duration),
        ))))
    }

    /// Format converts the textual representation of the datetime string in v into time.Time
    /// if needed and formats it with the given layout.
    // Go: tpl/time/time.go:Format
    pub fn format(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Format")?;
        let layout = args::string(a, 0)?;
        let t = htime::to_time_in_default_location_e(&a[1], &self.location)?;
        Ok(Value::string(
            self.time_formatter.format_bytes(&t, layout.as_bytes()),
        ))
    }

    /// In returns the time t in the IANA time zone specified by timeZoneName. If timeZoneName
    /// is "" or "UTC", the time is returned in UTC. If timeZoneName is "Local", the time is
    /// returned in the system's local time zone. Otherwise, timeZoneName must be a valid IANA
    /// location name (e.g., "Europe/Oslo").
    // Go: tpl/time/time.go:In
    pub fn in_(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "In")?;
        let time_zone_name = args::string(a, 0)?;
        let t = match &a[1] {
            Value::Time(t) => t.clone(),
            Value::Invalid => return Err(args::invalid_value("time.Time")),
            v => return Err(args::wrong_type("time.Time", v)),
        };
        let name = time_zone_name.to_str_lossy().into_owned();
        let location =
            self.cache_in
                .get_or_create(nh_common::dynacache::clean_key(&name), |_| {
                    go_time::load_location(&name)
                        .map_err(|e| nh_common::herrors::Error::new(e.to_string()))
                })?;

        Ok(Value::Time(t.in_loc(&location)))
    }

    /// Now returns the current local time or `clock` time
    // Go: tpl/time/time.go:Now
    pub fn now(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Now")?;
        Ok(Value::Time(htime::now()))
    }

    /// ParseDuration parses the duration string s.
    // Go: tpl/time/time.go:ParseDuration
    pub fn parse_duration(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ParseDuration")?;
        let ss = caste::to_string_e(&a[0])?;
        go_time::parse_duration(ss.as_bytes())
            .map(|d| Value::object(DurationValue(d)))
            .map_err(time_err)
    }
}

nh_common::go_methods!(Namespace {
    "AsTime" => |n, ctx, a| n.as_time(ctx, a),
    "Duration" => |n, ctx, a| n.duration(ctx, a),
    "Format" => |n, ctx, a| n.format(ctx, a),
    "In" => |n, ctx, a| n.in_(ctx, a),
    "Now" => |n, ctx, a| n.now(ctx, a),
    "ParseDuration" => |n, ctx, a| n.parse_duration(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*time.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/time/time.go (150 lines; 3/7 funcs executed)
//   types: Namespace
// OK L29-44: New(timeFormatter htime.TimeFormatter, location *time.Location, deps *deps.Deps) *Namespace
// OK L56-70: (ns *Namespace) AsTime(v any, args ...any) (any, error)
// OK L74-81: (ns *Namespace) Format(layout string, v any) (string, error)
// OK L84-86: (ns *Namespace) Now() time.Time
// OK L92-101: (ns *Namespace) In(timeZoneName string, t time.Time) (time.Time, error)
// OK L109-116: (ns *Namespace) ParseDuration(s any) (time.Duration, error)
// OK L136-150: (ns *Namespace) Duration(unit any, number any) (time.Duration, error)
// ---------------------------------------------------------------------------
