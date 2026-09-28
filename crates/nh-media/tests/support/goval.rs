//! The typed JSON format of `tools/go-oracle/nh-common/goval` (decode into `go_value::Value`,
//! encode results back), copied from nh-config's T04 test support.

#![allow(dead_code)]

use std::any::Any;
use std::borrow::Cow;
use std::io::Read;
use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, List, Map, Object, SafeKind, UintKind, Value,
};
use nh_common::hreflect::{map_type_from_name, slice_type_from_name};
use nh_common::maps::params::ParamsMergeStrategy;
use nh_common::types::hstring::Html;
use serde_json::{Value as J, json};

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture(rel: &str) -> J {
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let text = if rel.ends_with(".gz") {
        let mut s = String::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_string(&mut s)
            .unwrap();
        s
    } else {
        String::from_utf8(raw).unwrap()
    };
    serde_json::from_str(&text).unwrap()
}

/// A named basic type of the oracle (`time.Duration`, `time.Month`, ...), printed through its
/// underlying value.
pub struct NamedBasic {
    pub name: String,
    pub under: Value,
}

impl Object for NamedBasic {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Owned(self.name.clone())
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<go_value::Result<Value>> {
        None
    }
    fn underlying(&self) -> Option<Value> {
        Some(self.under.clone())
    }
    fn go_string(&self) -> Option<GoString> {
        // The fmt.Stringer of the time package's named types.
        let i = match &self.under {
            Value::Int(i, _) => *i,
            _ => return None,
        };
        Some(GoString::from(match self.name.as_str() {
            "time.Duration" => go_time::Duration(i).string(),
            "time.Month" => go_time::Month(i).string(),
            "time.Weekday" => go_time::Weekday(i).string(),
            _ => return None,
        }))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// go-toml's `LocalDate`/`LocalTime`/`LocalDateTime` as the oracle encodes them (type + String()).
pub struct TomlLocal {
    pub ty: String,
    pub s: String,
}

impl Object for TomlLocal {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Owned(self.ty.clone())
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "AsTime" | "String")
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        _: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "String" => Some(Ok(Value::string(self.s.as_str()))),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.s.as_str()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Decoding

pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A goval string (`"s"` or `{"hex": ...}`) as bytes.
pub fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().unwrap()),
        _ => panic!("bad string {v}"),
    }
}

pub fn gostr(v: &J) -> GoString {
    GoString::from(bytes(v))
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
        "template.HTMLAttr" => SafeKind::HtmlAttr,
        "template.CSS" => SafeKind::Css,
        "template.JS" => SafeKind::Js,
        "template.JSStr" => SafeKind::JsStr,
        "template.URL" => SafeKind::Url,
        "template.Srcset" => SafeKind::Srcset,
        _ => return None,
    })
}

pub fn fbits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

pub fn location(name: &str, abbr: &str, off: i64) -> Arc<go_value::Location> {
    if name == "UTC" && abbr == "UTC" && off == 0 {
        go_time::utc()
    } else {
        assert_eq!(name, abbr, "only fixed zones are decoded");
        go_time::fixed_zone(abbr, off)
    }
}

