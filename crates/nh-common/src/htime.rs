//! Port of `common/htime/time.go`.
//!
//! plus github.com/bep/clocks (Start/System)
//!
//! Owner: Wave B task T01 (common-values).

//! Go `common/htime` + `github.com/bep/clocks` v0.5.0. [`now`] is `Clock.Now()`; `--clock`
//! installs `clocks.Start(t)`: the system clock shifted by `t - time.Now()` at start, so `now`
//! advances with the wall clock and is in the **Local** location (like `time.Now()`), not in `t`'s.

use std::sync::{Arc, OnceLock};

use go_time::{Duration, GoTimeExt};
use go_value::{GoString, Location, Object, Time, Value};

use crate::herrors::{Error, Result};
use crate::locales::Translator;
use crate::object::GoResult;

static LONG_DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

static SHORT_DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

static SHORT_MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

static LONG_MONTH_NAMES: [&str; 12] = [
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

/// Go: `htime.Clock` (`bep/clocks.Clock`).
pub trait Clock: Send + Sync {
    /// Go: `Clock.Now()`.
    fn now(&self) -> Time;

    /// Go: `Clock.Since(t)` = `c.Now().Sub(t)`.
    fn since(&self, t: &Time) -> Duration {
        self.now().sub(t)
    }

    /// Go: `Clock.Until(t)` = `t.Sub(c.Now())`.
    fn until(&self, t: &Time) -> Duration {
        t.sub(&self.now())
    }

    /// Go: `Clock.Offset()`: the offset of this clock relative to the system clock.
    fn offset(&self) -> Duration;
}

/// Go: `clocks.System()` — `time.Now()` (Local location).
pub struct SystemClock;

impl Clock for SystemClock {
    // Go: bep/clocks clock.go:(*systemClock).Now
    fn now(&self) -> Time {
        go_time::now()
    }

    // Go: bep/clocks clock.go:(*systemClock).Offset
    fn offset(&self) -> Duration {
        Duration(0)
    }
}

/// Go: `clocks.Start(t)` — a clock `offset = t.Sub(time.Now())` ahead of the system clock.
pub struct StartClock {
    pub offset: Duration,
}

impl StartClock {
    // Go: bep/clocks clock.go:Start
    /// Start creates a new Clock starting at t.
    pub fn new(t: &Time) -> Self {
        StartClock {
            offset: t.sub(&go_time::now()),
        }
    }
}

impl Clock for StartClock {
    // Go: bep/clocks clock.go:(*clock).Now
    /// Now returns the current time relative to the configured start time (Local location).
    fn now(&self) -> Time {
        go_time::now().add(self.offset)
    }

    // Go: bep/clocks clock.go:(*clock).Offset
    fn offset(&self) -> Duration {
        self.offset
    }
}

static CLOCK: OnceLock<Arc<dyn Clock>> = OnceLock::new();

/// Installs the process clock (Go: `htime.Clock = clocks.Start(t)` in commands, before the
/// build). The first call wins; [`now`] installs the system clock if nothing was set.
pub fn set_clock(c: Arc<dyn Clock>) {
    let _ = CLOCK.set(c);
}

fn clock() -> &'static Arc<dyn Clock> {
    CLOCK.get_or_init(|| Arc::new(SystemClock))
}

// Go: common/htime/time.go:Now
/// Now returns `time.Now()` or the time value based on the `clock` flag.
pub fn now() -> Time {
    clock().now()
}

// Go: common/htime/time.go:Since
/// Since returns the time elapsed since t, on the process clock.
pub fn since(t: &Time) -> Duration {
    clock().since(t)
}

/// Go: `htime.TimeFormatter` — Go layout formatting followed by locale substitution of
/// month/weekday names, and the `:date_*`/`:time_*` locale layouts.
#[derive(Clone)]
pub struct TimeFormatter {
    pub ltr: Arc<dyn Translator>,
}

impl TimeFormatter {
    // Go: common/htime/time.go:NewTimeFormatter
    pub fn new(ltr: Arc<dyn Translator>) -> Self {
        TimeFormatter { ltr }
    }

    /// Go: `TimeFormatter.Format(t, layout)` for a UTF-8 layout.
    pub fn format(&self, t: &Time, layout: &str) -> String {
        String::from_utf8_lossy(&self.format_bytes(t, layout.as_bytes())).into_owned()
    }

