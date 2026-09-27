//! The Go value model hashstructure walks (a stand-in for `reflect.Value`).
//!
//! [`HashValue`] mirrors the reflect kinds hashstructure distinguishes, plus
//! the method sets it consults (`Hashable`, `Includable`, `IncludableMap`,
//! `fmt.Stringer`, and neohugo's `Key() string` keyer). Values of the shared
//! model ([`go_value::Value`]) are converted on the fly.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use go_value::{
    FloatKind, GoString, IntKind, Kind as ObjKind, MapType, Object, SliceType, Time, UintKind,
    Value,
};

use crate::include::{Hashable, Includable, IncludableMap};

/// reflect kinds hashstructure cannot hash ("unknown kind to hash: <kind>").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedKind {
    Chan,
    Func,
    UnsafePointer,
    /// The zero value of an interface type (only reachable with `ZeroNil`).
    Interface,
    /// The zero value of a pointer type (only reachable with `ZeroNil`).
    Ptr,
}

impl UnsupportedKind {
    /// Go: `reflect.Kind.String()`.
    pub fn go_name(self) -> &'static str {
        match self {
            UnsupportedKind::Chan => "chan",
            UnsupportedKind::Func => "func",
            UnsupportedKind::UnsafePointer => "unsafe.Pointer",
            UnsupportedKind::Interface => "interface",
            UnsupportedKind::Ptr => "ptr",
        }
    }
}

/// Which method set a method belongs to (Go value vs pointer receiver).
/// Pointer-receiver methods are only seen when the struct is addressable
/// (reached through a pointer, a slice element, a map entry copy, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receiver {
    Value,
    Pointer,
}

/// A Go value as hashstructure sees it.
#[derive(Clone)]
pub enum HashValue {
    /// A nil interface / the invalid `reflect.Value` (hashes like `int(0)`).
    Nil,
    Bool(bool),
    Int(i64, IntKind),
    Uint(u64, UintKind),
    /// `float32` values are stored widened and hashed as 4 bytes.
    Float(f64, FloatKind),
    Complex64(f32, f32),
    Complex128(f64, f64),
    /// Any string kind (`string`, `template.HTML`, named string types).
    String(GoString),
    Time(Time),
    /// A Go array `[N]T`.
    Array(Vec<HashValue>),
    /// A Go slice; `None` is a nil slice.
    Slice(Option<Vec<HashValue>>),
    Map(GoMap),
    Struct(GoStruct),
    /// A pointer. `None` is a nil pointer. The second field is the zero value
    /// of the pointee type, used only with `HashOptions::zero_nil` (when
    /// absent, a nil pointer hashes like `int(0)` in both modes).
    Ptr(Option<Box<HashValue>>, Option<Box<HashValue>>),
    /// An interface-typed value holding a dynamic value (e.g. the elements
    /// of `[]interface{}`, fields of interface type). Only affects
    /// addressability of what it holds.
    Interface(Box<HashValue>),
    /// chan / func / unsafe.Pointer values (hashing them is an error);
    /// `nil` matters for `IsZero` (IgnoreZeroValue).
    Unsupported {
        kind: UnsupportedKind,
        nil: bool,
    },
    /// A value of the shared value model, converted when visited.
    Value(Value),
}

