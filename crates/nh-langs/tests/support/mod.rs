//! Shared helpers for the T03 nh-langs oracle tests: fixture reading and a decoder of the
//! typed JSON of `tools/go-oracle/nh-parser/tval` into `go_value::Value`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Read;

use go_value::{FloatKind, GoString, IntKind, Map, MapType, SliceType, UintKind, Value};
use serde_json::Value as J;

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

/// Decodes a `goval.Str` value.
pub fn j_bytes(j: &J) -> Vec<u8> {
    match j {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => {
            let h = o["hex"].as_str().unwrap();
            (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect()
        }
        other => panic!("not a string: {other}"),
    }
}

pub fn j_string(j: &J) -> String {
    String::from_utf8_lossy(&j_bytes(j)).into_owned()
}

/// Decodes a typed JSON value.
pub fn dec(j: &J) -> Value {
    let t = j["t"].as_str().unwrap();
    let int = |k: IntKind| Value::Int(j["v"].as_str().unwrap().parse().unwrap(), k);
    let uint = |k: UintKind| Value::Uint(j["v"].as_str().unwrap().parse().unwrap(), k);
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(j["v"].as_bool().unwrap()),
        "int" => int(IntKind::Int),
        "int8" => int(IntKind::Int8),
        "int16" => int(IntKind::Int16),
        "int32" => int(IntKind::Int32),
        "int64" => int(IntKind::Int64),
        "uint" => uint(UintKind::Uint),
        "uint8" => uint(UintKind::Uint8),
        "uint16" => uint(UintKind::Uint16),
        "uint32" => uint(UintKind::Uint32),
        "uint64" => uint(UintKind::Uint64),
        "float64" => Value::Float(
            f64::from_bits(u64::from_str_radix(j["v"].as_str().unwrap(), 16).unwrap()),
            FloatKind::F64,
        ),
        "string" => Value::String(GoString::from(j_bytes(&j["s"]))),
        "map[string]interface {}" | "maps.Params" => {
            let ty = if t == "maps.Params" {
                MapType::Params
            } else {
                MapType::StringAny
            };
            let mut entries = BTreeMap::new();
            for e in j["entries"].as_array().unwrap() {
                entries.insert(GoString::from(j_bytes(&e[0])), dec(&e[1]));
            }
            Value::map(Map::with_entries(ty, entries))
        }
        "[]interface {}" | "[]uint8" => {
            let ty = if t == "[]uint8" {
                SliceType::Uint8
            } else {
                SliceType::Any
            };
            Value::list(ty, j["items"].as_array().unwrap().iter().map(dec).collect())
        }
        other => panic!("cannot decode {other}"),
    }
}
