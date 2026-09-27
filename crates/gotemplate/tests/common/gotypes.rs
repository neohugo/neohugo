//! The host types of the fork's html/template Go tests (exec_test.go,
//! escape_test.go, content_test.go, ...) as `Object`s, and the value spec
//! nodes of tools/go-oracle/gotemplate/htmlexec_types.go that build them.
//!
//! Only compiled by the integration tests (it needs `gotemplate::text`);
//! `html_exec.rs` registers [`extra_nodes`] with `common::set_extra_nodes`.
//!
//! Model notes (differences from Go that the tests account for):
//! - pointers to basic values (`*int`, `*string`, `*[]int`) are their
//!   pointees (the escapers and fmt print them the same);
//! - arrays are lists, int-keyed maps have string keys, complex fields are
//!   float zero, channels do not exist;
//! - a nil `*T` has no methods (Go calls pointer-receiver methods on nil).

#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{
    FloatKind, GoString, HostCtx, Kind, List, Map, MapType, Object, Result, SliceType, UintKind,
    Value,
};
use gotemplate::text::FuncValue;

use super::common::{Node, node_value};

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn named_list(ty: &str, items: Vec<Value>) -> Value {
    Value::List(Arc::new(List::new(SliceType::Named(Arc::from(ty)), items)))
}

fn named_map(ty: &str, kvs: &[(&str, Value)]) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from(ty)));
    for (k, v) in kvs {
        m.insert(*k, v.clone());
    }
    Value::map(m)
}

fn int(i: i64) -> Value {
    Value::int(i)
}

fn ints(xs: &[i64]) -> Value {
    Value::list(SliceType::Int, xs.iter().map(|&x| int(x)).collect())
}

macro_rules! object_basics {
    () => {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    };
}

/// Go's `%v` of a value inside a method result (Method3).
fn sprint_v(v: &Value) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(v))).into_owned()
}

/// A function value (Go: a func stored in data), callable with `call`.
fn func_value(
    type_name: &str,
    f: impl Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
) -> Value {
    Value::object(FuncValue {
        type_name: type_name.to_string(),
        func: Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| f(args)),
    })
}

/// Go `evalArg` for an `int` parameter (the successful cases).
pub fn want_int(v: &Value) -> Result<i64> {
    match v {
        Value::Int(i, _) => Ok(*i),
        Value::Uint(u, _) => Ok(*u as i64),
        other => Err(err(format!(
            "wrong type for value; expected int; got {}",
            other.go_type_name()
        ))),
    }
}

/// Go `evalArg` for a `string` parameter.
pub fn want_string(v: &Value) -> Result<GoString> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Invalid => Err(err("invalid value; expected string")),
        other => Err(err(format!(
            "wrong type for value; expected string; got {}",
            other.go_type_name()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Generic structs (reflect.StructOf): st(...) / pst(...)

/// Go: a struct built by `reflect.StructOf` (or a pointer to one).
pub struct GenStruct {
    pub ptr: bool,
    /// (name, value, Go type, exported)
    pub fields: Vec<(String, Value, String)>,
}

impl GenStruct {
    fn go_type(&self) -> String {
        let fs: Vec<String> = self
            .fields
            .iter()
            .map(|(n, _, t)| format!("{n} {t}"))
            .collect();
        let s = if fs.is_empty() {
            "struct {}".to_string()
        } else {
            format!("struct {{ {} }}", fs.join("; "))
        };
        if self.ptr { format!("*{s}") } else { s }
    }
}

impl Object for GenStruct {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Owned(self.go_type())
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.fields
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, v, _)| v.clone())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.fields
                .iter()
                .map(|(n, v, _)| (Cow::Borrowed(n.as_str()), v.clone()))
                .collect(),
        )
    }
    object_basics!();
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn unhex_str(s: &str) -> String {
    String::from_utf8(unhex(s)).unwrap()
}

fn decode_struct(n: &Node, ptr: bool) -> Value {
    let mut fields = Vec::new();
    for c in n.children.as_deref().unwrap_or(&[]) {
        let cs = c.children.as_ref().expect("field value");
        let name = unhex_str(&c.payload);
        let v = node_value(&cs[0]);
        let ty = match c.name.as_str() {
            "fa" => "interface {}".to_string(),
            "f" => match &v {
                Value::Invalid => "interface {}".to_string(),
                v => v.go_type_name().into_owned(),
            },
            other => panic!("bad struct field {other}"),
        };
        // A nil of an interface type stored in an interface field is a nil
        // interface (the engine hands it to functions as nil).
        fields.push((name, v, ty));
    }
    Value::object(GenStruct { ptr, fields })
}

