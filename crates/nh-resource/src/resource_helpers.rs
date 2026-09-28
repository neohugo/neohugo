//! Port of `resources/resource/resource_helpers.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use go_value::{GoString, IntKind, Map, MapType, SliceType, Value};

use crate::resourcetypes::Resource;

/// Go: `resource.GetParam(r, key)` — nil (`Invalid`) if not found.
// Go: resources/resource/resource_helpers.go:GetParam
pub fn get_param_of(r: &dyn Resource, key: &str) -> Value {
    get_param(&r.params(), key, false)
}

/// Go: `resource.GetParamToLower(r, key)` — like [`get_param_of`], lower-casing string results.
// Go: resources/resource/resource_helpers.go:GetParamToLower
pub fn get_param_to_lower(r: &dyn Resource, key: &str) -> Value {
    get_param(&r.params(), key, true)
}

/// Go: `resource.GetParam(r, key)` / `GetParamToLower` (typed conversions of front-matter params:
/// time -> time, bool, int, float, []string lower-cased when asked ...). `params` is `r.Params()`.
// Go: resources/resource/resource_helpers.go:getParam
pub fn get_param(params: &Map, key: &str, string_to_lower: bool) -> Value {
    let v = match nh_common::maps::params::get_nested_param(key.as_bytes(), b".", &[params]) {
        Ok(v) => v,
        Err(_) => return Value::Invalid,
    };

    match &v {
        Value::Invalid => Value::Invalid,
        Value::Bool(_) => v,
        Value::String(s) => {
            if string_to_lower {
                Value::String(GoString::from(
                    go_unicode::strings::to_lower(s.as_bytes()).into_owned(),
                ))
            } else {
                v
            }
        }
        Value::Int(
            _,
            IntKind::Int64 | IntKind::Int32 | IntKind::Int16 | IntKind::Int8 | IntKind::Int,
        ) => Value::int(nh_common::cast::caste::to_int(&v)),
        Value::Float(..) => Value::float64(nh_common::cast::caste::to_float64(&v)),
        Value::Time(_) => v,
        // go-toml's LocalDate / LocalDateTime: `val.AsTime(time.UTC)` (not LocalTime).
        Value::Object(o) if matches!(&*o.type_name(), "toml.LocalDate" | "toml.LocalDateTime") => {
            let arg = Value::object(nh_common::htime::LocationRef(go_time::utc()));
            match o.call_method(&(), "AsTime", &[arg]) {
                Some(Ok(t @ Value::Time(_))) => t,
                _ => Value::Invalid,
            }
        }
        Value::List(l) if l.ty == SliceType::String => {
            if string_to_lower {
                Value::list(
                    SliceType::String,
                    l.items
                        .iter()
                        .map(|x| match x {
                            Value::String(s) => Value::String(GoString::from(
                                go_unicode::strings::to_lower(s.as_bytes()).into_owned(),
                            )),
                            other => other.clone(),
                        })
                        .collect(),
                )
            } else {
                v
            }
        }
        // A nil []string or map[string]any (Go's `v == nil` is false for a typed nil).
        Value::TypedNil(t) if matches!(&**t, "[]string" | "map[string]interface {}") => v,
        // map[string]any (Go does not list maps.Params, which is a distinct named type).
        Value::Map(m) if m.ty == MapType::StringAny => v,
        _ => Value::Invalid,
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/resource_helpers.go (76 lines; 0/3 funcs executed)
// OK L30-32: GetParam(r Resource, key string) any
// OK L36-38: GetParamToLower(r Resource, key string) any
// OK L40-76: getParam(r Resource, key string, stringToLower bool) any
// ---------------------------------------------------------------------------
