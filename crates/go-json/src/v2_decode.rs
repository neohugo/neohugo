//! Port of `encoding/json/v2_decode.go` (go1.27.1): `Unmarshal` (implemented
//! with encoding/json/v2 and v1 options) for the targets Hugo uses.

use go_value::{Map, MapType, Value};

use crate::arshal::{self, Target};
use crate::error::Error;
use crate::jsonopts::{Opt, default_options_v1};
use crate::v2_inject::transform_unmarshal_error;

// Go: v2_decode.go:Unmarshal (target `*any`)
/// Unmarshal parses the JSON-encoded data into a Go `interface{}` value:
///
///   - bool, for JSON booleans
///   - float64, for JSON numbers
///   - string, for JSON strings
///   - `[]interface {}`, for JSON arrays
///   - `map[string]interface {}`, for JSON objects (a later duplicate name
///     replaces an earlier one)
///   - nil ([`Value::Invalid`]) for JSON null
///
/// Invalid UTF-8 and invalid UTF-16 surrogate escapes in strings are
/// replaced by U+FFFD.
///
/// A syntax error returns [`Error::Syntax`] before anything is decoded. A
/// number that does not fit a float64 returns [`Error::UnmarshalType`]
/// after the whole value has been decoded (Go stores ±Inf); use
/// [`unmarshal_partial`] to get that value too.
pub fn unmarshal(data: &[u8]) -> Result<Value, Error> {
    match unmarshal_partial(data) {
        (v, None) => Ok(v),
        (_, Some(err)) => Err(err),
    }
}

/// Go's `json.Unmarshal(data, &v)` with `var v any`, returning what Go
/// leaves in `v` together with the error.
pub fn unmarshal_partial(data: &[u8]) -> (Value, Option<Error>) {
    let (v, err) = arshal::unmarshal(
        data,
        Target::Any,
        &[Opt::Struct(Box::new(default_options_v1()))],
    );
    (v, err.map(|e| transform_unmarshal_error("", e)))
}

// Go: v2_decode.go:Unmarshal (target `*map[string]any`)
/// Go's `json.Unmarshal(data, &m)` with `m := make(map[string]any)`, as
/// Hugo's `metadecoders.UnmarshalToMap` does. A JSON `null` sets the map to
/// nil ([`Value::TypedNil`] `map[string]interface {}`); any other
/// non-object is an [`Error::UnmarshalType`].
pub fn unmarshal_map(data: &[u8]) -> Result<Value, Error> {
    match unmarshal_map_partial(data) {
        (v, None) => Ok(v),
        (_, Some(err)) => Err(err),
    }
}

/// [`unmarshal_map`] returning what Go leaves in the map together with the error.
pub fn unmarshal_map_partial(data: &[u8]) -> (Value, Option<Error>) {
    let (v, err) = arshal::unmarshal(
        data,
        Target::Map(Map::new(MapType::StringAny)),
        &[Opt::Struct(Box::new(default_options_v1()))],
    );
    (v, err.map(|e| transform_unmarshal_error("", e)))
}
