//! Port of `common/types/convert.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::{GoString, Value};

use crate::herrors::Result;

/// Go: `types.ToStringSlicePreserveString(v)`: a string becomes `[s]` (not split), `[]string`
/// as-is, `[]any` via `cast.ToStringSliceE` (nil -> "", any nested slice/map -> error -> nil).
// Go: common/types/convert.go:ToStringSlicePreserveString
pub fn to_string_slice_preserve_string(v: &Value) -> Vec<GoString> {
    to_string_slice_preserve_string_e(v).unwrap_or_default()
}

// Go: common/types/convert.go:ToStringSlicePreserveStringE
pub fn to_string_slice_preserve_string_e(v: &Value) -> Result<Vec<GoString>> {
    todo!()
}

/// Go: `types.ToString(v)` — `cast.ToString` with `fmt.Stringer` support.
// Go: common/types/convert.go:ToString
pub fn to_string(v: &Value) -> GoString {
    todo!()
}

/// Go: `types.ToDuration`/`ToDurationE`.
pub fn to_duration_e(v: &Value) -> Result<std::time::Duration> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/convert.go (129 lines; 2/7 funcs executed)
//    L28-31: ToDuration(v any) time.Duration
//    L34-43: ToDurationE(v any) (time.Duration, error)
// EX L47-50: ToStringSlicePreserveString(v any) []string
// EX L54-83: ToStringSlicePreserveStringE(v any) ([]string, error)
//    L88-109: TypeToString(v any) (string, bool)
//    L112-115: ToString(v any) string
//    L118-129: ToStringE(v any) (string, error)
// ---------------------------------------------------------------------------
