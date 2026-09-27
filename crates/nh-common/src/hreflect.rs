//! Port of `common/hreflect/helpers.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! Truthiness and kind helpers over [`go_value::Value`] (Go: `common/hreflect/helpers.go`).

use go_value::{Kind, MapType, Value};

/// Go: `hreflect.IsTruthful(v any)` / `IsTruthfulValue` — used by `if`, `with`, `and`, `or`,
/// `not` and `default`:
/// 1. `Invalid` -> false; typed nil -> the type's `IsZero` if it has one (e.g. nil `*source.File`), else false.
/// 2. `types.Zeroer` first: `Object::is_zero()`, `time.Time` zero, `maps.Params` empty-or-only-`_merge`.
/// 3. By kind: string/slice/map -> len > 0; bool; numbers != 0; objects (pointers/structs) -> true.
// Go: common/hreflect/helpers.go:IsTruthfulValue
pub fn is_truthful(v: &Value) -> bool {
    match v {
        Value::Invalid => false,
        Value::TypedNil(_) => false,
        Value::Bool(b) => *b,
        Value::Int(i, _) => *i != 0,
        Value::Uint(u, _) => *u != 0,
        Value::Float(f, _) => *f != 0.0,
        Value::String(s) | Value::Safe(_, s) => !s.is_empty(),
        Value::Time(t) => !t.is_zero(),
        Value::List(l) => !l.is_empty(),
        Value::Map(m) => match m.ty {
            MapType::Params => !crate::maps::params::params_is_zero(m),
            _ => !m.is_empty(),
        },
        Value::Object(o) => match o.is_zero() {
            Some(z) => !z,
            None => match o.kind() {
                Kind::Slice => o.list().map(|l| !l.is_empty()).unwrap_or(false),
                Kind::Map => !o.map_keys().is_empty(),
                _ => true,
            },
        },
    }
}

/// Go: `hreflect.IsSlice(v)` — `reflect.ValueOf(v).Kind() == reflect.Slice`.
// Go: common/hreflect/helpers.go:IsSlice
pub fn is_slice(v: &Value) -> bool {
    match v {
        Value::List(_) => true,
        Value::Object(o) => o.kind() == Kind::Slice,
        _ => false,
    }
}

/// Go: `hreflect.IsMap`.
pub fn is_map(v: &Value) -> bool {
    match v {
        Value::Map(_) => true,
        Value::Object(o) => o.kind() == Kind::Map,
        _ => false,
    }
}

/// Go: `hreflect.IsNumber` (int, uint or float kinds).
pub fn is_number(v: &Value) -> bool {
    matches!(v, Value::Int(..) | Value::Uint(..) | Value::Float(..))
}

/// Go: `types.IsNil(v)` — untyped nil or typed nil.
pub fn is_nil(v: &Value) -> bool {
    v.is_nil()
}

/// Go: `hreflect.AsTime(v, loc)` — `time.Time` or a value with `AsTime(loc)` (go-toml local dates).
// Go: common/hreflect/helpers.go:AsTime
pub fn as_time(v: &Value, loc: &std::sync::Arc<go_value::Location>) -> Option<go_value::Time> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hreflect/helpers.go (295 lines; 11/17 funcs executed)
//   types: methodKey, k
//    L32-34: IsNumber(kind reflect.Kind) bool
// EX L37-44: IsInt(kind reflect.Kind) bool
// EX L47-54: IsUint(kind reflect.Kind) bool
// EX L57-64: IsFloat(kind reflect.Kind) bool
//    L68-75: IsTruthful(in any) bool
//    L78-80: IsMap(v any) bool
// EX L83-85: IsSlice(v any) bool
// EX L96-130: IsTruthfulValue(val reflect.Value) (truth bool)
// EX L141-149: GetMethodByName(v reflect.Value, name string) reflect.Value
// EX L153-171: GetMethodIndexByName(tp reflect.Type, name string) int
// EX L180-189: IsTime(tp reflect.Type) bool
//    L192-203: IsValid(v reflect.Value) bool
// EX L209-223: AsTime(v reflect.Value, loc *time.Location) (time.Time, bool)
//    L226-244: ToSliceAny(v any) ([]any, bool)
//    L246-261: CallMethodByName(cxt context.Context, name string, v reflect.Value) []reflect.Value
// EX L264-272: indirectInterface(v reflect.Value) reflect.Value
// EX L283-295: IsContextType(tp reflect.Type) bool
// ---------------------------------------------------------------------------
