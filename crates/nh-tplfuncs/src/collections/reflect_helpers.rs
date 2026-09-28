//! Port of `tpl/collections/reflect_helpers.go`, plus the small `reflect` emulation the
//! collection functions share (`indirect`, static element kinds, zero values, Go's
//! `map[any]bool` keys).
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! The value model has no interface wrapper values: an element of a `[]interface {}` (or of a
//! `page.Pages`, whose element type is the `page.Page` interface) *is* its dynamic value. Where Go
//! looks at the STATIC kind of an element (`l1v.Index(i).Kind() == reflect.Interface`), the port
//! asks [`elem_is_interface`] with the slice's element type.

use std::sync::Arc;

use go_value::{GoString, Kind, List, NilKind, SliceType, Value, typed_nil_kind};
use nh_common::hreflect::{self, ReflectKind};
use nh_common::object::GoResult;

/// Go `reflect.Kind.String()`.
pub(crate) fn kind_name(k: ReflectKind) -> &'static str {
    match k {
        ReflectKind::Invalid => "invalid",
        ReflectKind::Bool => "bool",
        ReflectKind::Int(k) => k.go_name(),
        ReflectKind::Uint(k) => k.go_name(),
        ReflectKind::Float32 => "float32",
        ReflectKind::Float64 => "float64",
        ReflectKind::String => "string",
        ReflectKind::Slice => "slice",
        ReflectKind::Map => "map",
        ReflectKind::Struct => "struct",
        ReflectKind::Ptr => "ptr",
        ReflectKind::Interface => "interface",
        ReflectKind::Func => "func",
        ReflectKind::Chan => "chan",
    }
}

/// `reflect.ValueOf(v).Kind()`.
pub(crate) fn kind(v: &Value) -> ReflectKind {
    hreflect::kind_of(v)
}

// Go: tpl/collections/apply.go:indirect
/// Go `indirect` (borrowed from text/template): follows pointers and interfaces. In the value
/// model a non-nil pointer object stands for its pointee as well, so the value is returned
/// unchanged and `isNil` is true only for a nil pointer (a nil interface is `Invalid`, which Go
/// returns unchanged with `isNil` false).
pub(crate) fn indirect(v: &Value) -> (Value, bool) {
    match v {
        Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Ptr => (v.clone(), true),
        _ => (v.clone(), false),
    }
}

/// The kind of `v` after [`indirect`]: a non-nil pointer object is its pointee (a struct).
pub(crate) fn indirect_kind(v: &Value) -> ReflectKind {
    match kind(v) {
        ReflectKind::Ptr if !v.is_nil() => ReflectKind::Struct,
        k => k,
    }
}

// Go: tpl/collections/apply.go:indirectInterface
/// Go `indirectInterface`: unwraps interfaces. No-op in the value model (see the module doc);
/// `isNil` is true for a nil interface element (`Invalid`).
pub(crate) fn indirect_interface(v: &Value) -> (Value, bool) {
    (v.clone(), v.is_invalid())
}

/// A slice-like value: its Go type and elements. `Value::List`, a typed nil slice, or a
/// `Kind::Slice` object.
pub(crate) struct SliceVal {
    pub ty: SliceType,
    pub items: Vec<Value>,
    /// A nil slice (Go `v.IsNil()` for a slice).
    pub nil: bool,
}

impl SliceVal {
    pub fn elem_type(&self) -> String {
        hreflect::slice_elem_type(&self.ty)
    }

    /// A value of this slice type with the given elements.
    pub fn with(&self, items: Vec<Value>) -> Value {
        Value::List(Arc::new(List::new(self.ty.clone(), items)))
    }

    /// Go `v.Slice(i, j).Interface()`: a nil slice stays nil.
    pub fn sub(&self, i: usize, j: usize) -> Value {
        if self.nil {
            return Value::TypedNil(Arc::from(&*self.ty.go_name()));
        }
        self.with(self.items[i..j].to_vec())
    }
}