    // Go: common/htime/time.go:Format
    /// Go: `TimeFormatter.Format(t, layout)` (Go strings are bytes).
    pub fn format_bytes(&self, t: &Time, layout: &[u8]) -> Vec<u8> {
        if layout.is_empty() {
            return Vec::new();
        }

        if layout[0] == b':' {
            // It may be one of Hugo's custom layouts.
            let ltr = &self.ltr;
            let custom = match &*go_unicode::strings::to_lower(&layout[1..]) {
                b"date_full" => Some(ltr.fmt_date_full(t)),
                b"date_long" => Some(ltr.fmt_date_long(t)),
                b"date_medium" => Some(ltr.fmt_date_medium(t)),
                b"date_short" => Some(ltr.fmt_date_short(t)),
                b"time_full" => Some(ltr.fmt_time_full(t)),
                b"time_long" => Some(ltr.fmt_time_long(t)),
                b"time_medium" => Some(ltr.fmt_time_medium(t)),
                b"time_short" => Some(ltr.fmt_time_short(t)),
                _ => None,
            };
            if let Some(s) = custom {
                return s.into_bytes();
            }
        }

        let mut s = t.format_bytes(layout);

        let month = t.month().0;
        let month_idx = (month - 1) as usize; // Month() starts at 1.
        let day_idx = t.weekday().0 as usize;

        use go_unicode::strings::{contains, replace_all};
        if contains(layout, b"January") {
            s = replace_all(
                &s,
                LONG_MONTH_NAMES[month_idx].as_bytes(),
                self.ltr.month_wide(month as u32).as_bytes(),
            )
            .into_owned();
        } else if contains(layout, b"Jan") {
            s = replace_all(
                &s,
                SHORT_MONTH_NAMES[month_idx].as_bytes(),
                self.ltr.month_abbreviated(month as u32).as_bytes(),
            )
            .into_owned();
        }

        if contains(layout, b"Monday") {
            s = replace_all(
                &s,
                LONG_DAY_NAMES[day_idx].as_bytes(),
                self.ltr.weekday_wide(day_idx as u32).as_bytes(),
            )
            .into_owned();
        } else if contains(layout, b"Mon") {
            s = replace_all(
                &s,
                SHORT_DAY_NAMES[day_idx].as_bytes(),
                self.ltr.weekday_abbreviated(day_idx as u32).as_bytes(),
            )
            .into_owned();
        }

        s
    }
}

// Go: common/htime/time.go:ToTimeInDefaultLocationE
/// ToTimeInDefaultLocationE: an `AsTimeProvider` (an object with an `AsTime` method, e.g.
/// go-toml's local dates) is asked for its time in `location`; a `time.Time` is formatted as
/// RFC3339 and re-parsed (issue #8895: this drops sub-seconds and turns a named zone into a fixed
/// offset); everything else goes to `cast.ToTimeInDefaultLocationE`.
pub fn to_time_in_default_location_e(v: &Value, loc: &Arc<Location>) -> Result<Time> {
    let mut i = v.clone();
    match v {
        Value::Object(o) if o.has_method("AsTime") => {
            let arg = Value::object(LocationRef(loc.clone()));
            return match o.call_method(&(), "AsTime", &[arg]) {
                Some(Ok(Value::Time(t))) => Ok(t),
                Some(Ok(other)) => Err(Error::new(format!(
                    "AsTime returned {} instead of time.Time",
                    other.go_type_name()
                ))),
                Some(Err(e)) => Err(e.into()),
                None => Err(Error::new("AsTime: no such method")),
            };
        }
        // issue #8895
        // datetimes parsed by `go-toml` have empty zone name
        // convert back them into string and use `cast`
        Value::Time(t) => {
            i = Value::string(t.format(go_time::RFC3339));
        }
        _ => {}
    }
    crate::cast::time::to_time_in_default_location_e(&i, loc)
}

// Go: common/htime/time.go:StopWatch
/// StopWatch is a simple helper to measure time during development: the returned closure prints
/// the elapsed time to stderr (Go: `log.Printf`, which also prefixes the date).
pub fn stop_watch(name: &str) -> impl FnOnce() {
    let name = name.to_string();
    let start = go_time::now();
    move || {
        eprintln!(
            "StopWatch {} took {}",
            go_strconv::quote(name.as_bytes()),
            go_time::since(&start).string()
        );
    }
}

/// A `*time.Location` as a template value (the argument of `AsTime(loc)` and the result of
/// `.Location` calls). Prints its name (Go: `(*Location).String()`).
#[derive(Clone, Debug)]
pub struct LocationRef(pub Arc<Location>);

crate::go_methods!(LocationRef {
    "String" => |l, _c, a| {
        crate::object::args::exactly(a, 0, "String")?;
        Ok(Value::string(l.0.name.as_str()))
    },
});

impl Object for LocationRef {
    crate::object_basics!("*time.Location");

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.name.as_str()))
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// Helper for hosts implementing `AsTime(loc)`: the location argument of an `AsTime` call.
pub fn location_arg(args: &[Value], i: usize) -> GoResult<Arc<Location>> {
    match args.get(i) {
        Some(v) => match v.downcast::<LocationRef>() {
            Some(l) => Ok(l.0.clone()),
            None => Err(crate::object::args::wrong_type("*time.Location", v)),
        },
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/htime/time.go (177 lines; 4/6 funcs executed)
//   types: TimeFormatter, AsTimeProvider
// OK L81-88: NewTimeFormatter(ltr locales.Translator) TimeFormatter
// OK L95-140: (f TimeFormatter) Format(t time.Time, layout string) string
// OK L142-154: ToTimeInDefaultLocationE(i any, location *time.Location) (tim time.Time, err error)
// OK L158-160: Now() time.Time
// OK L162-164: Since(t time.Time) time.Duration
// OK L172-177: StopWatch(name string) func()
// Source: github.com/bep/clocks@v0.5.0 clock.go (third-party; Start, System, Now, Since, Until, Offset)
// OK Start(t time.Time) Clock
// OK System() Clock
// ---------------------------------------------------------------------------
