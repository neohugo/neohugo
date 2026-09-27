//! Go: tpl/internal/go_templates/texttemplate/funcs.go
//!
//! Go's builtins are Go functions called through reflection; their
//! parameter types drive argument evaluation (`evalArg`) and arity checks
//! (`evalCall`). Here they are the [`Builtin`] enum with an explicit
//! signature ([`Builtin::sig`]) and implementations over `Value`.

use std::sync::Arc;

use std::sync::OnceLock;

use go_value::{
    HostCtx, IntKind, Kind, MapType, NilKind, Object, SliceType, UintKind, Value, typed_nil_kind,
};

use super::exec::StructView;

/// A template function (Go: a func in a `FuncMap`, or found by the
/// `ExecHelper`). It receives the host context (Go injects the
/// `context.Context` when the first parameter is one; Rust always passes
/// it) and the evaluated arguments (see the host contract C7 for how
/// arguments are converted).
pub type Func = Arc<dyn Fn(HostCtx<'_>, &[Value]) -> go_value::Result<Value> + Send + Sync>;

/// A function value as an `Object` of `Kind::Func` (Go: a `reflect.Value`
/// of kind Func stored in data), callable with the `call` builtin.
pub struct FuncValue {
    pub type_name: String,
    pub func: Func,
}

impl Object for FuncValue {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.type_name)
    }
    fn kind(&self) -> Kind {
        Kind::Func
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `builtins()` — the names, in Go's declaration order.
pub const BUILTIN_NAMES: &[&str] = &[
    "and", "call", "html", "index", "slice", "js", "len", "not", "or", "print", "printf",
    "println", "urlquery", "eq", "ge", "gt", "le", "lt", "ne",
];

/// The text/template builtin functions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Builtin {
    And,
    Call,
    Html,
    Index,
    Slice,
    Js,
    Len,
    Not,
    Or,
    Print,
    Printf,
    Println,
    UrlQuery,
    Eq,
    Ge,
    Gt,
    Le,
    Lt,
    Ne,
}

/// Parameter types of builtins that matter for `evalArg`/`validateType`.
/// Everything is `any`/`reflect.Value` except `printf`'s format.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ArgType {
    /// `any`, `reflect.Value` (and, for host functions, everything).
    Any,
    /// `string`.
    String,
}

/// A Go function signature, for `evalCall`'s arity checks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sig {
    /// Go `typ.NumIn()`.
    pub(crate) num_in: usize,
    pub(crate) variadic: bool,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Builtin> {
        Some(match name {
            "and" => Builtin::And,
            "call" => Builtin::Call,
            "html" => Builtin::Html,
            "index" => Builtin::Index,
            "slice" => Builtin::Slice,
            "js" => Builtin::Js,
            "len" => Builtin::Len,
            "not" => Builtin::Not,
            "or" => Builtin::Or,
            "print" => Builtin::Print,
            "printf" => Builtin::Printf,
            "println" => Builtin::Println,
            "urlquery" => Builtin::UrlQuery,
            "eq" => Builtin::Eq,
            "ge" => Builtin::Ge,
            "gt" => Builtin::Gt,
            "le" => Builtin::Le,
            "lt" => Builtin::Lt,
            "ne" => Builtin::Ne,
            _ => return None,
        })
    }

    pub(crate) fn sig(self) -> Sig {
        let (num_in, variadic) = match self {
            // and(arg0 reflect.Value, args ...reflect.Value)
            Builtin::And | Builtin::Or => (2, true),
            // emptyCall(fn reflect.Value, args ...reflect.Value)
            Builtin::Call => (2, true),
            // HTMLEscaper(args ...any) etc.
            Builtin::Html | Builtin::Js | Builtin::UrlQuery => (1, true),
            // index(item reflect.Value, indexes ...reflect.Value)
            Builtin::Index | Builtin::Slice => (2, true),
            // length(item reflect.Value)
            Builtin::Len | Builtin::Not => (1, false),
            // fmt.Sprint(a ...any), fmt.Sprintln(a ...any)
            Builtin::Print | Builtin::Println => (1, true),
            // fmt.Sprintf(format string, a ...any)
            Builtin::Printf => (2, true),
            // eq(arg1 reflect.Value, arg2 ...reflect.Value)
            Builtin::Eq => (2, true),
            // ne/lt/le/gt/ge(arg1, arg2 reflect.Value)
            Builtin::Ne | Builtin::Lt | Builtin::Le | Builtin::Gt | Builtin::Ge => (2, false),
        };
        Sig { num_in, variadic }
    }

    /// The type of parameter `i` (for variadic functions, the element type
    /// of the last parameter for `i >= num_in-1`).
    pub(crate) fn arg_type(self, i: usize) -> ArgType {
        if self == Builtin::Printf && i == 0 {
            ArgType::String
        } else {
            ArgType::Any
        }
    }

    /// Whether the (variadic) parameters are `any` rather than
    /// `reflect.Value`: a nil interface argument then arrives as an untyped
    /// nil (Go converts the reflect.Value to an interface for the call).
    pub(crate) fn takes_any(self) -> bool {
        matches!(
            self,
            Builtin::Html
                | Builtin::Js
                | Builtin::UrlQuery
                | Builtin::Print
                | Builtin::Printf
                | Builtin::Println
        )
    }
}

type R<T> = Result<T, String>;

