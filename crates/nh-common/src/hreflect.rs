//! Port of `common/hreflect/helpers.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! Truthiness, kind and type helpers over [`go_value::Value`] (Go: `common/hreflect/helpers.go`),
//! plus the small part of Go's `reflect` type system that the ported collection code needs
//! (`reflect.TypeOf`, `Type.Elem`, `Type.AssignableTo`, `reflect.SliceOf`, interface
//! implementation).
//!
//! Types are identified by their Go type string (`%T` spelling), as everywhere in the value
//! model. Two registries describe the named types that other crates own:
//! - [`register_named_elem`]: the element type of a named slice or map type (`page.Pages` ->
//!   `page.Page`); the neohugo types that exist in the Go code are registered by default;
//! - [`register_interface`]: which dynamic types implement a named Go interface (`page.Page`,
//!   `resource.Resource`, ...), as a predicate over type strings, registered by the owning crate.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use go_value::{IntKind, Kind, MapType, NilKind, SliceType, UintKind, Value, typed_nil_kind};

use crate::htime::LocationRef;

// ---------------------------------------------------------------------------
// Kinds

/// Go `reflect.Kind` of a value (the subset the value model can hold).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReflectKind {
    Invalid,
    Bool,
    Int(IntKind),
    Uint(UintKind),
    Float32,
    Float64,
    String,
    Slice,
    Map,
    Struct,
    Ptr,
    Interface,
    Func,
    Chan,
}

/// Go: `reflect.ValueOf(v).Kind()`. A nil interface (`Invalid`, or a `TypedNil` of interface
/// kind, which becomes an untyped nil when converted to `any`) is `Invalid`. Objects report
/// their [`go_value::Object::underlying`] kind when they are named basic types, else their
/// [`go_value::Object::kind`].
pub fn kind_of(v: &Value) -> ReflectKind {
    match v {
        Value::Invalid => ReflectKind::Invalid,
        Value::TypedNil(t) => match typed_nil_kind(t) {
            NilKind::Ptr => ReflectKind::Ptr,
            NilKind::Slice => ReflectKind::Slice,
            NilKind::Map => ReflectKind::Map,
            NilKind::Func => ReflectKind::Func,
            NilKind::Chan => ReflectKind::Chan,
            NilKind::Interface => ReflectKind::Invalid,
        },
        Value::Bool(_) => ReflectKind::Bool,
        Value::Int(_, k) => ReflectKind::Int(*k),
        Value::Uint(_, k) => ReflectKind::Uint(*k),
        Value::Float(_, go_value::FloatKind::F32) => ReflectKind::Float32,
        Value::Float(_, go_value::FloatKind::F64) => ReflectKind::Float64,
        Value::String(_) | Value::Safe(..) => ReflectKind::String,
        Value::Time(_) => ReflectKind::Struct,
        Value::List(_) => ReflectKind::Slice,
        Value::Map(_) => ReflectKind::Map,
        Value::Object(o) => {
            if let Some(u) = o.underlying() {
                match u {
                    Value::Bool(_)
                    | Value::Int(..)
                    | Value::Uint(..)
                    | Value::Float(..)
                    | Value::String(_) => return kind_of(&u),
                    _ => {}
                }
            }
            match o.kind() {
                Kind::Ptr => ReflectKind::Ptr,
                Kind::Struct => ReflectKind::Struct,
                Kind::Map => ReflectKind::Map,
                Kind::Slice => ReflectKind::Slice,
                Kind::Func => ReflectKind::Func,
                Kind::Interface => ReflectKind::Interface,
            }
        }
    }
}

// Go: common/hreflect/helpers.go:IsNumber
/// IsNumber returns whether the given kind is a number.
pub fn is_number_kind(kind: ReflectKind) -> bool {
    is_int_kind(kind) || is_uint_kind(kind) || is_float_kind(kind)
}

// Go: common/hreflect/helpers.go:IsInt
/// IsInt returns whether the given kind is an int.
pub fn is_int_kind(kind: ReflectKind) -> bool {
    matches!(kind, ReflectKind::Int(_))
}

// Go: common/hreflect/helpers.go:IsUint
/// IsUint returns whether the given kind is an uint (Go's list does not include `Uintptr`).
pub fn is_uint_kind(kind: ReflectKind) -> bool {
    matches!(kind, ReflectKind::Uint(k) if k != UintKind::Uintptr)
}

// Go: common/hreflect/helpers.go:IsFloat
/// IsFloat returns whether the given kind is a float.
pub fn is_float_kind(kind: ReflectKind) -> bool {
    matches!(kind, ReflectKind::Float32 | ReflectKind::Float64)
}

