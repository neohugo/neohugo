//! Dynamic Go value model for the neohugo Rust port.
//!
//! Go templates, config, front matter, data files, `fmt`, `encoding/json` and
//! `hashstructure` all work on `interface{}` values inspected through
//! reflection. This crate is the Rust stand-in for that: a [`Value`] enum that
//! keeps the Go type distinctions which leak into output bytes (int vs int64 vs
//! float64, `[]string` vs `[]interface {}`, `template.HTML` vs `string`, typed
//! nil vs missing value), plus the [`Object`] trait that host types (Page,
//! Site, Resource, ...) implement in place of Go's method sets.
//!
//! Rules every consumer must preserve:
//! - Go strings are byte sequences. [`GoString`] stores bytes; ordering is
//!   byte order, which is Go's string order (and `fmtsort` order).
//! - Maps only have string keys here (Hugo normalises YAML
//!   `map[interface{}]interface{}` to string keys). [`Map`] iterates in sorted
//!   byte order, like Go's `fmtsort`, `encoding/json` and `range`.
//! - A slice or map carries its Go type ([`SliceType`], [`MapType`]) so that
//!   collection functions return the same type they received, and so that
//!   methods declared on named slice/map types (`page.Pages.Reverse`,
//!   `page.Taxonomy.Alphabetical`, ...) stay reachable through the host.

mod nil_kind;
mod time;

pub use nil_kind::{NilKind, register_named_kind, typed_nil_kind};
pub use time::{Location, Time, UNIX_TO_INTERNAL, ZERO_TIME_UNIX, Zone, ZoneTrans};

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Errors

/// Error produced while evaluating Go-semantics operations.
#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    msg: String,
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error { msg: msg.into() }
    }

    pub fn message(&self) -> &str {
        &self.msg
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({:?})", self.msg)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// GoString

/// A Go string: an immutable byte sequence that is usually, but not
/// necessarily, valid UTF-8. Cheap to clone.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GoString(Arc<[u8]>);

impl GoString {
    pub fn new(b: impl Into<Arc<[u8]>>) -> Self {
        GoString(b.into())
    }

    pub fn empty() -> Self {
        GoString(Arc::from(&b""[..]))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The string as `&str` when it is valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }

    /// The string as UTF-8, replacing invalid sequences with U+FFFD.
    /// Only for diagnostics and places where Go itself would do the same.
    pub fn to_str_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<&str> for GoString {
    fn from(s: &str) -> Self {
        GoString(Arc::from(s.as_bytes()))
    }
}

impl From<String> for GoString {
    fn from(s: String) -> Self {
        GoString(Arc::from(s.into_bytes()))
    }
}

impl From<&String> for GoString {
    fn from(s: &String) -> Self {
        GoString::from(s.as_str())
    }
}

impl From<&[u8]> for GoString {
    fn from(b: &[u8]) -> Self {
        GoString(Arc::from(b))
    }
}

impl From<Vec<u8>> for GoString {
    fn from(b: Vec<u8>) -> Self {
        GoString(Arc::from(b))
    }
}

impl AsRef<[u8]> for GoString {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::borrow::Borrow<[u8]> for GoString {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl std::ops::Deref for GoString {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<str> for GoString {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for GoString {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == other.as_bytes()
    }
}

impl fmt::Debug for GoString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.to_str_lossy(), f)
    }
}

impl fmt::Display for GoString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_str_lossy())
    }
}

// ---------------------------------------------------------------------------
// Kinds and type tags

/// Go signed integer kinds. `Int` is Go's `int` (64-bit on all supported
/// platforms), distinct from `Int64` for `%T`, hashing and type checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntKind {
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
}

impl IntKind {
    pub fn go_name(self) -> &'static str {
        match self {
            IntKind::Int => "int",
            IntKind::Int8 => "int8",
            IntKind::Int16 => "int16",
            IntKind::Int32 => "int32",
            IntKind::Int64 => "int64",
        }
    }

    /// Size in bytes as written by `encoding/binary` (Go `int` is 8 bytes).
    pub fn size(self) -> usize {
        match self {
            IntKind::Int8 => 1,
            IntKind::Int16 => 2,
            IntKind::Int32 => 4,
            IntKind::Int | IntKind::Int64 => 8,
        }
    }
}

/// Go unsigned integer kinds. `Uint8` is also Go's `byte`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UintKind {
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uintptr,
}