// ---------------------------------------------------------------------------
// Value helpers standing in for reflect.

/// Go `indirectInterface`: values are already concrete, but a typed nil of
/// an interface type is a nil interface, i.e. the zero Value.
pub(crate) fn indirect_interface(v: &Value) -> Value {
    if let Value::TypedNil(t) = v {
        if typed_nil_kind(t) == NilKind::Interface {
            return Value::Invalid;
        }
    }
    v.clone()
}

// Go: exec.go:indirect
/// Returns the item at the end of indirection, and a bool to indicate if
/// it's nil. If the returned bool is true, the returned value's kind will be
/// either a pointer or interface. A non-nil `Kind::Ptr` object (a pointer to
/// a struct) becomes the struct it points to (its [`StructView`]).
pub(crate) fn indirect(v: &Value) -> (Value, bool) {
    match v {
        Value::TypedNil(t) => match typed_nil_kind(t) {
            NilKind::Ptr | NilKind::Interface => (v.clone(), true),
            _ => (v.clone(), false),
        },
        Value::Object(o)
            if o.kind() == Kind::Ptr && o.as_any().downcast_ref::<StructView>().is_none() =>
        {
            (Value::Object(Arc::new(StructView(o.clone()))), false)
        }
        _ => (v.clone(), false),
    }
}

/// The address of a struct reached by [`indirect`] (Go `v.Addr()`, which
/// `evalField` takes to see the methods of both `T` and `*T`): the pointer
/// object again. Other values are returned unchanged.
pub(crate) fn addr(v: Value) -> Value {
    match &v {
        Value::Object(o) => match o.as_any().downcast_ref::<StructView>() {
            Some(sv) => Value::Object(sv.0.clone()),
            None => v,
        },
        _ => v,
    }
}

/// The `%v` text of a value (Go `fmt.Sprintf("%v", v)`).
pub(crate) fn sprint_v(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(v))).into_owned()
}

/// The `%s` text of a value (Go `fmt.Sprintf("%s", v)`).
pub(crate) fn sprint_s(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%s", std::slice::from_ref(v))).into_owned()
}

/// Go `v.Type()` of a (valid) value, as printed by `%s`.
pub(crate) fn type_name(v: &Value) -> String {
    v.go_type_name().into_owned()
}

/// Length of a value that has one (Go reflect `Len` on Array, Chan, Map,
/// Slice, String), or None.
pub(crate) fn value_len(v: &Value) -> Option<usize> {
    match v {
        Value::String(s) | Value::Safe(_, s) => Some(s.len()),
        Value::List(l) => Some(l.items.len()),
        Value::Map(m) => Some(m.entries.len()),
        Value::TypedNil(t) => match typed_nil_kind(t) {
            NilKind::Slice | NilKind::Map | NilKind::Chan => Some(0),
            _ => None,
        },
        Value::Object(o) => match o.kind() {
            Kind::Slice => Some(o.list().map(|l| l.len()).unwrap_or(0)),
            Kind::Map => Some(o.map_keys().len()),
            _ => None,
        },
        _ => None,
    }
}

// Go: funcs.go:indexArg
/// Checks if a reflect.Value can be used as an index, and converts it to int if possible.
fn index_arg(index: &Value, cap: usize) -> R<usize> {
    let x: i64 = match index {
        Value::Int(i, _) => *i,
        Value::Uint(u, _) => *u as i64,
        Value::Invalid => return Err("cannot index slice/array with nil".to_string()),
        _ => {
            return Err(format!(
                "cannot index slice/array with type {}",
                type_name(index)
            ));
        }
    };
    if x < 0 || x as usize > cap {
        return Err(format!("index out of range: {x}"));
    }
    Ok(x as usize)
}

/// A Go func type, parsed from its type string (e.g. `func(string,
/// ...int) (bool, error)`): what `call` inspects through reflection.
#[derive(Debug, PartialEq)]
pub(crate) struct FuncSig {
    /// Parameter types; the last is `[]T` when variadic (Go `typ.In(i)`).
    pub(crate) params: Vec<String>,
    pub(crate) variadic: bool,
    pub(crate) results: Vec<String>,
}

/// Splits `s` at top-level occurrences of ", ".
fn split_types(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                out.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if !s[start..].trim().is_empty() || !out.is_empty() {
        out.push(s[start..].trim().to_string());
    }
    out
}