/// The slice behind `v`, if its kind is Slice.
pub(crate) fn as_slice(v: &Value) -> Option<SliceVal> {
    match v {
        Value::List(l) => Some(SliceVal {
            ty: l.ty.clone(),
            items: l.items.clone(),
            nil: false,
        }),
        Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Slice => Some(SliceVal {
            ty: hreflect::slice_type_from_name(t),
            items: Vec::new(),
            nil: true,
        }),
        Value::Object(o) if o.kind() == Kind::Slice => Some(SliceVal {
            ty: SliceType::Named(Arc::from(&*o.type_name())),
            items: o.list().unwrap_or_default(),
            nil: false,
        }),
        _ => None,
    }
}

/// Whether the static element type of a slice is an interface type (`interface {}`,
/// `page.Page`, `resource.Resource`, ...): Go's `v.Index(i).Kind() == reflect.Interface`.
pub(crate) fn elem_is_interface(elem: &str) -> bool {
    hreflect::is_interface_type(elem)
}

/// Whether Go's `reflect.Type.Comparable()` holds for the Go type `t` (as far as the type
/// string tells: slices, maps and funcs are not comparable).
pub(crate) fn type_comparable(t: &str) -> bool {
    !(t.starts_with("[]")
        || t.starts_with("map[")
        || t.starts_with("func(")
        || matches!(
            typed_nil_kind(t),
            NilKind::Slice | NilKind::Map | NilKind::Func
        ) && !t.starts_with('*'))
}

/// `v.Type().Comparable()` of a dynamic value.
pub(crate) fn value_comparable(v: &Value) -> bool {
    match v {
        Value::Invalid => true,
        Value::List(_) | Value::Map(_) => false,
        Value::TypedNil(t) => type_comparable(t),
        Value::Object(o) => !matches!(o.kind(), Kind::Slice | Kind::Map | Kind::Func),
        _ => true,
    }
}

/// The static kind of a slice element: `Interface` when the element type is an interface,
/// else the element's own kind.
pub(crate) fn static_elem_kind(elem_type: &str, v: &Value) -> ReflectKind {
    if elem_is_interface(elem_type) {
        ReflectKind::Interface
    } else {
        kind(v)
    }
}

/// Go `reflect.Zero(t).Interface()` for the Go type `t`.
pub(crate) fn zero_value(t: &str) -> Value {
    use go_value::{FloatKind, IntKind, UintKind};
    match t {
        "string" => Value::string(""),
        "bool" => Value::Bool(false),
        "int" => Value::Int(0, IntKind::Int),
        "int8" => Value::Int(0, IntKind::Int8),
        "int16" => Value::Int(0, IntKind::Int16),
        "int32" => Value::Int(0, IntKind::Int32),
        "int64" => Value::Int(0, IntKind::Int64),
        "uint" => Value::Uint(0, UintKind::Uint),
        "uint8" => Value::Uint(0, UintKind::Uint8),
        "uint16" => Value::Uint(0, UintKind::Uint16),
        "uint32" => Value::Uint(0, UintKind::Uint32),
        "uint64" => Value::Uint(0, UintKind::Uint64),
        "uintptr" => Value::Uint(0, UintKind::Uintptr),
        "float32" => Value::Float(0.0, FloatKind::F32),
        "float64" => Value::Float(0.0, FloatKind::F64),
        "template.HTML" => Value::Safe(go_value::SafeKind::Html, GoString::empty()),
        "time.Time" => Value::Time(go_value::Time::zero()),
        _ if hreflect::is_interface_type(t) => Value::Invalid,
        _ => match typed_nil_kind(t) {
            NilKind::Interface => Value::Invalid,
            _ => Value::TypedNil(Arc::from(t)),
        },
    }
}

// Go: tpl/collections/reflect_helpers.go:numberToFloat
pub(crate) fn number_to_float(v: &Value) -> GoResult<f64> {
    let k = kind(v);
    match k {
        _ if hreflect::is_float_kind(k) => Ok(float_of(v)),
        _ if hreflect::is_int_kind(k) => Ok(int_of(v) as f64),
        _ if hreflect::is_uint_kind(k) => Ok(uint_of(v) as f64),
        _ => Err(go_value::Error::new(format!(
            "invalid kind {} in numberToFloat",
            kind_name(k)
        ))),
    }
}

