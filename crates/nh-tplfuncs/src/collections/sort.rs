//! Port of `tpl/collections/sort.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/sort.go (197 lines; 5/5 funcs executed)
//   types: pair, pairList
// EX L30-141: (ns *Namespace) Sort(ctx context.Context, l any, args ...any) (any, error)
// EX L160-160: (p pairList) Swap(i, j int)
// EX L161-161: (p pairList) Len() int
// EX L162-182: (p pairList) Less(i, j int) bool
// EX L185-197: (p pairList) sort() any
// ---------------------------------------------------------------------------