/// The index of the parenthesis closing the one at `open`.
fn matching_paren(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

impl FuncSig {
    /// Parses a func literal type string; `None` for anything else (a
    /// named func type, whose signature the value model does not know).
    pub(crate) fn parse(t: &str) -> Option<FuncSig> {
        let rest = t.strip_prefix("func")?;
        if !rest.starts_with('(') {
            return None;
        }
        let b = rest.as_bytes();
        let close = matching_paren(b, 0)?;
        let mut params = split_types(&rest[1..close]);
        let mut variadic = false;
        if let Some(last) = params.last_mut() {
            if let Some(elem) = last.strip_prefix("...") {
                *last = format!("[]{elem}");
                variadic = true;
            }
        }
        let res = rest[close + 1..].trim();
        let results = if res.is_empty() {
            Vec::new()
        } else if res.starts_with('(') {
            let c = matching_paren(res.as_bytes(), 0)?;
            split_types(&res[1..c])
        } else {
            vec![res.to_string()]
        };
        Some(FuncSig {
            params,
            variadic,
            results,
        })
    }

    /// The type of argument `i` (Go `call`'s `argType`: the element type of
    /// the variadic parameter for the trailing arguments).
    pub(crate) fn in_type(&self, i: usize) -> String {
        let n = self.params.len();
        if !self.variadic || i < n - 1 {
            self.params[i].clone()
        } else {
            self.params[n - 1]
                .strip_prefix("[]")
                .unwrap_or("")
                .to_string()
        }
    }

    // Go: funcs.go:goodFunc
    /// Reports whether the function or method has the right result
    /// signature.
    pub(crate) fn good_func(&self, name: &str) -> R<()> {
        // We allow functions with 1 result or 2 results where the second is an error.
        match self.results.len() {
            1 => Ok(()),
            2 if self.results[1] == "error" => Ok(()),
            2 => Err(format!(
                "invalid function signature for {name}: second return value should be error; is {}",
                self.results[1]
            )),
            n => Err(format!(
                "function {name} has {n} return values; should be 1 or 2"
            )),
        }
    }
}

/// Go's basic (non-nillable, non-interface) type names.
fn is_basic_type(t: &str) -> bool {
    matches!(
        t,
        "bool"
            | "string"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uintptr"
            | "float32"
            | "float64"
            | "complex64"
            | "complex128"
    )
}

/// The integer kind named by a Go basic type name.
fn int_like_type(t: &str) -> Option<Value> {
    Some(match t {
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
        _ => return None,
    })
}

// Go: exec.go:canBeNil
/// Reports whether an untyped nil can be assigned to the type.
fn can_be_nil(t: &str) -> bool {
    !is_basic_type(t) && !t.starts_with("struct")
}

/// Go `value.Convert(t)` between integer kinds (two's complement wrapping).
fn convert_int(v: &Value, to: &Value) -> Value {
    let bits: u64 = match v {
        Value::Int(i, _) => *i as u64,
        Value::Uint(u, _) => *u,
        _ => 0,
    };
    match to {
        Value::Int(_, k) => {
            let i = match k {
                IntKind::Int8 => bits as i8 as i64,
                IntKind::Int16 => bits as i16 as i64,
                IntKind::Int32 => bits as i32 as i64,
                IntKind::Int | IntKind::Int64 => bits as i64,
            };
            Value::Int(i, *k)
        }
        Value::Uint(_, k) => {
            let u = match k {
                UintKind::Uint8 => bits as u8 as u64,
                UintKind::Uint16 => bits as u16 as u64,
                UintKind::Uint32 => bits as u32 as u64,
                UintKind::Uint | UintKind::Uint64 | UintKind::Uintptr => bits,
            };
            Value::Uint(u, *k)
        }
        _ => v.clone(),
    }
}

// Go: funcs.go:prepareArg (for `call`)
/// Checks if value can be used as an argument of type `arg_type` (a Go type
/// string), and converts an invalid value to appropriate zero if possible.
/// Assignability is decided on type names: `interface {}` accepts
/// everything, `error`/`fmt.Stringer` need the method, other interface or
/// unknown named types are accepted.
pub(crate) fn prepare_arg(value: &Value, arg_type: &str) -> R<Value> {
    if value.is_invalid() {
        if !can_be_nil(arg_type) {
            return Err(format!("value is nil; should be of type {arg_type}"));
        }
        // reflect.Zero(argType): a nil interface arrives as untyped nil.
        if arg_type == "interface {}" || typed_nil_kind(arg_type) == NilKind::Interface {
            return Ok(Value::Invalid);
        }
        return Ok(Value::TypedNil(Arc::from(arg_type)));
    }
    let vt = type_name(value);
    let assignable = vt == arg_type
        || arg_type == "interface {}"
        || match arg_type {
            "error" => matches!(value, Value::Object(o) if o.go_error().is_some()),
            "fmt.Stringer" => matches!(value, Value::Object(o) if o.go_string().is_some()),
            t => !is_basic_type(t) && typed_nil_kind(t) == NilKind::Interface && !t.contains('.'),
        };
    if assignable {
        return Ok(value.clone());
    }
    if let (Value::Int(..) | Value::Uint(..), Some(to)) = (value, int_like_type(arg_type)) {
        return Ok(convert_int(value, &to));
    }
    Err(format!("value has type {vt}; should be {arg_type}"))
}

// Go: funcs.go:prepareArg
/// Checks if value can be used as an argument of type argType (a map key
/// type, always `string` in this value model).
fn prepare_map_key(value: &Value) -> R<Vec<u8>> {
    match value {
        Value::Invalid => Err("value is nil; should be of type string".to_string()),
        Value::String(s) => Ok(s.to_vec()),
        _ => Err(format!(
            "value has type {}; should be string",
            type_name(value)
        )),
    }
}

/// The zero value of a map's element type (Go `reflect.Zero(item.Type().Elem())`);
/// an interface element's zero (a nil interface) is `Value::Invalid` here.
fn map_zero_elem(m: &Value) -> Value {
    map_elem_zero(m).unwrap_or(Value::Invalid)
}

/// Go `reflect.Zero(m.Type().Elem())` when the value model can construct
/// it: the element type of `map[string]string` or of an unnamed map type
/// `map[K]E` (named by `MapType::Named` or a typed nil) whose `E` is a
/// basic type or an unnamed pointer, slice, map, func or chan type. `None`
/// for interface elements (whose zero is a nil interface) and for element
/// types the model cannot construct (structs, arrays, named types).
pub(crate) fn map_elem_zero(m: &Value) -> Option<Value> {
    let ty: &str = match m {
        Value::Map(m) => match &m.ty {
            MapType::StringString => return Some(Value::string("")),
            MapType::Named(n) => n,
            _ => return None,
        },
        Value::TypedNil(t) => t,
        _ => return None,
    };
    zero_of_type(map_elem_type(ty)?)
}

/// The element type `E` of a Go type string `map[K]E`.
fn map_elem_type(ty: &str) -> Option<&str> {
    let rest = ty.strip_prefix("map[")?;
    let mut depth = 1;
    for (i, c) in rest.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[i + 1..]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Go `reflect.Zero(t)` for the types [`map_elem_zero`] handles.
fn zero_of_type(ty: &str) -> Option<Value> {
    Some(match ty {
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
        "float32" => Value::Float(0.0, go_value::FloatKind::F32),
        "float64" => Value::float64(0.0),
        _ if ty.starts_with('*')
            || ty.starts_with("[]")
            || ty.starts_with("map[")
            || ty.starts_with("func(")
            || ty == "func()"
            || ty.starts_with("chan ")
            || ty.starts_with("<-chan ") =>
        {
            Value::TypedNil(Arc::from(ty))
        }
        _ => return None,
    })
}

// Go: funcs.go:index
/// Returns the result of indexing its first argument by the following
/// arguments. Thus "index x 1 2 3" is, in Go syntax, x[1][2][3]. Each
/// indexed item must be a map, slice, or array.
pub(crate) fn index(item: &Value, indexes: &[Value]) -> R<Value> {
    let mut item = indirect_interface(item);
    if item.is_invalid() {
        return Err("index of untyped nil".to_string());
    }
    for index in indexes {
        let index = indirect_interface(index);
        let (it, is_nil) = indirect(&item);
        if is_nil || it.is_invalid() {
            return Err("index of nil pointer".to_string());
        }
        item = it;
        match &item {
            Value::String(s) | Value::Safe(_, s) => {
                let x = index_arg(&index, s.len())?;
                if x >= s.len() {
                    return Err("reflect: string index out of range".to_string());
                }
                item = Value::Uint(s[x] as u64, UintKind::Uint8);
            }
            Value::List(l) => {
                let x = index_arg(&index, l.items.len())?;
                if x >= l.items.len() {
                    return Err("reflect: slice index out of range".to_string());
                }
                item = l.items[x].clone();
            }
            Value::Object(o) if o.kind() == Kind::Slice => {
                let items = o.list().unwrap_or_default();
                let x = index_arg(&index, items.len())?;
                if x >= items.len() {
                    return Err("reflect: slice index out of range".to_string());
                }
                item = items[x].clone();
            }
            Value::Map(m) => {
                let key = prepare_map_key(&index)?;
                item = match m.get(&key) {
                    Some(x) => x.clone(),
                    None => map_zero_elem(&item),
                };
            }
            Value::Object(o) if o.kind() == Kind::Map => {
                let key = prepare_map_key(&index)?;
                item = o.map_get(&key).unwrap_or(Value::Invalid);
            }
            Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Map => {
                prepare_map_key(&index)?;
                item = map_zero_elem(&item);
            }
            Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Slice => {
                let x = index_arg(&index, 0)?;
                let _ = x;
                return Err("reflect: slice index out of range".to_string());
            }
            _ => return Err(format!("can't index item of type {}", type_name(&item))),
        }
    }
    Ok(item)
}

// Go: funcs.go:slice
/// Returns the result of slicing its first argument by the remaining
/// arguments. Thus "slice x 1 2" is, in Go syntax, x[1:2], while "slice x"
/// is x[:], "slice x 1" is x[1:], and "slice x 1 2 3" is x[1:2:3]. The first
/// argument must be a string, slice, or array.
pub(crate) fn slice(item: &Value, indexes: &[Value]) -> R<Value> {
    let item = indirect_interface(item);
    if item.is_invalid() {
        return Err("slice of untyped nil".to_string());
    }
    if indexes.len() > 3 {
        return Err(format!("too many slice indexes: {}", indexes.len()));
    }
    enum Sl<'a> {
        Str(&'a [u8], Option<go_value::SafeKind>),
        List(SliceType, Vec<Value>),
    }
    let sl = match &item {
        Value::String(s) => {
            if indexes.len() == 3 {
                return Err("cannot 3-index slice a string".to_string());
            }
            Sl::Str(s, None)
        }
        Value::Safe(k, s) => {
            if indexes.len() == 3 {
                return Err("cannot 3-index slice a string".to_string());
            }
            Sl::Str(s, Some(*k))
        }
        Value::List(l) => Sl::List(l.ty.clone(), l.items.clone()),
        Value::TypedNil(t) if typed_nil_kind(t) == NilKind::Slice => {
            Sl::List(SliceType::Named(t.clone()), Vec::new())
        }
        Value::Object(o) if o.kind() == Kind::Slice => Sl::List(
            SliceType::Named(Arc::from(o.type_name().as_ref())),
            o.list().unwrap_or_default(),
        ),
        _ => return Err(format!("can't slice item of type {}", type_name(&item))),
    };
    let len = match &sl {
        Sl::Str(s, _) => s.len(),
        Sl::List(_, l) => l.len(),
    };
    // This value model has cap == len.
    let cap = len;
    let mut idx = [0usize, len, 0];
    for (i, index) in indexes.iter().enumerate() {
        idx[i] = index_arg(index, cap)?;
    }
    if idx[0] > idx[1] {
        return Err(format!("invalid slice index: {} > {}", idx[0], idx[1]));
    }
    if indexes.len() == 3 && idx[1] > idx[2] {
        return Err(format!("invalid slice index: {} > {}", idx[1], idx[2]));
    }
    Ok(match sl {
        Sl::Str(s, None) => Value::string(&s[idx[0]..idx[1]]),
        Sl::Str(s, Some(k)) => Value::Safe(k, go_value::GoString::from(&s[idx[0]..idx[1]])),
        Sl::List(ty, l) => Value::list(ty, l[idx[0]..idx[1]].to_vec()),
    })
}

// Go: funcs.go:length
/// Returns the length of the item, with an error if it has no defined length.
pub(crate) fn length(item: &Value) -> R<Value> {
    let (item, is_nil) = indirect(item);
    if is_nil {
        return Err("len of nil pointer".to_string());
    }
    if item.is_invalid() {
        return Err("reflect: call of reflect.Value.Type on zero Value".to_string());
    }
    match value_len(&item) {
        Some(n) => Ok(Value::int(n as i64)),
        None => Err(format!("len of type {}", type_name(&item))),
    }
}

// ---------------------------------------------------------------------------
// Comparison.

// Go: funcs.go:kind
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BKind {
    Invalid,
    Bool,
    Int,
    Float,
    String,
    Uint,
}

const ERR_BAD_COMPARISON_TYPE: &str = "invalid type for comparison";
const ERR_BAD_COMPARISON: &str = "incompatible types for comparison";
const ERR_NO_COMPARISON: &str = "missing argument for comparison";

// Go: funcs.go:basicKind
fn basic_kind(v: &Value) -> BKind {
    match v {
        Value::Bool(_) => BKind::Bool,
        Value::Int(..) => BKind::Int,
        Value::Uint(..) => BKind::Uint,
        Value::Float(..) => BKind::Float,
        Value::String(_) | Value::Safe(..) => BKind::String,
        _ => BKind::Invalid,
    }
}

fn as_int(v: &Value) -> i64 {
    if let Value::Int(i, _) = v { *i } else { 0 }
}
fn as_uint(v: &Value) -> u64 {
    if let Value::Uint(u, _) = v { *u } else { 0 }
}
fn as_float(v: &Value) -> f64 {
    if let Value::Float(f, _) = v { *f } else { 0.0 }
}
fn as_bytes(v: &Value) -> &[u8] {
    match v {
        Value::String(s) | Value::Safe(_, s) => s,
        _ => b"",
    }
}

// Go: funcs.go:isNil
fn is_nil_value(v: &Value) -> bool {
    matches!(v, Value::Invalid | Value::TypedNil(_))
}

/// Go reflect Kind names for the non-basic comparison path.
fn reflect_kind(v: &Value) -> &'static str {
    match v {
        Value::Invalid => "invalid",
        Value::TypedNil(t) => match typed_nil_kind(t) {
            NilKind::Ptr => "ptr",
            NilKind::Slice => "slice",
            NilKind::Map => "map",
            NilKind::Func => "func",
            NilKind::Chan => "chan",
            NilKind::Interface => "interface",
        },
        Value::Bool(_) => "bool",
        Value::Int(..) => "int",
        Value::Uint(..) => "uint",
        Value::Float(..) => "float",
        Value::String(_) | Value::Safe(..) => "string",
        Value::Time(_) => "struct",
        Value::List(_) => "slice",
        Value::Map(_) => "map",
        Value::Object(o) => match o.kind() {
            Kind::Ptr => "ptr",
            Kind::Struct => "struct",
            Kind::Map => "map",
            Kind::Slice => "slice",
            Kind::Func => "func",
            Kind::Interface => "interface",
        },
    }
}

// Go: funcs.go:canCompare
fn can_compare(v1: &Value, v2: &Value) -> bool {
    let k1 = reflect_kind(v1);
    let k2 = reflect_kind(v2);
    if k1 == k2 {
        return true;
    }
    // We know the type can be compared to nil.
    k1 == "invalid" || k2 == "invalid"
}

/// Go `arg1.Interface() == arg.Interface()` for non-basic kinds.
fn interface_eq(a: &Value, b: &Value) -> R<bool> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            if x.type_name() != y.type_name() {
                return Ok(false);
            }
            if x.kind() == Kind::Struct {
                // Struct values compare field-wise.
                let fa = x.struct_fields();
                let fb = y.struct_fields();
                return Ok(match (fa, fb) {
                    (Some(fa), Some(fb)) => {
                        fa.len() == fb.len() && fa.iter().zip(fb.iter()).all(|(p, q)| p.1 == q.1)
                    }
                    _ => x.identity() == y.identity(),
                });
            }
            Ok(x.identity() == y.identity())
        }
        (Value::Time(x), Value::Time(y)) => Ok(x == y),
        (Value::List(_), _) | (Value::Map(_), _) => Err(format!(
            "non-comparable type {}: {}",
            sprint_s(b),
            type_name(b)
        )),
        _ => Ok(a == b),
    }
}

