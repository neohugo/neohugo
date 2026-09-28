//! Port of `tpl/collections/merge.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{GoString, Kind, Map, MapType, Value};
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::kind;

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    // Go: tpl/collections/merge.go:Merge
    /// Merge creates a copy of the final parameter in params and merges the preceding
    /// parameters into it in reverse order. Currently only maps are supported. Key handling is
    /// case insensitive.
    pub fn do_merge(&self, params: &[Value]) -> GoResult<Value> {
        if params.len() < 2 {
            return Err(err("merge requires at least two parameters"));
        }

        let mut result = params[params.len() - 1].clone();

        for i in (0..params.len() - 1).rev() {
            result = self.merge_one(&params[i], &result)?;
        }

        Ok(result)
    }

    // Go: tpl/collections/merge.go:merge
    /// merge creates a copy of dst and merges src into it.
    fn merge_one(&self, src: &Value, dst: &Value) -> GoResult<Value> {
        if kind(dst) != ReflectKind::Map {
            return Err(err(format!(
                "destination must be a map, got {}",
                dst.go_type_name()
            )));
        }

        if !hreflect::is_truthful_value(src) {
            return Ok(dst.clone());
        }

        if kind(src) != ReflectKind::Map {
            return Err(err(format!(
                "source must be a map, got {}",
                src.go_type_name()
            )));
        }

        // Map keys are strings in the value model: the key types always match.
        Ok(Value::map(merge_map(&as_map(dst), &as_map(src))?))
    }
}

/// A map-kind value as a `Map` (a nil map is empty; map objects are copied).
fn as_map(v: &Value) -> Map {
    match v {
        Value::Map(m) => (**m).clone(),
        Value::TypedNil(t) => Map::new(hreflect::map_type_from_name(t)),
        Value::Object(o) if o.kind() == Kind::Map => {
            let mut m = Map::new(MapType::Named(Arc::from(&*o.type_name())));
            for k in o.map_keys() {
                if let Some(v) = o.map_get(&k) {
                    m.insert(k, v);
                }
            }
            m
        }
        _ => Map::new(MapType::StringAny),
    }
}

/// The element type of a map value's type.
fn map_elem_type(m: &Map) -> String {
    match &m.ty {
        MapType::StringAny | MapType::Params => "interface {}".to_string(),
        MapType::StringString => "string".to_string(),
        MapType::Named(n) => hreflect::elem_type(n).unwrap_or_else(|| "interface {}".to_string()),
    }
}

/// Go `v.Elem()` of a map element of static type `elem`: legal for interfaces (and pointers),
/// else the reflect panic.
fn check_elem(elem: &str) -> GoResult<()> {
    match super::reflect_helpers::type_kind(elem) {
        ReflectKind::Interface | ReflectKind::Ptr => Ok(()),
        k => Err(err(format!(
            "reflect: call of reflect.Value.Elem on {} Value",
            super::reflect_helpers::kind_name(k)
        ))),
    }
}

// Go: tpl/collections/merge.go:caseInsensitiveLookup
/// The first key of `m` that EqualFolds `k` (Go iterates the map in random order; the port uses
/// the key order, PORTING.md).
fn case_insensitive_lookup<'a>(m: &'a Map, k: &[u8]) -> Option<&'a Value> {
    m.entries
        .iter()
        .find(|(k2, _)| go_unicode::strings::equal_fold(k, k2))
        .map(|(_, v)| v)
}

// Go: tpl/collections/merge.go:mergeMap
fn merge_map(dst: &Map, src: &Map) -> GoResult<Map> {
    let mut out = Map::new(dst.ty.clone());

    // If the destination is Params, we must lower case all keys.
    let lower_case = dst.ty == MapType::Params;

    // Copy the destination map.
    for (k, v) in &dst.entries {
        out.insert(k.clone(), v.clone());
    }

    // Add all keys in src not already in destination.
    // Maps of the same type will be merged.
    let dst_elem = map_elem_type(dst);
    let src_elem = map_elem_type(src);
    for (k, sv) in &src.entries {
        match case_insensitive_lookup(dst, k) {
            Some(dv) => {
                // If both are the same map key type, merge.
                check_elem(&dst_elem)?;
                if kind(dv) == ReflectKind::Map {
                    check_elem(&src_elem)?;
                    if kind(sv) != ReflectKind::Map {
                        continue;
                    }

                    out.insert(k.clone(), Value::map(merge_map(&as_map(dv), &as_map(sv))?));
                }
            }
            None => {
                let kk: GoString = if lower_case {
                    GoString::from(go_unicode::strings::to_lower(k).into_owned())
                } else {
                    k.clone()
                };
                if !hreflect::type_assignable_to(&src_elem, &dst_elem) {
                    return Err(err(format!(
                        "reflect.Value.SetMapIndex: value of type {src_elem} is not assignable to type {dst_elem}"
                    )));
                }
                out.insert(kk, sv.clone());
            }
        }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/merge.go (143 lines; 0/4 funcs executed)
// OK L30-46: (ns *Namespace) Merge(params ...any) (any, error)
// OK L49-69: (ns *Namespace) merge(src, dst any) (any, error)
// OK L71-89: caseInsensitiveLookup(m, k reflect.Value) (reflect.Value, bool)
// OK L91-143: mergeMap(dst, src reflect.Value) reflect.Value
// ---------------------------------------------------------------------------
