//! Module `decode`.
//!
//! NEW: mitchellh/mapstructure WeakDecode semantics (case-insensitive field match, weak conversions) over go_value::Value
//!
//! Owner: Wave B task T04 (config-base-media).

//! A port of `github.com/mitchellh/mapstructure@v1.5.1-0.20231216201459-8508981c8b6c`
//! (`mapstructure.go`, `error.go`, the `DecodeHookExec` part of `decode_hooks.go` and
//! `StringToTimeDurationHookFunc`) over [`go_value::Value`].
//!
//! Go decodes into any type through reflection. Here the *target* describes itself through the
//! [`Decode`] trait: its Go type string (for error texts), its kind, and the kind-specific
//! mapstructure function (`decodeString`, `decodeInt`, `decodeStruct`, ...). Implementations are
//! provided for the Go types Hugo's config structs use (`string`, `bool`, the int/uint/float
//! kinds, `interface {}`, `map[string]T`, `[]T`, `[N]T`, `*T`, `time.Duration`, maps as
//! [`Map`]), and [`decode_struct!`](crate::decode_struct) implements it for a struct from its
//! field list ([`FieldRef`]: Go field name, `mapstructure` tag, target). The decoding logic
//! (`Decoder::decode`, `decodeStructFromMap`, `decodeMapFromMap`, `decodeSlice`, ...) is Go's,
//! function by function, with Go's error texts and error accumulation
//! ([`DecodeError::Multi`] = `*mapstructure.Error`).

use std::any::Any;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, Map, MapType, SliceType, UintKind, Value,
};
use nh_common::hreflect::{ReflectKind, kind_of};
use nh_common::{Error, Result};

// ---------------------------------------------------------------------------
// Errors

/// A mapstructure error: a plain Go `error` or a `*mapstructure.Error` (a list of messages).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// A plain error (`fmt.Errorf`).
    Single(String),
    /// `*mapstructure.Error{Errors}`.
    Multi(Vec<String>),
    /// A Go runtime panic inside `reflect` (e.g. `reflect: array index out of range`): it
    /// aborts the whole decode (Go crashes; the port returns the panic text).
    Panic(String),
}

impl DecodeError {
    /// Go: `err.Error()`.
    // Go: error.go:(*Error).Error
    pub fn message(&self) -> String {
        match self {
            DecodeError::Single(s) | DecodeError::Panic(s) => s.clone(),
            DecodeError::Multi(errors) => {
                let mut points: Vec<String> = errors.iter().map(|e| format!("* {e}")).collect();
                points.sort();
                format!(
                    "{} error(s) decoding:\n\n{}",
                    errors.len(),
                    points.join("\n")
                )
            }
        }
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Error {
        Error::new(e.message())
    }
}

/// The result of a mapstructure decode function.
pub type DResult = std::result::Result<(), DecodeError>;

/// Go: `appendErrors`; a panic is not accumulated but propagated (`Err`).
// Go: error.go:appendErrors
fn append_errors(errors: &mut Vec<String>, err: DecodeError) -> DResult {
    match err {
        DecodeError::Multi(e) => errors.extend(e),
        DecodeError::Single(s) => errors.push(s),
        DecodeError::Panic(_) => return Err(err),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Targets

/// mapstructure's `getKind` of the output value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutKind {
    Bool,
    Interface,
    String,
    Int,
    Uint,
    Float32,
    Struct,
    Map,
    Ptr,
    Slice,
    Array,
}

/// A value mapstructure can decode into (Go: the `reflect.Value` of the result).
pub trait Decode: Any {
    /// Go: `val.Type().String()`, e.g. `"string"`, `"int"`, `"[]string"`, `"config.Server"`.
    fn go_type(&self) -> Cow<'static, str>;

    /// Go: `getKind(outVal)`.
    fn out_kind(&self) -> OutKind;

    /// The kind-specific decode function of `Decoder.decode` (`decodeString`, `decodeStruct`,
    /// ...), called with a non-nil input after the decode hook ran.
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult;

    /// `outVal.Set(reflect.Zero(outVal.Type()))`.
    fn set_zero(&mut self);

    /// `decodeStruct`: when the data has exactly this type, it is assigned as it is.
    fn assign_same(&mut self, _data: &Value) -> bool {
        false
    }

    /// For structs: the fields in declaration order.
    fn struct_fields(&mut self) -> Option<Vec<FieldRef<'_>>> {
        None
    }

    /// The Go reflect kind name of the element type, for slices whose element kind matters
    /// (`[]uint8` from a string).
    fn is_uint8(&self) -> bool {
        false
    }

    /// Whether the value is its type's zero value (`decodeArray` checks the whole array).
    fn is_zero_value(&self) -> bool {
        false
    }

    /// For map targets: the map operations of `decodeMap`.
    fn as_map_target(&mut self) -> Option<&mut dyn MapTarget> {
        None
    }

    /// For a non-nil pointer to a struct: the struct (mapstructure decodes such struct fields
    /// in place, as embedded structs).
    fn ptr_struct_elem(&mut self) -> Option<&mut dyn Decode> {
        None
    }

    /// Whether [`Decode::ptr_struct_elem`] returns the struct.
    fn is_nonnil_ptr_struct(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
}

/// A struct field as mapstructure sees it (`reflect.StructField` + the field's value).
pub struct FieldRef<'a> {
    /// The Go field name (`fieldType.Name`).
    pub name: &'static str,
    /// The `mapstructure:"..."` tag ("" when there is none).
    pub tag: &'static str,
    /// Unexported fields are matched but never set (`!fieldValue.CanSet()`).
    pub exported: bool,
    /// An embedded field (`fieldType.Anonymous`).
    pub anonymous: bool,
    pub target: &'a mut dyn Decode,
}

impl<'a> FieldRef<'a> {
    /// An exported field without a tag.
    pub fn new(name: &'static str, target: &'a mut dyn Decode) -> Self {
        FieldRef {
            name,
            tag: "",
            exported: true,
            anonymous: false,
            target,
        }
    }

    /// An embedded struct with the `mapstructure:",squash"` tag.
    pub fn squash(name: &'static str, target: &'a mut dyn Decode) -> Self {
        FieldRef {
            name,
            tag: ",squash",
            exported: true,
            anonymous: true,
            target,
        }
    }

    /// Sets the `mapstructure` tag.
    pub fn tag(mut self, tag: &'static str) -> Self {
        self.tag = tag;
        self
    }

    /// Marks the field unexported.
    pub fn unexported(mut self) -> Self {
        self.exported = false;
        self
    }

