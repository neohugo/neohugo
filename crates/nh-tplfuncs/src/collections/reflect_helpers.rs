//! Port of `tpl/collections/reflect_helpers.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/reflect_helpers.go (216 lines; 0/10 funcs executed)
//    L31-44: numberToFloat(v reflect.Value) (float64, error)
//    L49-67: normalize(v reflect.Value) any
//    L71-92: collectIdentities(seqs ...any) (map[any]bool, error)
//    L96-109: convertValue(v reflect.Value, to reflect.Type) (reflect.Value, error)
//    L114-168: convertNumber(v reflect.Value, to reflect.Kind) (reflect.Value, error)
//    L170-185: newSliceElement(items any) any
//    L187-189: isNumber(kind reflect.Kind) bool
//    L191-198: isInt(kind reflect.Kind) bool
//    L200-207: isUint(kind reflect.Kind) bool
//    L209-216: isFloat(kind reflect.Kind) bool
// ---------------------------------------------------------------------------