/// The basic value of a named basic object (its `underlying`), else the value itself.
pub(crate) fn basic(v: &Value) -> Value {
    if let Value::Object(o) = v
        && let Some(u) = o.underlying()
    {
        return u;
    }
    v.clone()
}

/// `v.Int()` (the caller checked the kind).
pub(crate) fn int_of(v: &Value) -> i64 {
    match basic(v) {
        Value::Int(i, _) => i,
        _ => 0,
    }
}

/// `v.Uint()`.
pub(crate) fn uint_of(v: &Value) -> u64 {
    match basic(v) {
        Value::Uint(u, _) => u,
        _ => 0,
    }
}

/// `v.Float()`.
pub(crate) fn float_of(v: &Value) -> f64 {
    match basic(v) {
        Value::Float(f, _) => f,
        _ => 0.0,
    }
}

/// `v.String()` of a string-kind value.
pub(crate) fn string_of(v: &Value) -> GoString {
    match basic(v) {
        Value::String(s) | Value::Safe(_, s) => s,
        _ => GoString::empty(),
    }
}

/// A value of the string type of `like` (`string`, the html/template types, the named string
/// types the port knows: `hstring.HTML`, `json.Number`, `neohugo.VersionString`) holding `s`.
/// Other named string objects become a plain `string` (PORTING.md).
pub(crate) fn with_string(like: &Value, s: GoString) -> Value {
    match like {
        Value::Safe(k, _) => Value::Safe(*k, s),
        Value::Object(o) => match &*o.type_name() {
            "hstring.HTML" => Value::object(nh_common::types::hstring::Html(s)),
            "json.Number" => Value::object(go_json::Number(s)),
            "neohugo.VersionString" => Value::object(nh_config::neohugo::version::VersionString(
                s.to_str_lossy().into_owned(),
            )),
            _ => Value::String(s),
        },
        _ => Value::String(s),
    }
}

/// `v.Bool()`.
pub(crate) fn bool_of(v: &Value) -> bool {
    matches!(basic(v), Value::Bool(true))
}

/// `v.Len()` of a string, slice or map kind value.
pub(crate) fn len_of(v: &Value) -> usize {
    match v {
        Value::String(s) | Value::Safe(_, s) => s.len(),
        Value::List(l) => l.len(),
        Value::Map(m) => m.len(),
        Value::Object(o) => match o.kind() {
            Kind::Slice => o.list().map(|l| l.len()).unwrap_or(0),
            Kind::Map => o.map_keys().len(),
            _ => match o.underlying() {
                Some(Value::String(s)) => s.len(),
                _ => 0,
            },
        },
        _ => 0,
    }
}

/// A number inside a [`NormKey`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum NumV {
    I(i64),
    U(u64),
    F(f64),
}

/// A key of Go's `map[any]bool` as built by `normalize`: Go compares interface keys by dynamic
/// type and value.
#[derive(Clone, Debug)]
pub(crate) enum NormKey {
    /// A nil interface.
    Nil,
    /// A number of the named type (`normalize` gives `float64`; a hash is a `uint64`).
    Num(String, NumV),
    /// A `bool`-kind value of the named type.
    Bool(String, bool),
    /// A string-kind value (`string`, `template.HTML`, named strings) of the named type; also
    /// `resource.TransientIdentifier` keys (type `string`).
    Str(String, GoString),
    /// `time.Time` (unix seconds, nanoseconds, location identity).
    Time(i64, i64, usize),
    /// A pointer (or other identity-compared value) of the named type.
    Ptr(String, usize),
    /// A typed nil of the named type.
    TypedNil(String),
}