    /// Marks the field embedded.
    pub fn anonymous(mut self) -> Self {
        self.anonymous = true;
        self
    }
}

/// Implements [`Decode`] for a struct from its field list, e.g.
///
/// ```ignore
/// decode_struct!(BuildStats, "config.BuildStats", |s| vec![
///     FieldRef::new("Enable", &mut s.enable),
///     FieldRef::new("DisableTags", &mut s.disable_tags),
/// ]);
/// ```
///
/// The struct must be `Clone + Default + 'static` (a value of the same Rust type stored in a
/// `Value::Object` is assigned as it is, like Go's same-type shortcut).
#[macro_export]
macro_rules! decode_struct {
    ($ty:ty, $go_type:expr, |$s:ident| $fields:expr) => {
        impl $crate::decode::Decode for $ty {
            fn go_type(&self) -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed($go_type)
            }
            fn out_kind(&self) -> $crate::decode::OutKind {
                $crate::decode::OutKind::Struct
            }
            fn decode_kind(
                &mut self,
                d: &$crate::decode::Decoder<'_>,
                name: &str,
                data: &::go_value::Value,
            ) -> $crate::decode::DResult {
                $crate::decode::decode_struct(d, name, data, self)
            }
            fn set_zero(&mut self) {
                *self = <$ty as ::std::default::Default>::default();
            }
            fn assign_same(&mut self, data: &::go_value::Value) -> bool {
                match data
                    .as_object()
                    .and_then(|o| o.as_any().downcast_ref::<$ty>())
                {
                    Some(v) => {
                        *self = v.clone();
                        true
                    }
                    None => false,
                }
            }
            fn struct_fields(&mut self) -> Option<Vec<$crate::decode::FieldRef<'_>>> {
                let $s = self;
                Some($fields)
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Decoder

/// A decode hook (Go `DecodeHookFuncType`: from type, to type, data). The from type is the
/// data's own; the target is passed so the hook can test its type (`as_any().is::<T>()`).
pub type DecodeHook<'a> = dyn Fn(&Value, &dyn Decode) -> std::result::Result<Value, String> + 'a;

/// Go: `mapstructure.DecoderConfig` (the options Hugo uses; `Metadata`, `TagName` and
/// `MatchName` keep their defaults: no metadata, `mapstructure`, `strings.EqualFold`).
#[derive(Clone, Copy, Default)]
pub struct DecoderConfig<'a> {
    pub decode_hook: Option<&'a DecodeHook<'a>>,
    pub error_unused: bool,
    pub error_unset: bool,
    pub zero_fields: bool,
    pub weakly_typed_input: bool,
    pub squash: bool,
}

/// Go: `mapstructure.Decoder`.
pub struct Decoder<'a> {
    config: DecoderConfig<'a>,
}

impl<'a> Decoder<'a> {
    // Go: mapstructure.go:NewDecoder
    pub fn new(config: DecoderConfig<'a>) -> Self {
        Decoder { config }
    }

    /// Go: `Decoder.Decode(input)`.
    // Go: mapstructure.go:(*Decoder).Decode
    pub fn decode_input(&self, input: &Value, out: &mut dyn Decode) -> DResult {
        self.decode("", input, out)
    }

    /// Go: `Decoder.decode(name, input, outVal)`: decodes an unknown data type into a specific
    /// reflection value.
    // Go: mapstructure.go:(*Decoder).decode
    pub fn decode(&self, name: &str, input: &Value, out: &mut dyn Decode) -> DResult {
        // We need to check here if input is a typed nil. Typed nils won't match the
        // "input == nil" below so we check that here.
        if is_nil_input(input) {
            // If the data is nil, then we don't set anything, unless ZeroFields is set to true.
            if self.config.zero_fields {
                out.set_zero();
            }
            return Ok(());
        }

        let mut input = Cow::Borrowed(input);
        if let Some(hook) = self.config.decode_hook {
            // We have a DecodeHook, so let's pre-process the input.
            match hook(&input, out) {
                Ok(v) => input = Cow::Owned(v),
                Err(e) => {
                    return Err(DecodeError::Single(format!("error decoding '{name}': {e}")));
                }
            }
        }

        out.decode_kind(self, name, &input)
    }

    fn weak(&self) -> bool {
        self.config.weakly_typed_input
    }
}

/// Go: `mapstructure.WeakDecode(input, &output)`.
// Go: mapstructure.go:WeakDecode
pub fn weak_decode_into(input: &Value, output: &mut dyn Decode) -> Result<()> {
    let d = Decoder::new(DecoderConfig {
        weakly_typed_input: true,
        ..Default::default()
    });
    d.decode_input(input, output).map_err(Error::from)
}

/// Go: `mapstructure.Decode(input, &output)` (no weak conversions).
// Go: mapstructure.go:Decode
pub fn decode_into(input: &Value, output: &mut dyn Decode) -> Result<()> {
    let d = Decoder::new(DecoderConfig::default());
    d.decode_input(input, output).map_err(Error::from)
}

/// `decode`'s nil test: a nil interface or a typed nil pointer.
fn is_nil_input(v: &Value) -> bool {
    match v {
        Value::Invalid => true,
        Value::TypedNil(_) => matches!(kind_of(v), ReflectKind::Ptr | ReflectKind::Invalid),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Reflection helpers

/// mapstructure's `getKind` of `reflect.Indirect(reflect.ValueOf(data))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MsKind {
    Invalid,
    Bool,
    Int,
    Uint,
    Float,
    String,
    Slice,
    Map,
    Struct,
    Interface,
    Func,
    Chan,
}

// Go: mapstructure.go:getKind
fn get_kind(v: &Value) -> MsKind {
    match kind_of(v) {
        ReflectKind::Invalid => MsKind::Invalid,
        ReflectKind::Bool => MsKind::Bool,
        ReflectKind::Int(_) => MsKind::Int,
        ReflectKind::Uint(_) => MsKind::Uint,
        ReflectKind::Float32 | ReflectKind::Float64 => MsKind::Float,
        ReflectKind::String => MsKind::String,
        ReflectKind::Slice => MsKind::Slice,
        ReflectKind::Map => MsKind::Map,
        // reflect.Indirect: a (non-nil) pointer object stands for the struct it points to.
        ReflectKind::Struct | ReflectKind::Ptr => MsKind::Struct,
        ReflectKind::Interface => MsKind::Interface,
        ReflectKind::Func => MsKind::Func,
        ReflectKind::Chan => MsKind::Chan,
    }
}

/// `reflect.Indirect(reflect.ValueOf(data)).Kind().String()`.
fn kind_name(v: &Value) -> Cow<'static, str> {
    Cow::Borrowed(match kind_of(v) {
        ReflectKind::Invalid => "invalid",
        ReflectKind::Bool => "bool",
        ReflectKind::Int(k) => k.go_name(),
        ReflectKind::Uint(k) => k.go_name(),
        ReflectKind::Float32 => "float32",
        ReflectKind::Float64 => "float64",
        ReflectKind::String => "string",
        ReflectKind::Slice => "slice",
        ReflectKind::Map => "map",
        ReflectKind::Struct | ReflectKind::Ptr => "struct",
        ReflectKind::Interface => "interface",
        ReflectKind::Func => "func",
        ReflectKind::Chan => "chan",
    })
}

/// `reflect.Indirect(reflect.ValueOf(data)).Type().String()`.
fn indirect_type_name(v: &Value) -> String {
    let t = v.go_type_name();
    if let Value::Object(o) = v
        && o.kind() == Kind::Ptr
        && let Some(s) = t.strip_prefix('*')
    {
        return s.to_string();
    }
    t.into_owned()
}

/// The underlying basic value (named basic types reflect as their kind).
fn basic(v: &Value) -> Cow<'_, Value> {
    match v {
        Value::Object(o) => match o.underlying() {
            Some(u) => Cow::Owned(u),
            None => Cow::Borrowed(v),
        },
        _ => Cow::Borrowed(v),
    }
}

fn data_str(v: &Value) -> GoString {
    match &*basic(v) {
        Value::String(s) | Value::Safe(_, s) => s.clone(),
        _ => GoString::empty(),
    }
}