/// Go: `hreflect.IsNumber(reflect.ValueOf(v).Kind())`.
pub fn is_number(v: &Value) -> bool {
    is_number_kind(kind_of(v))
}

// ---------------------------------------------------------------------------
// Truthiness

// Go: common/hreflect/helpers.go:IsTruthful
/// IsTruthful returns whether `v` represents a truthful value (see [`is_truthful_value`]).
pub fn is_truthful(v: &Value) -> bool {
    is_truthful_value(v)
}

// Go: common/hreflect/helpers.go:IsTruthfulValue
/// IsTruthfulValue: Go's `template.IsTrue`, but `types.Zeroer` (`IsZero`) is consulted first:
/// 1. `Invalid` (and nil interfaces) -> false.
/// 2. Zeroers: `Object::is_zero`, `time.Time`, `maps.Params` (empty or only `_merge`).
/// 3. By kind: string/slice/map -> len > 0; bool; numbers != 0; nil pointers/funcs/chans ->
///    false; non-nil pointers and structs -> true. Named basic types use their `underlying`.
///
/// A typed nil pointer is false: of the Go types with an `IsZero` method only `*source.File`
/// has a pointer receiver, and it reports nil as zero.
pub fn is_truthful_value(v: &Value) -> bool {
    match v {
        // Something like var x interface{}, never set. It's a form of nil.
        Value::Invalid => false,
        // Nil pointers, funcs, chans and interfaces are false; nil slices and maps have length
        // 0; a nil maps.Params IsZero.
        Value::TypedNil(_) => false,
        Value::Bool(b) => *b,
        Value::Int(i, _) => *i != 0,
        Value::Uint(u, _) => *u != 0,
        Value::Float(f, _) => *f != 0.0,
        Value::String(s) | Value::Safe(_, s) => !s.is_empty(),
        // time.Time implements types.Zeroer.
        Value::Time(t) => !t.is_zero(),
        Value::List(l) => !l.is_empty(),
        Value::Map(m) => match m.ty {
            // maps.Params implements types.Zeroer.
            MapType::Params => !crate::maps::params::params_is_zero(m),
            _ => !m.is_empty(),
        },
        Value::Object(o) => {
            if let Some(z) = o.is_zero() {
                return !z;
            }
            if let Some(u) = o.underlying() {
                match u {
                    Value::Bool(_)
                    | Value::Int(..)
                    | Value::Uint(..)
                    | Value::Float(..)
                    | Value::String(_) => return is_truthful_value(&u),
                    _ => {}
                }
            }
            match o.kind() {
                Kind::Slice => o.list().is_some_and(|l| !l.is_empty()),
                Kind::Map => !o.map_keys().is_empty(),
                // Non-nil pointers, funcs and interfaces; struct values are always true.
                Kind::Ptr | Kind::Func | Kind::Interface | Kind::Struct => true,
            }
        }
    }
}

// Go: common/hreflect/helpers.go:IsSlice
/// IsSlice reports whether v is a slice (`reflect.ValueOf(v).Kind() == reflect.Slice`).
pub fn is_slice(v: &Value) -> bool {
    kind_of(v) == ReflectKind::Slice
}

// Go: common/hreflect/helpers.go:IsMap
/// IsMap reports whether v is a map.
pub fn is_map(v: &Value) -> bool {
    kind_of(v) == ReflectKind::Map
}

/// Go: `types.IsNil(v)` — untyped nil or typed nil.
pub fn is_nil(v: &Value) -> bool {
    crate::types::types::is_nil(v)
}

// Go: common/hreflect/helpers.go:IsValid
/// IsValid returns whether v is not nil and a valid value.
pub fn is_valid(v: &Value) -> bool {
    !matches!(v, Value::Invalid | Value::TypedNil(_))
}

// ---------------------------------------------------------------------------
// Methods

// Go: common/hreflect/helpers.go:GetMethodByName
/// `GetMethodByName(v, name).IsValid()`: whether an object value has the exported method `name`.
/// Methods of named slice/map types are looked up in the template layer's
/// [`crate::object::NamedTypeRegistry`] instead (Go reaches both through the same reflect call).
pub fn has_method_by_name(v: &Value, name: &str) -> bool {
    match v {
        Value::Object(o) => o.has_method(name),
        _ => false,
    }
}

