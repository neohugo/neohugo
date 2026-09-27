//! Module `decode`.
//!
//! NEW: mitchellh/mapstructure WeakDecode semantics (case-insensitive field match, weak conversions) over go_value::Value
//!
//! Owner: Wave B task T04 (config-base-media).


//! `mitchellh/mapstructure` `WeakDecode` semantics used by every config decoder: field names are
//! matched case-insensitively against map keys; "weak" conversions apply (float with no fraction ->
//! int, "true" -> bool, single value -> slice, number -> string, ...); `squash` embeds.
//!
//! Rust shape: config structs implement [`WeakDecode`] (hand-written per struct, field by field,
//! using the helpers below) instead of reflection.

use go_value::{Map, Value};
use nh_common::Result;

/// Implemented by config structs decoded from a `map[string]any`.
pub trait WeakDecode: Sized {
    /// Decode `m` on top of `self` (defaults), like `mapstructure.WeakDecode(m, &self)`.
    fn weak_decode(self, m: &Map) -> Result<Self>;
}

/// Case-insensitive field lookup (mapstructure `MatchName` default: `strings.EqualFold`).
pub fn field<'a>(m: &'a Map, name: &str) -> Option<&'a Value> {
    todo!()
}

/// Weak conversions.
pub fn weak_string(v: &Value) -> Result<String> { todo!() }
pub fn weak_int(v: &Value) -> Result<i64> { todo!() }
pub fn weak_float(v: &Value) -> Result<f64> { todo!() }
pub fn weak_bool(v: &Value) -> Result<bool> { todo!() }
pub fn weak_string_slice(v: &Value) -> Result<Vec<String>> { todo!() }
pub fn weak_duration(v: &Value) -> Result<std::time::Duration> { todo!() }