impl PartialEq for NormKey {
    fn eq(&self, other: &NormKey) -> bool {
        use NormKey::*;
        match (self, other) {
            (Nil, Nil) => true,
            (Num(t1, a), Num(t2, b)) => {
                t1 == t2
                    && match (a, b) {
                        (NumV::I(a), NumV::I(b)) => a == b,
                        (NumV::U(a), NumV::U(b)) => a == b,
                        (NumV::F(a), NumV::F(b)) => a == b,
                        _ => false,
                    }
            }
            (Bool(t1, a), Bool(t2, b)) => t1 == t2 && a == b,
            (Str(t1, a), Str(t2, b)) => t1 == t2 && a == b,
            (Time(a1, b1, c1), Time(a2, b2, c2)) => a1 == a2 && b1 == b2 && c1 == c2,
            (Ptr(t1, a), Ptr(t2, b)) => t1 == t2 && a == b,
            (TypedNil(a), TypedNil(b)) => a == b,
            _ => false,
        }
    }
}

/// A float64 key (what `normalize` returns for numbers).
pub(crate) fn float_key(f: f64) -> NormKey {
    NormKey::Num("float64".into(), NumV::F(f))
}

/// Go's `map[any]bool` over [`NormKey`]s (linear, insertion ordered).
#[derive(Default)]
pub(crate) struct KeySet(Vec<NormKey>);

impl KeySet {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn contains(&self, k: &NormKey) -> bool {
        self.0.iter().any(|x| x == k)
    }
    /// Inserts `k`; false if it was already present.
    pub fn insert(&mut self, k: NormKey) -> bool {
        if self.contains(&k) {
            return false;
        }
        self.0.push(k);
        true
    }
}

fn loc_id(t: &go_value::Time) -> usize {
    match &t.loc {
        Some(l) => Arc::as_ptr(l) as *const () as usize,
        None => 0,
    }
}

/// The Go interface value `v` (a comparable dynamic value) as a map key.
pub(crate) fn interface_key(v: &Value) -> NormKey {
    match v {
        Value::Invalid => NormKey::Nil,
        Value::TypedNil(t) => {
            if typed_nil_kind(t) == NilKind::Interface {
                NormKey::Nil
            } else {
                NormKey::TypedNil(t.to_string())
            }
        }
        Value::Bool(b) => NormKey::Bool("bool".into(), *b),
        Value::Int(i, k) => NormKey::Num(k.go_name().into(), NumV::I(*i)),
        Value::Uint(u, k) => NormKey::Num(k.go_name().into(), NumV::U(*u)),
        Value::Float(f, k) => NormKey::Num(k.go_name().into(), NumV::F(*f)),
        Value::String(s) => NormKey::Str("string".into(), s.clone()),
        Value::Safe(k, s) => NormKey::Str(k.go_name().into(), s.clone()),
        Value::Time(t) => NormKey::Time(t.unix_sec, t.nsec as i64, loc_id(t)),
        Value::List(l) => NormKey::Ptr(l.ty.go_name().into_owned(), Arc::as_ptr(l) as usize),
        Value::Map(m) => NormKey::Ptr(m.ty.go_name().into_owned(), Arc::as_ptr(m) as usize),
        Value::Object(o) => {
            let t = o.type_name().into_owned();
            match o.underlying() {
                Some(Value::String(s)) => NormKey::Str(t, s),
                Some(Value::Bool(b)) => NormKey::Bool(t, b),
                Some(Value::Int(i, _)) => NormKey::Num(t, NumV::I(i)),
                Some(Value::Uint(u, _)) => NormKey::Num(t, NumV::U(u)),
                Some(Value::Float(f, _)) => NormKey::Num(t, NumV::F(f)),
                _ => NormKey::Ptr(t, object_identity(v)),
            }
        }
    }
}

/// The identity Go's `==` compares for an object. A page is its `*pageState` (`page_id`) except
/// for pointer wrappers such as `*hugolib.pageWithOrdinal`, which Go compares by their own
/// pointer: the `Arc` of the wrapper.
pub(crate) fn object_identity(v: &Value) -> usize {
    if let Some(p) = v.downcast::<nh_page::page::PageRef>() {
        let t = p.0.tpl_type_name();
        if t.starts_with('*') && t != "*hugolib.pageState" {
            return Arc::as_ptr(&p.0) as *const () as usize;
        }
    }
    match v {
        Value::Object(o) => o.identity(),
        _ => 0,
    }
}

