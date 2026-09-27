//! Port of `tpl/collections/merge.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/merge.go (143 lines; 0/4 funcs executed)
//    L30-46: (ns *Namespace) Merge(params ...any) (any, error)
//    L49-69: (ns *Namespace) merge(src, dst any) (any, error)
//    L71-89: caseInsensitiveLookup(m, k reflect.Value) (reflect.Value, bool)
//    L91-143: mergeMap(dst, src reflect.Value) reflect.Value
// ---------------------------------------------------------------------------
