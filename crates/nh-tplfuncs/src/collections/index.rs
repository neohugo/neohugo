//! Port of `tpl/collections/index.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/index.go (144 lines; 3/4 funcs executed)
// EX L33-39: (ns *Namespace) Index(item any, args ...any) (any, error)
// EX L41-116: (ns *Namespace) doIndex(item any, args ...any) (any, error)
// EX L122-133: prepareArg(value reflect.Value, argType reflect.Type) (reflect.Value, error)
//    L138-144: canBeNil(typ reflect.Type) bool
// ---------------------------------------------------------------------------