/// Go's `a == b` on two interface values of pointer or struct kind (after `types.Unwrapv`).
pub(crate) fn interface_equal(a: &Value, b: &Value) -> bool {
    interface_key(a) == interface_key(b)
}

/// The key of `types.Unwrapv(v)` (with the `resource.TransientIdentifier` rule): the tail of
/// `normalize` for a comparable, non-number value.
fn unwrapped_key(v: &Value) -> GoResult<NormKey> {
    let vv = nh_common::types::types::unwrapv(v);
    // resource.TransientIdentifier (resources' TransientKey()).
    if let Value::Object(o) = &vv
        && o.has_method("TransientKey")
        && let Some(Ok(Value::String(s))) = o.call_method(&(), "TransientKey", &[])
    {
        return Ok(NormKey::Str("string".into(), s));
    }
    if !value_comparable(&vv) {
        // Go: a map insert with an uncomparable dynamic key panics.
        return Err(go_value::Error::new(format!(
            "runtime error: hash of unhashable type {}",
            type_string(&vv)
        )));
    }
    Ok(interface_key(&vv))
}

// Go: tpl/collections/reflect_helpers.go:normalize
/// normalizes different numeric types if isNumber or get the hash values if not Comparable
/// (such as map or struct) to make them comparable. `v` is a dynamic value (Go: a value that is
/// not of interface kind).
pub(crate) fn normalize(v: &Value) -> NormKey {
    let k = kind(v);
    if !value_comparable(v) {
        return NormKey::Num(
            "uint64".into(),
            NumV::U(nh_common::hashing::hash_uint64(std::slice::from_ref(v))),
        );
    }
    if hreflect::is_number_kind(k)
        && let Ok(f) = number_to_float(v)
    {
        return float_key(f);
    }
    unwrapped_key(v).unwrap_or(NormKey::Nil)
}

/// `normalize` of a value of static interface kind (Go: `v.Type()` is the interface type, which
/// is comparable, and numbers are not normalized). Errors with Go's runtime panic when the
/// dynamic value cannot be a map key.
pub(crate) fn normalize_interface(v: &Value) -> GoResult<NormKey> {
    unwrapped_key(v)
}

// Go: tpl/collections/reflect_helpers.go:collectIdentities
/// collects identities from the slices in seqs into a set. Numeric values are normalized,
/// pointers unwrapped.
pub(crate) fn collect_identities(seqs: &[Value]) -> GoResult<KeySet> {
    let mut seen = KeySet::default();
    for seq in seqs {
        match as_slice(seq) {
            Some(sl) => {
                let elem = sl.elem_type();
                for item in &sl.items {
                    let (ev, _) = indirect_interface(item);
                    let comparable = if elem_is_interface(&elem) {
                        // The interface value itself; ev.Type() of a nil interface element is
                        // the interface type (comparable).
                        value_comparable(&ev)
                    } else {
                        type_comparable(&elem)
                    };
                    if !comparable {
                        return Err(go_value::Error::new("elements must be comparable"));
                    }
                    seen.insert(normalize(&ev));
                }
            }
            None => {
                return Err(go_value::Error::new("arguments must be slices or arrays"));
            }
        }
    }
    Ok(seen)
}

// Go: tpl/collections/reflect_helpers.go:convertValue
/// We have some different numeric and string types that we try to behave like they were the
/// same.
pub(crate) fn convert_value(v: &Value, to: &str) -> GoResult<Value> {
    if hreflect::assignable_to(v, to) {
        return Ok(v.clone());
    }
    // A nil interface element: its (interface) type is assignable to an interface type.
    if v.is_invalid() && hreflect::is_interface_type(to) {
        return Ok(v.clone());
    }
    let to_kind = type_kind(to);
    if to_kind == ReflectKind::String {
        let s = to_string(v)?;
        return Ok(Value::String(s));
    }
    if hreflect::is_number_kind(to_kind) {
        return convert_number(v, to_kind);
    }
    Err(go_value::Error::new(format!(
        "{} is not assignable to {}",
        type_string(v),
        to
    )))
}