// ---------------------------------------------------------------------------
// exec_test.go: T, U, V, W, Tree

/// Go: `*main.U`.
pub struct UObj {
    pub v: GoString,
}

impl Object for UObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.U")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "TrueFalse"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<Result<Value>> {
        if name != "TrueFalse" {
            return None;
        }
        Some(match args {
            [Value::Bool(b)] => Ok(Value::string(if *b { "true" } else { "" })),
            _ => Err(err("wrong arguments for TrueFalse")),
        })
    }
    fn field(&self, name: &str) -> Option<Value> {
        (name == "V").then(|| Value::String(self.v.clone()))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![(Cow::Borrowed("V"), Value::String(self.v.clone()))])
    }
    object_basics!();
}

/// Go: `main.V` / `*main.V` (pointer-receiver String).
pub struct VObj {
    pub ptr: bool,
    pub j: i64,
}

impl Object for VObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr { "*main.V" } else { "main.V" })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, name: &str) -> bool {
        self.ptr && name == "String"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<Result<Value>> {
        (self.ptr && name == "String").then(|| Ok(Value::string(format!("<{}>", self.j))))
    }
    fn go_string(&self) -> Option<GoString> {
        self.ptr.then(|| GoString::from(format!("<{}>", self.j)))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![(Cow::Borrowed("j"), int(self.j))])
    }
    object_basics!();
}

/// Go: `main.W` / `*main.W` (pointer-receiver Error).
pub struct WObj {
    pub ptr: bool,
    pub k: i64,
}

impl Object for WObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr { "*main.W" } else { "main.W" })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, name: &str) -> bool {
        self.ptr && name == "Error"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<Result<Value>> {
        (self.ptr && name == "Error").then(|| Ok(Value::string(format!("[{}]", self.k))))
    }
    fn go_error(&self) -> Option<String> {
        self.ptr.then(|| format!("[{}]", self.k))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![(Cow::Borrowed("k"), int(self.k))])
    }
    object_basics!();
}

/// Registers the String/Error methods of nil `*main.V` / `*main.W` (Go
/// calls them on the nil receiver).
pub fn register_named_methods() {
    go_fmt::register_named_method(
        "*main.V",
        go_fmt::NamedMethod::String(|_| Some(b"nilV".to_vec())),
    );
    go_fmt::register_named_method(
        "*main.W",
        go_fmt::NamedMethod::Error(|_| Some(b"nilW".to_vec())),
    );
}

/// Go: `*bytes.Buffer` (a fmt.Stringer).
pub struct BufStr(pub GoString);

impl Object for BufStr {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*bytes.Buffer")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "String"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<Result<Value>> {
        (name == "String").then(|| Ok(Value::String(self.0.clone())))
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }
    object_basics!();
}

/// Go: `*errors.errorString`.
pub struct ErrString(pub String);

impl Object for ErrString {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*errors.errorString")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Error"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<Result<Value>> {
        (name == "Error").then(|| Ok(Value::string(self.0.as_str())))
    }
    fn go_error(&self) -> Option<String> {
        Some(self.0.clone())
    }
    object_basics!();
}

/// Go: `new(int)` (only compared, never printed).
pub struct NewInt;

impl Object for NewInt {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*int")
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    object_basics!();
}

/// Go: `*main.T` (exec_test.go).
pub struct TObj {
    pub fields: BTreeMap<&'static str, Value>,
    /// The field order of Go's struct (for printing).
    pub order: Vec<&'static str>,
}

