//! Module `cast::caste`.
//!
//! PORT spf13/cast@v1.9.2 caste.go/basic.go/number.go/slice.go/map.go (ToStringE, ToIntE, ToInt64E, ToFloat64E, ToBoolE, ToStringSliceE, ToIntSliceE, ToStringMapE ...)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).

//! Port of `github.com/spf13/cast@v1.9.2` conversions over [`go_value::Value`] (caste.go, basic.go,
//! number.go, slice.go, map.go, indirect.go). Semantics that reach output: `ToStringE` formatting of
//! numbers (`strconv.FormatFloat(f, 'f', -1, 64)` for floats, not `%v`!), `nil -> ""`,
//! `ToStringSliceE` failing on nested slices/maps, `ToIntE` of strings (`strconv.ParseInt` with
//! base 0, so `0x`/`0o`/`0b` prefixes, a leading `0` means octal and `_` separators are allowed;
//! a decimal fraction is cut first: `"8.3"` -> 8), `fmt.Stringer`/`error` support.
//!
//! Upstream: `github.com/spf13/cast v1.9.2`: cast.go (`ToE`), basic.go, number.go, slice.go,
//! map.go, indirect.go, alias.go, time.go (`ToDurationE`; the time functions are in
//! [`super::time`]) and zz_generated.go (the non-`E` wrappers).
//!
//! How Go's dynamic types map onto [`Value`]:
//! - `bool`, `int`..`int64`, `uint`..`uint64`/`uintptr`, `float32`/`float64`, `string`,
//!   `time.Time`, `nil` are the corresponding variants; a typed nil pointer or interface
//!   (`TypedNil` of pointer/interface kind) is what `indirect` turns into `nil`.
//! - `[]byte` is a `List` of `SliceType::Uint8`; the `html/template` string types are `Safe`.
//! - `json.Number` is [`go_json::Number`] (or any object whose type name is `json.Number`).
//! - `time.Duration`, `time.Month`, `time.Weekday` and other named basic types are objects that
//!   report their type name and implement [`go_value::Object::underlying`] (`resolveAlias`).
//! - `fmt.Stringer`/`error` are objects with `go_string`/`go_error`; `float64Provider` /
//!   `float64EProvider` are objects with a `Float64` method.
//! - `map[K]V` types: `MapType::StringAny` is `map[string]interface {}`,
//!   `MapType::StringString` is `map[string]string`, and `map[string]bool` is
//!   `MapType::Named("map[string]bool")`. `map[interface {}]...` types do not exist in the value
//!   model (Hugo normalises YAML maps to string keys), so those `case`s are unreachable here.
//!
//! Float to integer conversions follow the arm64 Go compiler (the golden platform): `int8/16/32`
//! and `uint8/16` saturate to int32 then truncate, `int`/`int64` saturate to int64, `uint32`
//! saturates to int64 then truncates, `uint`/`uint64` saturate to [0, 2^64-1], NaN is 0 (verified
//! against arm64 Go under qemu; amd64 Go differs for out-of-range values).

use std::borrow::Cow;
use std::fmt::Display;
use std::sync::Arc;

use go_value::{
    FloatKind, GoString, IntKind, Kind, List, Map, MapType, NilKind, Object, SliceType, UintKind,
    Value,
};

use crate::herrors::{Error, Result};

// ---------------------------------------------------------------------------
// Error messages (cast.go)

/// `fmt.Sprintf("%#v", v)`, with `json.Number` printed as Go prints a named string type.
fn sharp_v(v: &Value) -> String {
    if let Some(s) = json_number(v) {
        return go_strconv::quote(s.as_bytes());
    }
    // `*time.Time` has a GoString method (value receiver); fmt catches the nil-pointer panic
    // and prints "<nil>".
    if matches!(v, Value::TypedNil(t) if &**t == "*time.Time") {
        return "<nil>".to_string();
    }
    String::from_utf8_lossy(&go_fmt::sprintf("%#v", std::slice::from_ref(v))).into_owned()
}

/// `fmt.Sprintf("%T", v)`.
fn type_of(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%T", std::slice::from_ref(v))).into_owned()
}

/// Go: `fmt.Errorf(errorMsg, i, i, <zero of the target type>)`, where `target` is the target's
/// `%T` (e.g. `"int"`, `"[]string"`, `"map[string]interface {}"`).
pub(super) fn cast_error(i: &Value, target: &str) -> Error {
    Error::new(format!(
        "unable to cast {} of type {} to {}",
        sharp_v(i),
        type_of(i),
        target
    ))
}

/// Go: `fmt.Errorf(errorMsgWith, i, i, <zero of the target type>, err)`.
fn cast_error_with(i: &Value, target: &str, err: impl Display) -> Error {
    Error::new(format!(
        "unable to cast {} of type {} to {}: {}",
        sharp_v(i),
        type_of(i),
        target,
        err
    ))
}

// ---------------------------------------------------------------------------
// Dynamic type helpers (indirect.go, alias.go)

// Go: spf13/cast indirect.go:indirect
/// Returns the value after dereferencing pointers (or nil). In the value model the pointers that
/// can be dereferenced are typed nils of pointer (or interface) kind, which become `nil`, and
/// pointer objects to structs (a `*T` type name with struct fields), which become the struct
/// value ([`Pointee`]); Go's error messages print that value (`main.pg{...} of type main.pg`).
pub(super) fn indirect(i: &Value) -> (Value, bool) {
    match i {
        Value::TypedNil(t) => match go_value::typed_nil_kind(t) {
            NilKind::Ptr | NilKind::Interface => return (Value::Invalid, true),
            _ => {}
        },
        Value::Object(o)
            if o.kind() == Kind::Ptr
                && o.type_name().starts_with('*')
                && o.struct_fields().is_some() =>
        {
            return (Value::Object(Arc::new(Pointee(o.clone()))), true);
        }
        _ => {}
    }
    (i.clone(), false)
}

/// Go's `reflect.Value.Elem()` of a non-nil pointer to a struct, as `indirect` returns it: the
/// struct value, with the pointee's type name and fields. It has no methods: the value's method
/// set lacks the pointer-receiver methods, and the host objects do not say which methods have
/// value receivers.
struct Pointee(Arc<dyn Object>);

