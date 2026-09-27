//! Port of `tpl/transform/unmarshal.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/transform/unmarshal.go (201 lines; 1/3 funcs executed)
// EX L38-145: (ns *Namespace) Unmarshal(args ...any) (any, error)
//    L147-178: decodeDecoder(m map[string]any) (metadecoders.Decoder, error)
//    L180-201: stringToRune(v any) (rune, error)
// ---------------------------------------------------------------------------
