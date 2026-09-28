//! Port of `common/types/convert.go`.
//!
//! Owner: Wave B task T01 (common-values).

use go_value::{GoString, SliceType, Value};

use crate::cast::caste;
use crate::herrors::{Error, Result};

// Go: common/types/convert.go:ToDuration
/// ToDuration converts v to `time.Duration` (0 on error).
pub fn to_duration(v: &Value) -> go_time::Duration {
    to_duration_e(v).unwrap_or(go_time::Duration(0))
}

// Go: common/types/convert.go:ToDurationE
/// ToDurationE: a positive integer (`cast.ToInt`) is milliseconds; anything else is parsed with
/// `time.ParseDuration(cast.ToString(v))`.
pub fn to_duration_e(v: &Value) -> Result<go_time::Duration> {
    let n = caste::to_int(v);
    if n > 0 {
        return Ok(go_time::Duration(n.wrapping_mul(1_000_000)));
    }
    let s = caste::to_string(v);
    match go_time::parse_duration(s.as_bytes()) {
        Ok(d) => Ok(d),
        Err(_) => Err(Error::new(format!(
            "cannot convert {} to time.Duration",
            String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(v)))
        ))),
    }
}

// Go: common/types/convert.go:ToStringSlicePreserveString
/// ToStringSlicePreserveString is ToStringSlicePreserveStringE that never fails (nil on error).
pub fn to_string_slice_preserve_string(v: &Value) -> Vec<GoString> {
    to_string_slice_preserve_string_e(v).unwrap_or_default()
}

// Go: common/types/convert.go:ToStringSlicePreserveStringE
/// ToStringSlicePreserveStringE converts v to a string slice. A `string` is wrapped in a
/// one-element slice (not split on spaces); other values go through `cast.ToStringSliceE`, and
/// slices it rejects (e.g. `[]int`) are converted element by element with `cast.ToStringE`.
pub fn to_string_slice_preserve_string_e(v: &Value) -> Result<Vec<GoString>> {
    if v.is_invalid() {
        return Ok(Vec::new());
    }
    if let Value::String(s) = v {
        return Ok(vec![s.clone()]);
    }
    if let Ok(result) = caste::to_string_slice_e(v) {
        return Ok(result);
    }

    // Probably []int or similar. Fall back to reflect.
    match crate::hreflect::to_slice_any(v) {
        Some(items) => {
            let mut result = Vec::with_capacity(items.len());
            for item in &items {
                result.push(caste::to_string_e(item)?);
            }
            Ok(result)
        }
        None => Err(Error::new(format!(
            "failed to convert {} to a string slice",
            v.go_type_name()
        ))),
    }
}

// Go: common/types/convert.go:TypeToString
/// TypeToString converts v to a string if it's a valid string type (`string` and the
/// html/template string types); numbers etc. are not converted.
pub fn type_to_string(v: &Value) -> Option<GoString> {
    match v {
        Value::String(s) | Value::Safe(_, s) => Some(s.clone()),
        _ => None,
    }
}

// Go: common/types/convert.go:ToString
/// ToString converts v to a string ("" on error).
pub fn to_string(v: &Value) -> GoString {
    to_string_e(v).unwrap_or_default()
}

// Go: common/types/convert.go:ToStringE
/// ToStringE converts v to a string: the string types, `json.RawMessage` (its bytes), else
/// `cast.ToStringE`.
pub fn to_string_e(v: &Value) -> Result<GoString> {
    if let Some(s) = type_to_string(v) {
        return Ok(s);
    }
    if let Some(raw) = v.downcast::<go_json::RawMessage>() {
        return Ok(GoString::from(raw.0.clone().unwrap_or_default()));
    }
    if let Value::List(l) = v
        && matches!(&l.ty, SliceType::Named(n) if &**n == "json.RawMessage")
    {
        let b: Vec<u8> = l
            .items
            .iter()
            .map(|x| match x {
                Value::Uint(u, _) => *u as u8,
                _ => 0,
            })
            .collect();
        return Ok(GoString::from(b));
    }
    caste::to_string_e(v)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/convert.go (129 lines; 2/7 funcs executed)
// OK L28-31: ToDuration(v any) time.Duration
// OK L34-43: ToDurationE(v any) (time.Duration, error)
// OK L47-50: ToStringSlicePreserveString(v any) []string
// OK L54-83: ToStringSlicePreserveStringE(v any) ([]string, error)
// OK L88-109: TypeToString(v any) (string, bool)
// OK L112-115: ToString(v any) string
// OK L118-129: ToStringE(v any) (string, error)
// ---------------------------------------------------------------------------