impl UintKind {
    pub fn go_name(self) -> &'static str {
        match self {
            UintKind::Uint => "uint",
            UintKind::Uint8 => "uint8",
            UintKind::Uint16 => "uint16",
            UintKind::Uint32 => "uint32",
            UintKind::Uint64 => "uint64",
            UintKind::Uintptr => "uintptr",
        }
    }

    pub fn size(self) -> usize {
        match self {
            UintKind::Uint8 => 1,
            UintKind::Uint16 => 2,
            UintKind::Uint32 => 4,
            UintKind::Uint | UintKind::Uint64 | UintKind::Uintptr => 8,
        }
    }
}

/// Go float kinds. A `F32` value is stored widened to f64 but must be
/// formatted with 32-bit shortest digits and hashed as 4 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatKind {
    F32,
    F64,
}

impl FloatKind {
    pub fn go_name(self) -> &'static str {
        match self {
            FloatKind::F32 => "float32",
            FloatKind::F64 => "float64",
        }
    }
}

/// The `html/template` content types: named string types that carry
/// "already safe in this context" meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SafeKind {
    /// template.HTML
    Html,
    /// template.HTMLAttr
    HtmlAttr,
    /// template.CSS
    Css,
    /// template.JS
    Js,
    /// template.JSStr
    JsStr,
    /// template.URL
    Url,
    /// template.Srcset
    Srcset,
}

impl SafeKind {
    pub fn go_name(self) -> &'static str {
        match self {
            SafeKind::Html => "template.HTML",
            SafeKind::HtmlAttr => "template.HTMLAttr",
            SafeKind::Css => "template.CSS",
            SafeKind::Js => "template.JS",
            SafeKind::JsStr => "template.JSStr",
            SafeKind::Url => "template.URL",
            SafeKind::Srcset => "template.Srcset",
        }
    }
}

/// The Go type of a slice value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SliceType {
    /// `[]interface {}`
    Any,
    /// `[]string`
    String,
    /// `[]int`
    Int,
    /// `[]int64`
    Int64,
    /// `[]float64`
    Float64,
    /// `[]bool`
    Bool,
    /// `[]uint8` / `[]byte`
    Uint8,
    /// `[]map[string]interface {}`
    MapStringAny,
    /// Any other slice type, by its Go type string, e.g. `"page.Pages"`,
    /// `"resource.Resources"`, `"langs.Languages"`, `"[]*page.Pager"`.
    /// Methods on such named types are dispatched by the host.
    Named(Arc<str>),
}

impl SliceType {
    pub fn go_name(&self) -> Cow<'_, str> {
        match self {
            SliceType::Any => Cow::Borrowed("[]interface {}"),
            SliceType::String => Cow::Borrowed("[]string"),
            SliceType::Int => Cow::Borrowed("[]int"),
            SliceType::Int64 => Cow::Borrowed("[]int64"),
            SliceType::Float64 => Cow::Borrowed("[]float64"),
            SliceType::Bool => Cow::Borrowed("[]bool"),
            SliceType::Uint8 => Cow::Borrowed("[]uint8"),
            SliceType::MapStringAny => Cow::Borrowed("[]map[string]interface {}"),
            SliceType::Named(n) => Cow::Borrowed(n),
        }
    }
}

/// The Go type of a map value. All maps have string keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapType {
    /// `map[string]interface {}` (dict, JSON objects, data files).
    StringAny,
    /// `map[string]string`
    StringString,
    /// Hugo's `maps.Params`: keys are stored lower-cased and looked up
    /// case-insensitively; `IsZero` and other methods take precedence.
    Params,
    /// Any other named map type by Go type string, e.g. `"page.Taxonomy"`,
    /// `"page.TaxonomyList"`, `"page.Data"`. Methods are dispatched by the host.
    Named(Arc<str>),
}

impl MapType {
    pub fn go_name(&self) -> Cow<'_, str> {
        match self {
            MapType::StringAny => Cow::Borrowed("map[string]interface {}"),
            MapType::StringString => Cow::Borrowed("map[string]string"),
            MapType::Params => Cow::Borrowed("maps.Params"),
            MapType::Named(n) => Cow::Borrowed(n),
        }
    }
}