impl Object for Pointee {
    fn type_name(&self) -> Cow<'_, str> {
        let t = self.0.type_name();
        Cow::Owned(t.strip_prefix('*').unwrap_or(&t).to_string())
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.0.field(name)
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        self.0.struct_fields()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: spf13/cast alias.go:resolveAlias
/// Resolves a named type to its underlying basic type (if possible).
fn resolve_alias(i: &Value) -> Option<Value> {
    match i {
        // template.HTML & co. are named string types.
        Value::Safe(_, s) => Some(Value::String(s.clone())),
        Value::Object(o) => match o.underlying()? {
            v @ (Value::Bool(_)
            | Value::Int(..)
            | Value::Uint(..)
            | Value::Float(..)
            | Value::String(_)) => {
                // `kinds` has no entry for uintptr.
                if matches!(v, Value::Uint(_, UintKind::Uintptr)) {
                    return None;
                }
                Some(v)
            }
            _ => None,
        },
        _ => None,
    }
}

/// The text of a `json.Number` value.
pub(super) fn json_number(v: &Value) -> Option<GoString> {
    let Value::Object(o) = v else {
        return None;
    };
    if let Some(n) = o.as_any().downcast_ref::<go_json::Number>() {
        return Some(n.0.clone());
    }
    if o.type_name() == "json.Number" {
        return o.go_string();
    }
    None
}

/// The value of a named type `name` (e.g. `time.Duration`) with an integer underlying value.
fn named_int(v: &Value, name: &str) -> Option<i64> {
    let Value::Object(o) = v else {
        return None;
    };
    if o.type_name() != name {
        return None;
    }
    match o.underlying()? {
        Value::Int(n, _) => Some(n),
        _ => None,
    }
}

/// `float64Provider` (`Float64() float64`) / `float64EProvider` (`Float64() (float64, error)`):
/// `Some(Ok(f))`, or `Some(Err(()))` when the method returns an error.
fn float64_provider(v: &Value) -> Option<std::result::Result<f64, ()>> {
    let Value::Object(o) = v else {
        return None;
    };
    if json_number(v).is_some() || !o.has_method("Float64") {
        return None;
    }
    match o.call_method(&(), "Float64", &[])? {
        Ok(Value::Float(f, _)) => Some(Ok(f)),
        _ => Some(Err(())),
    }
}

/// `%T` of the zero value of a Go slice type `[]T`.
fn slice_type_name(elem: &str) -> String {
    format!("[]{elem}")
}

// ---------------------------------------------------------------------------
// basic.go

// Go: spf13/cast basic.go:ToBoolE
/// ToBoolE casts any value to a bool type.
pub fn to_bool_e(v: &Value) -> Result<bool> {
    let (i, _) = indirect(v);

    match &i {
        Value::Bool(b) => Ok(*b),
        Value::Invalid => Ok(false),
        Value::Int(b, _) => Ok(*b != 0),
        Value::Uint(b, k) if *k != UintKind::Uintptr => Ok(*b != 0),
        Value::Float(b, _) => Ok(*b != 0.0),
        Value::String(b) => {
            go_strconv::parse_bool(b.as_bytes()).map_err(|e| Error::new(e.to_string()))
        }
        _ => {
            if let Some(d) = named_int(&i, "time.Duration") {
                return Ok(d != 0);
            }
            if json_number(&i).is_some() {
                return match to_int64_e(&i) {
                    Ok(v) => Ok(v != 0),
                    Err(_) => Err(cast_error(&i, "bool")),
                };
            }
            if let Some(r) = resolve_alias(&i) {
                return to_bool_e(&r);
            }

            Err(cast_error(&i, "bool"))
        }
    }
}

/// `cast.ToBool` (errors -> false).
pub fn to_bool(v: &Value) -> bool {
    to_bool_e(v).unwrap_or(false)
}

// Go: spf13/cast basic.go:ToStringE
/// ToStringE casts any value to a string type.
pub fn to_string_e(v: &Value) -> Result<GoString> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(GoString::from(go_strconv::format_bool(*b))),
        Value::Float(f, FloatKind::F64) => Ok(go_strconv::format_float(*f, b'f', -1, 64).into()),
        Value::Float(f, FloatKind::F32) => Ok(go_strconv::format_float(*f, b'f', -1, 32).into()),
        Value::Int(n, _) => Ok(go_strconv::format_int(*n, 10).into()),
        Value::Uint(n, k) if *k != UintKind::Uintptr => Ok(go_strconv::format_uint(*n, 10).into()),
        // []byte
        Value::List(l) if l.ty == SliceType::Uint8 => Ok(GoString::from(bytes_of(l))),
        Value::TypedNil(t) if &**t == "[]uint8" => Ok(GoString::empty()),
        // fmt.Stringer: `*time.Time` has `time.Time`'s value method String, which Go calls on the
        // nil pointer and panics (text/template's safeCall reports the panic as an error).
        Value::TypedNil(t) if &**t == "*time.Time" => Err(Error::new(
            "value method time.Time.String called using nil *Time pointer",
        )),
        // template.HTML, template.URL, template.JS, template.CSS, template.HTMLAttr
        Value::Safe(
            go_value::SafeKind::Html
            | go_value::SafeKind::Url
            | go_value::SafeKind::Js
            | go_value::SafeKind::Css
            | go_value::SafeKind::HtmlAttr,
            s,
        ) => Ok(s.clone()),
        Value::Invalid => Ok(GoString::empty()),
        _ => {
            if let Some(s) = json_number(v) {
                return Ok(s);
            }
            if let Value::Time(t) = v {
                // fmt.Stringer
                return Ok(go_time::GoTimeExt::string(t).into());
            }
            if let Value::Object(o) = v {
                if let Some(s) = o.go_string() {
                    return Ok(s);
                }
                if let Some(e) = o.go_error() {
                    return Ok(e.into());
                }
            }

            let (i, ok) = indirect(v);
            if ok {
                return to_string_e(&i);
            }

            if let Some(i) = resolve_alias(v) {
                return to_string_e(&i);
            }

            Err(cast_error(v, "string"))
        }
    }
}

/// `cast.ToString` (errors -> "").
pub fn to_string(v: &Value) -> GoString {
    to_string_e(v).unwrap_or_default()
}

/// The bytes of a `[]byte` list.
fn bytes_of(l: &List) -> Vec<u8> {
    l.items
        .iter()
        .map(|b| match b {
            Value::Uint(b, _) => *b as u8,
            Value::Int(b, _) => *b as u8,
            _ => 0,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// number.go

/// A Go number type (`cast.Number`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumType {
    Int(IntKind),
    Uint(UintKind),
    Float(FloatKind),
}

impl NumType {
    pub const INT: NumType = NumType::Int(IntKind::Int);
    pub const INT8: NumType = NumType::Int(IntKind::Int8);
    pub const INT16: NumType = NumType::Int(IntKind::Int16);
    pub const INT32: NumType = NumType::Int(IntKind::Int32);
    pub const INT64: NumType = NumType::Int(IntKind::Int64);
    pub const UINT: NumType = NumType::Uint(UintKind::Uint);
    pub const UINT8: NumType = NumType::Uint(UintKind::Uint8);
    pub const UINT16: NumType = NumType::Uint(UintKind::Uint16);
    pub const UINT32: NumType = NumType::Uint(UintKind::Uint32);
    pub const UINT64: NumType = NumType::Uint(UintKind::Uint64);
    pub const FLOAT32: NumType = NumType::Float(FloatKind::F32);
    pub const FLOAT64: NumType = NumType::Float(FloatKind::F64);

    /// The Go type name (`%T` of the zero value).
    pub fn go_name(self) -> &'static str {
        match self {
            NumType::Int(k) => k.go_name(),
            NumType::Uint(k) => k.go_name(),
            NumType::Float(k) => k.go_name(),
        }
    }

    fn is_unsigned(self) -> bool {
        matches!(self, NumType::Uint(_))
    }

    fn is_float64(self) -> bool {
        self == NumType::FLOAT64
    }
}

/// A value of a [`NumType`]: `I` for signed, `U` for unsigned, `F` for float types (a float32 is
/// stored widened, like [`Value::Float`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Num {
    I(i64),
    U(u64),
    F(f64),
}