const T_FIELDS: &[&str] = &[
    "True",
    "I",
    "U16",
    "X",
    "S",
    "FloatZero",
    "ComplexZero",
    "U",
    "V0",
    "V1",
    "V2",
    "W0",
    "W1",
    "W2",
    "SI",
    "SICap",
    "SIEmpty",
    "SB",
    "AI",
    "MSI",
    "MSIone",
    "MSIEmpty",
    "MXI",
    "MII",
    "MI32S",
    "MI64S",
    "MUI32S",
    "MUI64S",
    "MI8S",
    "MUI8S",
    "SMSI",
    "Empty0",
    "Empty1",
    "Empty2",
    "Empty3",
    "Empty4",
    "NonEmptyInterface",
    "NonEmptyInterfacePtS",
    "NonEmptyInterfaceNil",
    "NonEmptyInterfaceTypedNil",
    "Str",
    "Err",
    "PI",
    "PS",
    "PSI",
    "NIL",
    "BinaryFunc",
    "VariadicFunc",
    "VariadicFuncInt",
    "NilOKFunc",
    "ErrFunc",
    "PanicFunc",
    "Tmpl",
];

fn tnil(t: &str) -> Value {
    Value::TypedNil(Arc::from(t))
}

/// The zero value of each field of `T` (Go `T{}`).
fn t_zero_fields() -> BTreeMap<&'static str, Value> {
    let mut m = BTreeMap::new();
    for &f in T_FIELDS {
        let v = match f {
            "True" => Value::Bool(false),
            "I" => int(0),
            "U16" => Value::Uint(0, UintKind::Uint16),
            "X" | "S" => Value::string(""),
            "FloatZero" | "ComplexZero" => Value::Float(0.0, FloatKind::F64),
            "U" => tnil("*main.U"),
            "V0" => Value::object(VObj { ptr: false, j: 0 }),
            "V1" | "V2" => tnil("*main.V"),
            "W0" => Value::object(WObj { ptr: false, k: 0 }),
            "W1" | "W2" => tnil("*main.W"),
            "SI" | "SICap" | "SIEmpty" => tnil("[]int"),
            "SB" => tnil("[]bool"),
            "AI" => named_list("[3]int", vec![int(0), int(0), int(0)]),
            "MSI" | "MSIone" | "MSIEmpty" => tnil("map[string]int"),
            "MXI" => tnil("map[interface {}]int"),
            "MII" => tnil("map[int]int"),
            "MI32S" => tnil("map[int32]string"),
            "MI64S" => tnil("map[int64]string"),
            "MUI32S" => tnil("map[uint32]string"),
            "MUI64S" => tnil("map[uint64]string"),
            "MI8S" => tnil("map[int8]string"),
            "MUI8S" => tnil("map[uint8]string"),
            "SMSI" => tnil("[]map[string]int"),
            "Empty0" | "Empty1" | "Empty2" | "Empty3" | "Empty4" => tnil("interface {}"),
            "NonEmptyInterface" | "NonEmptyInterfaceNil" | "NonEmptyInterfaceTypedNil" => {
                tnil("main.I")
            }
            "NonEmptyInterfacePtS" => tnil("*main.I"),
            "Str" => tnil("fmt.Stringer"),
            "Err" => tnil("error"),
            "PI" | "NIL" => tnil("*int"),
            "PS" => tnil("*string"),
            "PSI" => tnil("*[]int"),
            "BinaryFunc" => tnil("func(string, string) string"),
            "VariadicFunc" => tnil("func(...string) string"),
            "VariadicFuncInt" => tnil("func(int, ...string) string"),
            "NilOKFunc" => tnil("func(*int) bool"),
            "ErrFunc" => tnil("func() (string, error)"),
            "PanicFunc" => tnil("func() string"),
            "Tmpl" => tnil("*template.Template"),
            _ => unreachable!(),
        };
        m.insert(f, v);
    }
    m
}

fn strings_arg(args: &[Value]) -> Result<Vec<String>> {
    args.iter()
        .map(|a| want_string(a).map(|s| s.to_str_lossy().into_owned()))
        .collect()
}