impl fmt::Debug for HashValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashValue::Nil => f.write_str("Nil"),
            HashValue::Bool(b) => write!(f, "Bool({b})"),
            HashValue::Int(i, k) => write!(f, "{}({i})", k.go_name()),
            HashValue::Uint(u, k) => write!(f, "{}({u})", k.go_name()),
            HashValue::Float(x, k) => write!(f, "{}({x:?})", k.go_name()),
            HashValue::Complex64(a, b) => write!(f, "complex64({a},{b})"),
            HashValue::Complex128(a, b) => write!(f, "complex128({a},{b})"),
            HashValue::String(s) => write!(f, "String({s:?})"),
            HashValue::Time(t) => write!(f, "Time({t:?})"),
            HashValue::Array(v) => write!(f, "Array{v:?}"),
            HashValue::Slice(v) => write!(f, "Slice{v:?}"),
            HashValue::Map(m) => write!(f, "Map{:?}", m.entries),
            HashValue::Struct(s) => write!(f, "Struct({}, {:?})", s.name, s.fields),
            HashValue::Ptr(p, _) => write!(f, "Ptr({p:?})"),
            HashValue::Interface(v) => write!(f, "Interface({v:?})"),
            HashValue::Unsupported { kind, nil } => {
                write!(f, "Unsupported({}, nil={nil})", kind.go_name())
            }
            HashValue::Value(v) => write!(f, "Value({v:?})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Constructors

impl HashValue {
    /// A Go `int`.
    pub fn int(i: i64) -> HashValue {
        HashValue::Int(i, IntKind::Int)
    }

    /// A Go `uint64`.
    pub fn uint64(u: u64) -> HashValue {
        HashValue::Uint(u, UintKind::Uint64)
    }

    /// A Go `float64`.
    pub fn float64(f: f64) -> HashValue {
        HashValue::Float(f, FloatKind::F64)
    }

    /// A Go `float32`.
    pub fn float32(f: f32) -> HashValue {
        HashValue::Float(f as f64, FloatKind::F32)
    }

    pub fn string(s: impl Into<GoString>) -> HashValue {
        HashValue::String(s.into())
    }

    /// A `[]string`.
    pub fn string_slice<I, S>(items: I) -> HashValue
    where
        I: IntoIterator<Item = S>,
        S: Into<GoString>,
    {
        HashValue::Slice(Some(
            items
                .into_iter()
                .map(|s| HashValue::String(s.into()))
                .collect(),
        ))
    }

    /// A `[]interface{}` (each element wrapped as an interface value).
    pub fn any_slice(items: Vec<HashValue>) -> HashValue {
        HashValue::Slice(Some(items.into_iter().map(HashValue::iface).collect()))
    }

    /// Wrap a value as the dynamic value of an interface-typed slot.
    pub fn iface(v: HashValue) -> HashValue {
        match v {
            HashValue::Nil => HashValue::Nil,
            v => HashValue::Interface(Box::new(v)),
        }
    }

    /// A non-nil pointer to `v`.
    pub fn ptr(v: HashValue) -> HashValue {
        HashValue::Ptr(Some(Box::new(v)), None)
    }

    /// A `map[string]interface{}` from string keys and values.
    pub fn string_any_map<I, K>(entries: I) -> HashValue
    where
        I: IntoIterator<Item = (K, HashValue)>,
        K: Into<GoString>,
    {
        HashValue::Map(GoMap::new(
            entries
                .into_iter()
                .map(|(k, v)| (HashValue::String(k.into()), HashValue::iface(v)))
                .collect(),
        ))
    }
}

impl From<Value> for HashValue {
    fn from(v: Value) -> Self {
        HashValue::Value(v)
    }
}

impl From<&Value> for HashValue {
    fn from(v: &Value) -> Self {
        HashValue::Value(v.clone())
    }
}

impl From<&str> for HashValue {
    fn from(s: &str) -> Self {
        HashValue::String(s.into())
    }
}

impl From<String> for HashValue {
    fn from(s: String) -> Self {
        HashValue::String(s.into())
    }
}

impl From<bool> for HashValue {
    fn from(b: bool) -> Self {
        HashValue::Bool(b)
    }
}

impl From<GoStruct> for HashValue {
    fn from(s: GoStruct) -> Self {
        HashValue::Struct(s)
    }
}

impl From<GoMap> for HashValue {
    fn from(m: GoMap) -> Self {
        HashValue::Map(m)
    }
}

// ---------------------------------------------------------------------------
// Maps

/// A Go map value. Entry order does not matter (hashstructure XORs entries).
#[derive(Clone, Default)]
pub struct GoMap {
    /// `None` is a nil map.
    pub entries: Option<Vec<(HashValue, HashValue)>>,
    /// Set when the map's own type implements `IncludableMap`.
    pub include_map: Option<Arc<dyn IncludableMap>>,
}

impl GoMap {
    pub fn new(entries: Vec<(HashValue, HashValue)>) -> GoMap {
        GoMap {
            entries: Some(entries),
            include_map: None,
        }
    }

    /// A nil map.
    pub fn nil() -> GoMap {
        GoMap {
            entries: None,
            include_map: None,
        }
    }

    pub fn with_include_map(mut self, m: Arc<dyn IncludableMap>) -> GoMap {
        self.include_map = Some(m);
        self
    }
}

// ---------------------------------------------------------------------------
// Structs

/// A struct field.
#[derive(Clone, Debug)]
pub struct GoField {
    /// The Go field name (for embedded fields, the type name).
    pub name: String,
    /// The raw struct tag, e.g. `hash:"ignore" json:"x"`.
    pub tag: String,
    /// Exported fields (upper-case first letter, or embedded exported types)
    /// are hashed; unexported ones only count for `IsZero`.
    pub exported: bool,
    pub value: HashValue,
    /// `Some(s)` when the field's value implements `fmt.Stringer`
    /// (`String()` returns `s`); used with the `string` tag or `UseStringer`.
    pub stringer: Option<GoString>,
}

/// A Go struct value: the stand-in for a Rust type that represents a Go
/// struct (type name, ordered fields with tags, and the method sets that
/// hashstructure consults).
#[derive(Clone, Default)]
pub struct GoStruct {
    /// `reflect.Type.Name()`: the bare type name without package, `""` for
    /// unnamed struct types.
    pub name: String,
    pub fields: Vec<GoField>,
    pub hashable: Option<(Receiver, Arc<dyn Hashable>)>,
    pub includable: Option<(Receiver, Arc<dyn Includable>)>,
    /// Value-receiver `IncludableMap` (consulted for map-typed fields).
    pub include_map: Option<Arc<dyn IncludableMap>>,
    /// neohugo `hashing.keyer`: `Key() string` (only used by
    /// `hashing::hash_uint64`'s `toHashable` on top-level arguments).
    pub key: Option<GoString>,
}

impl fmt::Debug for GoStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GoStruct({}, {:?})", self.name, self.fields)
    }
}