/// `v.Type().String()` (`<nil>` for an invalid value, as `%T`).
pub(crate) fn type_string(v: &Value) -> String {
    v.go_type_name().into_owned()
}

/// The kind of the Go type `t`.
pub(crate) fn type_kind(t: &str) -> ReflectKind {
    use go_value::{IntKind, UintKind};
    match t {
        "string" | "template.HTML" | "template.CSS" | "template.JS" | "template.JSStr"
        | "template.URL" | "template.HTMLAttr" | "template.Srcset" | "hstring.HTML" => {
            ReflectKind::String
        }
        "bool" => ReflectKind::Bool,
        "int" => ReflectKind::Int(IntKind::Int),
        "int8" => ReflectKind::Int(IntKind::Int8),
        "int16" => ReflectKind::Int(IntKind::Int16),
        "int32" => ReflectKind::Int(IntKind::Int32),
        "int64" => ReflectKind::Int(IntKind::Int64),
        "uint" => ReflectKind::Uint(UintKind::Uint),
        "uint8" => ReflectKind::Uint(UintKind::Uint8),
        "uint16" => ReflectKind::Uint(UintKind::Uint16),
        "uint32" => ReflectKind::Uint(UintKind::Uint32),
        "uint64" => ReflectKind::Uint(UintKind::Uint64),
        "uintptr" => ReflectKind::Uint(UintKind::Uintptr),
        "float32" => ReflectKind::Float32,
        "float64" => ReflectKind::Float64,
        "time.Time" => ReflectKind::Struct,
        _ if hreflect::is_interface_type(t) => ReflectKind::Interface,
        _ => match typed_nil_kind(t) {
            NilKind::Ptr => ReflectKind::Ptr,
            NilKind::Slice => ReflectKind::Slice,
            NilKind::Map => ReflectKind::Map,
            NilKind::Func => ReflectKind::Func,
            NilKind::Chan => ReflectKind::Chan,
            NilKind::Interface => ReflectKind::Struct,
        },
    }
}

// Go: tpl/collections/reflect_helpers.go:convertNumber
/// There are potential overflows in this function, but the downconversion of int64 etc. into
/// int8 etc. is coming from the synthetic unit tests for Union etc.
pub(crate) fn convert_number(v: &Value, to: ReflectKind) -> GoResult<Value> {
    use go_value::{FloatKind, IntKind, UintKind};
    let n = if hreflect::is_float_kind(to) {
        let f = to_float(v)?;
        match to {
            ReflectKind::Float32 => Some(Value::Float(f as f32 as f64, FloatKind::F32)),
            _ => Some(Value::Float(f, FloatKind::F64)),
        }
    } else if hreflect::is_int_kind(to) {
        let i = to_int(v)?;
        match to {
            ReflectKind::Int(IntKind::Int) => Some(Value::Int(i, IntKind::Int)),
            ReflectKind::Int(IntKind::Int8) => Some(Value::Int(i as i8 as i64, IntKind::Int8)),
            ReflectKind::Int(IntKind::Int16) => Some(Value::Int(i as i16 as i64, IntKind::Int16)),
            ReflectKind::Int(IntKind::Int32) => Some(Value::Int(i as i32 as i64, IntKind::Int32)),
            ReflectKind::Int(IntKind::Int64) => Some(Value::Int(i, IntKind::Int64)),
            _ => None,
        }
    } else if hreflect::is_uint_kind(to) {
        let u = to_uint(v)?;
        match to {
            ReflectKind::Uint(UintKind::Uint) => Some(Value::Uint(u, UintKind::Uint)),
            ReflectKind::Uint(UintKind::Uint8) => {
                Some(Value::Uint(u as u8 as u64, UintKind::Uint8))
            }
            ReflectKind::Uint(UintKind::Uint16) => {
                Some(Value::Uint(u as u16 as u64, UintKind::Uint16))
            }
            ReflectKind::Uint(UintKind::Uint32) => {
                Some(Value::Uint(u as u32 as u64, UintKind::Uint32))
            }
            ReflectKind::Uint(UintKind::Uint64) => Some(Value::Uint(u, UintKind::Uint64)),
            _ => None,
        }
    } else {
        None
    };

    n.ok_or_else(|| go_value::Error::new("invalid values"))
}