impl Num {
    /// The number as a [`Value`] of type `t`.
    pub fn to_value(self, t: NumType) -> Value {
        match (self, t) {
            (Num::I(n), NumType::Int(k)) => Value::Int(n, k),
            (Num::U(n), NumType::Uint(k)) => Value::Uint(n, k),
            (Num::F(f), NumType::Float(k)) => Value::Float(f, k),
            // Unreachable: a Num always matches its NumType.
            (_, t) => zero(t).to_value(t),
        }
    }

    fn as_i64(self) -> i64 {
        match self {
            Num::I(n) => n,
            Num::U(n) => n as i64,
            Num::F(f) => f as i64,
        }
    }

    fn as_u64(self) -> u64 {
        match self {
            Num::I(n) => n as u64,
            Num::U(n) => n,
            Num::F(f) => f as u64,
        }
    }

    fn as_f64(self) -> f64 {
        match self {
            Num::I(n) => n as f64,
            Num::U(n) => n as f64,
            Num::F(f) => f,
        }
    }
}

fn zero(t: NumType) -> Num {
    match t {
        NumType::Int(_) => Num::I(0),
        NumType::Uint(_) => Num::U(0),
        NumType::Float(_) => Num::F(0.0),
    }
}

/// Go `intN(x)` / `uintN(x)` of an integer: two's complement truncation.
fn wrap_int(x: i64, k: IntKind) -> i64 {
    match k {
        IntKind::Int8 => x as i8 as i64,
        IntKind::Int16 => x as i16 as i64,
        IntKind::Int32 => x as i32 as i64,
        IntKind::Int | IntKind::Int64 => x,
    }
}

fn wrap_uint(x: u64, k: UintKind) -> u64 {
    match k {
        UintKind::Uint8 => x as u8 as u64,
        UintKind::Uint16 => x as u16 as u64,
        UintKind::Uint32 => x as u32 as u64,
        UintKind::Uint | UintKind::Uint64 | UintKind::Uintptr => x,
    }
}

/// Go `T(x)` for a signed integer `x`.
fn conv_i64(x: i64, t: NumType) -> Num {
    match t {
        NumType::Int(k) => Num::I(wrap_int(x, k)),
        NumType::Uint(k) => Num::U(wrap_uint(x as u64, k)),
        NumType::Float(FloatKind::F64) => Num::F(x as f64),
        NumType::Float(FloatKind::F32) => Num::F(x as f32 as f64),
    }
}

/// Go `T(x)` for an unsigned integer `x`.
fn conv_u64(x: u64, t: NumType) -> Num {
    match t {
        NumType::Int(k) => Num::I(wrap_int(x as i64, k)),
        NumType::Uint(k) => Num::U(wrap_uint(x, k)),
        NumType::Float(FloatKind::F64) => Num::F(x as f64),
        NumType::Float(FloatKind::F32) => Num::F(x as f32 as f64),
    }
}

/// Go `T(x)` for a float `x` (a float32 is passed widened; its conversions give the same
/// results), as the arm64 Go compiler lowers them: `int8/16/32` and `uint8/16` go through
/// `FCVTZS` to int32 (saturating, NaN -> 0) and truncate; `int`/`int64` use `FCVTZS` to int64;
/// `uint32` uses `FCVTZS` to int64 and truncates (go1.27.1's default `converthash`); `uint`/
/// `uint64` use `FCVTZU` (saturating to [0, 2^64-1]). Rust `as` saturates the same way.
fn conv_f64(x: f64, t: NumType) -> Num {
    match t {
        NumType::Int(k @ (IntKind::Int8 | IntKind::Int16 | IntKind::Int32)) => {
            Num::I(wrap_int(x as i32 as i64, k))
        }
        NumType::Int(IntKind::Int | IntKind::Int64) => Num::I(x as i64),
        NumType::Uint(k @ (UintKind::Uint8 | UintKind::Uint16)) => {
            Num::U(wrap_uint(x as i32 as u32 as u64, k))
        }
        NumType::Uint(UintKind::Uint32) => Num::U(x as i64 as u32 as u64),
        NumType::Uint(UintKind::Uint | UintKind::Uint64 | UintKind::Uintptr) => Num::U(x as u64),
        NumType::Float(FloatKind::F64) => Num::F(x),
        NumType::Float(FloatKind::F32) => Num::F(x as f32 as f64),
    }
}

// Go: spf13/cast number.go:toNumber
/// Returns `None` if the conversion fails, to signal callers that they should proceed with their
/// own conversions.
fn to_number(v: &Value, t: NumType) -> Option<Num> {
    let (i, _) = indirect(v);

    match &i {
        Value::Int(s, _) => Some(conv_i64(*s, t)),
        Value::Uint(s, k) if *k != UintKind::Uintptr => Some(conv_u64(*s, t)),
        Value::Float(s, _) => Some(conv_f64(*s, t)),
        Value::Bool(s) => Some(if *s { conv_i64(1, t) } else { zero(t) }),
        Value::Invalid => Some(zero(t)),
        _ => {
            if let Some(s) = named_int(&i, "time.Weekday").or_else(|| named_int(&i, "time.Month")) {
                return Some(conv_i64(s, t));
            }
            None
        }
    }
}

// Go: spf13/cast number.go:parseInt, parseUint, parseFloat, parseNumber
fn parse_number(s: &[u8], t: NumType) -> std::result::Result<Num, go_strconv::NumError> {
    match t {
        NumType::Int(_) => {
            let v = go_strconv::parse_int(trim_decimal(s), 0, 0)?;
            Ok(conv_i64(v, t))
        }
        NumType::Uint(_) => {
            let v = go_strconv::parse_uint(
                go_unicode::strings::trim_left(&trim_decimal(s), b"+"),
                0,
                0,
            )?;
            Ok(conv_u64(v, t))
        }
        NumType::Float(FloatKind::F32) => {
            let n = go_strconv::parse_float(s, 32)?;
            Ok(Num::F(n as f32 as f64))
        }
        NumType::Float(FloatKind::F64) => Ok(Num::F(go_strconv::parse_float(s, 64)?)),
    }
}

