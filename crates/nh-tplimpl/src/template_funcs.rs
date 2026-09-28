//! Port of `tpl/tplimpl/template_funcs.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `tpl/tplimpl/template_funcs.go`: `templateExecHelper`, plus the methods of `time.Time` (and
//! the named `time.Month`, `time.Weekday`, `time.Duration` values they return) that Go's
//! reflection finds on a `time.Time` receiver (`hreflect.GetMethodByName`).

use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use go_time::{Duration, GoTimeExt, Month, Weekday};
use go_value::{GoString, HostCtx, IntKind, Kind, MapType, Object, Time, Value};
use nh_common::htime::LocationRef;
use nh_common::object::{GoResult, NamedTypeRegistry, args, bad_results_error};
use nh_page::site::SiteRef;

use crate::engine::{ExecHelper, FuncMap, TplFunc};

/// Go: `templateExecHelper`.
///
/// `mainsections` (Go `GetMethod`, template_funcs.go:88-114): the special case fires only when
/// the receiver IS the site's params map (pointer equality). Rust: `Value::Map(m)` with
/// `Arc::ptr_eq(m, <site params Arc>)`; nh-hugolib must hand out the SAME `Arc<Map>` for
/// `.Site.Params` every time (never rebuild it per call).
pub struct TemplateExecHelper {
    /// Hugo funcs + html escaper funcs + text builtins (Hugo wins on name clashes).
    pub funcs: Arc<FuncMap>,
    pub site: Arc<OnceLock<SiteRef>>,
    pub named_types: Arc<NamedTypeRegistry>,
}

impl TemplateExecHelper {
    /// Go: `strings.EqualFold(name, "mainsections") && receiver.Type() == typeParams &&
    /// receiver.Pointer() == t.siteParams.Pointer()`: the site to call `MainSections` on.
    fn main_sections_site(&self, receiver: &Value, name: &str) -> Option<Value> {
        if !go_unicode::strings::equal_fold(name.as_bytes(), b"mainsections") {
            return None;
        }
        let Value::Map(m) = receiver else {
            return None;
        };
        if m.ty != MapType::Params {
            return None;
        }
        let site = self.site.get()?;
        if !Arc::ptr_eq(m, &site.0.params()) {
            return None;
        }
        Some(Value::Object(Arc::new(site.clone())))
    }

    /// `hreflect.GetMethodByName(receiver, name).IsValid()`.
    fn method_by_name_exists(&self, receiver: &Value, name: &str) -> bool {
        match receiver {
            Value::Object(o) => o.has_method(name),
            Value::List(_) | Value::Map(_) => self.named_types.has_method(receiver, name),
            // A nil value of a named slice/map type keeps its method set.
            Value::TypedNil(t) => self
                .named_types
                .lookup(t)
                .is_some_and(|m| (m.has_method)(name)),
            Value::Time(_) => time_has_method(name),
            _ => false,
        }
    }
}