/// Decodes a goval-encoded value.
pub fn decode(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    if let Some(k) = int_kind(t) {
        return Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    if let Some(k) = uint_kind(t) {
        return Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    if let Some(k) = safe_kind(t) {
        return Value::Safe(k, gostr(&v["s"]));
    }
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "float64" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F64),
        "float32" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F32),
        "string" => Value::String(gostr(&v["s"])),
        "time.Time" => {
            let loc = location(
                v["loc"].as_str().unwrap(),
                v["abbr"].as_str().unwrap(),
                v["off"].as_i64().unwrap(),
            );
            Value::Time(
                go_time::unix(v["unix"].as_i64().unwrap(), v["nsec"].as_i64().unwrap())
                    .in_loc(&loc),
            )
        }
        "named" => {
            let name = v["name"].as_str().unwrap();
            let under = decode(&v["under"]);
            match name {
                "hstring.HTML" => Value::object(Html(under.as_go_string().unwrap().clone())),
                "maps.ParamsMergeStrategy" => Value::object(
                    ParamsMergeStrategy::from_str_opt(under.as_go_string().unwrap()).unwrap(),
                ),
                _ => Value::object(NamedBasic {
                    name: name.to_string(),
                    under,
                }),
            }
        }
        _ if t.starts_with("toml.") => Value::object(TomlLocal {
            ty: t.to_string(),
            s: v["s"].as_str().unwrap().to_string(),
        }),
        _ if v.get("items").is_some() => Value::List(Arc::new(List::new(
            slice_type_from_name(t),
            v["items"].as_array().unwrap().iter().map(decode).collect(),
        ))),
        _ if v.get("entries").is_some() => {
            let mut m = Map::new(map_type_from_name(t));
            for e in v["entries"].as_array().unwrap() {
                m.insert(gostr(&e[0]), decode(&e[1]));
            }
            Value::map(m)
        }
        _ => panic!("cannot decode {v}"),
    }
}

/// Decodes a goval-encoded map.
pub fn decode_map(v: &J) -> Map {
    match decode(v) {
        Value::Map(m) => (*m).clone(),
        other => panic!("not a map: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Encoding

/// A string as goval encodes it.
pub fn str_enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": b.iter().map(|c| format!("{c:02x}")).collect::<String>()}),
    }
}

fn fbits_enc(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

/// Encodes a value like goval.Encode.
pub fn encode(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(i, k) => json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(u, k) => json!({"t": k.go_name(), "v": u.to_string()}),
        Value::Float(f, FloatKind::F64) => json!({"t": "float64", "v": fbits_enc(*f)}),
        Value::Float(f, FloatKind::F32) => json!({"t": "float32", "v": fbits_enc(*f)}),
        Value::String(s) => json!({"t": "string", "s": str_enc(s)}),
        Value::Safe(k, s) => json!({"t": k.go_name(), "s": str_enc(s)}),
        Value::Time(t) => {
            let (abbr, off) = t.zone();
            json!({
                "t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(),
                "loc": go_time::location_string(t.loc.as_ref()), "abbr": abbr, "off": off,
            })
        }
        Value::List(l) => {
            json!({"t": l.ty.go_name(), "items": l.items.iter().map(encode).collect::<Vec<_>>()})
        }
        Value::Map(m) => json!({
            "t": m.ty.go_name(),
            "entries": m.entries.iter().map(|(k, v)| json!([str_enc(k), encode(v)])).collect::<Vec<_>>(),
        }),
        Value::Object(o) => {
            if let Some(t) = v.downcast::<TomlLocal>() {
                return json!({"t": t.ty, "s": t.s});
            }
            if let Some(u) = o.underlying() {
                return json!({"t": "named", "name": o.type_name(), "under": encode(&u)});
            }
            panic!("cannot encode {v:?}")
        }
    }
}

/// Encodes like goval.Shallow: maps as their sorted keys, slices as their length.
pub fn shallow(v: &Value) -> J {
    match v {
        Value::Map(m) => json!({
            "t": m.ty.go_name(),
            "keys": m.entries.keys().map(|k| str_enc(k)).collect::<Vec<_>>(),
        }),
        Value::List(l) => json!({"t": l.ty.go_name(), "len": l.items.len()}),
        _ => encode(v),
    }
}

/// `{"ok": encode(v)}` / `{"err": message}`.
pub fn result<E: std::fmt::Display>(r: Result<Value, E>) -> J {
    match r {
        Ok(v) => json!({"ok": encode(&v)}),
        Err(e) => json!({"err": e.to_string()}),
    }
}

/// Compares a Rust result with Go's `{"ok"}`/`{"err"}`/`{"panic"}` (Go panics are errors here).
pub fn same_result(want: &J, got: &J) -> bool {
    if let Some(p) = want.get("panic") {
        return got.get("err") == Some(p);
    }
    if want.get("err").and_then(J::as_str) == Some("<redacted>") {
        return got.get("err").is_some();
    }
    want == got
}
