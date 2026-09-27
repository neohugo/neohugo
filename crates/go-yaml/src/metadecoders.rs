//! neohugo `parser/metadecoders` for the YAML format, producing
//! [`go_value::Value`]s.
//!
//! Go: parser/metadecoders/decoder.go — `Decoder.UnmarshalToMap(data, YAML)`,
//! `Decoder.Unmarshal(data, YAML)` (the YAML branch of `UnmarshalTo`) and
//! `stringifyMapKeys`, with `cast.ToStringE` (spf13/cast v1.9.2) for keys.
//!
//! Value mapping (Go dynamic type → `go_value::Value`):
//! `map[string]interface{}` → `Value::Map(MapType::StringAny)`,
//! `[]interface{}` → `Value::List(SliceType::Any)`, `int` →
//! `Value::Int(_, IntKind::Int)`, `uint64` → `Value::Uint(_, UintKind::Uint64)`,
//! `float64` → `Value::Float(_, FloatKind::F64)`, `bool`, `string`, and nil
//! → `Value::Invalid`.
//!
//! Errors: Go wraps YAML errors as
//! `toFileError(YAML, data, fmt.Errorf("failed to unmarshal YAML: %w", err))`.
//! The file-error decoration (position extraction, file name) belongs to
//! the herrors port; [`MetaError`] carries the yaml.v2 error and renders the
//! `failed to unmarshal YAML: ` prefix.

use std::collections::BTreeMap;
use std::fmt;

use go_value::{FloatKind, GoString, IntKind, List, Map, MapType, SliceType, UintKind, Value};

use crate::gostd::format_float_f;
use crate::{Error, Yaml};

/// A metadecoders YAML error: `failed to unmarshal YAML: <yaml.v2 error>`.
#[derive(Clone, PartialEq, Eq)]
pub struct MetaError {
    pub yaml: Error,
}

impl MetaError {
    /// Go's `err.Error()` of the wrapped error, byte for byte.
    pub fn message_bytes(&self) -> Vec<u8> {
        let mut m = b"failed to unmarshal YAML: ".to_vec();
        m.extend_from_slice(self.yaml.message_bytes());
        m
    }
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to unmarshal YAML: {}", self.yaml)
    }
}

impl fmt::Debug for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetaError({:?})", self.to_string())
    }
}

impl std::error::Error for MetaError {}

/// Go: `metadecoders.Default.UnmarshalToMap(data, metadecoders.YAML)`.
///
/// `Ok(None)` is Go's nil map (a `null` document); an empty document gives
/// an empty map. Front matter, config files and `transform.Remarshal` use
/// this path: top-level keys are decoded directly as strings (so `true: x`
/// keeps the key `"true"` as written, `0x10: x` keeps `"0x10"`), nested maps
/// get their keys through `cast.ToStringE`.
pub fn unmarshal_to_map(data: &[u8]) -> Result<Option<Map>, MetaError> {
    let mut p = crate::decode::NodeParser::new(data);
    let node = p.parse().map_err(|yaml| MetaError { yaml })?;
    let Some(node) = node else {
        return Ok(Some(Map::new(MapType::StringAny)));
    };
    let depth = p.decode_depth(node);
    let nodes = &p.nodes;
    crate::with_stack(depth, || {
        let mut d = crate::decode::Decoder::new(nodes, false);
        let mut m = Some(crate::StrMap::new());
        d.unmarshal(node, &mut crate::decode::Out::TopMap(&mut m))
            .map_err(|yaml| MetaError { yaml })?;
        if !d.terrors.is_empty() {
            return Err(MetaError {
                yaml: Error::type_error(std::mem::take(&mut d.terrors)),
            });
        }
        Ok(m.map(str_map_to_value_map))
    })
}

/// Go: `metadecoders.Default.Unmarshal(data, metadecoders.YAML)` (data
/// files, `transform.Unmarshal`).
///
/// Empty input gives an empty `map[string]interface{}`; otherwise the
/// document is decoded into `interface{}` and `stringifyMapKeys` is
/// applied.
pub fn unmarshal(data: &[u8]) -> Result<Value, MetaError> {
    if data.is_empty() {
        return Ok(Value::map(Map::new(MapType::StringAny)));
    }
    let mut p = crate::decode::NodeParser::new(data);
    let node = p.parse().map_err(|yaml| MetaError { yaml })?;
    let Some(node) = node else {
        return Ok(Value::Invalid);
    };
    let depth = p.decode_depth(node);
    let nodes = &p.nodes;
    crate::with_stack(depth, || {
        let mut d = crate::decode::Decoder::new(nodes, false);
        let mut v = Yaml::Nil;
        d.unmarshal(node, &mut crate::decode::Out::Iface(&mut v))
            .map_err(|yaml| MetaError { yaml })?;
        if !d.terrors.is_empty() {
            return Err(MetaError {
                yaml: Error::type_error(std::mem::take(&mut d.terrors)),
            });
        }
        Ok(to_value(&v))
    })
}

