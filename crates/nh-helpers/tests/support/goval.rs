//! Decodes the goval-encoded values the nh-helpers oracles use (config maps), the format of
//! `tools/go-oracle/nh-common/goval`.

use std::sync::Arc;

use go_value::{FloatKind, IntKind, List, Map, MapType, SliceType, Value};
use serde_json::Value as J;

use super::gostr;

/// nil, typed nils, bool, ints, float64, strings, slices and string-keyed maps.
pub fn decode_goval(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    let int = |k: IntKind| Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "int" => int(IntKind::Int),
        "int64" => int(IntKind::Int64),
        "float64" => Value::Float(
            f64::from_bits(u64::from_str_radix(v["v"].as_str().unwrap(), 16).unwrap()),
            FloatKind::F64,
        ),
        "string" => Value::String(gostr(&v["s"]).into()),
        _ if v.get("items").is_some() => {
            let ty = match t {
                "[]interface {}" => SliceType::Any,
                "[]string" => SliceType::String,
                other => SliceType::Named(Arc::from(other)),
            };
            Value::List(Arc::new(List::new(
                ty,
                v["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(decode_goval)
                    .collect(),
            )))
        }
        _ if v.get("entries").is_some() => {
            let ty = match t {
                "maps.Params" => MapType::Params,
                "map[string]interface {}" => MapType::StringAny,
                "map[string]string" => MapType::StringString,
                other => MapType::Named(Arc::from(other)),
            };
            let mut m = Map::new(ty);
            for e in v["entries"].as_array().unwrap() {
                m.insert(gostr(&e[0]), decode_goval(&e[1]));
            }
            Value::map(m)
        }
        _ => panic!("cannot decode {v}"),
    }
}

/// A goval-encoded map (Go's nil map is an empty map).
pub fn decode_goval_map(v: &J) -> Map {
    match decode_goval(v) {
        Value::Map(m) => (*m).clone(),
        Value::Invalid | Value::TypedNil(_) => Map::new(MapType::StringAny),
        other => panic!("not a map: {other:?}"),
    }
}