// Go: funcs.go:eq
/// Evaluates the comparison a == b || a == c || ...
pub(crate) fn eq(arg1: &Value, arg2: &[Value]) -> R<bool> {
    let arg1 = indirect_interface(arg1);
    if arg2.is_empty() {
        return Err(ERR_NO_COMPARISON.to_string());
    }
    let k1 = basic_kind(&arg1);
    for arg in arg2 {
        let arg = indirect_interface(arg);
        let k2 = basic_kind(&arg);
        let mut truth = false;
        if k1 != k2 {
            // Special case: Can compare integer values regardless of type's sign.
            if k1 == BKind::Int && k2 == BKind::Uint {
                truth = as_int(&arg1) >= 0 && as_int(&arg1) as u64 == as_uint(&arg);
            } else if k1 == BKind::Uint && k2 == BKind::Int {
                truth = as_int(&arg) >= 0 && as_uint(&arg1) == as_int(&arg) as u64;
            } else if !arg1.is_invalid() && !arg.is_invalid() {
                return Err(ERR_BAD_COMPARISON.to_string());
            }
        } else {
            match k1 {
                BKind::Bool => {
                    truth = matches!((&arg1, &arg), (Value::Bool(a), Value::Bool(b)) if a == b)
                }
                BKind::Float => truth = as_float(&arg1) == as_float(&arg),
                BKind::Int => truth = as_int(&arg1) == as_int(&arg),
                BKind::String => truth = as_bytes(&arg1) == as_bytes(&arg),
                BKind::Uint => truth = as_uint(&arg1) == as_uint(&arg),
                BKind::Invalid => {
                    if !can_compare(&arg1, &arg) {
                        return Err(format!(
                            "non-comparable types {}: {}, {}: {}",
                            sprint_s(&arg1),
                            type_name(&arg1),
                            type_name(&arg),
                            sprint_v(&arg)
                        ));
                    }
                    if is_nil_value(&arg1) || is_nil_value(&arg) {
                        truth = is_nil_value(&arg) == is_nil_value(&arg1);
                    } else {
                        truth = interface_eq(&arg1, &arg)?;
                    }
                }
            }
        }
        if truth {
            return Ok(true);
        }
    }
    Ok(false)
}

