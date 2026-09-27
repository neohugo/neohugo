//! Port of `tpl/collections/apply.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/apply.go (162 lines; 1/5 funcs executed)
//    L27-65: (ns *Namespace) Apply(ctx context.Context, c any, fname string, args ...any) (any, error)
//    L67-106: applyFnToThis(ctx context.Context, fn, this reflect.Value, args ...any) (reflect.Value, error)
//    L108-137: (ns *Namespace) lookupFunc(ctx context.Context, fname string) (reflect.Value, bool)
// EX L140-150: indirect(v reflect.Value) (rv reflect.Value, isNil bool)
//    L152-162: indirectInterface(v reflect.Value) (rv reflect.Value, isNil bool)
// ---------------------------------------------------------------------------
