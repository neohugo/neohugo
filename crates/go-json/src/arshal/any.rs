//! Port of the marshal half of `encoding/json/v2/arshal_any.go` (go1.27.1):
//! the fast paths for values held in an `interface{}`.
//!
//! (The unmarshal fast path, `unmarshalValueAny`, is disabled whenever
//! `AllowDuplicateNames` is set, which v1 always does.)

use std::borrow::Cow;

use go_value::{FloatKind, List, Map, MapType, SliceType, Value};

use super::default::{marshal_map_entries, marshal_slice};
use super::{Enc, Static};
use crate::goerr::Err;
use crate::jsontext::token::Token;

// Go: arshal_any.go:marshalValueAny
/// marshalValueAny marshals a Go any as a JSON value.
/// This exists purely as an optimization.
pub(crate) fn marshal_value_any(enc: &mut Enc, val: &Value) -> Option<Err> {
    match val {
        Value::Invalid => return enc.write_token(&Token::Null),
        Value::Bool(b) => return enc.write_token(&Token::bool(*b)),
        Value::String(s) => return enc.write_token(&Token::string(s)),
        Value::Float(f, FloatKind::F64) => {
            if !(f.is_nan() || f.is_infinite()) {
                return enc.write_token(&Token::float(*f));
            }
            // use default logic below
        }
        Value::Map(m) if m.ty == MapType::StringAny => return marshal_object_any(enc, m),
        Value::List(l) if l.ty == SliceType::Any => return marshal_array_any(enc, l),
        _ => {}
    }
    super::marshal_concrete(enc, val)
}

// Go: arshal_any.go:marshalObjectAny
/// marshalObjectAny marshals a Go map[string]any as a JSON object
/// (a nil map is a Value::TypedNil and never reaches here).
fn marshal_object_any(enc: &mut Enc, obj: &Map) -> Option<Err> {
    // Go: the object is written with WriteToken(String(name)) and
    // marshalValueAny for each value, in sorted name order under
    // Deterministic; marshal_map_entries does exactly that for Static::Any
    // values (marshalValueAny == the interface arshaler without tags).
    marshal_map_entries(
        enc,
        obj.entries
            .iter()
            .map(|(k, v)| (Cow::Borrowed(k.as_bytes()), Cow::Borrowed(v))),
        obj.entries.len(),
        Static::Any,
        "map[string]interface {}",
    )
}

// Go: arshal_any.go:marshalArrayAny
/// marshalArrayAny marshals a Go []any as a JSON array
/// (a nil slice is a Value::TypedNil and never reaches here).
fn marshal_array_any(enc: &mut Enc, arr: &List) -> Option<Err> {
    marshal_slice(enc, &arr.items, Static::Any, "[]interface {}")
}
