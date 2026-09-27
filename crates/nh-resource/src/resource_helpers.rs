//! Port of `resources/resource/resource_helpers.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use go_value::{Map, Value};

/// Go: `resource.GetParam(r, key)` / `GetParamToLower` (typed conversions of front-matter params:
/// time -> time, bool, int, float, []string lower-cased when asked ...).
// Go: resources/resource/resource_helpers.go:getParam
pub fn get_param(params: &Map, key: &str, string_to_lower: bool) -> Value {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/resource_helpers.go (76 lines; 0/3 funcs executed)
//    L30-32: GetParam(r Resource, key string) any
//    L36-38: GetParamToLower(r Resource, key string) any
//    L40-76: getParam(r Resource, key string, stringToLower bool) any
// ---------------------------------------------------------------------------