fn data_bool(v: &Value) -> bool {
    matches!(&*basic(v), Value::Bool(true))
}

fn data_int(v: &Value) -> i64 {
    match &*basic(v) {
        Value::Int(i, _) => *i,
        _ => 0,
    }
}

fn data_uint(v: &Value) -> u64 {
    match &*basic(v) {
        Value::Uint(u, _) => *u,
        _ => 0,
    }
}

fn data_float(v: &Value) -> f64 {
    match &*basic(v) {
        Value::Float(f, _) => *f,
        _ => 0.0,
    }
}

/// `fmt.Sprintf("%v", data)`.
fn fmt_v(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(v))).into_owned()
}

/// `'%s' expected type '%s', got unconvertible type '%s', value: '%v'`.
fn unconvertible(name: &str, typ: &str, data: &Value) -> DecodeError {
    DecodeError::Single(format!(
        "'{}' expected type '{}', got unconvertible type '{}', value: '{}'",
        name,
        typ,
        indirect_type_name(data),
        fmt_v(data)
    ))
}

fn lossy(s: &[u8]) -> String {
    String::from_utf8_lossy(s).into_owned()
}

/// The elements of a slice (or of a map, for `Len`) value.
fn slice_items(v: &Value) -> Option<Cow<'_, [Value]>> {
    match v {
        Value::List(l) => Some(Cow::Borrowed(&l.items[..])),
        Value::TypedNil(_) => Some(Cow::Owned(Vec::new())),
        Value::Object(o) => o.list().map(Cow::Owned),
        _ => None,
    }
}

fn is_nil_container(v: &Value) -> bool {
    matches!(v, Value::TypedNil(_))
}

// ---------------------------------------------------------------------------
// Basic kinds

/// Go: `decodeString` into a string target of type `typ`.
// Go: mapstructure.go:(*Decoder).decodeString
pub fn decode_string(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut GoString,
    typ: &str,
) -> DResult {
    let weak = d.weak();
    match get_kind(data) {
        MsKind::String => *val = data_str(data),
        MsKind::Bool if weak => {
            *val = GoString::from(if data_bool(data) { "1" } else { "0" });
        }
        MsKind::Int if weak => *val = GoString::from(data_int(data).to_string()),
        MsKind::Uint if weak => *val = GoString::from(data_uint(data).to_string()),
        MsKind::Float if weak => {
            *val = GoString::from(go_strconv::format_float(data_float(data), b'f', -1, 64));
        }
        MsKind::Slice if weak => match data {
            Value::List(l) if l.ty == SliceType::Uint8 => {
                let bytes: Vec<u8> = l
                    .items
                    .iter()
                    .map(|x| match &*basic(x) {
                        Value::Uint(u, _) => *u as u8,
                        _ => 0,
                    })
                    .collect();
                *val = GoString::from(bytes);
            }
            Value::TypedNil(t) if &**t == "[]uint8" => *val = GoString::empty(),
            _ => return Err(unconvertible(name, typ, data)),
        },
        _ => return Err(unconvertible(name, typ, data)),
    }
    Ok(())
}

/// Go: `reflect.Value.SetInt` on an int kind of `bits` bits (the value is truncated).
fn trunc_int(i: i64, bits: u32) -> i64 {
    match bits {
        8 => i as i8 as i64,
        16 => i as i16 as i64,
        32 => i as i32 as i64,
        _ => i,
    }
}

fn trunc_uint(u: u64, bits: u32) -> u64 {
    match bits {
        8 => u as u8 as u64,
        16 => u as u16 as u64,
        32 => u as u32 as u64,
        _ => u,
    }
}

/// Go: `decodeInt` into an int target of `bits` bits and type `typ`.
// Go: mapstructure.go:(*Decoder).decodeInt
pub fn decode_int(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut i64,
    bits: u32,
    typ: &str,
) -> DResult {
    let weak = d.weak();
    match get_kind(data) {
        MsKind::Int => *val = trunc_int(data_int(data), bits),
        MsKind::Uint => *val = trunc_int(data_uint(data) as i64, bits),
        // Go: int64(f) — truncation; arm64 saturates out-of-range values and NaN -> 0, like
        // Rust's `as`.
        MsKind::Float => *val = trunc_int(data_float(data) as i64, bits),
        MsKind::Bool if weak => *val = i64::from(data_bool(data)),
        MsKind::String if weak => {
            let s = data_str(data);
            let str: &[u8] = if s.is_empty() { b"0" } else { s.as_bytes() };
            match go_strconv::parse_int(str, 0, i64::from(bits)) {
                Ok(i) => *val = trunc_int(i, bits),
                Err(err) => {
                    return Err(DecodeError::Single(format!(
                        "cannot parse '{name}' as int: {err}"
                    )));
                }
            }
        }
        _ => return Err(unconvertible(name, typ, data)),
    }
    Ok(())
}

/// Go: `decodeUint`.
// Go: mapstructure.go:(*Decoder).decodeUint
pub fn decode_uint(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut u64,
    bits: u32,
    typ: &str,
) -> DResult {
    let weak = d.weak();
    match get_kind(data) {
        MsKind::Int => {
            let i = data_int(data);
            if i < 0 && !weak {
                return Err(DecodeError::Single(format!(
                    "cannot parse '{name}', {i} overflows uint"
                )));
            }
            *val = trunc_uint(i as u64, bits);
        }
        MsKind::Uint => *val = trunc_uint(data_uint(data), bits),
        MsKind::Float => {
            let f = data_float(data);
            if f < 0.0 && !weak {
                return Err(DecodeError::Single(format!(
                    "cannot parse '{name}', {} overflows uint",
                    go_strconv::format_float(f, b'f', 6, 64)
                )));
            }
            // Go: uint64(f) — arm64 saturates (negative and NaN -> 0), like Rust's `as`.
            *val = trunc_uint(f as u64, bits);
        }
        MsKind::Bool if weak => *val = u64::from(data_bool(data)),
        MsKind::String if weak => {
            let s = data_str(data);
            let str: &[u8] = if s.is_empty() { b"0" } else { s.as_bytes() };
            match go_strconv::parse_uint(str, 0, i64::from(bits)) {
                Ok(u) => *val = u,
                Err(err) => {
                    return Err(DecodeError::Single(format!(
                        "cannot parse '{name}' as uint: {err}"
                    )));
                }
            }
        }
        _ => return Err(unconvertible(name, typ, data)),
    }
    Ok(())
}

/// Go: `decodeBool`.
// Go: mapstructure.go:(*Decoder).decodeBool
pub fn decode_bool(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut bool,
    typ: &str,
) -> DResult {
    let weak = d.weak();
    match get_kind(data) {
        MsKind::Bool => *val = data_bool(data),
        MsKind::Int if weak => *val = data_int(data) != 0,
        MsKind::Uint if weak => *val = data_uint(data) != 0,
        MsKind::Float if weak => *val = data_float(data) != 0.0,
        MsKind::String if weak => {
            let s = data_str(data);
            match go_strconv::parse_bool(s.as_bytes()) {
                Ok(b) => *val = b,
                Err(_) if s.is_empty() => *val = false,
                Err(err) => {
                    return Err(DecodeError::Single(format!(
                        "cannot parse '{name}' as bool: {err}"
                    )));
                }
            }
        }
        _ => return Err(unconvertible(name, typ, data)),
    }
    Ok(())
}