// Go: spf13/cast number.go:toNumberE
fn to_number_e_impl(v: &Value, t: NumType) -> Result<Num> {
    if let Some(n) = to_number(v, t) {
        return Ok(n);
    }

    let (i, _) = indirect(v);

    let text = match &i {
        Value::String(s) => Some(s.clone()),
        _ => json_number(&i),
    };
    if let Some(s) = text {
        if s.is_empty() {
            return Ok(zero(t));
        }

        return parse_number(s.as_bytes(), t).map_err(|err| cast_error_with(&i, t.go_name(), err));
    }

    if let Some(p) = float64_provider(&i) {
        if !t.is_float64() {
            return Err(cast_error(&i, t.go_name()));
        }

        return match p {
            Ok(f) => Ok(conv_f64(f, t)),
            Err(()) => Err(cast_error(&i, t.go_name())),
        };
    }

    if let Some(r) = resolve_alias(&i) {
        return to_number_e_impl(&r, t);
    }

    Err(cast_error(&i, t.go_name()))
}

/// Go: `errNegativeNotAllowed`.
fn err_negative_not_allowed() -> Error {
    Error::new("unable to cast negative value")
}

// Go: spf13/cast number.go:toUnsignedNumber
/// `(n, valid, ok)`.
fn to_unsigned_number(v: &Value, t: NumType) -> (Num, bool, bool) {
    let (i, _) = indirect(v);

    match &i {
        Value::Int(s, _) => {
            if *s < 0 {
                return (zero(t), false, false);
            }

            (conv_i64(*s, t), true, true)
        }
        Value::Uint(s, k) if *k != UintKind::Uintptr => (conv_u64(*s, t), true, true),
        Value::Float(s, _) => {
            if *s < 0.0 {
                return (zero(t), false, false);
            }

            (conv_f64(*s, t), true, true)
        }
        Value::Bool(s) => {
            if *s {
                return (conv_i64(1, t), true, true);
            }

            (zero(t), true, true)
        }
        Value::Invalid => (zero(t), true, true),
        _ => {
            if let Some(s) = named_int(&i, "time.Weekday").or_else(|| named_int(&i, "time.Month")) {
                if s < 0 {
                    return (zero(t), false, false);
                }

                return (conv_i64(s, t), true, true);
            }
            (zero(t), true, false)
        }
    }
}

// Go: spf13/cast number.go:toUnsignedNumberE
fn to_unsigned_number_e_impl(v: &Value, t: NumType) -> Result<Num> {
    let (n, valid, ok) = to_unsigned_number(v, t);
    if ok {
        return Ok(n);
    }

    let (i, _) = indirect(v);

    if !valid {
        return Err(err_negative_not_allowed());
    }

    let text = match &i {
        Value::String(s) => Some(s.clone()),
        _ => json_number(&i),
    };
    if let Some(s) = text {
        if s.is_empty() {
            return Ok(zero(t));
        }

        return parse_number(s.as_bytes(), t).map_err(|err| cast_error_with(&i, t.go_name(), err));
    }

    if let Some(p) = float64_provider(&i) {
        if !t.is_float64() {
            return Err(cast_error(&i, t.go_name()));
        }

        return match p {
            Ok(f) => {
                if f < 0.0 {
                    return Err(err_negative_not_allowed());
                }
                Ok(conv_f64(f, t))
            }
            Err(()) => Err(cast_error(&i, t.go_name())),
        };
    }

    if let Some(r) = resolve_alias(&i) {
        return to_unsigned_number_e_impl(&r, t);
    }

    Err(cast_error(&i, t.go_name()))
}

// Go: spf13/cast number.go:ToNumberE (and cast.go:ToE for number types)
/// ToNumberE casts any value to a [`NumType`], returned as a [`Value`] of that type.
pub fn to_number_e(v: &Value, t: NumType) -> Result<Value> {
    to_num_e(v, t).map(|n| n.to_value(t))
}

fn to_num_e(v: &Value, t: NumType) -> Result<Num> {
    if t.is_unsigned() {
        to_unsigned_number_e_impl(v, t)
    } else {
        to_number_e_impl(v, t)
    }
}

// Go: spf13/cast number.go:ToFloat64E
/// ToFloat64E casts an interface to a float64 type.
pub fn to_float64_e(v: &Value) -> Result<f64> {
    to_number_e_impl(v, NumType::FLOAT64).map(Num::as_f64)
}

// Go: spf13/cast number.go:ToFloat32E
/// ToFloat32E casts an interface to a float32 type.
pub fn to_float32_e(v: &Value) -> Result<f32> {
    to_number_e_impl(v, NumType::FLOAT32).map(|n| n.as_f64() as f32)
}

// Go: spf13/cast number.go:ToInt64E
/// ToInt64E casts an interface to an int64 type.
pub fn to_int64_e(v: &Value) -> Result<i64> {
    to_number_e_impl(v, NumType::INT64).map(Num::as_i64)
}

// Go: spf13/cast number.go:ToInt32E
/// ToInt32E casts an interface to an int32 type.
pub fn to_int32_e(v: &Value) -> Result<i32> {
    to_number_e_impl(v, NumType::INT32).map(|n| n.as_i64() as i32)
}

// Go: spf13/cast number.go:ToInt16E
/// ToInt16E casts an interface to an int16 type.
pub fn to_int16_e(v: &Value) -> Result<i16> {
    to_number_e_impl(v, NumType::INT16).map(|n| n.as_i64() as i16)
}

// Go: spf13/cast number.go:ToInt8E
/// ToInt8E casts an interface to an int8 type.
pub fn to_int8_e(v: &Value) -> Result<i8> {
    to_number_e_impl(v, NumType::INT8).map(|n| n.as_i64() as i8)
}

// Go: spf13/cast number.go:ToIntE (Go int = i64)
/// ToIntE casts an interface to an int type.
pub fn to_int_e(v: &Value) -> Result<i64> {
    to_number_e_impl(v, NumType::INT).map(Num::as_i64)
}

// Go: spf13/cast number.go:ToUintE (Go uint = u64)
/// ToUintE casts an interface to a uint type.
pub fn to_uint_e(v: &Value) -> Result<u64> {
    to_unsigned_number_e_impl(v, NumType::UINT).map(Num::as_u64)
}

// Go: spf13/cast number.go:ToUint64E
/// ToUint64E casts an interface to a uint64 type.
pub fn to_uint64_e(v: &Value) -> Result<u64> {
    to_unsigned_number_e_impl(v, NumType::UINT64).map(Num::as_u64)
}