// Go: common/hreflect/helpers.go:CallMethodByName
/// CallMethodByName calls the (context-taking or argument-less) method `name` of an object.
/// In the value model the context is always passed separately, so this is `call_method(ctx,
/// name, [])`. Go panics when the method does not exist; this returns `None`.
pub fn call_method_by_name(
    ctx: go_value::HostCtx<'_>,
    name: &str,
    v: &Value,
) -> Option<go_value::Result<Value>> {
    match v {
        Value::Object(o) => o.call_method(ctx, name, &[]),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Time

// Go: common/hreflect/helpers.go:IsTime
/// IsTime returns whether v's type is `time.Time` or implements `htime.AsTimeProvider`
/// (an object with an `AsTime` method, e.g. go-toml's local dates).
pub fn is_time(v: &Value) -> bool {
    match v {
        Value::Time(_) => true,
        Value::Object(o) => o.has_method("AsTime"),
        _ => false,
    }
}

// Go: common/hreflect/helpers.go:AsTime
/// AsTime returns v as a `time.Time` if possible. The location is only used when the value
/// implements `AsTimeProvider`: its `AsTime` method is called with one argument, a
/// [`LocationRef`] object (`*time.Location`). Strings are not accepted.
pub fn as_time(v: &Value, loc: &Arc<go_value::Location>) -> Option<go_value::Time> {
    match v {
        Value::Time(t) => Some(t.clone()),
        Value::Object(o) if o.has_method("AsTime") => {
            let arg = Value::object(LocationRef(loc.clone()));
            match o.call_method(&(), "AsTime", &[arg]) {
                Some(Ok(Value::Time(t))) => Some(t),
                _ => None,
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Slices

// Go: common/hreflect/helpers.go:ToSliceAny
/// ToSliceAny converts the given value to a slice of any if possible.
pub fn to_slice_any(v: &Value) -> Option<Vec<Value>> {
    match v {
        Value::Invalid => None,
        Value::List(l) => Some(l.items.clone()),
        Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Slice => Some(Vec::new()),
        Value::Object(o) if o.kind() == Kind::Slice => Some(o.list().unwrap_or_default()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Types (reflect.Type emulation by Go type string)

type ElemRegistry = RwLock<HashMap<String, String>>;

fn elem_registry() -> &'static ElemRegistry {
    static REG: OnceLock<ElemRegistry> = OnceLock::new();
    REG.get_or_init(|| {
        // The named slice/map types declared in the neohugo Go code (element types from their
        // declarations, e.g. `type Pages []Page`).
        let defaults: &[(&str, &str)] = &[
            ("page.Pages", "page.Page"),
            ("page.PagesGroup", "page.PageGroup"),
            ("page.WeightedPages", "page.WeightedPage"),
            ("page.OrderedTaxonomy", "page.OrderedTaxonomyEntry"),
            ("page.OutputFormats", "page.OutputFormat"),
            ("page.Sites", "page.Site"),
            ("resource.Resources", "resource.Resource"),
            ("langs.Languages", "*langs.Language"),
            ("output.Formats", "output.Format"),
            ("media.Types", "media.Type"),
            ("navigation.Menu", "*navigation.MenuEntry"),
            ("tableofcontents.Headings", "*tableofcontents.Heading"),
            ("hooks.TableRow", "hooks.TableCell"),
            ("collections.SortedStringSlice", "string"),
            ("json.RawMessage", "uint8"),
            ("maps.Params", "interface {}"),
            ("page.Data", "interface {}"),
            ("page.Taxonomy", "page.WeightedPages"),
            ("page.TaxonomyList", "page.Taxonomy"),
            ("page.AuthorList", "page.Author"),
            ("page.AuthorSocial", "string"),
            ("navigation.Menus", "navigation.Menu"),
            ("navigation.PageMenus", "*navigation.MenuEntry"),
            ("exif.Tags", "interface {}"),
            ("attributes.Attributes", "interface {}"),
        ];
        RwLock::new(
            defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    })
}

/// Declares the element type of a named slice or map type (Go `reflect.Type.Elem()`), e.g.
/// `register_named_elem("page.Pages", "page.Page")`. The neohugo named types are registered by
/// default; a crate that adds another named slice/map type registers it (and its kind with
/// [`go_value::register_named_kind`]).
pub fn register_named_elem(named: &str, elem: &str) {
    elem_registry()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(named.to_string(), elem.to_string());
}

type InterfacePred = fn(type_name: &str) -> bool;

fn interface_registry() -> &'static RwLock<HashMap<String, InterfacePred>> {
    static REG: OnceLock<RwLock<HashMap<String, InterfacePred>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Declares which dynamic Go types implement the named interface `iface` (e.g. `page.Page`,
/// `resource.Resource`), as a predicate over Go type strings. Implementation is a property of
/// the type, as in Go; the predicate must also answer for interface types that embed `iface`
/// (a `page.Page` value is a `resource.Resource`).
pub fn register_interface(iface: &str, implemented_by: fn(type_name: &str) -> bool) {
    interface_registry()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(iface.to_string(), implemented_by);
}

/// Whether the Go type `t` implements the interface `iface`: `interface {}` is implemented by
/// every type, an interface by itself, registered interfaces by their predicate.
pub fn type_implements(t: &str, iface: &str) -> bool {
    if iface == "interface {}" || t == iface {
        return true;
    }
    let pred = interface_registry()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(iface)
        .copied();
    pred.is_some_and(|p| p(t))
}

/// Whether the Go type `t` is an interface type: `interface {...}` literals, `error`, and the
/// registered named interfaces.
pub fn is_interface_type(t: &str) -> bool {
    t.starts_with("interface {")
        || t == "error"
        || interface_registry()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(t)
}

// Go: reflect.TypeOf
/// Go `reflect.TypeOf(v)` as a type string; `None` for nil (`Invalid`, or a `TypedNil` of
/// interface kind, which is an untyped nil once converted to `any`).
pub fn type_of(v: &Value) -> Option<Cow<'_, str>> {
    match v {
        Value::Invalid => None,
        Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Interface => None,
        _ => Some(v.go_type_name()),
    }
}

/// Whether the Go type `t` has slice kind.
pub fn is_slice_type(t: &str) -> bool {
    t.starts_with("[]") || typed_nil_kind(t) == NilKind::Slice
}

// Go: reflect.Type.Elem (slices and maps)
/// The element type of a slice or map type (`[]T` -> `T`, `map[K]V` -> `V`, registered named
/// types), `None` when unknown.
pub fn elem_type(t: &str) -> Option<String> {
    if let Some(e) = t.strip_prefix("[]") {
        return Some(e.to_string());
    }
    if let Some(rest) = t.strip_prefix("map[") {
        // Map keys are string or interface {} in the value model.
        for k in ["string]", "interface {}]"] {
            if let Some(v) = rest.strip_prefix(k) {
                return Some(v.to_string());
            }
        }
        return None;
    }
    elem_registry()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(t)
        .cloned()
}

/// The element type of a slice value's type.
pub fn slice_elem_type(ty: &SliceType) -> String {
    match ty {
        SliceType::Any => "interface {}".to_string(),
        SliceType::String => "string".to_string(),
        SliceType::Int => "int".to_string(),
        SliceType::Int64 => "int64".to_string(),
        SliceType::Float64 => "float64".to_string(),
        SliceType::Bool => "bool".to_string(),
        SliceType::Uint8 => "uint8".to_string(),
        SliceType::MapStringAny => "map[string]interface {}".to_string(),
        SliceType::Named(n) => elem_type(n).unwrap_or_else(|| "interface {}".to_string()),
    }
}

/// The [`SliceType`] of the Go slice type string `t`.
pub fn slice_type_from_name(t: &str) -> SliceType {
    match t {
        "[]interface {}" => SliceType::Any,
        "[]string" => SliceType::String,
        "[]int" => SliceType::Int,
        "[]int64" => SliceType::Int64,
        "[]float64" => SliceType::Float64,
        "[]bool" => SliceType::Bool,
        "[]uint8" => SliceType::Uint8,
        "[]map[string]interface {}" => SliceType::MapStringAny,
        _ => SliceType::Named(Arc::from(t)),
    }
}

/// The [`MapType`] of the Go map type string `t`.
pub fn map_type_from_name(t: &str) -> MapType {
    match t {
        "map[string]interface {}" => MapType::StringAny,
        "map[string]string" => MapType::StringString,
        "maps.Params" => MapType::Params,
        _ => MapType::Named(Arc::from(t)),
    }
}

// Go: reflect.SliceOf
/// Go `reflect.SliceOf(elem)`: the unnamed slice type `[]elem`.
pub fn slice_of(elem: &str) -> SliceType {
    slice_type_from_name(&format!("[]{elem}"))
}

/// Whether `t` is a named (defined or predeclared) type. Unnamed types are the composite type
/// literals.
fn is_named_type(t: &str) -> bool {
    !(t.starts_with("[]")
        || t.starts_with("map[")
        || t.starts_with('*')
        || t.starts_with("func(")
        || t.starts_with("chan ")
        || t.starts_with("<-chan ")
        || t.starts_with("struct {")
        || t.starts_with("interface {"))
}

/// The underlying type of `t`, as far as the value model can tell: named slice/map types from
/// their registered element types; every other type is its own underlying type.
fn underlying_type(t: &str) -> Cow<'_, str> {
    if is_named_type(t) {
        match typed_nil_kind(t) {
            NilKind::Slice => {
                if let Some(e) = elem_type(t) {
                    return Cow::Owned(format!("[]{e}"));
                }
            }
            NilKind::Map => {
                if let Some(e) = elem_type(t) {
                    return Cow::Owned(format!("map[string]{e}"));
                }
            }
            _ => {}
        }
    }
    Cow::Borrowed(t)
}

// Go: reflect.Type.AssignableTo
/// Go `V.AssignableTo(T)` for type strings: identical types; `T` an interface that `V`
/// implements; or identical underlying types where at least one of the two is unnamed (e.g.
/// `maps.Params` and `map[string]interface {}`).
pub fn type_assignable_to(v: &str, t: &str) -> bool {
    if v == t {
        return true;
    }
    if is_interface_type(t) {
        return type_implements(v, t);
    }
    (!is_named_type(v) || !is_named_type(t)) && underlying_type(v) == underlying_type(t)
}

/// `reflect.ValueOf(v).Type().AssignableTo(t)`; false for nil.
pub fn assignable_to(v: &Value, t: &str) -> bool {
    match type_of(v) {
        Some(vt) => type_assignable_to(&vt, t),
        None => false,
    }
}

// Go: reflect.Value.Set (Value.assignTo)
/// The value stored when `v` (assignable to `t`) is set into a slot of type `t`: unchanged for
/// identical types and interfaces, re-typed to `t` for a slice or map of identical underlying
/// type (`maps.Params` stored into a `map[string]interface {}` slot is a
/// `map[string]interface {}`).
pub fn assign_to(v: Value, t: &str) -> Value {
    if is_interface_type(t) || type_of(&v).is_some_and(|vt| vt == t) {
        return v;
    }
    match v {
        Value::Map(m) => {
            let mut m2 = (*m).clone();
            m2.ty = map_type_from_name(t);
            Value::map(m2)
        }
        Value::List(l) => {
            let mut l2 = (*l).clone();
            l2.ty = slice_type_from_name(t);
            Value::List(Arc::new(l2))
        }
        Value::TypedNil(_) => Value::TypedNil(Arc::from(t)),
        other => other,
    }
}

// Go: common/hreflect/helpers.go:indirectInterface
/// The value model has no interface wrapper values: an interface holding a value *is* that
/// value, and a nil interface is `Invalid`. Kept for the checklist; returns `v`.
pub fn indirect_interface(v: &Value) -> &Value {
    v
}

// Go: common/hreflect/helpers.go:IsContextType
/// Methods receive the template context as a separate `HostCtx` parameter; a value is never a
/// `context.Context`. Kept for the checklist.
pub fn is_context_type(_type_name: &str) -> bool {
    false
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hreflect/helpers.go (295 lines; 11/17 funcs executed)
//   types: methodKey, k
// OK L32-34: IsNumber(kind reflect.Kind) bool
// OK L37-44: IsInt(kind reflect.Kind) bool
// OK L47-54: IsUint(kind reflect.Kind) bool
// OK L57-64: IsFloat(kind reflect.Kind) bool
// OK L68-75: IsTruthful(in any) bool
// OK L78-80: IsMap(v any) bool
// OK L83-85: IsSlice(v any) bool
// OK L96-130: IsTruthfulValue(val reflect.Value) (truth bool)
// OK L141-149: GetMethodByName(v reflect.Value, name string) reflect.Value (has_method_by_name; named slice/map types: object::NamedTypeRegistry)
// OK L153-171: GetMethodIndexByName(tp reflect.Type, name string) int (Object::has_method; no cache needed)
// OK L180-189: IsTime(tp reflect.Type) bool
// OK L192-203: IsValid(v reflect.Value) bool
// OK L209-223: AsTime(v reflect.Value, loc *time.Location) (time.Time, bool)
// OK L226-244: ToSliceAny(v any) ([]any, bool)
// OK L246-261: CallMethodByName(cxt context.Context, name string, v reflect.Value) []reflect.Value
// OK L264-272: indirectInterface(v reflect.Value) reflect.Value (identity: no interface wrappers)
// OK L283-295: IsContextType(tp reflect.Type) bool (always false: ctx is passed separately)
// ---------------------------------------------------------------------------
