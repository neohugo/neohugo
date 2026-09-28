//! Shared helpers of the nh-i18n oracle tests: fixture loading and Go value specs.

#![allow(dead_code)]

use std::borrow::Cow;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use go_value::{HostCtx, IntKind, Kind, Map, MapType, Value};
use serde_json::Value as J;

pub fn fixture_path(topic: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(topic)
        .join(name)
}

/// Reads a gzip-compressed JSON fixture (`{"...": header, "cases": [...]}`).
pub fn load_fixture(topic: &str, name: &str) -> J {
    let f = std::fs::File::open(fixture_path(topic, name)).unwrap();
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

/// A Go test struct (`main.countField`, `main.countMethod`, ...), as the oracles describe it:
/// exported fields and zero-argument methods with fixed results.
pub struct GoStruct {
    pub type_name: String,
    pub ptr: bool,
    pub fields: Vec<(String, Value)>,
    pub methods: Vec<(String, Value)>,
}

impl go_value::Object for GoStruct {
    fn type_name(&self) -> Cow<'_, str> {
        if self.ptr {
            Cow::Owned(format!("*{}", self.type_name))
        } else {
            Cow::Borrowed(&self.type_name)
        }
    }

    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }

    fn has_method(&self, name: &str) -> bool {
        self.methods.iter().any(|(n, _)| n == name)
    }

    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        self.methods
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| Ok(v.clone()))
    }

    fn field(&self, name: &str) -> Option<Value> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    }

    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.fields
                .iter()
                .map(|(n, v)| (Cow::Borrowed(n.as_str()), v.clone()))
                .collect(),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A Go named int type (`time.Month`).
pub struct NamedInt {
    pub type_name: String,
    pub v: i64,
}

impl go_value::Object for NamedInt {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.type_name)
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::int(self.v))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Builds the Go value an oracle "arg spec" describes (see the oracles' `argSpec`).
pub fn value_from_spec(s: &J) -> Value {
    let t = s["t"].as_str().unwrap();
    let v = &s["v"];
    match t {
        "nil" => Value::Invalid,
        "int" => Value::int(v.as_i64().unwrap()),
        "int64" => Value::int64(v.as_i64().unwrap()),
        "int8" => Value::Int(v.as_i64().unwrap(), IntKind::Int8),
        "uint" => Value::Uint(v.as_u64().unwrap(), go_value::UintKind::Uint),
        "float64" => Value::float64(v.as_f64().unwrap()),
        "float32" => Value::Float(v.as_f64().unwrap(), go_value::FloatKind::F32),
        "string" => Value::string(v.as_str().unwrap()),
        "html" => Value::html(v.as_str().unwrap()),
        "bool" => Value::Bool(v.as_bool().unwrap()),
        "map" | "params" => {
            let mut m = Map::new(if t == "map" {
                MapType::StringAny
            } else {
                MapType::Params
            });
            for (k, x) in v.as_object().unwrap() {
                m.insert(k.as_str(), value_from_spec(x));
            }
            Value::map(m)
        }
        "slice" => Value::any_list(v.as_array().unwrap().iter().map(value_from_spec).collect()),
        "struct" => {
            let fields = s["fields"]
                .as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, x)| (k.clone(), value_from_spec(x)))
                        .collect()
                })
                .unwrap_or_default();
            let methods = s["methods"]
                .as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, x)| (k.clone(), value_from_spec(x)))
                        .collect()
                })
                .unwrap_or_default();
            Value::Object(Arc::new(GoStruct {
                type_name: s["type"].as_str().unwrap().to_string(),
                ptr: s["ptr"].as_bool().unwrap_or(false),
                fields,
                methods,
            }))
        }
        "nilptr" => Value::TypedNil(Arc::from(s["type"].as_str().unwrap())),
        "month" => Value::Object(Arc::new(NamedInt {
            type_name: "time.Month".to_string(),
            v: v.as_i64().unwrap(),
        })),
        _ => panic!("unknown spec {s}"),
    }
}