impl GoStruct {
    /// A struct of the named type (bare name, as `reflect.Type.Name()`).
    pub fn new(name: impl Into<String>) -> GoStruct {
        GoStruct {
            name: name.into(),
            ..Default::default()
        }
    }

    fn push(
        mut self,
        name: &str,
        tag: &str,
        exported: bool,
        value: HashValue,
        stringer: Option<GoString>,
    ) -> GoStruct {
        self.fields.push(GoField {
            name: name.to_string(),
            tag: tag.to_string(),
            exported,
            value,
            stringer,
        });
        self
    }

    /// An exported field without tag.
    pub fn field(self, name: &str, value: impl Into<HashValue>) -> GoStruct {
        self.push(name, "", true, value.into(), None)
    }

    /// An exported field with a struct tag (e.g. `hash:"set"`).
    pub fn tagged(self, name: &str, tag: &str, value: impl Into<HashValue>) -> GoStruct {
        self.push(name, tag, true, value.into(), None)
    }

    /// An exported field whose value implements `fmt.Stringer`.
    pub fn stringer_field(
        self,
        name: &str,
        tag: &str,
        value: impl Into<HashValue>,
        s: impl Into<GoString>,
    ) -> GoStruct {
        self.push(name, tag, true, value.into(), Some(s.into()))
    }

    /// An unexported field (never hashed; counts for `IsZero`).
    pub fn unexported(self, name: &str, value: impl Into<HashValue>) -> GoStruct {
        self.push(name, "", false, value.into(), None)
    }

    /// A blank (`_`) field.
    pub fn blank(self, value: impl Into<HashValue>) -> GoStruct {
        self.push("_", "", false, value.into(), None)
    }

    pub fn with_hashable(mut self, recv: Receiver, h: Arc<dyn Hashable>) -> GoStruct {
        self.hashable = Some((recv, h));
        self
    }

    pub fn with_includable(mut self, recv: Receiver, i: Arc<dyn Includable>) -> GoStruct {
        self.includable = Some((recv, i));
        self
    }

    pub fn with_include_map(mut self, m: Arc<dyn IncludableMap>) -> GoStruct {
        self.include_map = Some(m);
        self
    }