/// exec_test.go's `tVal`.
pub fn t_val() -> TObj {
    let mut m = t_zero_fields();
    let mut set = |k: &'static str, v: Value| {
        m.insert(k, v);
    };
    set("True", Value::Bool(true));
    set("I", int(17));
    set("U16", Value::Uint(16, UintKind::Uint16));
    set("X", Value::string("x"));
    set("S", Value::string("xyz"));
    set("U", Value::object(UObj { v: "v".into() }));
    set(
        "V0",
        Value::object(VObj {
            ptr: false,
            j: 6666,
        }),
    );
    set("V1", Value::object(VObj { ptr: true, j: 7777 }));
    set("W0", Value::object(WObj { ptr: false, k: 888 }));
    set("W1", Value::object(WObj { ptr: true, k: 999 }));
    set("SI", ints(&[3, 4, 5]));
    // make([]int, 5, 10): the value model has no capacity.
    set("SICap", ints(&[0, 0, 0, 0, 0]));
    set("AI", named_list("[3]int", vec![int(3), int(4), int(5)]));
    set(
        "SB",
        Value::list(SliceType::Bool, vec![Value::Bool(true), Value::Bool(false)]),
    );
    set(
        "MSI",
        named_map(
            "map[string]int",
            &[("one", int(1)), ("two", int(2)), ("three", int(3))],
        ),
    );
    set("MSIone", named_map("map[string]int", &[("one", int(1))]));
    set("MXI", named_map("map[interface {}]int", &[("one", int(1))]));
    set("MII", named_map("map[int]int", &[("1", int(1))]));
    set(
        "MI32S",
        named_map(
            "map[int32]string",
            &[("1", Value::string("one")), ("2", Value::string("two"))],
        ),
    );
    set(
        "MI64S",
        named_map(
            "map[int64]string",
            &[("2", Value::string("i642")), ("3", Value::string("i643"))],
        ),
    );
    set(
        "MUI32S",
        named_map(
            "map[uint32]string",
            &[("2", Value::string("u322")), ("3", Value::string("u323"))],
        ),
    );
    set(
        "MUI64S",
        named_map(
            "map[uint64]string",
            &[("2", Value::string("ui642")), ("3", Value::string("ui643"))],
        ),
    );
    set(
        "MI8S",
        named_map(
            "map[int8]string",
            &[("2", Value::string("i82")), ("3", Value::string("i83"))],
        ),
    );
    set(
        "MUI8S",
        named_map(
            "map[uint8]string",
            &[("2", Value::string("u82")), ("3", Value::string("u83"))],
        ),
    );
    set(
        "SMSI",
        named_list(
            "[]map[string]int",
            vec![
                named_map("map[string]int", &[("one", int(1)), ("two", int(2))]),
                named_map(
                    "map[string]int",
                    &[("eleven", int(11)), ("twelve", int(12))],
                ),
            ],
        ),
    );
    set("Empty1", int(3));
    set("Empty2", Value::string("empty2"));
    set("Empty3", ints(&[7, 8]));
    set(
        "Empty4",
        Value::object(UObj {
            v: "UinEmpty".into(),
        }),
    );
    let mut inner = t_zero_fields();
    inner.insert("X", Value::string("x"));
    set(
        "NonEmptyInterface",
        Value::object(TObj {
            fields: inner,
            order: T_FIELDS.to_vec(),
        }),
    );
    // &siVal: a pointer to an interface holding S{"a", "b"} (its pointee).
    set(
        "NonEmptyInterfacePtS",
        Value::list(
            SliceType::Named(Arc::from("main.S")),
            vec![Value::string("a"), Value::string("b")],
        ),
    );
    set("NonEmptyInterfaceTypedNil", tnil("*main.T"));
    set("Str", Value::object(BufStr("foozle".into())));
    set("Err", Value::object(ErrString("erroozle".to_string())));
    // Pointers to basic values are represented by their pointees.
    set("PI", int(23));
    set("PS", Value::string("a string"));
    set("PSI", ints(&[21, 22, 23]));
    set(
        "BinaryFunc",
        func_value("func(string, string) string", |args| {
            if args.len() != 2 {
                return Err(err(format!(
                    "wrong number of args: got {} want 2",
                    args.len()
                )));
            }
            let a = strings_arg(args)?;
            Ok(Value::string(format!("[{}={}]", a[0], a[1])))
        }),
    );
    set(
        "VariadicFunc",
        func_value("func(...string) string", |args| {
            let a = strings_arg(args)?;
            Ok(Value::string(format!("<{}>", a.join("+"))))
        }),
    );
    set(
        "VariadicFuncInt",
        func_value("func(int, ...string) string", |args| {
            let Some((first, rest)) = args.split_first() else {
                return Err(err("wrong number of args: got 0 want at least 1"));
            };
            let a = want_int(first)?;
            let s = strings_arg(rest)?;
            Ok(Value::string(format!("{a}=<{}>", s.join("+"))))
        }),
    );
    set(
        "NilOKFunc",
        func_value("func(*int) bool", |args| match args {
            [v] => Ok(Value::Bool(v.is_nil())),
            _ => Err(err("wrong number of args")),
        }),
    );
    set(
        "ErrFunc",
        func_value("func() (string, error)", |_| Ok(Value::string("bla"))),
    );
    set(
        "PanicFunc",
        func_value("func() string", |_| Err(err("test panic"))),
    );
    TObj {
        fields: m,
        order: T_FIELDS.to_vec(),
    }
}

