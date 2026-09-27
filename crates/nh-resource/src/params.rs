//! Port of `resources/resource/params.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use go_value::{Map, Value};
use nh_common::Result;

/// Go: `resource.Param(r, fallback, key)` — case-insensitive nested lookup in `r.Params()`, then fallback.
// Go: resources/resource/params.go:Param
pub fn param(params: &Map, fallback: Option<&Map>, key: &Value) -> Result<Value> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/params.go (33 lines; 1/1 funcs executed)
// EX L22-33: Param(r ResourceParamsProvider, fallback maps.Params, key any) (any, error)
// ---------------------------------------------------------------------------
