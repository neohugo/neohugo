//! Port of `tpl/strings/regexp.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/regexp.go (115 lines; 0/3 funcs executed)
//    L23-44: (ns *Namespace) FindRE(expr string, content any, limit ...any) ([]string, error)
//    L54-73: (ns *Namespace) FindRESubmatch(expr string, content any, limit ...any) ([][]string, error)
//    L78-115: (ns *Namespace) ReplaceRE(pattern, repl, s any, n ...any) (_ string, err error)
// ---------------------------------------------------------------------------