impl Object for TObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.T")
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(
            name,
            "Method0" | "Method1" | "Method2" | "Method3" | "Copy" | "MAdd" | "MyError" | "GetU"
        )
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<Result<Value>> {
        let r = match name {
            "Method0" => Ok(Value::string("M0")),
            "Method1" => match args {
                [a] => want_int(a).map(int),
                _ => Err(err("wrong number of args for Method1")),
            },
            "Method2" => match args {
                [a, b] => want_int(a).and_then(|a| {
                    want_string(b)
                        .map(|b| Value::string(format!("Method2: {a} {}", b.to_str_lossy())))
                }),
                _ => Err(err("wrong number of args for Method2")),
            },
            "Method3" => match args {
                [v] => Ok(Value::string(format!("Method3: {}", sprint_v(v)))),
                _ => Err(err("wrong number of args for Method3")),
            },
            "Copy" => Ok(Value::object(TObj {
                fields: self.fields.clone(),
                order: self.order.clone(),
            })),
            "MAdd" => match args {
                [a, Value::List(b)] => want_int(a).and_then(|a| {
                    b.items
                        .iter()
                        .map(|x| want_int(x).map(|x| int(x + a)))
                        .collect::<Result<Vec<_>>>()
                        .map(|v| Value::list(SliceType::Int, v))
                }),
                _ => Err(err("wrong arguments for MAdd")),
            },
            "MyError" => match args {
                [Value::Bool(true)] => Err(err("my error")),
                [Value::Bool(false)] => Ok(Value::Bool(false)),
                _ => Err(err("wrong arguments for MyError")),
            },
            "GetU" => Ok(self.fields["U"].clone()),
            _ => return None,
        };
        Some(r)
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.fields.get(name).cloned()
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.order
                .iter()
                .map(|&f| (Cow::Borrowed(f), self.fields[f].clone()))
                .collect(),
        )
    }
    object_basics!();
}

/// Go: `*main.Tree` (exec_test.go TestTree).
pub struct TreeObj {
    pub val: i64,
    pub left: Value,
    pub right: Value,
}

impl Object for TreeObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.Tree")
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Val" => Some(int(self.val)),
            "Left" => Some(self.left.clone()),
            "Right" => Some(self.right.clone()),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Val"), int(self.val)),
            (Cow::Borrowed("Left"), self.left.clone()),
            (Cow::Borrowed("Right"), self.right.clone()),
        ])
    }
    object_basics!();
}

fn decode_tree(n: &Node) -> Value {
    match n.name.as_str() {
        "nil" => tnil("*main.Tree"),
        "tree" => {
            let cs = n.children.as_ref().expect("tree children");
            Value::object(TreeObj {
                val: n.payload.parse().unwrap(),
                left: decode_tree(&cs[0]),
                right: decode_tree(&cs[1]),
            })
        }
        other => panic!("bad tree node {other}"),
    }
}

// ---------------------------------------------------------------------------
// escape_test.go, content_test.go

/// Go: `*main.badMarshaler` / `*main.goodMarshaler`.
pub struct Marshaler {
    pub good: bool,
}

impl Object for Marshaler {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.good {
            "*main.goodMarshaler"
        } else {
            "*main.badMarshaler"
        })
    }
    fn has_method(&self, name: &str) -> bool {
        name == "MarshalJSON"
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn marshal_json(&self) -> Option<Result<Vec<u8>>> {
        Some(Ok(if self.good {
            br#"{ "<foo>": "O'Reilly" }"#.to_vec()
        } else {
            b"{ foo: 'not quite valid JSON' }".to_vec()
        }))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![])
    }
    object_basics!();
}

/// Go: `main.Issue7379` (an int type with a value-receiver method).
pub struct Issue7379(pub i64);

