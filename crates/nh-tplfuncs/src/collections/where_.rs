//! Port of `tpl/collections/where.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/where.go (543 lines; 5/11 funcs executed)
// EX L29-56: (ns *Namespace) Where(ctx context.Context, c, key any, args ...any) (any, error)
// EX L58-294: (ns *Namespace) checkCondition(v, mv reflect.Value, op string) (bool, error)
// EX L296-376: evaluateSubElem(ctx, obj reflect.Value, elemName string) (reflect.Value, error)
// EX L380-396: parseWhereArgs(args ...any) (mv reflect.Value, op string, err error)
// EX L400-442: (ns *Namespace) checkWhereArray(ctxv, seqv, kv, mv reflect.Value, path []string, op string) (any, error)
//    L445-489: (ns *Namespace) checkWhereMap(ctxv, seqv, kv, mv reflect.Value, path []string, op string) (any, error)
//    L492-502: toFloat(v reflect.Value) (float64, error)
//    L506-514: toInt(v reflect.Value) (int64, error)
//    L516-524: toUint(v reflect.Value) (uint64, error)
//    L527-535: toString(v reflect.Value) (string, error)
//    L537-543: (ns *Namespace) toTimeUnix(v reflect.Value) int64
// ---------------------------------------------------------------------------