// Go: spf13/cast number.go:ToUint32E
/// ToUint32E casts an interface to a uint32 type.
pub fn to_uint32_e(v: &Value) -> Result<u32> {
    to_unsigned_number_e_impl(v, NumType::UINT32).map(|n| n.as_u64() as u32)
}

// Go: spf13/cast number.go:ToUint16E
/// ToUint16E casts an interface to a uint16 type.
pub fn to_uint16_e(v: &Value) -> Result<u16> {
    to_unsigned_number_e_impl(v, NumType::UINT16).map(|n| n.as_u64() as u16)
}

// Go: spf13/cast number.go:ToUint8E
/// ToUint8E casts an interface to a uint8 type.
pub fn to_uint8_e(v: &Value) -> Result<u8> {
    to_unsigned_number_e_impl(v, NumType::UINT8).map(|n| n.as_u64() as u8)
}

/// `cast.ToInt` (errors -> 0).
pub fn to_int(v: &Value) -> i64 {
    to_int_e(v).unwrap_or(0)
}

/// `cast.ToInt64` (errors -> 0).
pub fn to_int64(v: &Value) -> i64 {
    to_int64_e(v).unwrap_or(0)
}

/// `cast.ToUint64` (errors -> 0).
pub fn to_uint64(v: &Value) -> u64 {
    to_uint64_e(v).unwrap_or(0)
}

/// `cast.ToFloat64` (errors -> 0).
pub fn to_float64(v: &Value) -> f64 {
    to_float64_e(v).unwrap_or(0.0)
}

/// `cast.ToFloat32` (errors -> 0).
pub fn to_float32(v: &Value) -> f32 {
    to_float32_e(v).unwrap_or(0.0)
}

// Go: spf13/cast number.go:trimZeroDecimal
pub(super) fn trim_zero_decimal(s: &[u8]) -> &[u8] {
    let mut found_zero = false;
    let mut i = s.len();
    while i > 0 {
        match s[i - 1] {
            b'.' => {
                if found_zero {
                    return &s[..i - 1];
                }
            }
            b'0' => found_zero = true,
            _ => return s,
        }
        i -= 1;
    }
    s
}

// Go: spf13/cast number.go:trimDecimal
/// `stringNumberRe = ^([-+]?\d*)(\.\d*)?$` (`\d` is ASCII in Go RE2): a decimal string with a
/// fraction is cut to its integer part.
fn trim_decimal(s: &[u8]) -> Vec<u8> {
    if !go_unicode::strings::contains(s, b".") {
        return s.to_vec();
    }

    // matches := stringNumberRe.FindStringSubmatch(s)
    let mut j = 0;
    if j < s.len() && (s[j] == b'-' || s[j] == b'+') {
        j += 1;
    }
    while j < s.len() && s[j].is_ascii_digit() {
        j += 1;
    }
    let int_end = j;
    if j < s.len() && s[j] == b'.' {
        j += 1;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j == s.len() {
        // matches[1] is the captured integer part with sign
        let mut s = s[..int_end].to_vec();

        // handle special cases
        match &s[..] {
            b"-" | b"+" => s.push(b'0'),
            b"" => s = b"0".to_vec(),
            _ => {}
        }

        return s;
    }

    s.to_vec()
}

// ---------------------------------------------------------------------------
// cast.go

/// A `cast.Basic` target type of the generic `ToE[T]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasicType {
    String,
    Bool,
    Number(NumType),
    Time,
    Duration,
}

impl BasicType {
    /// The Go type name (`%T` of the zero value).
    pub fn go_name(self) -> &'static str {
        match self {
            BasicType::String => "string",
            BasicType::Bool => "bool",
            BasicType::Number(t) => t.go_name(),
            BasicType::Time => "time.Time",
            BasicType::Duration => "time.Duration",
        }
    }
}

/// The result of [`to_e`]: a Go value of a [`BasicType`].
#[derive(Clone, Debug)]
pub enum Basic {
    String(GoString),
    Bool(bool),
    Number(Num),
    Time(go_value::Time),
    Duration(go_time::Duration),
}

// Go: spf13/cast cast.go:ToE
/// ToE casts any value to a [`BasicType`].
pub fn to_e(v: &Value, t: BasicType) -> Result<Basic> {
    Ok(match t {
        BasicType::String => Basic::String(to_string_e(v)?),
        BasicType::Bool => Basic::Bool(to_bool_e(v)?),
        BasicType::Number(n) => Basic::Number(to_num_e(v, n)?),
        BasicType::Time => Basic::Time(super::time::to_time_e(v)?),
        BasicType::Duration => Basic::Duration(to_duration_e(v)?),
    })
}

// ---------------------------------------------------------------------------
// time.go (ToDurationE)

// Go: spf13/cast time.go:ToDurationE
/// ToDurationE casts any value to a `time.Duration` type.
pub fn to_duration_e(v: &Value) -> Result<go_time::Duration> {
    let (i, _) = indirect(v);

    if let Some(d) = named_int(&i, "time.Duration") {
        return Ok(go_time::Duration(d));
    }
    match &i {
        Value::Int(..) => match to_int64_e(&i) {
            Ok(v) => Ok(go_time::Duration(v)),
            Err(err) => Err(Error::new(err.message().replace(" int64", "time.Duration"))),
        },
        Value::Uint(_, k) if *k != UintKind::Uintptr => match to_int64_e(&i) {
            Ok(v) => Ok(go_time::Duration(v)),
            Err(err) => Err(Error::new(err.message().replace(" int64", "time.Duration"))),
        },
        Value::Float(..) => to_duration_from_float(&i),
        Value::String(s) => {
            let r = if !go_unicode::strings::contains_any(s, "nsuµmh".as_bytes()) {
                let mut s = s.to_vec();
                s.extend_from_slice(b"ns");
                go_time::parse_duration(&s)
            } else {
                go_time::parse_duration(s.as_bytes())
            };
            r.map_err(|e| Error::new(e.error()))
        }
        Value::Invalid => Ok(go_time::Duration(0)),
        _ => {
            // float64EProvider, float64Provider (json.Number is a float64EProvider).
            if json_number(&i).is_some() || float64_provider(&i).is_some() {
                return to_duration_from_float(&i);
            }
            if let Some(r) = resolve_alias(&i) {
                return to_duration_e(&r);
            }

            Err(cast_error(&i, "time.Duration"))
        }
    }
}

/// `case float32, float64, float64EProvider, float64Provider:` of `ToDurationE`.
fn to_duration_from_float(i: &Value) -> Result<go_time::Duration> {
    match to_float64_e(i) {
        // Go `time.Duration(v)`: float64 -> int64 (arm64: saturating, NaN -> 0).
        Ok(v) => Ok(go_time::Duration(v as i64)),
        Err(err) => Err(Error::new(
            err.message().replace(" float64", "time.Duration"),
        )),
    }
}