/// Emulated `reflect.Kind` of an [`Object`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A pointer to a struct (the common case for Hugo objects such as
    /// `*hugolib.pageState`).
    Ptr,
    /// A struct value (e.g. `media.Type`, `page.OutputFormat`).
    Struct,
    /// An object that behaves as a map (implements `map_get`/`map_keys`).
    Map,
    /// An object that behaves as a slice (implements `list`).
    Slice,
    /// A func value.
    Func,
    /// An interface value holding something else.
    Interface,
}

// ---------------------------------------------------------------------------
// Collections

/// A Go slice value together with its Go type.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub ty: SliceType,
    pub items: Vec<Value>,
}

impl List {
    pub fn new(ty: SliceType, items: Vec<Value>) -> Self {
        List { ty, items }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A Go map value with string keys, iterated in sorted byte order.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub ty: MapType,
    pub entries: BTreeMap<GoString, Value>,
}

impl Map {
    pub fn new(ty: MapType) -> Self {
        Map {
            ty,
            entries: BTreeMap::new(),
        }
    }

    pub fn with_entries(ty: MapType, entries: BTreeMap<GoString, Value>) -> Self {
        Map { ty, entries }
    }

    /// Exact-key lookup. Case-insensitive `Params` lookup is the host's job,
    /// because it needs Go's `strings.ToLower`.
    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: impl Into<GoString>, v: Value) -> Option<Value> {
        self.entries.insert(key.into(), v)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Objects

/// Opaque host context passed through to [`Object`] methods. The template
/// engine forwards whatever context the host gave it (in Go this is the
/// `context.Context` injected as a method's first argument); host objects
/// downcast it to their own type.
pub type HostCtx<'a> = &'a dyn Any;

/// A host value with Go-style methods and fields: the replacement for Go's
/// reflection on pointer/struct types. Member names are exact-case.
pub trait Object: Send + Sync + 'static {
    /// Go type string, used for `%T`, error messages and type checks,
    /// e.g. `"*hugolib.pageState"`, `"media.Type"`.
    fn type_name(&self) -> Cow<'_, str>;

    /// Emulated reflect kind.
    fn kind(&self) -> Kind {
        Kind::Ptr
    }

    /// Whether the value has an exported method with this exact name.
    /// Go resolves methods before fields and map keys.
    fn has_method(&self, name: &str) -> bool;

    /// Call an exported method. `None` means "no such method"; the caller
    /// then tries fields and map keys, as Go's `evalField` does. Argument
    /// count and type errors are returned as `Some(Err(..))`.
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<Result<Value>>;

    /// An exported struct field. `None` means "no such field".
    fn field(&self, _name: &str) -> Option<Value> {
        None
    }

    /// For `Kind::Map` objects: exact-key map index.
    fn map_get(&self, _key: &[u8]) -> Option<Value> {
        None
    }

    /// For `Kind::Map` objects: keys in Go's sorted (`fmtsort`) order.
    fn map_keys(&self) -> Vec<GoString> {
        Vec::new()
    }

    /// For `Kind::Slice` objects: the elements, for `range`, `len`, `index`.
    fn list(&self) -> Option<Vec<Value>> {
        None
    }

    /// `types.Zeroer` (`IsZero() bool`): consulted first by Hugo's
    /// truthiness check (`hreflect.IsTruthfulValue`).
    fn is_zero(&self) -> Option<bool> {
        None
    }

    /// `fmt.Stringer`.
    fn go_string(&self) -> Option<GoString> {
        None
    }

    /// `fmt.GoStringer` (`GoString() string`, used by `%#v`).
    fn go_go_string(&self) -> Option<GoString> {
        None
    }

    /// `error`.
    fn go_error(&self) -> Option<String> {
        None
    }

    /// Hugo's `types.PrintableValueProvider`: the value html/template should
    /// print instead of this one (e.g. `hstring.HTML` -> `template.HTML`).
    fn printable_value(&self) -> Option<Value> {
        None
    }

    /// For a named basic type (a Go type whose underlying type is `bool`, a
    /// numeric kind or `string`, e.g. `time.Month`, `time.Duration`,
    /// `hstring.HTML`): the value converted to its underlying type, e.g.
    /// `Value::Int(9, IntKind::Int)` for `time.September` or
    /// `Value::String(..)` for an `hstring.HTML`. This is the value's
    /// `reflect.Kind` and contents where Go reflects on it: `fmt` formats it
    /// when no method applies (`%d` of a `time.Month`, `%#v`, bad verbs),
    /// `Sprint` counts a string kind as a string, and a `*` width accepts an
    /// integer kind. `type_name` stays the named type. `None` (the default)
    /// for every other object; values of other kinds are ignored.
    fn underlying(&self) -> Option<Value> {
        None
    }

    /// Exported struct fields in declaration order, for `fmt` `%v`/`%+v`,
    /// `encoding/json` and `hashstructure` of struct-like objects.
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        None
    }

