//! Port of `tpl/collections/sort.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! A pair key is `Option<Value>`: `None` is Go's invalid `reflect.Value`. `Less` compares an
//! invalid key against `reflect.Zero(other.Type())` passed as a `reflect.Value` (a struct that
//! is neither a time nor a `Comparer`), which `compareGetWithCollator` reads as `0` without a
//! string, exactly like a nil: the port passes `Value::Invalid`.

use go_value::{GoString, HostCtx, List, MapType, Value};
use nh_common::cast::caste;
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::{as_slice, kind};
use super::where_::{EvalFail, evaluate_sub_elem};
use nh_common::hreflect::{self, ReflectKind};

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

/// Go `strings.Split(s, ".")`.
pub(crate) fn split_dot(s: &[u8]) -> Vec<GoString> {
    s.split(|b| *b == b'.').map(GoString::from).collect()
}

/// Go: `pair`.
struct Pair {
    key: Option<Value>,
    value: Value,
}

impl Namespace {
    // Go: tpl/collections/sort.go:Sort
    /// Sort returns a sorted copy of the list l.
    pub fn do_sort(&self, ctx: HostCtx<'_>, l: &Value, args: &[Value]) -> GoResult<Value> {
        if l.is_invalid() {
            return Err(err("sequence must be provided"));
        }

        let (seqv, is_nil) = super::reflect_helpers::indirect(l);
        if is_nil {
            return Err(err("can't iterate over a nil value"));
        }

        let seq_kind = kind(&seqv);
        let (slice_type, elem_type) = match seq_kind {
            ReflectKind::Slice => {
                let s = as_slice(&seqv).unwrap();
                let e = s.elem_type();
                (s.ty, e)
            }
            ReflectKind::Map => {
                let t = seqv.go_type_name();
                let elem = match &seqv {
                    Value::Map(m) if m.ty == MapType::StringString => "string".to_string(),
                    _ => hreflect::elem_type(&t).unwrap_or_else(|| "interface {}".to_string()),
                };
                (hreflect::slice_of(&elem), elem)
            }
            _ => {
                return Err(err(format!("can't sort {}", l.go_type_name())));
            }
        };

        let language = self.d.conf.language();
        let collator = language.collator1().clone();

        let mut sort_asc = true;
        let mut sort_by_field = GoString::empty();
        for (i, l) in args.iter().enumerate() {
            let d_str = caste::to_string_e(l);
            match (i, d_str) {
                (0, Err(_)) => sort_by_field = GoString::empty(),
                (0, Ok(s)) => sort_by_field = s,
                (1, Ok(s)) if s == "desc" => sort_asc = false,
                (1, _) => sort_asc = true,
                _ => {}
            }
        }
        let path = split_dot(go_unicode::strings::trim(&sort_by_field, b"."));

        let mut pairs: Vec<Pair> = Vec::new();

        let by_value = sort_by_field.is_empty() || sort_by_field == "value";
        match seq_kind {
            ReflectKind::Slice => {
                let s = as_slice(&seqv).unwrap();
                for item in &s.items {
                    let key = if by_value {
                        Some(item.clone())
                    } else {
                        self.sort_key(ctx, item, &elem_type, &path)?
                    };
                    pairs.push(Pair {
                        key,
                        value: item.clone(),
                    });
                }
            }
            ReflectKind::Map => {
                // Go iterates the map in random order; ties then sort randomly too. The port
                // uses the key order (PORTING.md).
                let entries: Vec<(GoString, Value)> = match &seqv {
                    Value::Map(m) => m
                        .entries
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    Value::Object(o) => o
                        .map_keys()
                        .into_iter()
                        .filter_map(|k| o.map_get(&k).map(|v| (k, v)))
                        .collect(),
                    _ => Vec::new(),
                };
                for (k, value) in entries {
                    let key = if sort_by_field.is_empty() {
                        Some(Value::String(k))
                    } else if sort_by_field == "value" {
                        Some(value.clone())
                    } else {
                        self.sort_key(ctx, &value, &elem_type, &path)?
                    };
                    pairs.push(Pair { key, value });
                }
            }
            _ => unreachable!(),
        }

        let mut guard = collator.lock();
        let mut cmp = |a: &[u8], b: &[u8]| guard.compare_strings(a, b);
        {
            let mut less = |a: &Pair, b: &Pair| self.pair_less(&mut cmp, a, b);
            if sort_asc {
                go_sort::stable_by(&mut pairs, |a, b| less(a, b));
            } else {
                go_sort::stable_by(&mut pairs, |a, b| less(b, a));
            }
        }

        Ok(Value::List(std::sync::Arc::new(List::new(
            slice_type,
            pairs.into_iter().map(|p| p.value).collect(),
        ))))
    }

    /// The key of one element: the path walked with `evaluateSubElem`, `maps.Params` looked up
    /// with the rest of the path.
    fn sort_key(
        &self,
        ctx: HostCtx<'_>,
        item: &Value,
        elem_type: &str,
        path: &[GoString],
    ) -> GoResult<Option<Value>> {
        let mut v: Option<Value> = Some(item.clone());
        let mut typ = elem_type.to_string();
        for (i, elem_name) in path.iter().enumerate() {
            (v, typ) = match evaluate_sub_elem(ctx, v.as_ref(), &typ, elem_name) {
                Ok(r) => r,
                Err(EvalFail::Err(e)) => return Err(err(e)),
                Err(EvalFail::Panic(e)) => return Err(e),
            };
            let Some(vv) = &v else {
                continue;
            };
            // Special handling of lower cased maps.
            if let Value::Map(m) = vv
                && m.ty == MapType::Params
            {
                let idx: Vec<&[u8]> = path[i + 1..].iter().map(|p| p.as_bytes()).collect();
                let nested = nh_common::maps::params::get_nested(m, &idx);
                v = if nested.is_invalid() {
                    None
                } else {
                    Some(nested)
                };
                break;
            }
        }
        Ok(v)
    }

    // Go: tpl/collections/sort.go:Less
    fn pair_less(&self, cmp: &mut dyn FnMut(&[u8], &[u8]) -> i32, a: &Pair, b: &Pair) -> bool {
        let lt = |x: &Value, y: &Value, cmp: &mut dyn FnMut(&[u8], &[u8]) -> i32| {
            self.sort_comp
                .do_lt_collate(Some(cmp), x, std::slice::from_ref(y))
                .unwrap_or(false)
        };
        match (&a.key, &b.key) {
            (Some(iv), Some(jv)) => lt(iv, jv, cmp),
            // if j is invalid, test i against i's zero value
            (Some(iv), None) => lt(iv, &Value::Invalid, cmp),
            // if i is invalid, test j against j's zero value
            (None, Some(jv)) => lt(&Value::Invalid, jv, cmp),
            (None, None) => false,
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/sort.go (197 lines; 5/5 funcs executed)
//   types: pair, pairList
// OK L30-141: (ns *Namespace) Sort(ctx context.Context, l any, args ...any) (any, error)
// OK L160-160: (p pairList) Swap(i, j int) (go_sort::stable_by)
// OK L161-161: (p pairList) Len() int (go_sort::stable_by)
// OK L162-182: (p pairList) Less(i, j int) bool
// OK L185-197: (p pairList) sort() any
// ---------------------------------------------------------------------------
