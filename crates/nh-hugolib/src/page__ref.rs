//! Port of `hugolib/page__ref.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

// Wave B: port per the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__ref.go (114 lines; 4/8 funcs executed)
//   types: pageRef, refArgs
// EX L24-26: newPageRef(p *pageState) pageRef
//    L32-34: (p pageRef) Ref(argsm map[string]any) (string, error)
// EX L36-38: (p pageRef) RefFrom(argsm map[string]any, source any) (string, error)
//    L40-42: (p pageRef) RelRef(argsm map[string]any) (string, error)
//    L44-46: (p pageRef) RelRefFrom(argsm map[string]any, source any) (string, error)
// EX L48-74: (p pageRef) decodeRefArgs(args map[string]any) (refArgs, *Site, error)
// EX L76-91: (p pageRef) ref(argsm map[string]any, source any) (string, error)
//    L93-108: (p pageRef) relRef(argsm map[string]any, source any) (string, error)
// ---------------------------------------------------------------------------
