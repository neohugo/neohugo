//! Port of `tpl/debug/debug.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use go_value::{HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::loggers::Level;
use nh_common::object::{GoResult, args};
use nh_config::neohugo::neohugo::deprecate;
use nh_config::neohugo::version::CURRENT_VERSION;
use nh_deps::deps::Deps;

/// Go: `debug.Namespace` (template value `*debug.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    /// Go: `timers` (nil unless the log level is info or lower). Go ranges the map when it
    /// prints the timers; the port sorts them (by total duration, as Go does afterwards).
    timers: Option<Arc<Mutex<BTreeMap<String, Vec<Arc<Timer>>>>>>,
}

/// Go: `*debug.timer`.
pub struct Timer {
    start: Instant,
    /// Go: `elapsed` + `stopOnce` (the elapsed time once stopped).
    elapsed: OnceLock<i64>,
}

impl Timer {
    // Go: tpl/debug/debug.go:(*timer).Stop
    fn stop(&self) -> Value {
        self.elapsed
            .get_or_init(|| self.start.elapsed().as_nanos() as i64);
        // This is used in templates, we need to return something.
        Value::string("")
    }

    fn elapsed(&self) -> i64 {
        self.elapsed.get().copied().unwrap_or(0)
    }
}

nh_common::go_methods!(Timer {
    "Stop" => |t, _c, a| { args::exactly(a, 0, "Stop")?; Ok(t.stop()) },
});

impl Object for Timer {
    nh_common::object_basics!("*debug.timer");
}

/// Go: `debug.nopTimerImpl`.
pub struct NopTimer;

nh_common::go_methods!(NopTimer {
    // Go: tpl/debug/debug.go:(nopTimerImpl).Stop
    "Stop" => |_t, _c, a| { args::exactly(a, 0, "Stop")?; Ok(Value::string("")) },
});

impl Object for NopTimer {
    nh_common::object_basics!("debug.nopTimerImpl");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

fn dur(ns: i64) -> String {
    go_time::Duration(ns).string()
}

impl Namespace {
    /// New returns a new instance of the debug-namespaced template functions.
    // Go: tpl/debug/debug.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let timers = if d.log.level() <= Level::Info {
            Some(Arc::new(Mutex::new(BTreeMap::new())))
        } else {
            None
        };

        let Some(t) = timers.clone() else {
            return Namespace { d, timers };
        };

        let log = d.log.clone();
        d.build_end_listeners.add(Box::new(move |_: &[()]| {
            struct Data {
                name: String,
                count: usize,
                average: i64,
                median: i64,
                duration: i64,
            }

            let mut timers_sorted: Vec<Data> = Vec::new();

            let mut timers = t.lock().unwrap_or_else(|e| e.into_inner());
            for (k, v) in timers.iter_mut() {
                let mut total: i64 = 0;
                let mut median: i64 = 0;
                go_sort::sort::slice(v, |x, i, j| x[i].elapsed() < x[j].elapsed());
                if !v.is_empty() {
                    median = v[v.len() / 2].elapsed();
                }
                for t in v.iter() {
                    // Stop any running timers.
                    t.stop();
                    total += t.elapsed();
                }
                let average = total / v.len() as i64;
                timers_sorted.push(Data {
                    name: k.clone(),
                    count: v.len(),
                    average,
                    median,
                    duration: total,
                });
            }

            // Sort it so the slowest gets printed last.
            go_sort::sort::slice(&mut timers_sorted, |x, i, j| x[i].duration < x[j].duration);

            for t in &timers_sorted {
                log.infof(format!(
                    "timer:  name={} count={} duration={} average={} median={}",
                    t.name,
                    t.count,
                    dur(t.duration),
                    dur(t.average),
                    dur(t.median)
                ));
            }

            timers.clear();

            false
        }));