// Go: funcs.go:ne
/// Evaluates the comparison a != b.
pub(crate) fn ne(arg1: &Value, arg2: &Value) -> R<bool> {
    // != is the inverse of ==.
    let equal = eq(arg1, std::slice::from_ref(arg2))?;
    Ok(!equal)
}

// Go: funcs.go:lt
/// Evaluates the comparison a < b.
pub(crate) fn lt(arg1: &Value, arg2: &Value) -> R<bool> {
    let arg1 = indirect_interface(arg1);
    let k1 = basic_kind(&arg1);
    if k1 == BKind::Invalid {
        return Err(ERR_BAD_COMPARISON_TYPE.to_string());
    }
    let arg2 = indirect_interface(arg2);
    let k2 = basic_kind(&arg2);
    if k2 == BKind::Invalid {
        return Err(ERR_BAD_COMPARISON_TYPE.to_string());
    }
    let truth;
    if k1 != k2 {
        // Special case: Can compare integer values regardless of type's sign.
        if k1 == BKind::Int && k2 == BKind::Uint {
            truth = as_int(&arg1) < 0 || (as_int(&arg1) as u64) < as_uint(&arg2);
        } else if k1 == BKind::Uint && k2 == BKind::Int {
            truth = as_int(&arg2) >= 0 && as_uint(&arg1) < as_int(&arg2) as u64;
        } else {
            return Err(ERR_BAD_COMPARISON.to_string());
        }
    } else {
        truth = match k1 {
            BKind::Bool => return Err(ERR_BAD_COMPARISON_TYPE.to_string()),
            BKind::Float => as_float(&arg1) < as_float(&arg2),
            BKind::Int => as_int(&arg1) < as_int(&arg2),
            BKind::String => as_bytes(&arg1) < as_bytes(&arg2),
            BKind::Uint => as_uint(&arg1) < as_uint(&arg2),
            BKind::Invalid => panic!("invalid kind"),
        };
    }
    Ok(truth)
}