/// `cast.ToDuration` (errors -> 0).
pub fn to_duration(v: &Value) -> go_time::Duration {
    to_duration_e(v).unwrap_or(go_time::Duration(0))
}

// ---------------------------------------------------------------------------
// slice.go

/// The elements of a value of reflect kind Slice (or Array), if it is one.
fn slice_elems(i: &Value) -> Option<Vec<Value>> {
    match i {
        Value::List(l) => Some(l.items.clone()),
        Value::TypedNil(t) if go_value::typed_nil_kind(t) == NilKind::Slice => Some(Vec::new()),
        Value::Object(o) if o.kind() == go_value::Kind::Slice => Some(o.list().unwrap_or_default()),
        _ => None,
    }
}

// Go: spf13/cast slice.go:ToSliceE
/// ToSliceE casts any value to a `[]interface {}` type.
pub fn to_slice_e(v: &Value) -> Result<Vec<Value>> {
    let (i, _) = indirect(v);

    match &i {
        Value::List(l) if matches!(l.ty, SliceType::Any | SliceType::MapStringAny) => {
            Ok(l.items.clone())
        }
        Value::TypedNil(t) if &**t == "[]interface {}" || &**t == "[]map[string]interface {}" => {
            Ok(Vec::new())
        }
        _ => Err(cast_error(&i, "[]interface {}")),
    }
}

/// `cast.ToSlice` (errors -> nil).
pub fn to_slice(v: &Value) -> Vec<Value> {
    to_slice_e(v).unwrap_or_default()
}

// Go: spf13/cast slice.go:toSliceEOk
/// `(v, ok, err)` of Go as `Some(Ok(v))`, `Some(Err(err))` (ok with an error) or `None` (not ok).
fn to_slice_e_ok<T>(
    v: &Value,
    elem: &str,
    conv: fn(&Value) -> Result<T>,
) -> Option<Result<Vec<T>>> {
    let (i, _) = indirect(v);
    if i.is_invalid() {
        return Some(Err(cast_error(&i, &slice_type_name(elem))));
    }

    // `case []T:` returns the slice as is; converting its elements (already of type T) is the
    // identity, so it shares the reflect path below.
    let items = slice_elems(&i)?;
    let mut a = Vec::with_capacity(items.len());

    for item in &items {
        match conv(item) {
            Ok(val) => a.push(val),
            Err(_) => return Some(Err(cast_error(&i, &slice_type_name(elem)))),
        }
    }

    Some(Ok(a))
}

// Go: spf13/cast slice.go:toSliceE
fn to_slice_e_typed<T>(v: &Value, elem: &str, conv: fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    match to_slice_e_ok(v, elem, conv) {
        Some(r) => r,
        None => Err(cast_error(v, &slice_type_name(elem))),
    }
}

// Go: spf13/cast slice.go:ToStringSliceE
/// ToStringSliceE casts any value to a `[]string` type.
pub fn to_string_slice_e(v: &Value) -> Result<Vec<GoString>> {
    if let Some(r) = to_slice_e_ok(v, "string", to_string_e) {
        return r;
    }

    match v {
        Value::String(s) => Ok(go_unicode::strings::fields(s)
            .into_iter()
            .map(GoString::from)
            .collect()),
        // case any: (every non-nil interface)
        _ => match to_string_e(v) {
            Ok(s) => Ok(vec![s]),
            Err(_) => Err(cast_error(v, "[]string")),
        },
    }
}

/// `cast.ToStringSlice` (errors -> nil).
pub fn to_string_slice(v: &Value) -> Vec<GoString> {
    to_string_slice_e(v).unwrap_or_default()
}

// Go: spf13/cast zz_generated.go:ToIntSliceE
/// ToIntSliceE casts any value to a `[]int` type.
pub fn to_int_slice_e(v: &Value) -> Result<Vec<i64>> {
    to_slice_e_typed(v, "int", to_int_e)
}

/// `cast.ToIntSlice` (errors -> nil).
pub fn to_int_slice(v: &Value) -> Vec<i64> {
    to_int_slice_e(v).unwrap_or_default()
}

// Go: spf13/cast zz_generated.go:ToBoolSliceE
/// ToBoolSliceE casts any value to a `[]bool` type.
pub fn to_bool_slice_e(v: &Value) -> Result<Vec<bool>> {
    to_slice_e_typed(v, "bool", to_bool_e)
}

// Go: spf13/cast zz_generated.go:ToDurationSliceE
/// ToDurationSliceE casts any value to a `[]time.Duration` type.
pub fn to_duration_slice_e(v: &Value) -> Result<Vec<go_time::Duration>> {
    to_slice_e_typed(v, "time.Duration", to_duration_e)
}

// Go: spf13/cast zz_generated.go:ToInt64SliceE
/// ToInt64SliceE casts any value to a `[]int64` type.
pub fn to_int64_slice_e(v: &Value) -> Result<Vec<i64>> {
    to_slice_e_typed(v, "int64", to_int64_e)
}

// Go: spf13/cast zz_generated.go:ToUintSliceE
/// ToUintSliceE casts any value to a `[]uint` type.
pub fn to_uint_slice_e(v: &Value) -> Result<Vec<u64>> {
    to_slice_e_typed(v, "uint", to_uint_e)
}

// Go: spf13/cast zz_generated.go:ToFloat64SliceE
/// ToFloat64SliceE casts any value to a `[]float64` type.
pub fn to_float64_slice_e(v: &Value) -> Result<Vec<f64>> {
    to_slice_e_typed(v, "float64", to_float64_e)
}

// ---------------------------------------------------------------------------
// map.go

/// The Go value type of a `map[string]V` target.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MapValue {
    Any,
    String,
    Bool,
}

impl MapValue {
    fn map_type(self) -> MapType {
        match self {
            MapValue::Any => MapType::StringAny,
            MapValue::String => MapType::StringString,
            MapValue::Bool => MapType::Named(Arc::from("map[string]bool")),
        }
    }

    /// `valFn` of `toStringMapE`: `func(i any) any { return i }`, `ToString` or `ToBool`.
    fn val_fn(self, v: &Value) -> Value {
        match self {
            MapValue::Any => v.clone(),
            MapValue::String => Value::String(to_string(v)),
            MapValue::Bool => Value::Bool(to_bool(v)),
        }
    }
}

// Go: spf13/cast map.go:toMapE (K = string)
fn to_map_e(i: &Value, val: MapValue) -> Result<Map> {
    let target = val.map_type();
    let target_name = target.go_name().into_owned();

    if i.is_invalid() {
        return Err(cast_error(i, &target_name));
    }

    match i {
        // case map[K]V:
        Value::Map(m) if m.ty == target => Ok((**m).clone()),
        Value::TypedNil(t) if **t == *target_name => Ok(Map::new(target)),
        // case map[K]any:
        Value::Map(m) if m.ty == MapType::StringAny => {
            let mut out = Map::new(target);
            for (k, v) in &m.entries {
                out.insert(k.clone(), val.val_fn(v));
            }
            Ok(out)
        }
        Value::TypedNil(t) if &**t == "map[string]interface {}" => Ok(Map::new(target)),
        // case map[any]V, map[any]any: not representable (string-keyed maps only).
        Value::String(s) => json_string_to_object(s.as_bytes(), val),
        _ => Err(cast_error(i, &target_name)),
    }
}

