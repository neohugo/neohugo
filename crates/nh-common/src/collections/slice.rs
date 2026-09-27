//! Port of `common/collections/slice.go`.
//!
//! Owner: Wave B task T01 (common-values).


use go_value::{HostCtx, Value};

/// Go: `collections.Slicer` is `Slice(items any) (any, error)`, implemented by `*pageState`
/// (-> `page.Pages`), resources (`commonResource.Slice` -> `resource.Resources`), `WeightedPage`,
/// `PageGroup`. In Rust: the first element is an `Object` whose `has_method("Slice")` is true.
pub const SLICER_METHOD: &str = "Slice";

/// Go: `collections.Slice(args ...any) any`: no args -> `[]interface {}`; first arg a Slicer ->
/// its result (or `[]interface{}` on error); all args of the same Go type T -> `[]T`
/// (`[]string`, `[]int`, `[]map[string]interface {}`, or `SliceType::Named("[]"+T)`);
/// otherwise `[]interface {}`.
// Go: common/collections/slice.go:Slice
pub fn slice(ctx: HostCtx<'_>, args: &[Value]) -> Value {
    todo!()
}

/// Go: `collections.StringSliceToInterfaceSlice`.
pub fn string_slice_to_interface_slice(ss: &[go_value::GoString]) -> Vec<Value> {
    ss.iter().map(|s| Value::String(s.clone())).collect()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/slice.go (95 lines; 1/4 funcs executed)
//   types: Slicer, SortedStringSlice
// EX L29-67: Slice(args ...any) any
//    L70-76: StringSliceToInterfaceSlice(ss []string) []any
//    L81-84: (ss SortedStringSlice) Contains(s string) bool
//    L87-95: (ss SortedStringSlice) Count(s string) int
// ---------------------------------------------------------------------------