// Go: funcs.go:le
/// Evaluates the comparison <= b.
pub(crate) fn le(arg1: &Value, arg2: &Value) -> R<bool> {
    // <= is < or ==.
    let less_than = lt(arg1, arg2)?;
    if less_than {
        return Ok(less_than);
    }
    eq(arg1, std::slice::from_ref(arg2))
}

// Go: funcs.go:gt
/// Evaluates the comparison a > b.
pub(crate) fn gt(arg1: &Value, arg2: &Value) -> R<bool> {
    // > is the inverse of <=.
    let less_or_equal = le(arg1, arg2)?;
    Ok(!less_or_equal)
}

// Go: funcs.go:ge
/// Evaluates the comparison a >= b.
pub(crate) fn ge(arg1: &Value, arg2: &Value) -> R<bool> {
    // >= is the inverse of <.
    let less_than = lt(arg1, arg2)?;
    Ok(!less_than)
}

// ---------------------------------------------------------------------------
// HTML escaping.

const HTML_QUOT: &[u8] = b"&#34;"; // shorter than "&quot;"
const HTML_APOS: &[u8] = b"&#39;"; // shorter than "&apos;" and apos was not in HTML until HTML5
const HTML_AMP: &[u8] = b"&amp;";
const HTML_LT: &[u8] = b"&lt;";
const HTML_GT: &[u8] = b"&gt;";
const HTML_NULL: &[u8] = "\u{FFFD}".as_bytes();

