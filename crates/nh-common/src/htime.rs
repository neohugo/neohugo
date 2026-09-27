//! Port of `common/htime/time.go`.
//!
//! plus github.com/bep/clocks (Start/System)
//!
//! Owner: Wave B task T01 (common-values).


//! Go `common/htime` + `github.com/bep/clocks`. `now` is `Clock.Now()`; `--clock` installs
//! `clocks.Start(t)` (time starts at t and advances with the wall clock).

use std::sync::{Arc, OnceLock};

use go_value::{Location, Time, Value};

use crate::herrors::Result;
use crate::locales::Translator;

/// Go: `htime.Clock` (bep/clocks.Clock).
pub trait Clock: Send + Sync {
    fn now(&self) -> Time;
}

/// Go: `clocks.System()` — machine time in the Local zone.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Time {
        todo!("std::time::SystemTime::now() in go-time Local location")
    }
}

/// Go: `clocks.Start(t)` — `t + elapsed since start`, keeping t's location.
pub struct StartClock {
    pub start: Time,
    pub started_at: std::time::Instant,
}

impl Clock for StartClock {
    fn now(&self) -> Time {
        let d = self.started_at.elapsed();
        Time::from_unix(self.start.unix_sec, self.start.nsec as i64 + d.as_nanos() as i64, self.start.loc.clone())
    }
}

static CLOCK: OnceLock<Arc<dyn Clock>> = OnceLock::new();

/// Installs the process clock (commands: `--clock`). First call wins.
pub fn set_clock(c: Arc<dyn Clock>) {
    let _ = CLOCK.set(c);
}

/// Go: `htime.Now()`.
// Go: common/htime/time.go:Now
pub fn now() -> Time {
    CLOCK.get_or_init(|| Arc::new(SystemClock)).now()
}

/// Go: `htime.TimeFormatter` — Go layout formatting followed by locale substitution of
/// month/weekday names (`dateFormat`), and `:date_*`/`:time_*` locale layouts.
#[derive(Clone)]
pub struct TimeFormatter {
    pub ltr: Arc<dyn Translator>,
}

impl TimeFormatter {
    // Go: common/htime/time.go:NewTimeFormatter
    pub fn new(ltr: Arc<dyn Translator>) -> Self {
        TimeFormatter { ltr }
    }

    /// Go: `TimeFormatter.Format(t, layout)`.
    // Go: common/htime/time.go:Format
    pub fn format(&self, t: &Time, layout: &str) -> String {
        todo!("go_time::format + strings.ReplaceAll of English month/day names with ltr names")
    }
}

/// Go: `htime.ToTimeInDefaultLocationE(i any, location)` — go-toml local date/time via
/// `AsTime(loc)`; `time.Time` is formatted RFC3339 and re-parsed (drops sub-seconds!); strings via
/// `cast.ToTimeInDefaultLocationE`.
// Go: common/htime/time.go:ToTimeInDefaultLocationE
pub fn to_time_in_default_location_e(v: &Value, loc: &Arc<Location>) -> Result<Time> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/htime/time.go (177 lines; 4/6 funcs executed)
//   types: TimeFormatter, AsTimeProvider
// EX L81-88: NewTimeFormatter(ltr locales.Translator) TimeFormatter
// EX L95-140: (f TimeFormatter) Format(t time.Time, layout string) string
// EX L142-154: ToTimeInDefaultLocationE(i any, location *time.Location) (tim time.Time, err error)
// EX L158-160: Now() time.Time
//    L162-164: Since(t time.Time) time.Duration
//    L172-177: StopWatch(name string) func()
// ---------------------------------------------------------------------------