// Go: spf13/cast map.go:jsonStringToObject
/// `json.Unmarshal([]byte(s), &m)` for `m` a `map[string]V`.
fn json_string_to_object(s: &[u8], val: MapValue) -> Result<Map> {
    let target = val.map_type();
    // Unmarshal reports syntax errors (the whole input is checked) before type errors, and a
    // top-level value of the wrong kind before looking inside it (e.g. `1e400` is a type error
    // for a map, not a float64 overflow).
    if !go_json::valid(s) {
        let err = go_json::unmarshal(s).err().map(|e| e.to_string());
        return Err(Error::new(err.unwrap_or_default()));
    }
    match top_level_kind(s) {
        Some("object") => {}
        // JSON null sets the map to nil.
        Some("null") | None => return Ok(Map::new(target)),
        Some(kind) => {
            return Err(Error::new(json_unmarshal_type_error(
                kind,
                "",
                &target.go_name(),
            )));
        }
    }
    let v = if val == MapValue::Any {
        go_json::unmarshal(s)
    } else {
        // Go reports the first value of the wrong type in document order.
        let ty = if val == MapValue::String {
            "string"
        } else {
            "bool"
        };
        if let Some(err) = json_map_type_error(s, ty, |v| match val {
            MapValue::String => matches!(v, Value::String(_) | Value::Invalid),
            _ => matches!(v, Value::Bool(_) | Value::Invalid),
        }) {
            return Err(Error::new(err));
        }
        // Decoding numbers as json.Number keeps a huge number a type error for string/bool
        // targets (checked above), as in Go.
        let mut d = go_json::Decoder::new(s);
        d.use_number();
        d.decode()
    }
    .map_err(|e| Error::new(e.to_string()))?;
    let m = match &v {
        Value::Map(m) => m,
        // JSON null sets the map to nil.
        Value::Invalid => return Ok(Map::new(target)),
        other => {
            return Err(Error::new(json_unmarshal_type_error(
                json_kind(other),
                "",
                &target.go_name(),
            )));
        }
    };
    let mut out = Map::new(target);
    for (k, v) in &m.entries {
        let ev = match (val, v) {
            (MapValue::Any, v) => v.clone(),
            // null leaves the zero value
            (MapValue::String, Value::Invalid) => Value::String(GoString::empty()),
            (MapValue::Bool, Value::Invalid) => Value::Bool(false),
            (MapValue::String, Value::String(_)) | (MapValue::Bool, Value::Bool(_)) => v.clone(),
            (MapValue::String | MapValue::Bool, other) => {
                let ty = if val == MapValue::String {
                    "string"
                } else {
                    "bool"
                };
                return Err(Error::new(json_unmarshal_type_error(
                    json_kind(other),
                    &k.to_str_lossy(),
                    ty,
                )));
            }
        };
        out.insert(k.clone(), ev);
    }
    Ok(out)
}

/// The first `UnmarshalTypeError` message, in document order, of decoding the JSON object `s`
/// (its first value must be syntactically valid) into a `map[string]<ty>`, where `fits` tells
/// whether a decoded value (numbers as `json.Number`) can be stored. `None` if every value fits
/// (or `s` is not an object).
pub(crate) fn json_map_type_error(
    s: &[u8],
    ty: &str,
    fits: impl Fn(&Value) -> bool,
) -> Option<String> {
    let mut d = go_json::Decoder::new(s);
    d.use_number();
    if !matches!(d.token(), Ok(go_json::Token::Delim(b'{'))) {
        return None;
    }
    while d.more() {
        let Ok(go_json::Token::Value(Value::String(k))) = d.token() else {
            return None;
        };
        let v = d.decode().ok()?;
        if !fits(&v) {
            return Some(json_unmarshal_type_error(
                json_kind(&v),
                &k.to_str_lossy(),
                ty,
            ));
        }
    }
    None
}

/// The kind of the top-level value of valid JSON text (`None` for empty input).
pub(crate) fn top_level_kind(s: &[u8]) -> Option<&'static str> {
    let c = *s
        .iter()
        .find(|c| !matches!(c, b' ' | b'\t' | b'\n' | b'\r'))?;
    Some(match c {
        b'{' => "object",
        b'[' => "array",
        b'"' => "string",
        b't' | b'f' => "bool",
        b'n' => "null",
        _ => "number",
    })
}

/// The JSON kind name Go's decoder reports in an `UnmarshalTypeError` for a decoded value.
pub(crate) fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "bool",
        Value::String(_) => "string",
        Value::List(_) => "array",
        Value::Map(_) => "object",
        _ => "number",
    }
}

// Go: encoding/json v2_decode.go:UnmarshalTypeError.Error
/// The message of Go's (jsonv2-backed v1) `UnmarshalTypeError` for a JSON `value` kind that
/// cannot be stored in Go type `ty` at `field` (the JSON path joined with '.', "" at the top).
pub(crate) fn json_unmarshal_type_error(value: &str, field: &str, ty: &str) -> String {
    if !field.is_empty() {
        // As a heuristic, if the last path token looks like an index into a JSON array
        // (e.g., ".foo.bar.0"), avoid the phrase "Go struct field ".
        let mut into_what = "Go struct field ";
        let i = field.rfind('.').map_or(0, |i| i + 1);
        if !field[i..].is_empty() && field[i..].bytes().all(|c| c.is_ascii_digit()) {
            into_what = "";
        }
        return format!("json: cannot unmarshal {value} into {into_what}.{field} of type {ty}");
    }
    format!("json: cannot unmarshal {value} into Go value of type {ty}")
}

// Go: spf13/cast map.go:ToStringMapE
/// ToStringMapE casts any value to a `map[string]interface {}` type.
pub fn to_string_map_e(v: &Value) -> Result<Map> {
    to_map_e(v, MapValue::Any)
}

/// `cast.ToStringMap` (errors -> nil, here an empty map).
pub fn to_string_map(v: &Value) -> Map {
    to_string_map_e(v).unwrap_or_else(|_| Map::new(MapType::StringAny))
}

// Go: spf13/cast map.go:ToStringMapStringE
/// ToStringMapStringE casts any value to a `map[string]string` type (`MapType::StringString`).
pub fn to_string_map_string_e(v: &Value) -> Result<Map> {
    to_map_e(v, MapValue::String)
}