// Go: funcs.go:HTMLEscape
/// Writes to w the escaped HTML equivalent of the plain text data b.
pub fn html_escape(w: &mut Vec<u8>, b: &[u8]) {
    let mut last = 0;
    for (i, &c) in b.iter().enumerate() {
        let html: &[u8] = match c {
            b'\0' => HTML_NULL,
            b'"' => HTML_QUOT,
            b'\'' => HTML_APOS,
            b'&' => HTML_AMP,
            b'<' => HTML_LT,
            b'>' => HTML_GT,
            _ => continue,
        };
        w.extend_from_slice(&b[last..i]);
        w.extend_from_slice(html);
        last = i + 1;
    }
    w.extend_from_slice(&b[last..]);
}

// Go: funcs.go:HTMLEscapeString
/// Returns the escaped HTML equivalent of the plain text data s.
pub fn html_escape_string(s: &[u8]) -> Vec<u8> {
    // Avoid allocation if we can.
    if !s.iter().any(|c| b"'\"&<>\0".contains(c)) {
        return s.to_vec();
    }
    let mut b = Vec::with_capacity(s.len() + 8);
    html_escape(&mut b, s);
    b
}

// Go: funcs.go:HTMLEscaper
/// Returns the escaped HTML equivalent of the textual representation of its
/// arguments.
pub fn html_escaper(args: &[Value]) -> Vec<u8> {
    html_escape_string(&eval_args(args))
}

// ---------------------------------------------------------------------------
// JavaScript escaping.

const JS_LOW_UNI: &[u8] = b"\\u00";
const HEX: &[u8] = b"0123456789ABCDEF";
const JS_BACKSLASH: &[u8] = b"\\\\";
const JS_APOS: &[u8] = b"\\'";
const JS_QUOT: &[u8] = b"\\\"";
const JS_LT: &[u8] = b"\\u003C";
const JS_GT: &[u8] = b"\\u003E";
const JS_AMP: &[u8] = b"\\u0026";
const JS_EQ: &[u8] = b"\\u003D";

// Go: funcs.go:JSEscape
/// Writes to w the escaped JavaScript equivalent of the plain text data b.
pub fn js_escape(w: &mut Vec<u8>, b: &[u8]) {
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];

        if !js_is_special(c as i32) {
            // fast path: nothing to do
            i += 1;
            continue;
        }
        w.extend_from_slice(&b[last..i]);

        if c < go_unicode::utf8::RUNE_SELF as u8 {
            // Quotes, slashes and angle brackets get quoted.
            // Control characters get written as \u00XX.
            match c {
                b'\\' => w.extend_from_slice(JS_BACKSLASH),
                b'\'' => w.extend_from_slice(JS_APOS),
                b'"' => w.extend_from_slice(JS_QUOT),
                b'<' => w.extend_from_slice(JS_LT),
                b'>' => w.extend_from_slice(JS_GT),
                b'&' => w.extend_from_slice(JS_AMP),
                b'=' => w.extend_from_slice(JS_EQ),
                _ => {
                    w.extend_from_slice(JS_LOW_UNI);
                    let (t, b) = (c >> 4, c & 0x0f);
                    w.push(HEX[t as usize]);
                    w.push(HEX[b as usize]);
                }
            }
        } else {
            // Unicode rune.
            let (r, size) = go_unicode::utf8::decode_rune(&b[i..]);
            if go_unicode::is_print(r) {
                w.extend_from_slice(&b[i..i + size]);
            } else {
                w.extend_from_slice(format!("\\u{:04X}", r).as_bytes());
            }
            i += size - 1;
        }
        last = i + 1;
        i += 1;
    }
    w.extend_from_slice(&b[last..]);
}

// Go: funcs.go:JSEscapeString
/// Returns the escaped JavaScript equivalent of the plain text data s.
pub fn js_escape_string(s: &[u8]) -> Vec<u8> {
    // Avoid allocation if we can.
    if go_unicode::strings::index_func(s, js_is_special) < 0 {
        return s.to_vec();
    }
    let mut b = Vec::with_capacity(s.len() + 8);
    js_escape(&mut b, s);
    b
}

// Go: funcs.go:jsIsSpecial
fn js_is_special(r: i32) -> bool {
    match r {
        0x5c | 0x27 | 0x22 | 0x3c | 0x3e | 0x26 | 0x3d => true,
        _ => r < ' ' as i32 || go_unicode::utf8::RUNE_SELF <= r,
    }
}

// Go: funcs.go:JSEscaper
/// Returns the escaped JavaScript equivalent of the textual representation
/// of its arguments.
pub fn js_escaper(args: &[Value]) -> Vec<u8> {
    js_escape_string(&eval_args(args))
}

// Go: funcs.go:URLQueryEscaper
/// Returns the escaped value of the textual representation of its arguments
/// in a form suitable for embedding in a URL query.
pub fn url_query_escaper(args: &[Value]) -> Vec<u8> {
    go_url::query_escape(eval_args(args))
}