    /// `json.Marshaler`: raw JSON bytes (they are compacted and HTML-escaped
    /// by the encoder, as in Go).
    fn marshal_json(&self) -> Option<Result<Vec<u8>>> {
        None
    }

    /// `encoding.TextMarshaler`.
    fn marshal_text(&self) -> Option<Result<Vec<u8>>> {
        None
    }

    /// Hugo's `hashing` "Key() string" provider (`toHashable`).
    fn hash_key(&self) -> Option<GoString> {
        None
    }

    /// Identity for equality of pointer-like values (Go compares pointers).
    /// Defaults to the object's address.
    fn identity(&self) -> usize {
        self as *const Self as *const () as usize
    }

    /// Downcasting support for host code.
    fn as_any(&self) -> &dyn Any;
}

// ---------------------------------------------------------------------------
// Value

/// A Go dynamic value (`interface{}` plus reflection).
#[derive(Clone)]
pub enum Value {
    /// The zero `reflect.Value`: a missing map key, an untyped nil interface,
    /// a YAML `null`. text/template prints `<no value>`; html/template prints
    /// nothing; `fmt` prints `<nil>`.
    Invalid,
    /// A typed nil pointer, map, slice or interface, with its Go type string
    /// (e.g. `"*source.File"`). Falsy. Use [`typed_nil_kind`] for its reflect
    /// kind (a nil slice prints `[]` in `fmt`, a nil pointer `<nil>`).
    TypedNil(Arc<str>),
    Bool(bool),
    Int(i64, IntKind),
    Uint(u64, UintKind),
    Float(f64, FloatKind),
    String(GoString),
    /// `template.HTML` and the other html/template content types.
    Safe(SafeKind, GoString),
    Time(Time),
    List(Arc<List>),
    Map(Arc<Map>),
    Object(Arc<dyn Object>),
}

impl Value {
    // -- constructors ----------------------------------------------------

    pub fn string(s: impl Into<GoString>) -> Value {
        Value::String(s.into())
    }

    pub fn html(s: impl Into<GoString>) -> Value {
        Value::Safe(SafeKind::Html, s.into())
    }

    /// A Go `int`.
    pub fn int(i: i64) -> Value {
        Value::Int(i, IntKind::Int)
    }

    /// A Go `int64`.
    pub fn int64(i: i64) -> Value {
        Value::Int(i, IntKind::Int64)
    }

    /// A Go `float64`.
    pub fn float64(f: f64) -> Value {
        Value::Float(f, FloatKind::F64)
    }

    pub fn list(ty: SliceType, items: Vec<Value>) -> Value {
        Value::List(Arc::new(List::new(ty, items)))
    }

    /// A `[]interface {}`.
    pub fn any_list(items: Vec<Value>) -> Value {
        Value::list(SliceType::Any, items)
    }

    /// A `[]string`.
    pub fn string_list<I, S>(items: I) -> Value
    where
        I: IntoIterator<Item = S>,
        S: Into<GoString>,
    {
        Value::list(
            SliceType::String,
            items.into_iter().map(|s| Value::String(s.into())).collect(),
        )
    }

    pub fn map(m: Map) -> Value {
        Value::Map(Arc::new(m))
    }

    pub fn object<O: Object>(o: O) -> Value {
        Value::Object(Arc::new(o))
    }

    // -- inspection --------------------------------------------------------

    pub fn is_invalid(&self) -> bool {
        matches!(self, Value::Invalid)
    }