impl ExecHelper for TemplateExecHelper {
    // Go: tpl/tplimpl/template_funcs.go:Init
    /// Dependency tracking in watch mode only: a no-op for a one-shot build.
    fn init(&self, _ctx: HostCtx<'_>, _template_name: &str) {}

    // Go: tpl/tplimpl/template_funcs.go:GetFunc
    fn get_func(&self, _ctx: HostCtx<'_>, name: &str) -> Option<TplFunc> {
        self.funcs.get(name).cloned()
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMethod (membership)
    fn has_method(&self, _ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        if let Some(site) = self.main_sections_site(receiver, name) {
            // Moved to site.MainSections in Hugo 0.112.0.
            return self.method_by_name_exists(&site, "MainSections");
        }
        self.method_by_name_exists(receiver, name)
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMethod (+ evalCall)
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> go_value::Result<Value> {
        let (receiver, name): (Cow<'_, Value>, &str) = match self.main_sections_site(receiver, name)
        {
            // Moved to site.MainSections in Hugo 0.112.0.
            Some(site) => (Cow::Owned(site), "MainSections"),
            None => (Cow::Borrowed(receiver), name),
        };
        let not_found = || go_value::Error::new(format!("method {name} not found"));
        match &*receiver {
            Value::Object(o) => o
                .call_method(ctx, name, args)
                .unwrap_or_else(|| Err(not_found())),
            Value::List(_) | Value::Map(_) => self
                .named_types
                .call(ctx, &receiver, name, args)
                .unwrap_or_else(|| Err(not_found())),
            Value::TypedNil(t) => match self.named_types.lookup(t) {
                Some(m) if (m.has_method)(name) => {
                    (m.call)(ctx, &receiver, name, args).unwrap_or_else(|| Err(not_found()))
                }
                _ => Err(not_found()),
            },
            Value::Time(t) => time_call_method(t, name, args).unwrap_or_else(|| Err(not_found())),
            _ => Err(not_found()),
        }
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMapValue
    fn get_map_value(&self, _ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        let key = key.as_go_string()?;
        match receiver {
            Value::Map(m) if m.ty == MapType::Params => {
                // Case insensitive.
                let keystr = go_unicode::strings::to_lower(key.as_bytes());
                match m.get(&keystr) {
                    // Go: `reflect.ValueOf(nil)` is the invalid value (a missing key for the
                    // engine).
                    Some(Value::Invalid) | None => None,
                    Some(v) => Some(v.clone()),
                }
            }
            Value::Map(m) => m.get(key.as_bytes()).cloned(),
            Value::Object(o) if o.kind() == Kind::Map => o.map_get(key.as_bytes()),
            _ => None,
        }
    }

    // Go: tpl/tplimpl/template_funcs.go:OnCalled
    /// Dependency tracking in watch mode only: a no-op for a one-shot build.
    fn on_called(&self, _ctx: HostCtx<'_>, _name: &str, _args: &[Value], _result: &Value) {}

    /// Go: `hreflect.IsTruthfulValue` (common/hreflect/helpers.go:96-130), used by the fork's
    /// `isTrue` (texttemplate/hugo_template.go:434-436).
    // Go: common/hreflect/helpers.go:IsTruthfulValue
    fn is_true(&self, v: &Value) -> bool {
        nh_common::hreflect::is_truthful(v)
    }
}

// ---------------------------------------------------------------------------
// time.Time methods

/// The exported methods of `time.Time` with a value receiver (go1.27.1), the method set Go's
/// template engine sees on a (non-addressable) `time.Time`.
const TIME_METHODS: &[&str] = &[
    "Add",
    "AddDate",
    "After",
    "AppendBinary",
    "AppendFormat",
    "AppendText",
    "Before",
    "Clock",
    "Compare",
    "Date",
    "Day",
    "Equal",
    "Format",
    "GoString",
    "GobEncode",
    "Hour",
    "ISOWeek",
    "In",
    "IsDST",
    "IsZero",
    "Local",
    "Location",
    "MarshalBinary",
    "MarshalJSON",
    "MarshalText",
    "Minute",
    "Month",
    "Nanosecond",
    "Round",
    "Second",
    "String",
    "Sub",
    "Truncate",
    "UTC",
    "Unix",
    "UnixMicro",
    "UnixMilli",
    "UnixNano",
    "Weekday",
    "Year",
    "YearDay",
    "Zone",
    "ZoneBounds",
];

/// Methods of `time.Time` reachable from templates (`.Format`, `.IsZero`, `.Year`, `.Unix`,
/// `.UTC`, `.Local`, `.In`, `.Before`, `.After`, `.Equal`, `.AddDate`, `.Month`, `.Day`, ...),
/// implemented over go-time. Used by `has_method`/`call_method` for `Value::Time` receivers.
pub fn time_has_method(name: &str) -> bool {
    TIME_METHODS.contains(&name)
}

/// A `time.Time` argument (Go `validateType` for a `time.Time` parameter).
fn time_arg(a: &[Value], i: usize) -> GoResult<Time> {
    match a.get(i) {
        Some(Value::Time(t)) => Ok(t.clone()),
        Some(Value::Invalid) => Err(args::invalid_value("time.Time")),
        Some(v) => Err(args::wrong_type("time.Time", v)),
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

/// A `time.Duration` argument: a `time.Duration` value, or an integer literal (Go `evalInteger`
/// converts an ideal constant to the parameter type).
fn duration_arg(a: &[Value], i: usize) -> GoResult<Duration> {
    match a.get(i) {
        Some(Value::Int(n, IntKind::Int)) => Ok(Duration(*n)),
        Some(v @ Value::Object(o)) => match o.as_any().downcast_ref::<DurationValue>() {
            Some(d) => Ok(d.0),
            None => Err(args::wrong_type("time.Duration", v)),
        },
        Some(Value::Invalid) => Err(args::invalid_value("time.Duration")),
        Some(v) => Err(args::wrong_type("time.Duration", v)),
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

/// The unsupported error for methods whose results the value model cannot represent.
fn unsupported(name: &str) -> go_value::Error {
    go_value::Error::new(format!(
        "neohugo-rs: time.Time.{name} is not supported in templates"
    ))
}

/// Calls the `time.Time` method `name`; `None` if there is no such method.
pub fn time_call_method(
    t: &go_value::Time,
    name: &str,
    a: &[Value],
) -> Option<go_value::Result<Value>> {
    let r = match name {
        "Add" => args::exactly(a, 1, name)
            .and_then(|_| duration_arg(a, 0))
            .map(|d| Value::Time(t.add(d))),
        "AddDate" => args::exactly(a, 3, name).and_then(|_| {
            Ok(Value::Time(t.add_date(
                args::int(a, 0)?,
                args::int(a, 1)?,
                args::int(a, 2)?,
            )))
        }),
        "After" => args::exactly(a, 1, name)
            .and_then(|_| time_arg(a, 0))
            .map(|u| Value::Bool(t.go_after(&u))),
        "Before" => args::exactly(a, 1, name)
            .and_then(|_| time_arg(a, 0))
            .map(|u| Value::Bool(t.go_before(&u))),
        "Compare" => args::exactly(a, 1, name)
            .and_then(|_| time_arg(a, 0))
            .map(|u| Value::int(t.compare(&u))),
        "Equal" => args::exactly(a, 1, name)
            .and_then(|_| time_arg(a, 0))
            .map(|u| Value::Bool(t.go_equal(&u))),
        "Day" => args::exactly(a, 0, name).map(|_| Value::int(t.day())),
        "Format" => args::exactly(a, 1, name)
            .and_then(|_| args::string(a, 0))
            .map(|l| Value::string(t.format_bytes(l.as_bytes()))),
        "GoString" => args::exactly(a, 0, name).map(|_| Value::string(t.go_string())),
        "Hour" => args::exactly(a, 0, name).map(|_| Value::int(t.hour())),
        "In" => args::exactly(a, 1, name).and_then(|_| {
            let loc = nh_common::htime::location_arg(a, 0)?;
            Ok(Value::Time(t.in_loc(&loc)))
        }),
        "IsDST" => args::exactly(a, 0, name).map(|_| Value::Bool(t.is_dst())),
        "IsZero" => args::exactly(a, 0, name).map(|_| Value::Bool(t.go_is_zero())),
        "Local" => args::exactly(a, 0, name).map(|_| Value::Time(t.local())),
        "Location" => {
            args::exactly(a, 0, name).map(|_| Value::object(LocationRef(t.go_location())))
        }
        "Minute" => args::exactly(a, 0, name).map(|_| Value::int(t.minute())),
        "Month" => args::exactly(a, 0, name).map(|_| Value::object(MonthValue(t.month()))),
        "Nanosecond" => args::exactly(a, 0, name).map(|_| Value::int(t.nanosecond())),
        "Round" => args::exactly(a, 1, name)
            .and_then(|_| duration_arg(a, 0))
            .map(|d| Value::Time(t.round(d))),
        "Second" => args::exactly(a, 0, name).map(|_| Value::int(t.second())),
        "String" => args::exactly(a, 0, name).map(|_| Value::string(t.string())),
        "Sub" => args::exactly(a, 1, name)
            .and_then(|_| time_arg(a, 0))
            .map(|u| Value::object(DurationValue(t.sub(&u)))),
        "Truncate" => args::exactly(a, 1, name)
            .and_then(|_| duration_arg(a, 0))
            .map(|d| Value::Time(t.truncate(d))),
        "UTC" => args::exactly(a, 0, name).map(|_| Value::Time(t.utc())),
        "Unix" => args::exactly(a, 0, name).map(|_| Value::int64(t.go_unix())),
        "UnixMicro" => args::exactly(a, 0, name).map(|_| Value::int64(t.unix_micro())),
        "UnixMilli" => args::exactly(a, 0, name).map(|_| Value::int64(t.unix_milli())),
        "UnixNano" => args::exactly(a, 0, name).map(|_| Value::int64(t.go_unix_nano())),
        "Weekday" => args::exactly(a, 0, name).map(|_| Value::object(WeekdayValue(t.weekday()))),
        "Year" => args::exactly(a, 0, name).map(|_| Value::int(t.year())),
        "YearDay" => args::exactly(a, 0, name).map(|_| Value::int(t.year_day())),
        // Go: more than one result that is not (T, error).
        "Clock" | "Date" => Err(bad_results_error(name, 3)),
        "ISOWeek" | "Zone" | "ZoneBounds" => Err(bad_results_error(name, 2)),
        // []byte results and []byte arguments.
        "AppendBinary" | "AppendFormat" | "AppendText" | "GobEncode" | "MarshalBinary"
        | "MarshalJSON" | "MarshalText" => Err(unsupported(name)),
        _ => return None,
    };
    Some(r)
}

/// Go `time.Month` (a named `int`): `String()` and the underlying `int`.
#[derive(Clone, Copy, Debug)]
pub struct MonthValue(pub Month);

nh_common::go_methods!(MonthValue {
    "String" => |m, _c, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::string(m.0.string()))
    },
});

impl Object for MonthValue {
    nh_common::object_basics!("time.Month");

    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::int(self.0.0))
    }
}

/// Go `time.Weekday` (a named `int`): `String()` and the underlying `int`.
#[derive(Clone, Copy, Debug)]
pub struct WeekdayValue(pub Weekday);

nh_common::go_methods!(WeekdayValue {
    "String" => |m, _c, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::string(m.0.string()))
    },
});

impl Object for WeekdayValue {
    nh_common::object_basics!("time.Weekday");

    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::int(self.0.0))
    }
}

/// Go `time.Duration` (a named `int64`) with its methods.
#[derive(Clone, Copy, Debug)]
pub struct DurationValue(pub Duration);

nh_common::go_methods!(DurationValue {
    "Abs" => |d, _c, a| {
        args::exactly(a, 0, "Abs")?;
        Ok(Value::object(DurationValue(d.0.abs())))
    },
    "Hours" => |d, _c, a| {
        args::exactly(a, 0, "Hours")?;
        Ok(Value::float64(d.0.hours()))
    },
    "Microseconds" => |d, _c, a| {
        args::exactly(a, 0, "Microseconds")?;
        Ok(Value::int64(d.0.microseconds()))
    },
    "Milliseconds" => |d, _c, a| {
        args::exactly(a, 0, "Milliseconds")?;
        Ok(Value::int64(d.0.milliseconds()))
    },
    "Minutes" => |d, _c, a| {
        args::exactly(a, 0, "Minutes")?;
        Ok(Value::float64(d.0.minutes()))
    },
    "Nanoseconds" => |d, _c, a| {
        args::exactly(a, 0, "Nanoseconds")?;
        Ok(Value::int64(d.0.nanoseconds()))
    },
    "Round" => |d, _c, a| {
        args::exactly(a, 1, "Round")?;
        Ok(Value::object(DurationValue(d.0.round(duration_arg(a, 0)?))))
    },
    "Seconds" => |d, _c, a| {
        args::exactly(a, 0, "Seconds")?;
        Ok(Value::float64(d.0.seconds()))
    },
    "String" => |d, _c, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::string(d.0.string()))
    },
    "Truncate" => |d, _c, a| {
        args::exactly(a, 1, "Truncate")?;
        Ok(Value::object(DurationValue(d.0.truncate(duration_arg(a, 0)?))))
    },
});

