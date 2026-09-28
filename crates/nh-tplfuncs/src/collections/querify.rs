//! Port of `tpl/collections/querify.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use go_value::{GoString, Map, MapType, SliceType, Value};
use nh_common::cast::caste;
use nh_common::object::GoResult;

use super::collections::Namespace;

const ERR_WRONG_ARG_STRUCTURE: &str = "expected a map, a slice with an even number of elements, or an even number of scalar values, and each key must be a string";
const ERR_KEY_IS_EMPTY_STRING: &str = "one of the keys is an empty string";

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    // Go: tpl/collections/querify.go:Querify
    /// Querify returns a URL query string composed of the given key-value pairs, encoded and
    /// sorted by key.
    pub fn do_querify(&self, params: &[Value]) -> GoResult<GoString> {
        if params.is_empty() {
            return Ok(GoString::empty());
        }

        if params.len() == 1 {
            return match &params[0] {
                // created with collections.Dictionary / site configuration or page parameters
                Value::Map(m) if matches!(m.ty, MapType::StringAny | MapType::Params) => {
                    map_to_query_string(m)
                }
                Value::TypedNil(t) if matches!(&**t, "map[string]interface {}" | "maps.Params") => {
                    Ok(GoString::empty())
                }
                Value::List(l) if l.ty == SliceType::String => {
                    let s: Vec<GoString> = l
                        .items
                        .iter()
                        .map(super::reflect_helpers::string_of)
                        .collect();
                    string_slice_to_query_string(&s)
                }
                Value::TypedNil(t) if &**t == "[]string" => Ok(GoString::empty()),
                Value::List(l) if l.ty == SliceType::Any => {
                    let s = interface_slice_to_string_slice(&l.items)?;
                    string_slice_to_query_string(&s)
                }
                Value::TypedNil(t) if &**t == "[]interface {}" => Ok(GoString::empty()),
                _ => Err(err(ERR_WRONG_ARG_STRUCTURE)),
            };
        }

        if !params.len().is_multiple_of(2) {
            return Err(err(ERR_WRONG_ARG_STRUCTURE));
        }

        let s = interface_slice_to_string_slice(params)?;
        string_slice_to_query_string(&s)
    }
}

// Go: tpl/collections/querify.go:mapToQueryString
/// mapToQueryString returns a URL query string derived from the given string map, encoded and
/// sorted by key. The function returns an error if it cannot convert an element value to a
/// string. (Go ranges the map in random order; which error is reported first follows the key
/// order here.)
fn map_to_query_string(m: &Map) -> GoResult<GoString> {
    if m.is_empty() {
        return Ok(GoString::empty());
    }

    let mut qs = go_url::Values::new();
    for (k, v) in &m.entries {
        if k.is_empty() {
            return Err(err(ERR_KEY_IS_EMPTY_STRING));
        }
        let vs = caste::to_string_e(v)?;
        qs.add(k, vs);
    }
    Ok(GoString::from(qs.encode()))
}

// Go: tpl/collections/querify.go:stringSliceToQueryString
/// sliceToQueryString returns a URL query string derived from the given slice of strings,
/// encoded and sorted by key. The function returns an error if there are an odd number of
/// elements.
fn string_slice_to_query_string(s: &[GoString]) -> GoResult<GoString> {
    if s.is_empty() {
        return Ok(GoString::empty());
    }
    if !s.len().is_multiple_of(2) {
        return Err(err(ERR_WRONG_ARG_STRUCTURE));
    }

    let mut qs = go_url::Values::new();
    let mut i = 0;
    while i < s.len() {
        if s[i].is_empty() {
            return Err(err(ERR_KEY_IS_EMPTY_STRING));
        }
        qs.add(&s[i], &s[i + 1]);
        i += 2;
    }
    Ok(GoString::from(qs.encode()))
}

// Go: tpl/collections/querify.go:interfaceSliceToStringSlice
/// interfaceSliceToStringSlice converts a slice of interfaces to a slice of strings, returning
/// an error if it cannot convert an element to a string.
fn interface_slice_to_string_slice(s: &[Value]) -> GoResult<Vec<GoString>> {
    if s.is_empty() {
        return Ok(Vec::new());
    }

    let mut ss = Vec::with_capacity(s.len());
    for v in s {
        ss.push(caste::to_string_e(v)?);
    }
    Ok(ss)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/querify.go (125 lines; 0/4 funcs executed)
// OK L31-64: (ns *Namespace) Querify(params ...any) (string, error)
// OK L69-86: mapToQueryString[T map[string]any | maps.Params](m T) (string, error)
// OK L91-107: stringSliceToQueryString(s []string) (string, error)
// OK L111-125: interfaceSliceToStringSlice(s []any) ([]string, error)
// ---------------------------------------------------------------------------