impl Object for Issue7379 {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.Issue7379")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        name == "SomeMethod"
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<Result<Value>> {
        (name == "SomeMethod").then(|| match args {
            [a] => want_int(a).map(|x| Value::string(format!("<{x}>"))),
            _ => Err(err("wrong number of args for SomeMethod")),
        })
    }
    object_basics!();
}

/// Go: `*main.myStringer` / `*main.errorer`.
pub struct Printer {
    pub error: bool,
    pub v: i64,
}

impl Object for Printer {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.error {
            "*main.errorer"
        } else {
            "*main.myStringer"
        })
    }
    fn has_method(&self, name: &str) -> bool {
        name == if self.error { "Error" } else { "String" }
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn go_string(&self) -> Option<GoString> {
        (!self.error).then(|| GoString::from(format!("string={}", self.v)))
    }
    fn go_error(&self) -> Option<String> {
        self.error.then(|| format!("error={}", self.v))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![(Cow::Borrowed("v"), int(self.v))])
    }
    object_basics!();
}

/// Go: `*main.dataItem` / `main.dataItem` (TestEscapeSet).
pub struct DataItem {
    pub ptr: bool,
    pub children: Vec<Value>,
    pub x: GoString,
}

impl Object for DataItem {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr {
            "*main.dataItem"
        } else {
            "main.dataItem"
        })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Children" => Some(if self.children.is_empty() {
                tnil("[]*main.dataItem")
            } else {
                named_list("[]*main.dataItem", self.children.clone())
            }),
            "X" => Some(Value::String(self.x.clone())),
            _ => None,
        }
    }
    object_basics!();
}

fn decode_data_item(n: &Node, ptr: bool) -> Value {
    Value::object(DataItem {
        ptr,
        children: n
            .children
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|c| decode_data_item(c, true))
            .collect(),
        x: unhex(&n.payload).into(),
    })
}

/// Go: `struct{ a int; b string }{7, "seven"}` (unexported fields).
pub struct Seven;

impl Object for Seven {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.sevenStruct")
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
    ) -> Option<Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("a"), int(7)),
            (Cow::Borrowed("b"), Value::string("seven")),
        ])
    }
    object_basics!();
}

// ---------------------------------------------------------------------------
// Spec nodes

/// Decodes the Go test spec nodes (see htmlexec_types.go:decodeTestNode).
pub fn extra_nodes(n: &Node) -> Option<Value> {
    let p = n.payload.as_str();
    Some(match n.name.as_str() {
        "tval" | "ivalptr" => Value::object(t_val()),
        "tzero" | "tzeroptr" => Value::object(TObj {
            fields: t_zero_fields(),
            order: T_FIELDS.to_vec(),
        }),
        "tree" => decode_tree(n),
        "st" => decode_struct(n, false),
        "pst" => decode_struct(n, true),
        "bufstr" => Value::object(BufStr(unhex(p).into())),
        "newint" => Value::object(NewInt),
        "badm" => Value::object(Marshaler { good: false }),
        "goodm" => Value::object(Marshaler { good: true }),
        "issue7379" => Value::object(Issue7379(p.parse().unwrap())),
        "mystringer" => Value::object(Printer {
            error: false,
            v: p.parse().unwrap(),
        }),
        "errorer" => Value::object(Printer {
            error: true,
            v: p.parse().unwrap(),
        }),
        "vval" | "vptr" => Value::object(VObj {
            ptr: n.name == "vptr",
            j: p.parse().unwrap(),
        }),
        "wval" | "wptr" => Value::object(WObj {
            ptr: n.name == "wptr",
            k: p.parse().unwrap(),
        }),
        "lvval" => named_list(
            "[]main.V",
            n.children
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|c| {
                    Value::object(VObj {
                        ptr: false,
                        j: c.payload.parse().unwrap(),
                    })
                })
                .collect(),
        ),
        "dataitem" => decode_data_item(n, true),
        "dataitemv" => decode_data_item(n, false),
        "fnplusone" => func_value("func(int) int", |args| match args {
            [a] => want_int(a).map(|n| int(n + 1)),
            _ => Err(err("wrong number of args")),
        }),
        "seven" => Value::object(Seven),
        _ => return None,
    })
}

/// Whether a value spec uses a Go value the Rust model cannot represent.
pub fn unsupported_spec(spec: &str) -> bool {
    spec.contains("cplx:")
}