/// Go: `decodeFloat` into a float target of `bits` bits.
// Go: mapstructure.go:(*Decoder).decodeFloat
pub fn decode_float(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut f64,
    bits: u32,
    typ: &str,
) -> DResult {
    let weak = d.weak();
    // Go: reflect.Value.SetFloat on a float32 rounds to float32.
    let set = |f: f64| if bits == 32 { f as f32 as f64 } else { f };
    match get_kind(data) {
        MsKind::Int => *val = set(data_int(data) as f64),
        MsKind::Uint => *val = set(data_uint(data) as f64),
        MsKind::Float => *val = set(data_float(data)),
        MsKind::Bool if weak => *val = if data_bool(data) { 1.0 } else { 0.0 },
        MsKind::String if weak => {
            let s = data_str(data);
            let str: &[u8] = if s.is_empty() { b"0" } else { s.as_bytes() };
            match go_strconv::parse_float(str, i64::from(bits)) {
                Ok(f) => *val = set(f),
                Err(err) => {
                    return Err(DecodeError::Single(format!(
                        "cannot parse '{name}' as float: {err}"
                    )));
                }
            }
        }
        _ => return Err(unconvertible(name, typ, data)),
    }
    Ok(())
}

impl Decode for GoString {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("string")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::String
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_string(d, name, data, self, "string")
    }
    fn set_zero(&mut self) {
        *self = GoString::empty();
    }
    fn is_zero_value(&self) -> bool {
        self.is_empty()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go strings are bytes; a Rust `String` field receives them converted lossily (see PORTING.md).
impl Decode for String {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("string")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::String
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        let mut s = GoString::from(self.as_str());
        let r = decode_string(d, name, data, &mut s, "string");
        *self = lossy(s.as_bytes());
        r
    }
    fn set_zero(&mut self) {
        self.clear();
    }
    fn is_zero_value(&self) -> bool {
        self.is_empty()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Decode for bool {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("bool")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Bool
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_bool(d, name, data, self, "bool")
    }
    fn set_zero(&mut self) {
        *self = false;
    }
    fn is_zero_value(&self) -> bool {
        !*self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

macro_rules! impl_int {
    ($t:ty, $go:expr, $bits:expr) => {
        impl Decode for $t {
            fn go_type(&self) -> Cow<'static, str> {
                Cow::Borrowed($go)
            }
            fn out_kind(&self) -> OutKind {
                OutKind::Int
            }
            fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
                let mut v = *self as i64;
                let r = decode_int(d, name, data, &mut v, $bits, $go);
                *self = v as $t;
                r
            }
            fn set_zero(&mut self) {
                *self = 0;
            }
            fn is_zero_value(&self) -> bool {
                *self == 0
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

// Go `int` is 64 bits: `i64` fields are Go `int`s.
impl_int!(i64, "int", 64);
impl_int!(i32, "int32", 32);
impl_int!(i16, "int16", 16);
impl_int!(i8, "int8", 8);

macro_rules! impl_uint {
    ($t:ty, $go:expr, $bits:expr, $u8:expr) => {
        impl Decode for $t {
            fn go_type(&self) -> Cow<'static, str> {
                Cow::Borrowed($go)
            }
            fn out_kind(&self) -> OutKind {
                OutKind::Uint
            }
            fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
                let mut v = *self as u64;
                let r = decode_uint(d, name, data, &mut v, $bits, $go);
                *self = v as $t;
                r
            }
            fn set_zero(&mut self) {
                *self = 0;
            }
            fn is_uint8(&self) -> bool {
                $u8
            }
            fn is_zero_value(&self) -> bool {
                *self == 0
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

// `u64` fields are Go `uint`s.
impl_uint!(u64, "uint", 64, false);
impl_uint!(u32, "uint32", 32, false);
impl_uint!(u16, "uint16", 16, false);
impl_uint!(u8, "uint8", 8, true);

impl Decode for f64 {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("float64")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Float32
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_float(d, name, data, self, 64, "float64")
    }
    fn set_zero(&mut self) {
        *self = 0.0;
    }
    fn is_zero_value(&self) -> bool {
        *self == 0.0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Decode for f32 {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("float32")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Float32
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        let mut v = f64::from(*self);
        let r = decode_float(d, name, data, &mut v, 32, "float32");
        *self = v as f32;
        r
    }
    fn set_zero(&mut self) {
        *self = 0.0;
    }
    fn is_zero_value(&self) -> bool {
        *self == 0.0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Go `int64` field (`i64` fields are Go `int`s; the type name is in error texts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Int64(pub i64);

impl Decode for Int64 {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("int64")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Int
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_int(d, name, data, &mut self.0, 64, "int64")
    }
    fn set_zero(&mut self) {
        self.0 = 0;
    }
    fn is_zero_value(&self) -> bool {
        self.0 == 0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Go `uint64` field (`u64` fields are Go `uint`s).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uint64(pub u64);

impl Decode for Uint64 {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("uint64")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Uint
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_uint(d, name, data, &mut self.0, 64, "uint64")
    }
    fn set_zero(&mut self) {
        self.0 = 0;
    }
    fn is_zero_value(&self) -> bool {
        self.0 == 0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An `interface {}` element of a slice or map (`[]any`, `map[string]T` with `T = any`):
/// [`Value`] with Go's nil as the default.
#[derive(Clone, Debug, PartialEq)]
pub struct AnyValue(pub Value);

impl Default for AnyValue {
    fn default() -> Self {
        AnyValue(Value::Invalid)
    }
}

impl Decode for AnyValue {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("interface {}")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Interface
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_basic(d, name, data, &mut self.0)
    }
    fn set_zero(&mut self) {
        self.0 = Value::Invalid;
    }
    fn is_zero_value(&self) -> bool {
        self.0.is_invalid()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `time.Duration` (an `int64` kind).
impl Decode for go_time::Duration {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("time.Duration")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Int
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_int(d, name, data, &mut self.0, 64, "time.Duration")
    }
    fn set_zero(&mut self) {
        self.0 = 0;
    }
    fn is_zero_value(&self) -> bool {
        self.0 == 0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// interface {}

/// An `interface {}` target (`Value::Invalid` = nil).
impl Decode for Value {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed("interface {}")
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Interface
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_basic(d, name, data, self)
    }
    fn set_zero(&mut self) {
        *self = Value::Invalid;
    }
    fn is_zero_value(&self) -> bool {
        self.is_invalid()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `decodeBasic` into an `interface {}` value.
// Go: mapstructure.go:(*Decoder).decodeBasic
fn decode_basic(d: &Decoder<'_>, name: &str, data: &Value, val: &mut Value) -> DResult {
    if !matches!(val, Value::Invalid) {
        // The interface holds a value: decode into a copy of it (its dynamic type) and store
        // the result.
        return decode_into_dynamic(d, name, data, val);
    }

    // Any value is assignable to interface {}.
    *val = data.clone();
    Ok(())
}

/// Decodes `data` into a copy of the dynamic value held by an interface and stores it.
fn decode_into_dynamic(d: &Decoder<'_>, name: &str, data: &Value, val: &mut Value) -> DResult {
    match val.clone() {
        Value::String(s) => {
            let mut t = s;
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::String(t);
            }
            r
        }
        Value::Bool(b) => {
            let mut t = b;
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::Bool(t);
            }
            r
        }
        Value::Int(i, k) => {
            let mut t = TypedInt(i, k);
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::Int(t.0, k);
            }
            r
        }
        Value::Uint(u, k) => {
            let mut t = TypedUint(u, k);
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::Uint(t.0, k);
            }
            r
        }
        Value::Float(f, FloatKind::F64) => {
            let mut t = f;
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::Float(t, FloatKind::F64);
            }
            r
        }
        Value::Float(f, FloatKind::F32) => {
            let mut t = f as f32;
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::Float(f64::from(t), FloatKind::F32);
            }
            r
        }
        Value::Map(m) => {
            let mut t = (*m).clone();
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::map(t);
            }
            r
        }
        Value::List(l) => {
            let mut t = (*l).clone();
            let r = d.decode(name, data, &mut t);
            if r.is_ok() {
                *val = Value::List(Arc::new(t));
            }
            r
        }
        // Other dynamic types (pointers, structs, typed nils): assigned as they are (see
        // PORTING.md).
        _ => {
            *val = data.clone();
            Ok(())
        }
    }
}

/// An int of a given kind (the dynamic value of an interface).
struct TypedInt(i64, IntKind);

impl Decode for TypedInt {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.1.go_name())
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Int
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        let bits = (self.1.size() * 8) as u32;
        decode_int(d, name, data, &mut self.0, bits, self.1.go_name())
    }
    fn set_zero(&mut self) {
        self.0 = 0;
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct TypedUint(u64, UintKind);

impl Decode for TypedUint {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.1.go_name())
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Uint
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        let bits = (self.1.size() * 8) as u32;
        decode_uint(d, name, data, &mut self.0, bits, self.1.go_name())
    }
    fn set_zero(&mut self) {
        self.0 = 0;
    }
    fn is_uint8(&self) -> bool {
        self.1 == UintKind::Uint8
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Maps

/// A map target with string keys (Go `map[string]T`).
pub trait MapTarget {
    /// Go: `val.Type().String()`.
    fn map_go_type(&self) -> String;
    /// The element type string.
    fn elem_go_type(&self) -> String;
    /// Whether an element of the given type is assignable (for `decodeMapFromStruct`).
    fn elem_is_interface(&self) -> bool;
    /// `val.Set(nil)` / `val.Set(make(map))`: remove all entries.
    fn clear_map(&mut self);
    /// Decodes `v` into a new zero element and inserts it under `key`.
    fn decode_entry(&mut self, d: &Decoder<'_>, name: &str, key: &GoString, v: &Value) -> DResult;
    /// Inserts a value as it is (struct fields assignable to the element type).
    fn insert_value(&mut self, key: &GoString, v: Value);
}

/// Go: `decodeMap` (`val` is a map target: [`Decode::as_map_target`]).
// Go: mapstructure.go:(*Decoder).decodeMap
pub fn decode_map(d: &Decoder<'_>, name: &str, data: &Value, val: &mut dyn Decode) -> DResult {
    match get_kind(data) {
        MsKind::Map => decode_map_from_map(d, name, data, map_target(val)),
        MsKind::Struct => decode_map_from_struct(d, name, data, map_target(val)),
        MsKind::Slice if d.weak() => decode_map_from_slice(d, name, data, val),
        _ => Err(DecodeError::Single(format!(
            "'{}' expected a map, got '{}'",
            name,
            kind_name(data)
        ))),
    }
}

fn map_target(val: &mut dyn Decode) -> &mut dyn MapTarget {
    val.as_map_target()
        .expect("decodeMap: the target is not a map")
}

// Go: mapstructure.go:(*Decoder).decodeMapFromSlice
fn decode_map_from_slice(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut dyn Decode,
) -> DResult {
    let items = slice_items(data).unwrap_or_default();
    // Special case for BC reasons (covered by tests)
    if items.is_empty() {
        return Ok(());
    }

    for (i, item) in items.iter().enumerate() {
        let n = format!("{name}[{i}]");
        d.decode(&n, item, val)?;
    }
    Ok(())
}

/// The type of a map value's keys as mapstructure sees them.
fn map_key_is_interface(data: &Value) -> bool {
    match data {
        Value::Map(m) => matches!(&m.ty, MapType::Named(n) if n.starts_with("map[interface {}]")),
        _ => false,
    }
}

// Go: mapstructure.go:(*Decoder).decodeMapFromMap
fn decode_map_from_map(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut dyn MapTarget,
) -> DResult {
    // By default we overwrite keys in the current map; if we're purposely zeroing fields, a new
    // map is made.
    if d.config.zero_fields {
        val.clear_map();
    }

    // Accumulate errors
    let mut errors: Vec<String> = Vec::new();

    let entries: Cow<'_, BTreeMap<GoString, Value>> = match data {
        Value::Map(m) => Cow::Borrowed(&m.entries),
        _ => Cow::Owned(BTreeMap::new()),
    };

    // If the input data is empty, then we just match what the input data is.
    if entries.is_empty() {
        if is_nil_container(data) {
            // A nil map sets the target to nil.
            val.clear_map();
        }
        return Ok(());
    }

    let iface_keys = map_key_is_interface(data);
    for (k, v) in entries.iter() {
        // reflect.Value.String() of an interface-typed key is "<interface {} Value>".
        let field_name = if iface_keys {
            format!("{name}[<interface {{}} Value>]")
        } else {
            format!("{}[{}]", name, lossy(k.as_bytes()))
        };

        // The key is a string; decoding it into the string key type cannot fail.
        if let Err(e) = val.decode_entry(d, &field_name, k, v) {
            append_errors(&mut errors, e)?;
        }
    }

    // If we had errors, return those
    if !errors.is_empty() {
        return Err(DecodeError::Multi(errors));
    }
    Ok(())
}

/// The exported fields of a struct value (Go `decodeMapFromStruct`'s input): the fields of a
/// struct object, none for `time.Time`.
fn struct_value_fields(data: &Value) -> Vec<(String, Value)> {
    match data {
        Value::Object(o) => o
            .struct_fields()
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k.into_owned(), v))
            .collect(),
        _ => Vec::new(),
    }
}

// Go: mapstructure.go:(*Decoder).decodeMapFromStruct
fn decode_map_from_struct(
    d: &Decoder<'_>,
    _name: &str,
    data: &Value,
    val: &mut dyn MapTarget,
) -> DResult {
    for (key_name, v) in struct_value_fields(data) {
        // Next get the actual value of this field and verify it is assignable to the map
        // value.
        let assignable = val.elem_is_interface() || v.go_type_name() == val.elem_go_type();
        if !assignable {
            return Err(DecodeError::Single(format!(
                "cannot assign type '{}' to map value field of type '{}'",
                v.go_type_name(),
                val.elem_go_type()
            )));
        }
        let key = GoString::from(key_name.as_str());
        match get_kind(&v) {
            // this is an embedded struct, so handle it differently
            MsKind::Struct if !matches!(v, Value::Object(ref o) if o.kind() == Kind::Ptr) => {
                let mut sub = Map::new(MapType::StringAny);
                d.decode(&key_name, &v, &mut sub)?;
                val.insert_value(&key, Value::map(sub));
            }
            _ => val.insert_value(&key, v),
        }
    }
    Ok(())
}

/// A Go map value (`map[string]interface {}`, `maps.Params`, `map[string]string`, ...).
impl MapTarget for Map {
    fn map_go_type(&self) -> String {
        self.ty.go_name().into_owned()
    }
    fn elem_go_type(&self) -> String {
        match self.ty {
            MapType::StringString => "string".to_string(),
            _ => "interface {}".to_string(),
        }
    }
    fn elem_is_interface(&self) -> bool {
        self.ty != MapType::StringString
    }
    fn clear_map(&mut self) {
        self.entries.clear();
    }
    fn decode_entry(&mut self, d: &Decoder<'_>, name: &str, key: &GoString, v: &Value) -> DResult {
        if self.ty == MapType::StringString {
            let mut s = GoString::empty();
            d.decode(name, v, &mut s)?;
            self.entries.insert(key.clone(), Value::String(s));
        } else {
            let mut e = Value::Invalid;
            d.decode(name, v, &mut e)?;
            self.entries.insert(key.clone(), e);
        }
        Ok(())
    }
    fn insert_value(&mut self, key: &GoString, v: Value) {
        self.entries.insert(key.clone(), v);
    }
}

impl Decode for Map {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(self.map_go_type())
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Map
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_map(d, name, data, self)
    }
    fn set_zero(&mut self) {
        self.entries.clear();
    }
    fn is_zero_value(&self) -> bool {
        self.entries.is_empty()
    }
    fn as_map_target(&mut self) -> Option<&mut dyn MapTarget> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go `map[string]T` (keys as Rust strings; see PORTING.md).
impl<T: Decode + Default> MapTarget for BTreeMap<String, T> {
    fn map_go_type(&self) -> String {
        format!("map[string]{}", self.elem_go_type())
    }
    fn elem_go_type(&self) -> String {
        T::default().go_type().into_owned()
    }
    fn elem_is_interface(&self) -> bool {
        T::default().out_kind() == OutKind::Interface
    }
    fn clear_map(&mut self) {
        self.clear();
    }
    fn decode_entry(&mut self, d: &Decoder<'_>, name: &str, key: &GoString, v: &Value) -> DResult {
        let mut e = T::default();
        d.decode(name, v, &mut e)?;
        self.insert(lossy(key.as_bytes()), e);
        Ok(())
    }
    fn insert_value(&mut self, key: &GoString, v: Value) {
        let mut e = T::default();
        let d = Decoder::new(DecoderConfig::default());
        let _ = d.decode("", &v, &mut e);
        self.insert(lossy(key.as_bytes()), e);
    }
}

impl<T: Decode + Default> Decode for BTreeMap<String, T> {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(self.map_go_type())
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Map
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_map(d, name, data, self)
    }
    fn set_zero(&mut self) {
        self.clear();
    }
    fn is_zero_value(&self) -> bool {
        self.is_empty()
    }
    fn as_map_target(&mut self) -> Option<&mut dyn MapTarget> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Pointers, slices, arrays

/// Go `*T` (`None` = nil).
impl<T: Decode + Default> Decode for Option<Box<T>> {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(format!("*{}", T::default().go_type()))
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Ptr
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_ptr(d, name, data, self)
    }
    fn set_zero(&mut self) {
        *self = None;
    }
    fn is_zero_value(&self) -> bool {
        self.is_none()
    }
    fn ptr_struct_elem(&mut self) -> Option<&mut dyn Decode> {
        match self {
            Some(b) if b.out_kind() == OutKind::Struct => Some(&mut **b),
            _ => None,
        }
    }
    fn is_nonnil_ptr_struct(&self) -> bool {
        matches!(self, Some(b) if b.out_kind() == OutKind::Struct)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Go: mapstructure.go:(*Decoder).decodePtr
fn decode_ptr<T: Decode + Default>(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut Option<Box<T>>,
) -> DResult {
    // If the input data is nil, then we want to just set the output pointer to be nil as well.
    let is_nil = match data {
        Value::Invalid => true,
        Value::TypedNil(_) => matches!(
            kind_of(data),
            ReflectKind::Chan
                | ReflectKind::Func
                | ReflectKind::Invalid
                | ReflectKind::Map
                | ReflectKind::Slice
        ),
        _ => false,
    };
    if is_nil {
        *val = None;
        return Ok(());
    }

    // Create an element of the concrete (non pointer) type and decode into that. Then set the
    // value of the pointer to this type.
    match val {
        Some(b) if !d.config.zero_fields => d.decode(name, data, &mut **b),
        _ => {
            let mut real = T::default();
            d.decode(name, data, &mut real)?;
            *val = Some(Box::new(real));
            Ok(())
        }
    }
}

/// Go `[]T` (an empty `Vec` is Go's nil slice; see PORTING.md).
impl<T: Decode + Default> Decode for Vec<T> {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(format!("[]{}", T::default().go_type()))
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Slice
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_slice(d, name, data, self, &T::default)
    }
    fn set_zero(&mut self) {
        self.clear();
    }
    fn is_zero_value(&self) -> bool {
        self.is_empty()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `[]interface {}{data}`.
fn lift(data: &Value) -> Value {
    Value::any_list(vec![data.clone()])
}

// Go: mapstructure.go:(*Decoder).decodeSlice
fn decode_slice<T: Decode>(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut Vec<T>,
    zero: &dyn Fn() -> T,
) -> DResult {
    let data_kind = get_kind(data);

    // If we have a non array/slice type then we first attempt to convert.
    if data_kind != MsKind::Slice {
        if d.weak() {
            return match data_kind {
                // Empty maps turn into empty slices
                MsKind::Map => {
                    let empty = match data {
                        Value::Map(m) => m.entries.is_empty(),
                        _ => true,
                    };
                    if empty {
                        val.clear();
                        return Ok(());
                    }
                    // Create slice of maps of other sizes
                    decode_slice(d, name, &lift(data), val, zero)
                }
                MsKind::String if zero().is_uint8() => {
                    let bytes = data_str(data);
                    let items = bytes
                        .as_bytes()
                        .iter()
                        .map(|b| Value::Uint(u64::from(*b), UintKind::Uint8))
                        .collect();
                    decode_slice(d, name, &Value::list(SliceType::Uint8, items), val, zero)
                }
                // All other types we try to convert to the slice type and "lift" it into it.
                // i.e. a string becomes a string slice.
                _ => decode_slice(d, name, &lift(data), val, zero),
            };
        }

        return Err(DecodeError::Single(format!(
            "'{}': source data must be an array or slice, got {}",
            name,
            kind_name(data)
        )));
    }

    // If the input value is nil, then don't allocate since empty != nil
    if is_nil_container(data) {
        return Ok(());
    }

    let items = slice_items(data).unwrap_or_default();

    if d.config.zero_fields {
        val.clear();
    }
    if val.len() > items.len() {
        val.truncate(items.len());
    }

    // Accumulate any errors
    let mut errors: Vec<String> = Vec::new();

    for (i, current_data) in items.iter().enumerate() {
        while val.len() <= i {
            val.push(zero());
        }
        let field_name = format!("{name}[{i}]");
        if let Err(e) = d.decode(&field_name, current_data, &mut val[i]) {
            append_errors(&mut errors, e)?;
        }
    }

    // If there were errors, we return those
    if !errors.is_empty() {
        return Err(DecodeError::Multi(errors));
    }
    Ok(())
}

/// A Go `[]T` slice value typed by its [`SliceType`] (the dynamic value of an interface).
impl Decode for go_value::List {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(self.ty.go_name().into_owned())
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Slice
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        // Decode element-wise into interface {} elements typed like the existing ones.
        let mut items: Vec<Value> = std::mem::take(&mut self.items);
        let r = decode_slice(d, name, data, &mut items, &|| Value::Invalid);
        self.items = items;
        r
    }
    fn set_zero(&mut self) {
        self.items.clear();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go `[N]T`.
impl<T: Decode + Default, const N: usize> Decode for [T; N] {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(format!("[{}]{}", N, T::default().go_type()))
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Array
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        decode_array(d, name, data, self)
    }
    fn set_zero(&mut self) {
        for e in self.iter_mut() {
            e.set_zero();
        }
    }
    fn is_zero_value(&self) -> bool {
        self.iter().all(|e| e.is_zero_value())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Go: mapstructure.go:(*Decoder).decodeArray
fn decode_array<T: Decode + Default, const N: usize>(
    d: &Decoder<'_>,
    name: &str,
    data: &Value,
    val: &mut [T; N],
) -> DResult {
    let data_kind = get_kind(data);

    // The elements to decode: dataVal.Len() / dataVal.Index(i).
    let items: Vec<Value>;
    // Go indexes the array past its end (a reflect panic) after decoding the first N.
    let mut array_overflow = false;
    let zero = val.iter().all(|e| e.is_zero_value());
    if zero || d.config.zero_fields {
        // Check input type
        if data_kind != MsKind::Slice {
            if d.weak() {
                if data_kind == MsKind::Map {
                    // Empty maps turn into empty arrays
                    if matches!(data, Value::Map(m) if m.entries.is_empty())
                        || is_nil_container(data)
                    {
                        for e in val.iter_mut() {
                            e.set_zero();
                        }
                        return Ok(());
                    }
                    // Go falls through to the error below.
                } else {
                    // All other types we try to convert to the array type and "lift" it into
                    // it. i.e. a string becomes a string array.
                    return decode_array(d, name, &lift(data), val);
                }
            }

            return Err(DecodeError::Single(format!(
                "'{}': source data must be an array or slice, got {}",
                name,
                kind_name(data)
            )));
        }
        items = slice_items(data).unwrap_or_default().into_owned();
        if items.len() > N {
            return Err(DecodeError::Single(format!(
                "'{}': expected source data to have length less or equal to {}, got {}",
                name,
                N,
                items.len()
            )));
        }

        // Make a new array to hold our result, same size as the original data.
        for e in val.iter_mut() {
            e.set_zero();
        }
    } else {
        // Go indexes the data directly: a string by byte, a slice by element; any other kind
        // panics in reflect.
        items = match data {
            Value::List(l) => l.items.clone(),
            Value::TypedNil(_) => Vec::new(),
            _ if data_kind == MsKind::String => data_str(data)
                .as_bytes()
                .iter()
                .map(|b| Value::Uint(u64::from(*b), UintKind::Uint8))
                .collect(),
            Value::Map(m) if m.entries.is_empty() => Vec::new(),
            _ if data_kind == MsKind::Map => {
                return Err(DecodeError::Panic(
                    "reflect: call of reflect.Value.Index on map Value".to_string(),
                ));
            }
            _ => {
                return Err(DecodeError::Panic(format!(
                    "reflect: call of reflect.Value.Len on {} Value",
                    kind_name(data)
                )));
            }
        };
        array_overflow = items.len() > N;
    }

    // Accumulate any errors
    let mut errors: Vec<String> = Vec::new();
    for (i, current_data) in items.iter().enumerate() {
        if i >= N && array_overflow {
            return Err(DecodeError::Panic(
                "reflect: array index out of range".to_string(),
            ));
        }
        let field_name = format!("{name}[{i}]");
        if let Err(e) = d.decode(&field_name, current_data, &mut val[i]) {
            append_errors(&mut errors, e)?;
        }
    }

    if !errors.is_empty() {
        return Err(DecodeError::Multi(errors));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Structs

/// Go: `decodeStruct` (for [`Decode::decode_kind`] of struct types).
// Go: mapstructure.go:(*Decoder).decodeStruct
pub fn decode_struct(d: &Decoder<'_>, name: &str, data: &Value, val: &mut dyn Decode) -> DResult {
    // If the type of the value to write to and the data match directly, then we just set it
    // directly instead of recursing into the structure.
    if val.assign_same(data) {
        return Ok(());
    }

    match get_kind(data) {
        MsKind::Map => {
            let entries = match data {
                Value::Map(m) => Cow::Borrowed(&m.entries),
                _ => Cow::Owned(BTreeMap::new()),
            };
            decode_struct_from_map(d, name, &entries, val)
        }
        MsKind::Struct => {
            // Not the most efficient way to do this but we can optimize later if we want to.
            // To convert from struct to struct we go to map first as an intermediary.
            let mut m = Map::new(MapType::StringAny);
            decode_map_from_struct(d, name, data, &mut m)?;
            decode_struct_from_map(d, name, &m.entries, val)
        }
        _ => Err(DecodeError::Single(format!(
            "'{}' expected a map, got '{}'",
            name,
            kind_name(data)
        ))),
    }
}

/// A struct kind name for the squash error (`reflect.Kind.String()` of an output kind).
fn out_kind_name(k: OutKind) -> &'static str {
    match k {
        OutKind::Bool => "bool",
        OutKind::Interface => "interface",
        OutKind::String => "string",
        OutKind::Int => "int",
        OutKind::Uint => "uint",
        OutKind::Float32 => "float64",
        OutKind::Struct => "struct",
        OutKind::Map => "map",
        OutKind::Ptr => "ptr",
        OutKind::Slice => "slice",
        OutKind::Array => "array",
    }
}

// Go: mapstructure.go:(*Decoder).decodeStructFromMap
fn decode_struct_from_map(
    d: &Decoder<'_>,
    name: &str,
    entries: &BTreeMap<GoString, Value>,
    val: &mut dyn Decode,
) -> DResult {
    let mut data_val_keys_unused: BTreeSet<GoString> = entries.keys().cloned().collect();
    let mut target_val_keys_unused: BTreeSet<String> = BTreeSet::new();
    let mut errors: Vec<String> = Vec::new();

    // This slice will keep track of all the structs we'll be decoding. There can be more than
    // one struct if there are embedded structs that are squashed.
    let mut structs: VecDeque<&mut dyn Decode> = VecDeque::new();
    structs.push_back(val);

    // remainField is set to a valid field set with the "remain" tag if we are keeping track of
    // remaining values.
    let mut remain_field: Option<FieldRef<'_>> = None;

    let mut fields: Vec<FieldRef<'_>> = Vec::new();
    while let Some(struct_val) = structs.pop_front() {
        let Some(struct_fields) = struct_val.struct_fields() else {
            continue;
        };
        for mut f in struct_fields {
            // Handle embedded struct pointers as embedded structs (any non-nil pointer to a
            // struct field).
            if f.target.is_nonnil_ptr_struct() {
                let t = f.target;
                f.target = t.ptr_struct_elem().expect("a non-nil pointer to a struct");
            }

            // If "squash" is specified in the tag, we squash the field down.
            let mut squash =
                d.config.squash && f.target.out_kind() == OutKind::Struct && f.anonymous;
            let mut remain = false;

            // We always parse the tags cause we're looking for other tags too
            for tag in f.tag.split(',').skip(1) {
                if tag == "squash" {
                    squash = true;
                    break;
                }
                if tag == "remain" {
                    remain = true;
                    break;
                }
            }

            if squash {
                if f.target.out_kind() != OutKind::Struct {
                    errors.push(format!(
                        "{}: unsupported type for squash: {}",
                        f.name,
                        out_kind_name(f.target.out_kind())
                    ));
                } else {
                    structs.push_back(f.target);
                }
                continue;
            }

            // Build our field
            if remain {
                remain_field = Some(f);
            } else {
                // Normal struct field, store it away
                fields.push(f);
            }
        }
    }

    for f in fields {
        let tag_value = f.tag.split(',').next().unwrap_or("");
        let mut field_name: String = if tag_value.is_empty() {
            f.name.to_string()
        } else {
            tag_value.to_string()
        };

        let mut raw: Option<(&GoString, &Value)> = entries.get_key_value(field_name.as_bytes());
        if raw.is_none() {
            // Do a slower search by iterating over each key and doing case-insensitive search.
            // (Go iterates the map in random order; the first match in byte order is used.)
            raw = entries.iter().find(|(k, _)| {
                go_unicode::strings::equal_fold(k.as_bytes(), field_name.as_bytes())
            });

            if raw.is_none() {
                // There was no matching key in the map for the value in the struct. Remember it
                // for potential errors and metadata.
                target_val_keys_unused.insert(field_name);
                continue;
            }
        }
        let (raw_key, raw_val) = raw.expect("found");

        // If we can't set the field, then it is unexported or something, and we just continue
        // onwards.
        if !f.exported {
            continue;
        }

        // Delete the key we're using from the unused map so we stop tracking
        data_val_keys_unused.remove(raw_key);

        // If the name is empty string, then we're at the root, and we don't dot-join the
        // fields.
        if !name.is_empty() {
            field_name = format!("{name}.{field_name}");
        }

        if let Err(e) = d.decode(&field_name, raw_val, f.target) {
            append_errors(&mut errors, e)?;
        }
    }

    // If we have a "remain"-tagged field and we have unused keys then we put the unused keys
    // directly into the remain field.
    if let Some(rf) = remain_field
        && !data_val_keys_unused.is_empty()
    {
        // Build a map of only the unused values
        let mut remain = Map::new(MapType::Named(Arc::from("map[interface {}]interface {}")));
        for key in &data_val_keys_unused {
            remain.insert(key.clone(), entries[key].clone());
        }

        // Decode it as-if we were just decoding this map onto our map.
        if let Err(e) = rf.target.decode_kind(d, name, &Value::map(remain)) {
            append_errors(&mut errors, e)?;
        }

        // Set the map to nil so we have none so that the next check will not error
        // (ErrorUnused)
        data_val_keys_unused.clear();
    }

    if d.config.error_unused && !data_val_keys_unused.is_empty() {
        let keys: Vec<String> = data_val_keys_unused
            .iter()
            .map(|k| lossy(k.as_bytes()))
            .collect();
        errors.push(format!("'{}' has invalid keys: {}", name, keys.join(", ")));
    }

    if d.config.error_unset && !target_val_keys_unused.is_empty() {
        let keys: Vec<String> = target_val_keys_unused.into_iter().collect();
        errors.push(format!("'{}' has unset fields: {}", name, keys.join(", ")));
    }

    if !errors.is_empty() {
        return Err(DecodeError::Multi(errors));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Decode hooks

/// A `time.Duration` value (the result of [`string_to_time_duration_hook`]).
#[derive(Clone, Copy, Debug)]
struct DurationValue(i64);

impl go_value::Object for DurationValue {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("time.Duration")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::Int(self.0, IntKind::Int64))
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(go_time::Duration(self.0).string()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `mapstructure.StringToTimeDurationHookFunc()`: a string decoded into a `time.Duration`
/// is parsed with `time.ParseDuration`.
// Go: decode_hooks.go:StringToTimeDurationHookFunc
pub fn string_to_time_duration_hook(
    data: &Value,
    target: &dyn Decode,
) -> std::result::Result<Value, String> {
    if kind_of(data) != ReflectKind::String {
        return Ok(data.clone());
    }
    if !target.as_any().is::<go_time::Duration>() {
        return Ok(data.clone());
    }
    let s = match data {
        Value::String(s) => s.clone(),
        // Go: `data.(string)` panics for a named string type.
        _ => {
            return Err(format!(
                "interface conversion: interface {{}} is {}, not string",
                data.go_type_name()
            ));
        }
    };
    // Convert it by parsing
    match go_time::parse_duration(s.as_bytes()) {
        Ok(d) => Ok(Value::object(DurationValue(d.0))),
        Err(e) => Err(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// The skeleton's helpers

/// Implemented by config structs decoded from a `map[string]any`.
pub trait WeakDecode: Sized {
    /// Decode `m` on top of `self` (defaults), like `mapstructure.WeakDecode(m, &self)`.
    fn weak_decode(self, m: &Map) -> Result<Self>;
}

impl<T: Decode + Sized> WeakDecode for T {
    fn weak_decode(mut self, m: &Map) -> Result<Self> {
        weak_decode_into(&Value::map(m.clone()), &mut self)?;
        Ok(self)
    }
}

/// Case-insensitive field lookup (mapstructure `MatchName` default: `strings.EqualFold`): the
/// exact key first, then the first fold-equal key in byte order.
pub fn field<'a>(m: &'a Map, name: &str) -> Option<&'a Value> {
    if let Some(v) = m.get(name.as_bytes()) {
        return Some(v);
    }
    m.entries
        .iter()
        .find(|(k, _)| go_unicode::strings::equal_fold(k.as_bytes(), name.as_bytes()))
        .map(|(_, v)| v)
}

/// Weak conversions (`mapstructure.WeakDecode(v, &x)` for one value).
pub fn weak_string(v: &Value) -> Result<String> {
    let mut s = String::new();
    weak_decode_into(v, &mut s)?;
    Ok(s)
}

pub fn weak_int(v: &Value) -> Result<i64> {
    let mut i = 0i64;
    weak_decode_into(v, &mut i)?;
    Ok(i)
}

pub fn weak_float(v: &Value) -> Result<f64> {
    let mut f = 0f64;
    weak_decode_into(v, &mut f)?;
    Ok(f)
}

pub fn weak_bool(v: &Value) -> Result<bool> {
    let mut b = false;
    weak_decode_into(v, &mut b)?;
    Ok(b)
}

pub fn weak_string_slice(v: &Value) -> Result<Vec<String>> {
    let mut s: Vec<String> = Vec::new();
    weak_decode_into(v, &mut s)?;
    Ok(s)
}

/// A `time.Duration` decoded weakly (an int of nanoseconds; strings are parsed as ints, as
/// mapstructure does without the duration hook). Negative durations become zero.
pub fn weak_duration(v: &Value) -> Result<std::time::Duration> {
    let mut d = go_time::Duration(0);
    weak_decode_into(v, &mut d)?;
    Ok(std::time::Duration::from_nanos(d.0.max(0) as u64))
}
