//! Differential tests for `nh_common::cast` (spf13/cast) against `tests/fixtures/cast/cast.json`,
//! written by `tools/go-oracle/nh-common/cast` built for arm64 (out-of-range float to integer
//! conversions differ on amd64).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, Location, Map, MapType, Object, SafeKind,
    SliceType, Time, UintKind, Value,
};
use nh_common::cast::caste as cast;
use nh_common::cast::time as ctime;
use serde_json::{Value as J, json};

fn fixture() -> J {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cast/cast.json");
    let b = std::fs::read(path).expect("read fixture");
    serde_json::from_slice(&b).expect("parse fixture")
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

/// A fixture string: a JSON string, or `{"hex": ...}` for invalid UTF-8.
fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().expect("hex")),
        other => panic!("not a string: {other}"),
    }
}

/// The encoding of a Go string (corpus.Encode).
fn enc_str(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": to_hex(b)}),
    }
}

fn fbits(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn from_fbits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

// ---------------------------------------------------------------------------
// Host objects standing in for the oracle's Go types

/// time.Duration, time.Month, time.Weekday and the oracle's named basic types.
struct Named {
    name: String,
    under: Value,
    string: Option<String>,
}

impl Object for Named {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Owned(self.name.clone())
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        name == "String" && self.string.is_some()
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        _: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if name == "String" {
            return self.string.as_ref().map(|s| Ok(Value::string(s.as_str())));
        }
        None
    }
    fn go_string(&self) -> Option<GoString> {
        self.string.as_ref().map(|s| GoString::from(s.as_str()))
    }
    fn underlying(&self) -> Option<Value> {
        Some(self.under.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// main.stringer, main.myErr, main.f64p, main.f64ep.
struct Struct {
    name: &'static str,
    fields: Vec<(&'static str, Value)>,
    stringer: Option<String>,
    error: Option<String>,
    /// `Float64()`: Some(Ok(f)) or Some(Err(())) for the error provider.
    float64: Option<Result<f64, ()>>,
}

impl Object for Struct {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name)
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        match name {
            "String" => self.stringer.is_some(),
            "Error" => self.error.is_some(),
            "Float64" => self.float64.is_some(),
            _ => false,
        }
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        _: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "String" => self
                .stringer
                .as_ref()
                .map(|s| Ok(Value::string(s.as_str()))),
            "Error" => self.error.as_ref().map(|s| Ok(Value::string(s.as_str()))),
            "Float64" => self.float64.map(|r| match r {
                Ok(f) => Ok(Value::float64(f)),
                Err(()) => Err(go_value::Error::new("no float")),
            }),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        self.stringer.as_ref().map(|s| GoString::from(s.as_str()))
    }
    fn go_error(&self) -> Option<String> {
        self.error.clone()
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.fields
                .iter()
                .map(|(n, v)| (Cow::Borrowed(*n), v.clone()))
                .collect(),
        )
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn int_kind(t: &str) -> Option<IntKind> {
    Some(match t {
        "int" => IntKind::Int,
        "int8" => IntKind::Int8,
        "int16" => IntKind::Int16,
        "int32" => IntKind::Int32,
        "int64" => IntKind::Int64,
        _ => return None,
    })
}

fn uint_kind(t: &str) -> Option<UintKind> {
    Some(match t {
        "uint" => UintKind::Uint,
        "uint8" => UintKind::Uint8,
        "uint16" => UintKind::Uint16,
        "uint32" => UintKind::Uint32,
        "uint64" => UintKind::Uint64,
        "uintptr" => UintKind::Uintptr,
        _ => return None,
    })
}

fn safe_kind(t: &str) -> Option<SafeKind> {
    Some(match t {
        "template.HTML" => SafeKind::Html,
        "template.URL" => SafeKind::Url,
        "template.JS" => SafeKind::Js,
        "template.CSS" => SafeKind::Css,
        "template.HTMLAttr" => SafeKind::HtmlAttr,
        "template.JSStr" => SafeKind::JsStr,
        "template.Srcset" => SafeKind::Srcset,
        _ => return None,
    })
}

fn slice_type(t: &str) -> SliceType {
    match t {
        "[]interface {}" => SliceType::Any,
        "[]string" => SliceType::String,
        "[]int" => SliceType::Int,
        "[]int64" => SliceType::Int64,
        "[]float64" => SliceType::Float64,
        "[]bool" => SliceType::Bool,
        "[]uint8" => SliceType::Uint8,
        "[]map[string]interface {}" => SliceType::MapStringAny,
        other => SliceType::Named(Arc::from(other)),
    }
}

fn map_type(t: &str) -> MapType {
    match t {
        "map[string]interface {}" => MapType::StringAny,
        "map[string]string" => MapType::StringString,
        "maps.Params" => MapType::Params,
        other => MapType::Named(Arc::from(other)),
    }
}

fn named(name: &str, under: Value, string: Option<String>) -> Value {
    Value::object(Named {
        name: name.to_string(),
        under,
        string,
    })
}

/// Builds the Go input value the oracle described.
fn decode(v: &J, zones: &[Arc<Location>]) -> Value {
    let t = v["t"].as_str().unwrap();
    let int = |v: &J| v["v"].as_str().unwrap().parse::<i64>().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    if let Some(k) = int_kind(t) {
        return Value::Int(int(v), k);
    }
    if let Some(k) = uint_kind(t) {
        return Value::Uint(v["v"].as_str().unwrap().parse::<u64>().unwrap(), k);
    }
    if let Some(k) = safe_kind(t) {
        return Value::Safe(k, GoString::from(v["s"].as_str().unwrap()));
    }
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "float64" => Value::Float(from_fbits(v["v"].as_str().unwrap()), FloatKind::F64),
        "float32" => Value::Float(from_fbits(v["v"].as_str().unwrap()), FloatKind::F32),
        "string" => Value::String(GoString::from(bytes(&v["s"]))),
        "json.Number" => Value::object(go_json::Number(GoString::from(v["s"].as_str().unwrap()))),
        "time.Time" => {
            let zone = &zones[v["zone"].as_u64().unwrap() as usize];
            Value::Time(
                go_time::unix(v["unix"].as_i64().unwrap(), v["nsec"].as_i64().unwrap())
                    .in_loc(zone),
            )
        }
        "time.Duration" => named(
            "time.Duration",
            Value::Int(int(v), IntKind::Int64),
            Some(go_time::Duration(int(v)).string()),
        ),
        "time.Month" => named(
            "time.Month",
            Value::int(int(v)),
            Some(go_time::Month(int(v)).string()),
        ),
        "time.Weekday" => named(
            "time.Weekday",
            Value::int(int(v)),
            Some(go_time::Weekday(int(v)).string()),
        ),
        "[]uint8" => Value::list(
            SliceType::Uint8,
            hex(v["hex"].as_str().unwrap())
                .into_iter()
                .map(|b| Value::Uint(b as u64, UintKind::Uint8))
                .collect(),
        ),
        "named" => named(
            v["name"].as_str().unwrap(),
            decode(&v["under"], zones),
            None,
        ),
        "stringer" => Value::object(Struct {
            name: "main.stringer",
            fields: vec![("s", Value::string(v["s"].as_str().unwrap()))],
            stringer: Some(v["s"].as_str().unwrap().to_string()),
            error: None,
            float64: None,
        }),
        "error" => Value::object(Struct {
            name: "main.myErr",
            fields: vec![("msg", Value::string(v["s"].as_str().unwrap()))],
            stringer: None,
            error: Some(v["s"].as_str().unwrap().to_string()),
            float64: None,
        }),
        "f64p" => {
            let f = from_fbits(v["v"].as_str().unwrap());
            Value::object(Struct {
                name: "main.f64p",
                fields: vec![("v", Value::float64(f))],
                stringer: None,
                error: None,
                float64: Some(Ok(f)),
            })
        }
        "f64ep" => {
            let f = from_fbits(v["v"].as_str().unwrap());
            let fail = v["fail"].as_bool().unwrap();
            Value::object(Struct {
                name: "main.f64ep",
                fields: vec![("v", Value::float64(f)), ("fail", Value::Bool(fail))],
                stringer: None,
                error: None,
                float64: Some(if fail { Err(()) } else { Ok(f) }),
            })
        }
        _ if t.starts_with("[]") => Value::list(
            slice_type(t),
            v["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| decode(i, zones))
                .collect(),
        ),
        _ if t.starts_with("map[") || t == "maps.Params" => {
            let mut m = Map::new(map_type(t));
            for e in v["entries"].as_array().unwrap() {
                m.insert(GoString::from(bytes(&e[0])), decode(&e[1], zones));
            }
            Value::map(m)
        }
        other => panic!("unknown input type {other}"),
    }
}

// ---------------------------------------------------------------------------
// Result encoding (the oracle's encode)

fn enc_time(t: &Time, zones: &[Arc<Location>]) -> J {
    let l = t.go_location();
    let zone = zones
        .iter()
        .position(|z| Arc::ptr_eq(z, &l) || (go_time::is_utc_loc(z) && go_time::is_utc_loc(&l)))
        .map(|i| json!(i))
        .unwrap_or_else(|| {
            let (abbr, off) = t.zone();
            json!({"name": go_time::location_string(Some(&l)), "abbr": abbr, "offset": off})
        });
    json!({"t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(), "zone": zone, "str": t.string()})
}

fn enc(v: &Value, zones: &[Arc<Location>]) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(n, k) => json!({"t": k.go_name(), "v": n.to_string()}),
        Value::Uint(n, k) => json!({"t": k.go_name(), "v": n.to_string()}),
        Value::Float(f, k) => json!({"t": k.go_name(), "v": fbits(*f)}),
        Value::String(s) => json!({"t": "string", "s": enc_str(s)}),
        Value::Safe(k, s) => json!({"t": k.go_name(), "s": s.to_string()}),
        Value::Time(t) => enc_time(t, zones),
        Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
        Value::List(l) if l.ty == SliceType::Uint8 => {
            let b: Vec<u8> = l
                .items
                .iter()
                .map(|i| match i {
                    Value::Uint(b, _) => *b as u8,
                    _ => 0,
                })
                .collect();
            json!({"t": "[]uint8", "hex": to_hex(&b)})
        }
        Value::List(l) => {
            json!({"t": l.ty.go_name(), "items": l.items.iter().map(|i| enc(i, zones)).collect::<Vec<_>>()})
        }
        Value::Map(m) => enc_map(m, zones),
        Value::Object(o) => {
            if let Some(n) = o.as_any().downcast_ref::<go_json::Number>() {
                return json!({"t": "json.Number", "s": n.0.to_string()});
            }
            if let Some(n) = o.as_any().downcast_ref::<Named>() {
                return match n.name.as_str() {
                    "time.Duration" | "time.Month" | "time.Weekday" => match &n.under {
                        Value::Int(i, _) => json!({"t": n.name, "v": i.to_string()}),
                        _ => unreachable!(),
                    },
                    _ => json!({"t": "named", "name": n.name, "under": enc(&n.under, zones)}),
                };
            }
            if let Some(s) = o.as_any().downcast_ref::<Struct>() {
                let f = |i: usize| &s.fields[i].1;
                return match s.name {
                    "main.stringer" => {
                        json!({"t": "stringer", "s": f(0).as_go_string().unwrap().to_string()})
                    }
                    "main.myErr" => {
                        json!({"t": "error", "s": f(0).as_go_string().unwrap().to_string()})
                    }
                    "main.f64p" => match f(0) {
                        Value::Float(x, _) => json!({"t": "f64p", "v": fbits(*x)}),
                        _ => unreachable!(),
                    },
                    _ => match (f(0), f(1)) {
                        (Value::Float(x, _), Value::Bool(b)) => {
                            json!({"t": "f64ep", "v": fbits(*x), "fail": b})
                        }
                        _ => unreachable!(),
                    },
                };
            }
            panic!("cannot encode object {}", o.type_name())
        }
    }
}

fn enc_map(m: &Map, zones: &[Arc<Location>]) -> J {
    let entries: Vec<J> = m
        .entries
        .iter()
        .map(|(k, v)| json!([enc_str(k), enc(v, zones)]))
        .collect();
    json!({"t": m.ty.go_name(), "entries": entries})
}

fn enc_list<T>(t: &str, items: &[T], f: impl Fn(&T) -> J) -> J {
    json!({"t": t, "items": items.iter().map(f).collect::<Vec<_>>()})
}

fn num(t: &str, v: impl ToString) -> J {
    json!({"t": t, "v": v.to_string()})
}

fn dur(d: go_time::Duration) -> J {
    num("time.Duration", d.0)
}

/// `{"ok": encode(v)}` / `{"err": msg}`.
fn res<T>(r: nh_common::Result<T>, f: impl Fn(&T) -> J) -> J {
    match r {
        Ok(v) => json!({"ok": f(&v)}),
        Err(e) => json!({"err": e.message()}),
    }
}

/// A nil slice or map encodes as `{"t": "nil:<type>"}` in Go; Rust results do not distinguish
/// nil from empty, so compare both as empty.
fn normalize(j: &J) -> J {
    match j {
        J::Object(o) => {
            if let Some(t) = o.get("t").and_then(J::as_str) {
                if let Some(ty) = t.strip_prefix("nil:") {
                    if ty.starts_with("[]") {
                        return json!({"t": ty, "items": []});
                    }
                    if ty.starts_with("map[") || ty == "maps.Params" {
                        return json!({"t": ty, "entries": []});
                    }
                }
            }
            J::Object(o.iter().map(|(k, v)| (k.clone(), normalize(v))).collect())
        }
        J::Array(a) => J::Array(a.iter().map(normalize).collect()),
        other => other.clone(),
    }
}

fn zones(f: &J) -> Vec<Arc<Location>> {
    f["zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|z| {
            let name = z["name"].as_str().unwrap();
            match z["kind"].as_str().unwrap() {
                "utc" => go_time::utc(),
                "fixed" => go_time::fixed_zone(name, z["offset"].as_i64().unwrap()),
                "tzdata" => {
                    go_time::load_location_from_tz_data(name, &hex(z["tzdata"].as_str().unwrap()))
                        .unwrap()
                }
                k => panic!("zone kind {k}"),
            }
        })
        .collect()
}

/// Runs the Rust port of the oracle function `name`.
fn run(name: &str, v: &Value, zones: &[Arc<Location>]) -> J {
    let s = |s: &GoString| json!({"t": "string", "s": enc_str(s)});
    match name {
        "ToStringE" => res(cast::to_string_e(v), s),
        "ToBoolE" => res(cast::to_bool_e(v), |b| json!({"t": "bool", "v": b})),
        "ToIntE" => res(cast::to_int_e(v), |n| num("int", n)),
        "ToInt8E" => res(cast::to_int8_e(v), |n| num("int8", n)),
        "ToInt16E" => res(cast::to_int16_e(v), |n| num("int16", n)),
        "ToInt32E" => res(cast::to_int32_e(v), |n| num("int32", n)),
        "ToInt64E" => res(cast::to_int64_e(v), |n| num("int64", n)),
        "ToUintE" => res(cast::to_uint_e(v), |n| num("uint", n)),
        "ToUint8E" => res(cast::to_uint8_e(v), |n| num("uint8", n)),
        "ToUint16E" => res(cast::to_uint16_e(v), |n| num("uint16", n)),
        "ToUint32E" => res(cast::to_uint32_e(v), |n| num("uint32", n)),
        "ToUint64E" => res(cast::to_uint64_e(v), |n| num("uint64", n)),
        "ToFloat32E" => res(
            cast::to_float32_e(v),
            |f| json!({"t": "float32", "v": fbits(*f as f64)}),
        ),
        "ToFloat64E" => res(
            cast::to_float64_e(v),
            |f| json!({"t": "float64", "v": fbits(*f)}),
        ),
        "ToDurationE" => res(cast::to_duration_e(v), |d| dur(*d)),
        "ToTimeE" => res(ctime::to_time_e(v), |t| enc_time(t, zones)),
        "ToTimeInDefaultLocationE:1" => {
            res(ctime::to_time_in_default_location_e(v, &zones[1]), |t| {
                enc_time(t, zones)
            })
        }
        "ToTimeInDefaultLocationE:3" => {
            res(ctime::to_time_in_default_location_e(v, &zones[3]), |t| {
                enc_time(t, zones)
            })
        }
        "ToSliceE" => res(cast::to_slice_e(v), |l| {
            enc_list("[]interface {}", l, |i| enc(i, zones))
        }),
        "ToStringSliceE" => res(cast::to_string_slice_e(v), |l| enc_list("[]string", l, s)),
        "ToIntSliceE" => res(cast::to_int_slice_e(v), |l| {
            enc_list("[]int", l, |n| num("int", n))
        }),
        "ToInt64SliceE" => res(cast::to_int64_slice_e(v), |l| {
            enc_list("[]int64", l, |n| num("int64", n))
        }),
        "ToUintSliceE" => res(cast::to_uint_slice_e(v), |l| {
            enc_list("[]uint", l, |n| num("uint", n))
        }),
        "ToFloat64SliceE" => res(cast::to_float64_slice_e(v), |l| {
            enc_list("[]float64", l, |f| json!({"t": "float64", "v": fbits(*f)}))
        }),
        "ToBoolSliceE" => res(cast::to_bool_slice_e(v), |l| {
            enc_list("[]bool", l, |b| json!({"t": "bool", "v": b}))
        }),
        "ToDurationSliceE" => res(cast::to_duration_slice_e(v), |l| {
            enc_list("[]time.Duration", l, |d| dur(*d))
        }),
        "ToStringMapE" => res(cast::to_string_map_e(v), |m| enc_map(m, zones)),
        "ToStringMapStringE" => res(cast::to_string_map_string_e(v), |m| enc_map(m, zones)),
        "ToStringMapBoolE" => res(cast::to_string_map_bool_e(v), |m| enc_map(m, zones)),
        other => panic!("unknown function {other}"),
    }
}

#[test]
fn cast_matches_go() {
    // Go oracle: time.Local = time.UTC.
    go_time::set_local(go_time::utc());
    let f = fixture();
    assert_eq!(
        f["goarch"], "arm64",
        "the fixture must come from an arm64 oracle build"
    );
    let zones = zones(&f);
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 500, "too few inputs: {}", cases.len());

    let mut bad = Vec::new();
    let mut n = 0;
    for c in cases {
        let input = decode(&c["in"], &zones);
        for (name, want) in c.as_object().unwrap() {
            if name == "in" {
                continue;
            }
            n += 1;
            let got = normalize(&run(name, &input, &zones));
            // A Go panic (recovered by text/template's safeCall) is an error in the port.
            let want = match want.get("panic") {
                Some(p) => json!({"err": p}),
                None => normalize(want),
            };
            if got == want {
                continue;
            }
            bad.push(format!(
                "{name}({}):\n   got  {got}\n   want {want}",
                c["in"]
            ));
        }
    }
    assert!(n > 15000, "too few results: {n}");
    assert!(
        bad.is_empty(),
        "{} of {n} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

/// The generic `ToE[T]`/`ToNumberE[T]` and the non-E wrappers agree with the typed functions
/// (they share Go's code paths).
#[test]
fn generic_and_wrappers_agree() {
    use cast::{Basic, BasicType, NumType};

    go_time::set_local(go_time::utc());
    let f = fixture();
    let zones = zones(&f);
    let msg = |e: nh_common::Error| e.message().to_string();
    for c in f["cases"].as_array().unwrap() {
        let v = decode(&c["in"], &zones);
        let num = |t: NumType| cast::to_number_e(&v, t).map_err(msg);
        let basic = |t: BasicType| match cast::to_e(&v, t).map_err(msg) {
            Ok(Basic::Number(n)) => Ok(n.to_value(match t {
                BasicType::Number(t) => t,
                _ => unreachable!(),
            })),
            Ok(Basic::String(s)) => Ok(Value::String(s)),
            Ok(Basic::Bool(b)) => Ok(Value::Bool(b)),
            Ok(Basic::Time(t)) => Ok(Value::Time(t)),
            Ok(Basic::Duration(d)) => Ok(Value::int64(d.0)),
            Err(e) => Err(e),
        };
        type R = Result<Value, String>;
        let checks: Vec<(&str, R, R)> = vec![
            (
                "int",
                num(NumType::INT),
                cast::to_int_e(&v).map(Value::int).map_err(msg),
            ),
            (
                "int8",
                num(NumType::INT8),
                cast::to_int8_e(&v)
                    .map(|n| Value::Int(n as i64, IntKind::Int8))
                    .map_err(msg),
            ),
            (
                "uint16",
                num(NumType::UINT16),
                cast::to_uint16_e(&v)
                    .map(|n| Value::Uint(n as u64, UintKind::Uint16))
                    .map_err(msg),
            ),
            (
                "uint64",
                num(NumType::UINT64),
                cast::to_uint64_e(&v)
                    .map(|n| Value::Uint(n, UintKind::Uint64))
                    .map_err(msg),
            ),
            (
                "float32",
                num(NumType::FLOAT32),
                cast::to_float32_e(&v)
                    .map(|n| Value::Float(n as f64, FloatKind::F32))
                    .map_err(msg),
            ),
            (
                "float64",
                basic(BasicType::Number(NumType::FLOAT64)),
                cast::to_float64_e(&v).map(Value::float64).map_err(msg),
            ),
            (
                "string",
                basic(BasicType::String),
                cast::to_string_e(&v).map(Value::String).map_err(msg),
            ),
            (
                "bool",
                basic(BasicType::Bool),
                cast::to_bool_e(&v).map(Value::Bool).map_err(msg),
            ),
            (
                "duration",
                basic(BasicType::Duration),
                cast::to_duration_e(&v)
                    .map(|d| Value::int64(d.0))
                    .map_err(msg),
            ),
        ];
        for (name, a, b) in checks {
            assert_eq!(a, b, "{name} {}", c["in"]);
        }
        assert_eq!(
            cast::to_string(&v),
            cast::to_string_e(&v).unwrap_or_default()
        );
        assert_eq!(cast::to_int(&v), cast::to_int_e(&v).unwrap_or(0));
        assert_eq!(cast::to_bool(&v), cast::to_bool_e(&v).unwrap_or(false));
        assert_eq!(
            cast::to_string_slice(&v),
            cast::to_string_slice_e(&v).unwrap_or_default()
        );
        assert_eq!(
            cast::to_int_slice(&v),
            cast::to_int_slice_e(&v).unwrap_or_default()
        );
    }
}