// Go: funcs.go:evalArgs
/// Formats the list of arguments into a string. It is therefore equivalent
/// to
///
/// ```text
/// fmt.Sprint(args...)
/// ```
///
/// except that each argument is indirected (if a pointer), as required,
/// using the same rules as the default string evaluation during template
/// execution.
pub fn eval_args(args: &[Value]) -> Vec<u8> {
    // Fast path for simple common case.
    if args.len() == 1 {
        if let Value::String(s) = &args[0] {
            return s.to_vec();
        }
    }
    let mut out: Vec<Value> = Vec::with_capacity(args.len());
    for arg in args {
        match super::exec::printable_value(arg) {
            Ok(a) => out.push(a),
            Err(()) => out.push(arg.clone()), // else let fmt do its thing
        }
    }
    go_fmt::sprint(&out)
}

/// Builtins, as `Func`s, for hosts that merge them into their own function
/// lookup (Go: `texttemplate.GoFuncs`, used by Hugo's `configureSiteStorage`).
///
/// `and`/`or` are always short-circuited by the engine by name (their
/// `Func`s are never called by it); `call` behaves like Go's `emptyCall`
/// when reached through a host lookup (an error, as in Hugo); the others
/// run the builtin with Go's arity checks reported as call errors.
pub fn go_funcs() -> Vec<(&'static str, Func)> {
    builtin_funcs()
        .iter()
        .map(|(n, _, f)| (*n, f.clone()))
        .collect()
}

/// Go: `builtinFuncs()` — one `Func` per builtin, created once, so that the
/// engine can recognise its own builtins when a host hands them back
/// through `ExecHelper::get_func` (see [`builtin_of`]).
fn builtin_funcs() -> &'static [(&'static str, Builtin, Func)] {
    static FUNCS: OnceLock<Vec<(&'static str, Builtin, Func)>> = OnceLock::new();
    FUNCS.get_or_init(|| {
        BUILTIN_NAMES
            .iter()
            .map(|&name| {
                let b = Builtin::from_name(name).expect("builtin");
                let f: Func = Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| {
                    call_builtin_plain(b, args).map_err(go_value::Error::new)
                });
                (name, b, f)
            })
            .collect()
    })
}

/// Which builtin `f` is, if it is one of [`go_funcs`]' functions (Go: the
/// `reflect.Value` of a builtin, whose signature `evalCall` inspects: arity,
/// parameter types).
pub(crate) fn builtin_of(f: &Func) -> Option<Builtin> {
    builtin_funcs()
        .iter()
        .find(|(_, _, g)| Arc::ptr_eq(f, g))
        .map(|(_, b, _)| *b)
}

/// Runs a builtin outside the evaluator (no short-circuit, no truth hook
/// beyond the default truthiness).
fn call_builtin_plain(b: Builtin, args: &[Value]) -> R<Value> {
    let sig = b.sig();
    if sig.variadic {
        if args.len() < sig.num_in - 1 {
            return Err(format!(
                "wrong number of args for {}: want at least {} got {}",
                BUILTIN_NAMES[b as usize],
                sig.num_in - 1,
                args.len()
            ));
        }
    } else if args.len() != sig.num_in {
        return Err(format!(
            "wrong number of args for {}: want {} got {}",
            BUILTIN_NAMES[b as usize],
            sig.num_in,
            args.len()
        ));
    }
    match b {
        Builtin::And | Builtin::Or | Builtin::Call => Err("unreachable".to_string()),
        Builtin::Not => Ok(Value::Bool(!super::hugo::is_truthful_value(&args[0]))),
        _ => call_builtin_simple(b, args),
    }
}

/// The builtins that need no evaluator state.
pub(crate) fn call_builtin_simple(b: Builtin, args: &[Value]) -> R<Value> {
    Ok(match b {
        Builtin::Html => Value::string(html_escaper(args)),
        Builtin::Js => Value::string(js_escaper(args)),
        Builtin::UrlQuery => Value::string(url_query_escaper(args)),
        Builtin::Index => index(&args[0], &args[1..])?,
        Builtin::Slice => slice(&args[0], &args[1..])?,
        Builtin::Len => length(&args[0])?,
        Builtin::Print => Value::string(go_fmt::sprint(args)),
        Builtin::Println => Value::string(go_fmt::sprintln(args)),
        Builtin::Printf => {
            let format = match &args[0] {
                Value::String(s) => s.to_vec(),
                other => {
                    return Err(format!(
                        "wrong type for value; expected string; got {}",
                        type_name(other)
                    ));
                }
            };
            Value::string(go_fmt::sprintf(format, &args[1..]))
        }
        Builtin::Eq => Value::Bool(eq(&args[0], &args[1..])?),
        Builtin::Ne => Value::Bool(ne(&args[0], &args[1])?),
        Builtin::Lt => Value::Bool(lt(&args[0], &args[1])?),
        Builtin::Le => Value::Bool(le(&args[0], &args[1])?),
        Builtin::Gt => Value::Bool(gt(&args[0], &args[1])?),
        Builtin::Ge => Value::Bool(ge(&args[0], &args[1])?),
        Builtin::And | Builtin::Or | Builtin::Call | Builtin::Not => {
            unreachable!("handled by the evaluator")
        }
    })
}

/// Go's `int` for `Value::Int` results.
#[allow(dead_code)]
pub(crate) fn go_int(i: i64) -> Value {
    Value::Int(i, IntKind::Int)
}