        Namespace { d, timers }
    }

    /// Dump returns a object dump of val as a string.
    // Go: tpl/debug/debug.go:Dump
    pub fn dump(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Dump")?;
        match go_json::marshal_indent(&a[0], "", "  ") {
            Ok(b) => Ok(Value::string(b)),
            Err(_) => Ok(Value::string("")),
        }
    }

    /// Timer starts a timer with the given name (a no-op timer unless the log level is info or
    /// lower).
    // Go: tpl/debug/debug.go:Timer
    pub fn timer(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Timer")?;
        let name = args::string(a, 0)?;
        let Some(timers) = &self.timers else {
            return Ok(Value::object(NopTimer));
        };
        let mut timers = timers.lock().unwrap_or_else(|e| e.into_inner());
        let t = Arc::new(Timer {
            start: Instant::now(),
            elapsed: OnceLock::new(),
        });
        timers
            .entry(name.to_str_lossy().into_owned())
            .or_default()
            .push(t.clone());
        Ok(Value::Object(t))
    }

    /// VisualizeSpaces returns a string with spaces replaced by a visible string.
    // Go: tpl/debug/debug.go:VisualizeSpaces
    pub fn visualize_spaces(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "VisualizeSpaces")?;
        let s = caste::to_string(&a[0]);
        Ok(Value::string(goldmark::util::visualize_spaces(
            s.as_bytes(),
        )))
    }

    /// Internal template func, used in tests only.
    // Go: tpl/debug/debug.go:TestDeprecationInfo
    pub fn test_deprecation_info(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TestDeprecationInfo")?;
        let (item, alternative) = (args::string(a, 0)?, args::string(a, 1)?);
        let v = CURRENT_VERSION;
        deprecate(
            &item.to_str_lossy(),
            &alternative.to_str_lossy(),
            &v.string(),
        );
        Ok(Value::string(""))
    }

    /// Internal template func, used in tests only.
    // Go: tpl/debug/debug.go:TestDeprecationWarn
    pub fn test_deprecation_warn(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TestDeprecationWarn")?;
        let (item, alternative) = (args::string(a, 0)?, args::string(a, 1)?);
        let mut v = CURRENT_VERSION;
        v.minor -= 3;
        deprecate(
            &item.to_str_lossy(),
            &alternative.to_str_lossy(),
            &v.string(),
        );
        Ok(Value::string(""))
    }

    /// Internal template func, used in tests only.
    // Go: tpl/debug/debug.go:TestDeprecationErr
    pub fn test_deprecation_err(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TestDeprecationErr")?;
        let (item, alternative) = (args::string(a, 0)?, args::string(a, 1)?);
        let mut v = CURRENT_VERSION;
        v.minor -= 15;
        deprecate(
            &item.to_str_lossy(),
            &alternative.to_str_lossy(),
            &v.string(),
        );
        Ok(Value::string(""))
    }
}

nh_common::go_methods!(Namespace {
    "Dump" => |n, ctx, a| n.dump(ctx, a),
    "TestDeprecationErr" => |n, ctx, a| n.test_deprecation_err(ctx, a),
    "TestDeprecationInfo" => |n, ctx, a| n.test_deprecation_info(ctx, a),
    "TestDeprecationWarn" => |n, ctx, a| n.test_deprecation_warn(ctx, a),
    "Timer" => |n, ctx, a| n.timer(ctx, a),
    "VisualizeSpaces" => |n, ctx, a| n.visualize_spaces(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*debug.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/debug/debug.go (185 lines; 1/9 funcs executed)
//   types: Namespace, nopTimerImpl, Timer, timer
// OK L32-92: New(d *deps.Deps) *Namespace
// OK L109-115: (ns *Namespace) Dump(val any) string
// OK L118-121: (ns *Namespace) VisualizeSpaces(val any) string
// OK L123-132: (ns *Namespace) Timer(name string) Timer
// OK L138-140: (nopTimerImpl) Stop() string
// OK L156-162: (t *timer) Stop() string
// OK L165-169: (ns *Namespace) TestDeprecationInfo(item, alternative string) string
// OK L172-177: (ns *Namespace) TestDeprecationWarn(item, alternative string) string
// OK L180-185: (ns *Namespace) TestDeprecationErr(item, alternative string) string
// ---------------------------------------------------------------------------
