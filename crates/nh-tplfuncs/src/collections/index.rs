//! Port of `tpl/collections/index.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use go_value::{GoString, Kind, Map, MapType, Value};
use nh_common::cast::caste;
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::{as_slice, indirect, int_of, kind, string_of, uint_of, zero_value};

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    // Go: tpl/collections/index.go:Index
    /// Index returns the result of indexing its first argument by the following arguments.
    /// Thus "index x 1 2 3" is, in Go syntax, x[1][2][3]. Each indexed item must be a map,
    /// slice, or array.
    pub fn do_index_outer(&self, item: &Value, args: &[Value]) -> GoResult<Value> {
        match self.do_index(item, args) {
            Ok(v) => Ok(v),
            Err(e) => Err(err(format!(
                "index of type {} with args {} failed: {}",
                item.go_type_name(),
                String::from_utf8_lossy(&go_fmt::sprintf("%v", &[Value::any_list(args.to_vec())])),
                e.message()
            ))),
        }
    }

    // Go: tpl/collections/index.go:doIndex
    fn do_index(&self, item: &Value, args: &[Value]) -> GoResult<Value> {
        if item.is_invalid() {
            // See issue 10489
            // This used to be an error.
            return Ok(Value::Invalid);
        }

        let indices: Vec<Value> = if args.len() == 1 {
            match as_slice(&args[0]) {
                Some(s) if kind(&args[0]) == ReflectKind::Slice => s.items,
                _ => vec![args[0].clone()],
            }
        } else {
            args.to_vec()
        };

        let empty = Map::new(MapType::Params);
        let lowerm = match item {
            Value::Map(m) if m.ty == MapType::Params => Some(&**m),
            Value::TypedNil(t) if &**t == "maps.Params" => Some(&empty),
            _ => None,
        };
        if let Some(m) = lowerm {
            let keys = caste::to_string_slice(&Value::any_list(indices));
            let idx: Vec<&[u8]> = keys.iter().map(|s| s.as_bytes()).collect();
            return Ok(nh_common::maps::params::get_nested(m, &idx));
        }

        let mut v = item.clone();
        for index in &indices {
            let (vv, is_nil) = indirect(&v);
            if is_nil {
                // See issue 10489
                // This used to be an error.
                return Ok(Value::Invalid);
            }
            v = vv;
            match kind(&v) {
                ReflectKind::Slice | ReflectKind::String => {
                    let x: i64 = match kind(index) {
                        ReflectKind::Int(_) => int_of(index),
                        ReflectKind::Uint(_) => uint_of(index) as i64,
                        ReflectKind::Invalid => {
                            return Err(err("cannot index slice/array with nil"));
                        }
                        _ => {
                            return Err(err(format!(
                                "cannot index slice/array with type {}",
                                index.go_type_name()
                            )));
                        }
                    };
                    if kind(&v) == ReflectKind::String {
                        let s = string_of(&v);
                        if x < 0 || x >= s.len() as i64 {
                            // We deviate from stdlib here. Don't return an error if the
                            // index is out of range.
                            return Ok(Value::Invalid);
                        }
                        v = Value::Uint(s[x as usize] as u64, go_value::UintKind::Uint8);
                    } else {
                        let s = as_slice(&v).unwrap();
                        if x < 0 || x >= s.items.len() as i64 {
                            // We deviate from stdlib here. Don't return an error if the
                            // index is out of range.
                            return Ok(Value::Invalid);
                        }
                        v = s.items[x as usize].clone();
                    }
                }
                ReflectKind::Map => {
                    let (key_type, elem_type) = map_types(&v);
                    let index = prepare_arg(index, &key_type)?;
                    let k: GoString = string_of(&index);
                    let found = match &v {
                        Value::Map(m) => m.get(&k).cloned(),
                        Value::Object(o) if o.kind() == Kind::Map => o.map_get(&k),
                        _ => None,
                    };
                    v = match found {
                        Some(x) => x,
                        None => zero_value(&elem_type),
                    };
                }
                ReflectKind::Invalid => {
                    // the loop holds invariant: v.IsValid()
                    return Ok(Value::Invalid);
                }
                _ => {
                    // v.Type() after indirect: a pointer object is its pointee.
                    let t = v.go_type_name();
                    let t = match &v {
                        Value::Object(o) if o.kind() == Kind::Ptr => {
                            t.strip_prefix('*').unwrap_or(&t).to_string()
                        }
                        _ => t.into_owned(),
                    };
                    return Err(err(format!("can't index item of type {t}")));
                }
            }
        }
        Ok(v)
    }
}

/// Key and element types of a map-kind value (keys are strings in the value model).
fn map_types(v: &Value) -> (String, String) {
    let t = v.go_type_name();
    let elem = match v {
        Value::Map(m) if m.ty == MapType::StringString => "string".to_string(),
        _ => hreflect::elem_type(&t).unwrap_or_else(|| "interface {}".to_string()),
    };
    ("string".to_string(), elem)
}

// Go: tpl/collections/index.go:prepareArg
/// prepareArg checks if value can be used as an argument of type argType, and converts an
/// invalid value to appropriate zero if possible.
fn prepare_arg(value: &Value, arg_type: &str) -> GoResult<Value> {
    if value.is_invalid() {
        if !can_be_nil(arg_type) {
            return Err(err(format!("value is nil; should be of type {arg_type}")));
        }
        return Ok(zero_value(arg_type));
    }
    if !hreflect::assignable_to(value, arg_type) {
        return Err(err(format!(
            "value has type {}; should be {}",
            value.go_type_name(),
            arg_type
        )));
    }
    Ok(value.clone())
}

// Go: tpl/collections/index.go:canBeNil
/// canBeNil reports whether an untyped nil can be assigned to the type.
fn can_be_nil(typ: &str) -> bool {
    matches!(
        super::reflect_helpers::type_kind(typ),
        ReflectKind::Chan
            | ReflectKind::Func
            | ReflectKind::Interface
            | ReflectKind::Map
            | ReflectKind::Ptr
            | ReflectKind::Slice
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/index.go (144 lines; 3/4 funcs executed)
// OK L33-39: (ns *Namespace) Index(item any, args ...any) (any, error)
// OK L41-116: (ns *Namespace) doIndex(item any, args ...any) (any, error)
// OK L122-133: prepareArg(value reflect.Value, argType reflect.Type) (reflect.Value, error)
// OK L138-144: canBeNil(typ reflect.Type) bool
// ---------------------------------------------------------------------------