// Go: spf13/cast map.go:ToStringMapBoolE
/// ToStringMapBoolE casts any value to a `map[string]bool` type (a `Map` of
/// `MapType::Named("map[string]bool")` with `Bool` values).
pub fn to_string_map_bool_e(v: &Value) -> Result<Map> {
    to_map_e(v, MapValue::Bool)
}

/// `cast.ToStringMapBool` (errors -> nil, here an empty map).
pub fn to_string_map_bool(v: &Value) -> Map {
    to_string_map_bool_e(v).unwrap_or_else(|_| Map::new(MapValue::Bool.map_type()))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (third-party: spf13/cast v1.9.2; written by T26, not generated — the
// coverage run did not instrument third-party modules). `OK` = ported. Items without a prefix are
// not called by neohugo and are not ported (no stub needed: nothing can reach them).
// Source: cast.go
//   types: Basic
// OK L22-68: ToE[T Basic](i any) (T, error)
//    L71-77: Must[T any](i any, err error) T
//    L80-84: To[T Basic](i any) T
// Source: basic.go
// OK L17-67: ToBoolE(i any) (bool, error)
// OK L70-131: ToStringE(i any) (string, error)
// Source: indirect.go
// OK L16-37: indirect(i any) (any, bool)
// Source: alias.go
// OK L12-27: kindNames
// OK L29-44: kinds
// OK L49-69: resolveAlias(i any) (any, bool)
// Source: number.go
//   types: float64EProvider, float64Provider, Number, integer, unsigned, float
// OK L18: errNegativeNotAllowed
// OK L48-79: ToNumberE[T Number](i any) (T, error)
//    L82-86: ToNumber[T Number](i any) T
// OK L91-136: toNumber[T Number](i any) (T, bool)
// OK L138-193: toNumberE[T Number](i any, parseFn func(string) (T, error)) (T, error)
// OK L195-276: toUnsignedNumber[T Number](i any) (T, bool, bool)
// OK L278-347: toUnsignedNumberE[T Number](i any, parseFn func(string) (T, error)) (T, error)
// OK L349-405: parseNumber[T Number](s string) (T, error)
// OK L407-414: parseInt[T integer](s string) (T, error)
// OK L416-423: parseUint[T unsigned](s string) (T, error)
// OK L425-445: parseFloat[T float](s string) (T, error)
// OK L448-450: ToFloat64E(i any) (float64, error)
// OK L453-455: ToFloat32E(i any) (float32, error)
// OK L458-460: ToInt64E(i any) (int64, error)
// OK L463-465: ToInt32E(i any) (int32, error)
// OK L468-470: ToInt16E(i any) (int16, error)
// OK L473-475: ToInt8E(i any) (int8, error)
// OK L478-480: ToIntE(i any) (int, error)
// OK L483-485: ToUintE(i any) (uint, error)
// OK L488-490: ToUint64E(i any) (uint64, error)
// OK L493-495: ToUint32E(i any) (uint32, error)
// OK L498-500: ToUint16E(i any) (uint16, error)
// OK L503-505: ToUint8E(i any) (uint8, error)
// OK L507-522: trimZeroDecimal(s string) string
// OK L524: stringNumberRe (hand-written matcher in trim_decimal)
// OK L527-549: trimDecimal(s string) string
// Source: slice.go
// OK L15-33: ToSliceE(i any) ([]any, error)
// OK L35-46: toSliceE[T Basic](i any) ([]T, error)
// OK L48-79: toSliceEOk[T Basic](i any) ([]T, bool, error)
// OK L82-106: ToStringSliceE(i any) ([]string, error)
// Source: map.go (the map[any]... cases are not representable in the value model)
// OK L14-57: toMapE[K comparable, V any](i any, keyFn func(any) K, valFn func(any) V) (map[K]V, error)
// OK L59-61: toStringMapE[T any](i any, fn func(any) T) (map[string]T, error)
// OK L64-66: ToStringMapStringE(i any) (map[string]string, error)
//    L69-135: ToStringMapStringSliceE(i any) (map[string][]string, error)
// OK L138-140: ToStringMapBoolE(i any) (map[string]bool, error)
// OK L143-147: ToStringMapE(i any) (map[string]any, error)
//    L149-207: toStringMapIntE[T int | int64](i any, fn func(any) T, fnE func(any) (T, error)) (map[string]T, error)
//    L210-212: ToStringMapIntE(i any) (map[string]int, error)
//    L215-217: ToStringMapInt64E(i any) (map[string]int64, error)
// OK L221-224: jsonStringToObject(s string, v any) error
// Source: time.go (ToDurationE here; the rest in cast/time.rs)
// OK L64-101: ToDurationE(i any) (time.Duration, error)
// Source: zz_generated.go
// OK L8-11: ToBool(i any) bool
// OK L14-17: ToString(i any) string
// OK L20-23: ToTime(i any) time.Time (cast/time.rs)
// OK L26-29: ToTimeInDefaultLocation(i any, location *time.Location) time.Time (cast/time.rs)
// OK L32-35: ToDuration(i any) time.Duration
// OK L38-41: ToInt(i any) int
//    L44-59: ToInt8, ToInt16, ToInt32
// OK L62-65: ToInt64(i any) int64
//    L68-89: ToUint, ToUint8, ToUint16, ToUint32
// OK L92-95: ToUint64(i any) uint64
// OK L98-101: ToFloat32(i any) float32
// OK L104-107: ToFloat64(i any) float64
//    L110-113: ToStringMapString(i any) map[string]string
//    L116-119: ToStringMapStringSlice(i any) map[string][]string
// OK L122-125: ToStringMapBool(i any) map[string]bool
//    L128-137: ToStringMapInt, ToStringMapInt64
// OK L140-143: ToStringMap(i any) map[string]any
// OK L146-149: ToSlice(i any) []any
//    L152-155: ToBoolSlice(i any) []bool
// OK L158-161: ToStringSlice(i any) []string
// OK L164-167: ToIntSlice(i any) []int
//    L170-191: ToInt64Slice, ToUintSlice, ToFloat64Slice, ToDurationSlice
// OK L194-196: ToBoolSliceE(i any) ([]bool, error)
// OK L199-201: ToDurationSliceE(i any) ([]time.Duration, error)
// OK L204-206: ToIntSliceE(i any) ([]int, error)
//    L209-221: ToInt8SliceE, ToInt16SliceE, ToInt32SliceE
// OK L224-226: ToInt64SliceE(i any) ([]int64, error)
// OK L229-231: ToUintSliceE(i any) ([]uint, error)
//    L234-256: ToUint8SliceE, ToUint16SliceE, ToUint32SliceE, ToUint64SliceE, ToFloat32SliceE
// OK L259-261: ToFloat64SliceE(i any) ([]float64, error)
// ---------------------------------------------------------------------------
