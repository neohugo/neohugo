//! Module `cast::caste`.
//!
//! PORT spf13/cast@v1.9.2 caste.go/basic.go/number.go/slice.go/map.go (ToStringE, ToIntE, ToInt64E, ToFloat64E, ToBoolE, ToStringSliceE, ToIntSliceE, ToStringMapE ...)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).


//! Port of `github.com/spf13/cast@v1.9.2` conversions over [`go_value::Value`] (caste.go, basic.go,
//! number.go, slice.go, map.go, indirect.go). Semantics that reach output: `ToStringE` formatting of
//! numbers (`strconv.FormatFloat(f, 'f', -1, 64)` for floats, not `%v`!), `nil -> ""`,
//! `ToStringSliceE` failing on nested slices/maps, `ToIntE` of strings (base 0 parsing, leading
//! zeros trimmed), `fmt.Stringer`/`error` support.

use go_value::{GoString, Map, Value};

use crate::herrors::Result;

// Go: spf13/cast basic.go:ToStringE
pub fn to_string_e(v: &Value) -> Result<GoString> {
    todo!()
}

/// `cast.ToString` (errors -> "").
pub fn to_string(v: &Value) -> GoString {
    to_string_e(v).unwrap_or_default()
}

// Go: spf13/cast number.go:ToIntE (Go int = i64)
pub fn to_int_e(v: &Value) -> Result<i64> {
    todo!()
}

pub fn to_int(v: &Value) -> i64 {
    to_int_e(v).unwrap_or(0)
}

// Go: spf13/cast number.go:ToInt64E
pub fn to_int64_e(v: &Value) -> Result<i64> {
    todo!()
}

// Go: spf13/cast number.go:ToFloat64E
pub fn to_float64_e(v: &Value) -> Result<f64> {
    todo!()
}

// Go: spf13/cast basic.go:ToBoolE
pub fn to_bool_e(v: &Value) -> Result<bool> {
    todo!()
}

// Go: spf13/cast slice.go:ToStringSliceE
pub fn to_string_slice_e(v: &Value) -> Result<Vec<GoString>> {
    todo!()
}

pub fn to_string_slice(v: &Value) -> Vec<GoString> {
    to_string_slice_e(v).unwrap_or_default()
}

// Go: spf13/cast slice.go:ToIntSliceE
pub fn to_int_slice_e(v: &Value) -> Result<Vec<i64>> {
    todo!()
}

// Go: spf13/cast slice.go:ToSliceE (`[]interface{}`)
pub fn to_slice_e(v: &Value) -> Result<Vec<Value>> {
    todo!()
}

// Go: spf13/cast map.go:ToStringMapE
pub fn to_string_map_e(v: &Value) -> Result<Map> {
    todo!()
}

// Go: spf13/cast map.go:ToStringMapStringE
pub fn to_string_map_string_e(v: &Value) -> Result<Map> {
    todo!()
}

// Go: spf13/cast map.go:ToStringMapBoolE
pub fn to_string_map_bool_e(v: &Value) -> Result<Map> {
    todo!()
}

// Go: spf13/cast time.go:ToDurationE
pub fn to_duration_e(v: &Value) -> Result<std::time::Duration> {
    todo!()
}