    /// True for `Invalid` and typed nils.
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Invalid | Value::TypedNil(_))
    }

    /// The string bytes for `String` and `Safe` values.
    pub fn as_go_string(&self) -> Option<&GoString> {
        match self {
            Value::String(s) | Value::Safe(_, s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&List> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Arc<dyn Object>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Downcast an `Object` value to a concrete host type.
    pub fn downcast<T: Object>(&self) -> Option<&T> {
        self.as_object()
            .and_then(|o| o.as_any().downcast_ref::<T>())
    }

    /// The Go type string, as printed by `%T` (`"<nil>"` for `Invalid`).
    pub fn go_type_name(&self) -> Cow<'_, str> {
        match self {
            Value::Invalid => Cow::Borrowed("<nil>"),
            Value::TypedNil(t) => Cow::Borrowed(t),
            Value::Bool(_) => Cow::Borrowed("bool"),
            Value::Int(_, k) => Cow::Borrowed(k.go_name()),
            Value::Uint(_, k) => Cow::Borrowed(k.go_name()),
            Value::Float(_, k) => Cow::Borrowed(k.go_name()),
            Value::String(_) => Cow::Borrowed("string"),
            Value::Safe(k, _) => Cow::Borrowed(k.go_name()),
            Value::Time(_) => Cow::Borrowed("time.Time"),
            Value::List(l) => l.ty.go_name(),
            Value::Map(m) => m.ty.go_name(),
            Value::Object(o) => o.type_name(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Invalid => f.write_str("Invalid"),
            Value::TypedNil(t) => write!(f, "TypedNil({t})"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Int(i, k) => write!(f, "{}({i})", k.go_name()),
            Value::Uint(u, k) => write!(f, "{}({u})", k.go_name()),
            Value::Float(x, k) => write!(f, "{}({x:?})", k.go_name()),
            Value::String(s) => write!(f, "String({s:?})"),
            Value::Safe(k, s) => write!(f, "{}({s:?})", k.go_name()),
            Value::Time(t) => write!(f, "Time({t:?})"),
            Value::List(l) => write!(f, "{}{:?}", l.ty.go_name(), l.items),
            Value::Map(m) => write!(f, "{}{:?}", m.ty.go_name(), m.entries),
            Value::Object(o) => write!(f, "Object({})", o.type_name()),
        }
    }
}

/// Structural equality for tests and caches. Objects compare by identity.
/// This is NOT Go's `eq`: template comparison rules live in the template
/// function crates.
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Invalid, Value::Invalid) => true,
            (Value::TypedNil(a), Value::TypedNil(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a, ka), Value::Int(b, kb)) => a == b && ka == kb,
            (Value::Uint(a, ka), Value::Uint(b, kb)) => a == b && ka == kb,
            (Value::Float(a, ka), Value::Float(b, kb)) => a.to_bits() == b.to_bits() && ka == kb,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Safe(ka, a), Value::Safe(kb, b)) => ka == kb && a == b,
            (Value::Time(a), Value::Time(b)) => a == b,
            (Value::List(a), Value::List(b)) => Arc::ptr_eq(a, b) || a == b,
            (Value::Map(a), Value::Map(b)) => Arc::ptr_eq(a, b) || a == b,
            (Value::Object(a), Value::Object(b)) => a.identity() == b.identity(),
            _ => false,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s.into())
    }
}

impl From<GoString> for Value {
    fn from(s: GoString) -> Self {
        Value::String(s)
    }
}

impl From<Time> for Value {
    fn from(t: Time) -> Self {
        Value::Time(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gostring_orders_by_bytes() {
        let mut v = [
            GoString::from("a"),
            GoString::from("M"),
            GoString::from("z"),
            GoString::from("ä"),
        ];
        v.sort();
        let got: Vec<_> = v.iter().map(|s| s.to_string()).collect();
        assert_eq!(got, ["M", "a", "z", "ä"]);
    }

    #[test]
    fn type_names() {
        assert_eq!(Value::int(1).go_type_name(), "int");
        assert_eq!(Value::int64(1).go_type_name(), "int64");
        assert_eq!(Value::float64(1.0).go_type_name(), "float64");
        assert_eq!(Value::string_list(["a"]).go_type_name(), "[]string");
        assert_eq!(Value::any_list(vec![]).go_type_name(), "[]interface {}");
        assert_eq!(
            Value::map(Map::new(MapType::Params)).go_type_name(),
            "maps.Params"
        );
        assert_eq!(Value::html("x").go_type_name(), "template.HTML");
        assert_eq!(Value::Invalid.go_type_name(), "<nil>");
        assert_eq!(Value::Time(Time::zero()).go_type_name(), "time.Time");
    }

    #[test]
    fn map_iterates_sorted() {
        let mut m = Map::new(MapType::StringAny);
        m.insert("z", Value::int(1));
        m.insert("a", Value::int(2));
        m.insert("M", Value::int(3));
        let keys: Vec<_> = m.entries.keys().map(|k| k.to_string()).collect();
        assert_eq!(keys, ["M", "a", "z"]);
    }
}
