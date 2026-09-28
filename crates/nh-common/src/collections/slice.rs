//! Port of `common/collections/slice.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::sync::Arc;

use go_value::{GoString, HostCtx, List, Value};

use crate::hreflect::{slice_of, type_of};

/// Go: `collections.Slicer` is `Slice(items any) (any, error)`, implemented by `*pageState`
/// (-> `page.Pages`), resources (`commonResource.Slice` -> `resource.Resources`), `WeightedPage`,
/// `PageGroup`. In Rust: an `Object` whose `has_method("Slice")` is true; it is called with one
/// argument, the `[]interface {}` of all items.
pub const SLICER_METHOD: &str = "Slice";

// Go: common/collections/slice.go:Slice
/// Slice returns a slice of all passed arguments: no args -> `[]interface {}`; first arg a
/// Slicer -> its result (or `[]interface {}` on error); all args of the same Go type T -> `[]T`
/// (`[]string`, `[]int`, `[]map[string]interface {}`, or `SliceType::Named("[]"+T)`); otherwise
/// `[]interface {}`.
pub fn slice(ctx: HostCtx<'_>, args: &[Value]) -> Value {
    let as_args = || Value::any_list(args.to_vec());
    if args.is_empty() {
        return as_args();
    }

    let first = &args[0];
    let Some(first_type) = type_of(first) else {
        return as_args();
    };

    if let Value::Object(o) = first
        && o.has_method(SLICER_METHOD)
    {
        return match o.call_method(ctx, SLICER_METHOD, &[as_args()]) {
            Some(Ok(v)) => v,
            // If Slice fails, the items are not of the same type and
            // []interface{} is the best we can do.
            _ => as_args(),
        };
    }

    if args.len() > 1 {
        // This can be a mix of types.
        for arg in &args[1..] {
            if type_of(arg).as_deref() != Some(&*first_type) {
                // []interface{} is the best we can do
                return as_args();
            }
        }
    }

    Value::List(Arc::new(List::new(slice_of(&first_type), args.to_vec())))
}

// Go: common/collections/slice.go:StringSliceToInterfaceSlice
/// StringSliceToInterfaceSlice converts ss to []interface{}.
pub fn string_slice_to_interface_slice(ss: &[GoString]) -> Vec<Value> {
    ss.iter().map(|s| Value::String(s.clone())).collect()
}

/// Go: `collections.SortedStringSlice` — a sorted `[]string` with binary-search lookups.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SortedStringSlice(pub Vec<GoString>);

impl SortedStringSlice {
    // Go: common/collections/slice.go:Contains
    /// Contains returns true if s is in ss.
    pub fn contains(&self, s: &[u8]) -> bool {
        let i = go_sort::sort::search_strings(&self.0, s);
        i < self.0.len() && self.0[i].as_bytes() == s
    }

    // Go: common/collections/slice.go:Count
    /// Count returns the number of times s is in ss.
    pub fn count(&self, s: &[u8]) -> usize {
        let mut count = 0;
        let mut i = go_sort::sort::search_strings(&self.0, s);
        while i < self.0.len() && self.0[i].as_bytes() == s {
            count += 1;
            i += 1;
        }
        count
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/slice.go (95 lines; 1/4 funcs executed)
//   types: Slicer, SortedStringSlice
// OK L29-67: Slice(args ...any) any
// OK L70-76: StringSliceToInterfaceSlice(ss []string) []any
// OK L81-84: (ss SortedStringSlice) Contains(s string) bool
// OK L87-95: (ss SortedStringSlice) Count(s string) int
// ---------------------------------------------------------------------------