    pub fn with_key(mut self, key: impl Into<GoString>) -> GoStruct {
        self.key = Some(key.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Conversion from go_value

type ObjectHasher = Arc<dyn Fn(&dyn Object) -> Option<HashValue> + Send + Sync>;

static OBJECT_HASHERS: RwLock<Option<HashMap<TypeId, ObjectHasher>>> = RwLock::new(None);

/// Register how values of a host type (a `go_value::Object`) look to
/// hashstructure, e.g. a `GoStruct` with tags or a `Hashable`. Objects
/// without a registration are hashed from `Object::kind`,
/// `Object::type_name` and `Object::struct_fields` (all fields exported,
/// untagged).
pub fn register_object<T: Object>(f: impl Fn(&T) -> HashValue + Send + Sync + 'static) {
    let h: ObjectHasher = Arc::new(move |o: &dyn Object| o.as_any().downcast_ref::<T>().map(&f));
    let mut g = OBJECT_HASHERS.write().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(HashMap::new)
        .insert(TypeId::of::<T>(), h);
}

fn registered(o: &dyn Object) -> Option<HashValue> {
    let id = o.as_any().type_id();
    let h = {
        let g = OBJECT_HASHERS.read().unwrap_or_else(|e| e.into_inner());
        g.as_ref()?.get(&id)?.clone()
    };
    h(o)
}

/// The bare type name of a Go type string: `*hugolib.pageState` →
/// `pageState`, `images.filter` → `filter`.
pub fn bare_type_name(t: &str) -> &str {
    let t = t.trim_start_matches('*');
    match t.rfind('.') {
        Some(i) => &t[i + 1..],
        None => t,
    }
}

/// A typed nil of the Go type `t` (best effort from the type string).
fn typed_nil(t: &str) -> HashValue {
    if t.starts_with("[]") {
        HashValue::Slice(None)
    } else if t.starts_with("map[") || t == "maps.Params" {
        HashValue::Map(GoMap::nil())
    } else if t.starts_with("func") {
        HashValue::Unsupported {
            kind: UnsupportedKind::Func,
            nil: true,
        }
    } else if t.starts_with("chan") || t.starts_with("<-chan") {
        HashValue::Unsupported {
            kind: UnsupportedKind::Chan,
            nil: true,
        }
    } else {
        // Pointers and interfaces: hashes like int(0).
        HashValue::Ptr(None, None)
    }
}

fn struct_from_object(o: &dyn Object) -> GoStruct {
    let mut s = GoStruct::new(bare_type_name(&o.type_name()));
    if let Some(fields) = o.struct_fields() {
        for (name, v) in fields {
            let stringer = v.as_object().and_then(|fo| fo.go_string());
            s.fields.push(GoField {
                name: name.into_owned(),
                tag: String::new(),
                exported: true,
                value: HashValue::Value(v),
                stringer,
            });
        }
    }
    s.key = o.hash_key();
    s
}

fn object_to_hash(o: &Arc<dyn Object>) -> HashValue {
    if let Some(h) = registered(o.as_ref()) {
        return h;
    }
    match o.kind() {
        ObjKind::Ptr | ObjKind::Interface => HashValue::Ptr(
            Some(Box::new(HashValue::Struct(struct_from_object(o.as_ref())))),
            None,
        ),
        ObjKind::Struct => HashValue::Struct(struct_from_object(o.as_ref())),
        ObjKind::Map => {
            let entries = o
                .map_keys()
                .into_iter()
                .map(|k| {
                    let v = o.map_get(&k).unwrap_or(Value::Invalid);
                    (HashValue::String(k), HashValue::iface(HashValue::Value(v)))
                })
                .collect();
            HashValue::Map(GoMap::new(entries))
        }
        ObjKind::Slice => HashValue::Slice(o.list().map(|items| {
            items
                .into_iter()
                .map(|v| HashValue::iface(HashValue::Value(v)))
                .collect()
        })),
        ObjKind::Func => HashValue::Unsupported {
            kind: UnsupportedKind::Func,
            nil: false,
        },
    }
}

/// Convert one level of a `go_value::Value` (children stay lazily wrapped).
pub fn from_value(v: &Value) -> HashValue {
    match v {
        Value::Invalid => HashValue::Nil,
        Value::TypedNil(t) => typed_nil(t),
        Value::Bool(b) => HashValue::Bool(*b),
        Value::Int(i, k) => HashValue::Int(*i, *k),
        Value::Uint(u, k) => HashValue::Uint(*u, *k),
        Value::Float(f, k) => HashValue::Float(*f, *k),
        Value::String(s) | Value::Safe(_, s) => HashValue::String(s.clone()),
        Value::Time(t) => HashValue::Time(t.clone()),
        Value::List(l) => {
            // Elements of interface-typed slices are interface values.
            let wrap = matches!(l.ty, SliceType::Any | SliceType::Named(_));
            HashValue::Slice(Some(
                l.items
                    .iter()
                    .map(|e| {
                        if wrap {
                            HashValue::iface(HashValue::Value(e.clone()))
                        } else {
                            HashValue::Value(e.clone())
                        }
                    })
                    .collect(),
            ))
        }
        Value::Map(m) => {
            let wrap = !matches!(m.ty, MapType::StringString);
            HashValue::Map(GoMap::new(
                m.entries
                    .iter()
                    .map(|(k, e)| {
                        let ev = HashValue::Value(e.clone());
                        (
                            HashValue::String(k.clone()),
                            if wrap { HashValue::iface(ev) } else { ev },
                        )
                    })
                    .collect(),
            ))
        }
        Value::Object(o) => object_to_hash(o),
    }
}

// ---------------------------------------------------------------------------
// reflect.Value.IsZero

/// Go: `reflect.Value.IsZero` (go1.27.1): numbers compare `== 0` (so
/// `-0.0` is zero, NaN is not), slices/maps/pointers/interfaces/funcs are
/// zero when nil, strings when empty, arrays and structs when every element
/// / non-blank field is zero.
pub fn is_zero(v: &HashValue) -> bool {
    match v {
        HashValue::Nil => true,
        HashValue::Bool(b) => !*b,
        HashValue::Int(i, _) => *i == 0,
        HashValue::Uint(u, _) => *u == 0,
        HashValue::Float(f, FloatKind::F32) => (*f as f32) == 0.0,
        HashValue::Float(f, FloatKind::F64) => *f == 0.0,
        HashValue::Complex64(a, b) => *a == 0.0 && *b == 0.0,
        HashValue::Complex128(a, b) => *a == 0.0 && *b == 0.0,
        HashValue::String(s) => s.is_empty(),
        // time.Time{wall, ext, loc}: zero only for the zero instant with a
        // nil (UTC) location.
        HashValue::Time(t) => t.is_zero() && t.loc.as_ref().is_none_or(go_time::is_utc_loc),
        HashValue::Array(items) => items.iter().all(is_zero),
        HashValue::Slice(items) => items.is_none(),
        HashValue::Map(m) => m.entries.is_none(),
        HashValue::Struct(s) => s.fields.iter().all(|f| f.name == "_" || is_zero(&f.value)),
        HashValue::Ptr(p, _) => p.is_none(),
        HashValue::Interface(_) => false,
        HashValue::Unsupported { nil, .. } => *nil,
        HashValue::Value(v) => is_zero(&from_value(v)),
    }
}

/// Go: `v.Interface().(fmt.Stringer)` for the values of the model:
/// `time.Time` (and `*time.Time`, whose method set includes `String`), and
/// `go_value` objects with `Object::go_string`. `Some(Err)` is a Go panic
/// (calling `String` through a nil `*time.Time`).
pub fn natural_stringer(v: &HashValue) -> Option<Result<GoString, crate::Error>> {
    use go_time::GoTimeExt;
    match v {
        HashValue::Time(t) => Some(Ok(GoString::from(t.string()))),
        HashValue::Interface(inner) => natural_stringer(inner),
        HashValue::Ptr(Some(inner), _) => match inner.as_ref() {
            HashValue::Time(t) => Some(Ok(GoString::from(t.string()))),
            HashValue::Value(Value::Time(t)) => Some(Ok(GoString::from(t.string()))),
            _ => None,
        },
        HashValue::Ptr(None, Some(z))
            if matches!(
                z.as_ref(),
                HashValue::Time(_) | HashValue::Value(Value::Time(_))
            ) =>
        {
            Some(Err(crate::Error::GoPanic(
                "value method time.Time.String called using nil *Time pointer".into(),
            )))
        }
        HashValue::Value(Value::Time(t)) => Some(Ok(GoString::from(t.string()))),
        HashValue::Value(Value::Object(o)) => o.go_string().map(Ok),
        _ => None,
    }
}
