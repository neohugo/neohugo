//! Port of `tpl/collections/querify.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/querify.go (125 lines; 0/4 funcs executed)
//    L31-64: (ns *Namespace) Querify(params ...any) (string, error)
//    L69-86: mapToQueryString[T map[string]any | maps.Params](m T) (string, error)
//    L91-107: stringSliceToQueryString(s []string) (string, error)
//    L111-125: interfaceSliceToStringSlice(s []any) ([]string, error)
// ---------------------------------------------------------------------------