/// Go: spf13/cast v1.9.2 `ToStringE` for the key types yaml.v2 produces
/// (string, bool, int, uint64, float64, nil). Map and slice keys cannot
/// occur (yaml.v2 rejects them).
pub fn cast_to_string(k: &Yaml) -> Vec<u8> {
    match k {
        Yaml::String(s) => s.clone(),
        Yaml::Bool(b) => b.to_string().into_bytes(),
        Yaml::Float64(f) => format_float_f(*f).into_bytes(),
        Yaml::Int(i) => i.to_string().into_bytes(),
        Yaml::Uint64(u) => u.to_string().into_bytes(),
        Yaml::Nil => Vec::new(),
        // Go: fmt.Sprintf("%v", k) fallback; unreachable for yaml.v2 keys.
        Yaml::Seq(_) | Yaml::Map(_) => crate::decode::go_sharp_v(k).into_bytes(),
    }
}

/// Go: decoder.go:stringifyMapKeys followed by the conversion of the
/// resulting Go value to `go_value::Value`.
///
/// When two keys stringify to the same string (e.g. `1` and `"1"`), Go's
/// result depends on random map iteration order; here the entry that comes
/// later in the document wins.
pub fn to_value(v: &Yaml) -> Value {
    match v {
        Yaml::Nil => Value::Invalid,
        Yaml::Bool(b) => Value::Bool(*b),
        Yaml::Int(i) => Value::Int(*i, IntKind::Int),
        Yaml::Uint64(u) => Value::Uint(*u, UintKind::Uint64),
        Yaml::Float64(f) => Value::Float(*f, FloatKind::F64),
        Yaml::String(s) => Value::String(GoString::from(s.as_slice())),
        Yaml::Seq(items) => Value::List(std::sync::Arc::new(List::new(
            SliceType::Any,
            items.iter().map(to_value).collect(),
        ))),
        Yaml::Map(m) => {
            let mut entries: BTreeMap<GoString, Value> = BTreeMap::new();
            for (k, v) in m.iter() {
                entries.insert(GoString::from(cast_to_string(k)), to_value(v));
            }
            Value::map(Map::with_entries(MapType::StringAny, entries))
        }
    }
}

fn str_map_to_value_map(m: crate::StrMap) -> Map {
    let mut entries: BTreeMap<GoString, Value> = BTreeMap::new();
    for (k, v) in m.into_entries() {
        entries.insert(GoString::from(k), to_value(&v));
    }
    Map::with_entries(MapType::StringAny, entries)
}

/// Canonical typed dump of a converted value (shared with the Go oracle).
#[doc(hidden)]
pub fn dump_value(v: &Value) -> String {
    match v {
        Value::Invalid => "nil".to_string(),
        Value::Bool(b) => format!("bool:{b}"),
        Value::Int(i, IntKind::Int) => format!("int:{i}"),
        Value::Uint(u, UintKind::Uint64) => format!("uint64:{u}"),
        Value::Float(f, FloatKind::F64) => format!("float64:{:016x}", f.to_bits()),
        Value::String(s) => format!("str:\"{}\"", crate::dump_escape(s.as_bytes())),
        Value::List(l) => {
            let parts: Vec<String> = l.items.iter().map(dump_value).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Map(m) => {
            let parts: Vec<String> = m
                .entries
                .iter()
                .map(|(k, v)| {
                    format!(
                        "str:\"{}\"={}",
                        crate::dump_escape(k.as_bytes()),
                        dump_value(v)
                    )
                })
                .collect();
            format!("smap{{{}}}", parts.join(","))
        }
        other => format!("unexpected:{other:?}"),
    }
}

/// Dump of an `UnmarshalToMap` result.
#[doc(hidden)]
pub fn dump_map(m: &Option<Map>) -> String {
    match m {
        None => "smap(nil)".to_string(),
        Some(m) => dump_value(&Value::map(m.clone())),
    }
}
