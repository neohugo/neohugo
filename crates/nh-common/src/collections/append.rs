//! Port of `common/collections/append.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::Value;

use crate::herrors::Result;

/// Go: `collections.Append(to any, from ...any)` — used by `append` and `Scratch.Add`. Keeps and
/// tightens the slice type: `[]interface{}` + map -> `[]map[string]interface {}`,
/// `[]interface{}` + page -> `page.Pages` (via the element's `Slicer`), etc. Port the reflect
/// logic exactly (the result type is visible to `%T`, `jsonify` and later `range`/`sort`).
// Go: common/collections/append.go:Append
pub fn append(ctx: go_value::HostCtx<'_>, to: &Value, from: &[Value]) -> Result<Value> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/append.go (152 lines; 2/4 funcs executed)
// EX L24-110: Append(to any, from ...any) (any, error)
//    L112-126: appendToInterfaceSliceFromValues(slice1, slice2 reflect.Value) ([]any, error)
//    L128-138: appendToInterfaceSlice(tov reflect.Value, from ...any) ([]any, error)
// EX L142-152: indirect(v reflect.Value) (rv reflect.Value, isNil bool)
// ---------------------------------------------------------------------------