// Go: tpl/collections/reflect_helpers.go:newSliceElement
/// The Go type string of `reflect.New(elem)` for a slice/array `items` (`*T`; for a pointer
/// element type `*E`, `*E` again), `None` for other kinds or nil.
pub(crate) fn new_slice_element_type(items: &Value) -> Option<String> {
    let t = hreflect::type_of(items)?;
    if hreflect::is_slice_type(&t) || matches!(items, Value::Object(o) if o.kind() == Kind::Slice) {
        let mut e = hreflect::elem_type(&t).unwrap_or_else(|| "interface {}".to_string());
        if let Some(inner) = e.strip_prefix('*') {
            e = inner.to_string();
        }
        return Some(format!("*{e}"));
    }
    None
}

// Go: tpl/collections/reflect_helpers.go:isNumber
pub(crate) fn is_number(k: ReflectKind) -> bool {
    is_int(k) || is_uint(k) || is_float(k)
}

// Go: tpl/collections/reflect_helpers.go:isInt
pub(crate) fn is_int(k: ReflectKind) -> bool {
    matches!(k, ReflectKind::Int(_))
}

// Go: tpl/collections/reflect_helpers.go:isUint
pub(crate) fn is_uint(k: ReflectKind) -> bool {
    matches!(k, ReflectKind::Uint(u) if u != go_value::UintKind::Uintptr)
}

// Go: tpl/collections/reflect_helpers.go:isFloat
pub(crate) fn is_float(k: ReflectKind) -> bool {
    matches!(k, ReflectKind::Float32 | ReflectKind::Float64)
}

// Go: tpl/collections/where.go:toFloat
/// toFloat returns the float value if possible.
pub(crate) fn to_float(v: &Value) -> GoResult<f64> {
    match kind(v) {
        ReflectKind::Float32 | ReflectKind::Float64 => Ok(float_of(v)),
        ReflectKind::Int(_) => Ok(int_of(v) as f64),
        _ => Err(go_value::Error::new("unable to convert value to float")),
    }
}

// Go: tpl/collections/where.go:toInt
/// toInt returns the int value if possible, -1 if not.
pub(crate) fn to_int(v: &Value) -> GoResult<i64> {
    match kind(v) {
        ReflectKind::Int(_) => Ok(int_of(v)),
        _ => Err(go_value::Error::new("unable to convert value to int")),
    }
}

// Go: tpl/collections/where.go:toUint
pub(crate) fn to_uint(v: &Value) -> GoResult<u64> {
    match kind(v) {
        ReflectKind::Uint(k) if k != go_value::UintKind::Uintptr => Ok(uint_of(v)),
        _ => Err(go_value::Error::new("unable to convert value to uint")),
    }
}

// Go: tpl/collections/where.go:toString
/// toString returns the string value if possible, "" if not.
pub(crate) fn to_string(v: &Value) -> GoResult<GoString> {
    match kind(v) {
        ReflectKind::String => Ok(string_of(v)),
        _ => Err(go_value::Error::new("unable to convert value to string")),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/reflect_helpers.go (216 lines; 0/10 funcs executed)
// OK L31-44: numberToFloat(v reflect.Value) (float64, error)
// OK L49-67: normalize(v reflect.Value) any
// OK L71-92: collectIdentities(seqs ...any) (map[any]bool, error)
// OK L96-109: convertValue(v reflect.Value, to reflect.Type) (reflect.Value, error)
// OK L114-168: convertNumber(v reflect.Value, to reflect.Kind) (reflect.Value, error)
// OK L170-185: newSliceElement(items any) any
// OK L187-189: isNumber(kind reflect.Kind) bool
// OK L191-198: isInt(kind reflect.Kind) bool
// OK L200-207: isUint(kind reflect.Kind) bool
// OK L209-216: isFloat(kind reflect.Kind) bool
// ---------------------------------------------------------------------------