impl Object for DurationValue {
    nh_common::object_basics!("time.Duration");

    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::int64(self.0.0))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/template_funcs.go (175 lines; 5/6 funcs executed)
//   types: templateExecHelper
// OK L43-58: (t *templateExecHelper) GetFunc(ctx context.Context, tmpl texttemplate.Preparer, name string) (fn reflect.Value, firstArg reflect.Value, found bool)
// OK L60-68: (t *templateExecHelper) Init(ctx context.Context, tmpl texttemplate.Preparer) (watch mode only: no-op)
// OK L70-84: (t *templateExecHelper) GetMapValue(ctx context.Context, tmpl texttemplate.Preparer, receiver, key reflect.Value) (reflect.Value, bool)
// OK L88-114: (t *templateExecHelper) GetMethod(ctx context.Context, tmpl texttemplate.Preparer, receiver reflect.Value, name string) (method reflect.Value, firs...
// OK L116-138: (t *templateExecHelper) OnCalled(ctx context.Context, tmpl texttemplate.Preparer, name string, args []reflect.Value, result reflect.Value) (watch mode only: no-op)
// OK L140-175: (t *templateExecHelper) trackDependencies(ctx context.Context, tmpl texttemplate.Preparer, name string, receiver reflect.Value) context.Context (watch mode only: not ported)
// ---------------------------------------------------------------------------
