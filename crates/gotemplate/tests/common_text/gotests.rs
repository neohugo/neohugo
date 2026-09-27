//! The data of Go's `texttemplate/exec_test.go` (`T`, `tVal`, the test
//! func map, `cmpStruct`, ...) in the value model, mirroring
//! `tools/go-oracle/gotemplate/exectests` (a verbatim copy of the Go test
//! data, whose types print as `template.T` etc.).
//!
//! Go values the model cannot express are approximated and the tests that
//! observe the difference are listed as deviations in `exec_go.rs`:
//! complex numbers (`ComplexZero` is omitted), maps with non-string keys
//! (their keys are strings here), pointers to basic types (`PI *int` is the
//! int it points to), `cap != len` (`SICap`), channels and iterator funcs.

#![allow(dead_code)]

use std::sync::{Arc, Once};

use go_value::{HostCtx, IntKind, Map, MapType, NilKind, SliceType, UintKind, Value};
use gotemplate::text::{FuncMap, FuncValue};

use super::model::{GoErr, GoResult, SObj, check_args, int, nil_any, obj, s, tnil};

fn err(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_value::register_named_kind("template.S", NilKind::Slice);
        go_value::register_named_kind("template.I", NilKind::Interface);
        // Go calls the nil-receiver-safe methods of *V and *W when printing.
        go_fmt::register_named_method(
            "*template.V",
            go_fmt::NamedMethod::String(|_| Some(b"nilV".to_vec())),
        );
        go_fmt::register_named_method(
            "*template.W",
            go_fmt::NamedMethod::Error(|_| Some(b"nilW".to_vec())),
        );
    });
}

fn func_value(ty: &str, f: impl Fn(&[Value]) -> GoResult + Send + Sync + 'static) -> Value {
    Value::object(FuncValue {
        type_name: ty.to_string(),
        func: Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| f(args)),
    })
}

fn sprint(args: &[Value]) -> Vec<u8> {
    go_fmt::sprint(args)
}

fn ints(items: &[i64]) -> Value {
    Value::list(SliceType::Int, items.iter().map(|&i| int(i)).collect())
}

pub fn named_map(ty: &str, entries: &[(&str, Value)]) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from(ty)));
    for (k, v) in entries {
        m.insert(*k, v.clone());
    }
    Value::map(m)
}

fn any_map(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new(MapType::StringAny);
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::map(m)
}

fn str_of(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::String(s) => Some(s.to_vec()),
        _ => None,
    }
}

/// Go: `&U{v}`.
fn u(v: &str) -> Value {
    obj(SObj::new("template.U", true, vec![("V", s(v))]))
}

/// Go: `V{j}` (addressable when a field of a struct reached through a
/// pointer) or `&V{j}`.
pub fn v(j: i64, ptr: bool, addressable: bool) -> Value {
    let mut o = SObj::new("template.V", ptr, vec![("j", int(j))]);
    o.addressable = addressable;
    obj(o)
}

fn w(k: i64, ptr: bool, addressable: bool) -> Value {
    let mut o = SObj::new("template.W", ptr, vec![("k", int(k))]);
    o.addressable = addressable;
    obj(o)
}

fn buffer(content: &str) -> Value {
    obj(SObj::new("bytes.Buffer", true, vec![("buf", s(content))]))
}

/// The `T` fields that `tVal` sets, in declaration order; the others are
/// zero values.
fn t_value(ptr: bool, x: &str, full: bool) -> Value {
    let z = !full;
    let pick = |set: Value, zero: Value| if z { zero } else { set };
    let fields = vec![
        ("True", pick(Value::Bool(true), Value::Bool(false))),
        ("I", pick(int(17), int(0))),
        (
            "U16",
            pick(
                Value::Uint(16, UintKind::Uint16),
                Value::Uint(0, UintKind::Uint16),
            ),
        ),
        ("X", s(x)),
        ("S", pick(s("xyz"), s(""))),
        ("FloatZero", Value::float64(0.0)),
        ("U", pick(u("v"), tnil("*template.U"))),
        ("V0", v(if z { 0 } else { 6666 }, false, ptr)),
        ("V1", pick(v(7777, true, false), tnil("*template.V"))),
        ("V2", tnil("*template.V")),
        ("W0", w(if z { 0 } else { 888 }, false, ptr)),
        ("W1", pick(w(999, true, false), tnil("*template.W"))),
        ("W2", tnil("*template.W")),
        ("SI", pick(ints(&[3, 4, 5]), tnil("[]int"))),
        ("SICap", pick(ints(&[0, 0, 0, 0, 0]), tnil("[]int"))),
        ("SIEmpty", tnil("[]int")),
        (
            "SB",
            pick(
                Value::list(SliceType::Bool, vec![Value::Bool(true), Value::Bool(false)]),
                tnil("[]bool"),
            ),
        ),
        (
            "AI",
            Value::list(
                SliceType::Named(Arc::from("[3]int")),
                if z {
                    vec![int(0), int(0), int(0)]
                } else {
                    vec![int(3), int(4), int(5)]
                },
            ),
        ),
        (
            "MSI",
            pick(
                named_map(
                    "map[string]int",
                    &[("one", int(1)), ("two", int(2)), ("three", int(3))],
                ),
                tnil("map[string]int"),
            ),
        ),
        (
            "MSIone",
            pick(
                named_map("map[string]int", &[("one", int(1))]),
                tnil("map[string]int"),
            ),
        ),
        ("MSIEmpty", tnil("map[string]int")),
        (
            "MXI",
            pick(
                named_map("map[interface {}]int", &[("one", int(1))]),
                tnil("map[interface {}]int"),
            ),
        ),
        (
            "MII",
            pick(
                named_map("map[int]int", &[("1", int(1))]),
                tnil("map[int]int"),
            ),
        ),
        (
            "MI32S",
            pick(
                named_map("map[int32]string", &[("1", s("one")), ("2", s("two"))]),
                tnil("map[int32]string"),
            ),
        ),
        (
            "MI64S",
            pick(
                named_map("map[int64]string", &[("2", s("i642")), ("3", s("i643"))]),
                tnil("map[int64]string"),
            ),
        ),
        (
            "MUI32S",
            pick(
                named_map("map[uint32]string", &[("2", s("u322")), ("3", s("u323"))]),
                tnil("map[uint32]string"),
            ),
        ),
        (
            "MUI64S",
            pick(
                named_map("map[uint64]string", &[("2", s("ui642")), ("3", s("ui643"))]),
                tnil("map[uint64]string"),
            ),
        ),
        (
            "MI8S",
            pick(
                named_map("map[int8]string", &[("2", s("i82")), ("3", s("i83"))]),
                tnil("map[int8]string"),
            ),
        ),
        (
            "MUI8S",
            pick(
                named_map("map[uint8]string", &[("2", s("u82")), ("3", s("u83"))]),
                tnil("map[uint8]string"),
            ),
        ),
        (
            "SMSI",
            pick(
                Value::list(
                    SliceType::Named(Arc::from("[]map[string]int")),
                    vec![
                        named_map("map[string]int", &[("one", int(1)), ("two", int(2))]),
                        named_map(
                            "map[string]int",
                            &[("eleven", int(11)), ("twelve", int(12))],
                        ),
                    ],
                ),
                tnil("[]map[string]int"),
            ),
        ),
        ("Empty0", nil_any()),
        ("Empty1", pick(int(3), nil_any())),
        ("Empty2", pick(s("empty2"), nil_any())),
        ("Empty3", pick(ints(&[7, 8]), nil_any())),
        ("Empty4", pick(u("UinEmpty"), nil_any())),
        (
            "NonEmptyInterface",
            if z {
                tnil("template.I")
            } else {
                t_value(true, "x", false)
            },
        ),
        (
            "NonEmptyInterfacePtS",
            pick(
                Value::list(
                    SliceType::Named(Arc::from("template.S")),
                    vec![s("a"), s("b")],
                ),
                tnil("*template.I"),
            ),
        ),
        ("NonEmptyInterfaceNil", tnil("template.I")),
        (
            "NonEmptyInterfaceTypedNil",
            pick(tnil("*template.T"), tnil("template.I")),
        ),
        ("Str", pick(buffer("foozle"), tnil("fmt.Stringer"))),
        (
            "Err",
            pick(Value::object(GoErr("erroozle".to_string())), tnil("error")),
        ),
        ("PI", pick(int(23), tnil("*int"))),
        ("PS", pick(s("a string"), tnil("*string"))),
        ("PSI", pick(ints(&[21, 22, 23]), tnil("*[]int"))),
        ("NIL", tnil("*int")),
        (
            "BinaryFunc",
            pick(
                func_value("func(string, string) string", |a| {
                    let mut out = b"[".to_vec();
                    out.extend(str_of(&a[0]).unwrap_or_default());
                    out.push(b'=');
                    out.extend(str_of(&a[1]).unwrap_or_default());
                    out.push(b']');
                    Ok(s(out))
                }),
                tnil("func(string, string) string"),
            ),
        ),
        (
            "VariadicFunc",
            pick(
                func_value("func(...string) string", |a| {
                    let parts: Vec<Vec<u8>> =
                        a.iter().map(|x| str_of(x).unwrap_or_default()).collect();
                    Ok(s(sprint(&[s("<"), s(parts.join(&b'+')), s(">")])))
                }),
                tnil("func(...string) string"),
            ),
        ),
        (
            "VariadicFuncInt",
            pick(
                func_value("func(int, ...string) string", |a| {
                    let parts: Vec<Vec<u8>> = a[1..]
                        .iter()
                        .map(|x| str_of(x).unwrap_or_default())
                        .collect();
                    Ok(s(sprint(&[
                        a[0].clone(),
                        s("=<"),
                        s(parts.join(&b'+')),
                        s(">"),
                    ])))
                }),
                tnil("func(int, ...string) string"),
            ),
        ),
        (
            "NilOKFunc",
            pick(
                func_value("func(*int) bool", |a| Ok(Value::Bool(a[0].is_nil()))),
                tnil("func(*int) bool"),
            ),
        ),
        (
            "ErrFunc",
            pick(
                func_value("func() (string, error)", |_| Ok(s("bla"))),
                tnil("func() (string, error)"),
            ),
        ),
        (
            "PanicFunc",
            pick(
                func_value("func() string", |_| Err(err("test panic"))),
                tnil("func() string"),
            ),
        ),
        (
            "TooFewReturnCountFunc",
            pick(func_value("func()", |_| Ok(Value::Invalid)), tnil("func()")),
        ),
        (
            "TooManyReturnCountFunc",
            pick(
                func_value("func() (string, error, int)", |_| Ok(s(""))),
                tnil("func() (string, error, int)"),
            ),
        ),
        (
            "InvalidReturnTypeFunc",
            pick(
                func_value("func() (string, bool)", |_| Ok(s(""))),
                tnil("func() (string, bool)"),
            ),
        ),
        ("Tmpl", tnil("*template.Template")),
    ];
    obj(SObj::new("template.T", ptr, fields))
}

/// Go: `tVal`.
pub fn t_val() -> Value {
    t_value(true, "x", true)
}

/// The value named by the oracle's `dataID`, or why it cannot be modelled.
pub fn gt_data(id: &str) -> Result<Value, &'static str> {
    register();
    Ok(match id {
        "nil" => Value::Invalid,
        "tVal" | "&iVal" => t_val(),
        "(*T)(nil)" => tnil("*template.T"),
        "&T{}" => t_value(true, "", false),
        "T{}" => t_value(false, "", false),
        "tSliceOfNil" => Value::list(
            SliceType::Named(Arc::from("[]*template.T")),
            vec![tnil("*template.T")],
        ),
        "stringerMap" => any_map(vec![("S", buffer("foozle"))]),
        "ifaceValues" => any_map(vec![
            (
                "PlusOne",
                func_value("func(int) int", |a| match &a[0] {
                    Value::Int(i, _) => Ok(int(i + 1)),
                    _ => Err(err("PlusOne: want int")),
                }),
            ),
            ("Slice", ints(&[0, 1, 2, 3])),
            ("One", int(1)),
            ("Two", int(2)),
            ("Nil", Value::Invalid),
            ("Zero", int(0)),
        ]),
        "(*map[string]string)(nil)" => tnil("*map[string]string"),
        "&map[string]string{}" => Value::map(Map::new(MapType::StringString)),
        "cmpStruct" => cmp_struct(),
        "tree" => tree(),
        "chan" => return Err("channels are not in the value model"),
        "complex128|(16.2-17i)" => return Err("complex numbers are not in the value model"),
        "func:func() string" => func_value("func() string", |_| Ok(s("result"))),
        "struct { Str string }|{Hello, world}" => obj(SObj::new(
            "struct { Str string }",
            false,
            vec![("Str", s("Hello, world"))],
        )),
        "struct { a int; b string }|{7 seven}" => obj(SObj::new(
            "struct { a int; b string }",
            false,
            vec![("a", int(7)), ("b", s("seven"))],
        )),
        "[]int|[-1 -2 -3]" => ints(&[-1, -2, -3]),
        "map[string]int|map[two:22]" => named_map("map[string]int", &[("two", int(22))]),
        "map[string]string|map[cause:neglect]" => {
            let mut m = Map::new(MapType::StringString);
            m.insert("cause", s("neglect"));
            Value::map(m)
        }
        _ => {
            if id.starts_with("func:") {
                return Err("iterator funcs (iter.Seq) are not in the value model");
            }
            let (ty, val) = id.split_once('|').ok_or("unknown data")?;
            match ty {
                "int" => int(val.parse().unwrap()),
                "int8" => Value::Int(val.parse().unwrap(), IntKind::Int8),
                "int16" => Value::Int(val.parse().unwrap(), IntKind::Int16),
                "int32" => Value::Int(val.parse().unwrap(), IntKind::Int32),
                "int64" => Value::Int(val.parse().unwrap(), IntKind::Int64),
                "uint" => Value::Uint(val.parse().unwrap(), UintKind::Uint),
                "uint8" => Value::Uint(val.parse().unwrap(), UintKind::Uint8),
                "uint16" => Value::Uint(val.parse().unwrap(), UintKind::Uint16),
                "uint32" => Value::Uint(val.parse().unwrap(), UintKind::Uint32),
                "uint64" => Value::Uint(val.parse().unwrap(), UintKind::Uint64),
                "uintptr" => Value::Uint(val.parse().unwrap(), UintKind::Uintptr),
                "float64" => Value::float64(val.parse().unwrap()),
                "bool" => Value::Bool(val == "true"),
                "string" => s(val),
                _ => return Err("unknown data"),
            }
        }
    })
}

/// Go: TestComparison's `cmpStruct`.
fn cmp_struct() -> Value {
    obj(SObj::new(
        "struct { Uthree uint; Ufour uint; NegOne int; Three int; Ptr *int; NilPtr *int; NonNilMap map[int]int; \
         Map map[int]int; V1 template.V; V2 template.V; Iface1 fmt.Stringer; NilIface fmt.Stringer }",
        true,
        vec![
            ("Uthree", Value::Uint(3, UintKind::Uint)),
            ("Ufour", Value::Uint(4, UintKind::Uint)),
            ("NegOne", int(-1)),
            ("Three", int(3)),
            ("Ptr", obj(SObj::new("int", true, vec![]))),
            ("NilPtr", tnil("*int")),
            ("NonNilMap", named_map("map[int]int", &[])),
            ("Map", tnil("map[int]int")),
            ("V1", v(0, false, true)),
            ("V2", v(0, false, true)),
            (
                "Iface1",
                obj(SObj::new("strings.Builder", true, vec![("buf", s(""))])),
            ),
            ("NilIface", tnil("fmt.Stringer")),
        ],
    ))
}

/// Go: TestTree's tree.
fn tree() -> Value {
    fn node(val: i64, left: Value, right: Value) -> Value {
        obj(SObj::new(
            "template.Tree",
            true,
            vec![("Val", int(val)), ("Left", left), ("Right", right)],
        ))
    }
    let nil = || tnil("*template.Tree");
    node(
        1,
        node(
            2,
            node(3, node(4, nil(), nil()), nil()),
            node(5, node(6, nil(), nil()), nil()),
        ),
        node(
            7,
            node(8, node(9, nil(), nil()), nil()),
            node(10, node(11, nil(), nil()), nil()),
        ),
    )
}

// ---------------------------------------------------------------------------
// Methods.

pub fn method_set(ty: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match ty {
        "template.T" => (
            &[],
            &[
                "Copy", "GetU", "MAdd", "Method0", "Method1", "Method2", "Method3", "MyError",
            ],
        ),
        "template.S" => (&["Method0"], &[]),
        "template.U" => (&[], &["TrueFalse"]),
        "template.V" => (&[], &["String"]),
        "template.W" => (&[], &["Error"]),
        "bytes.Buffer" => (&[], &["String"]),
        "strings.Builder" => (&[], &["String"]),
        _ => (&[], &[]),
    }
}

fn want_int(v: &Value, name: &str) -> go_value::Result<i64> {
    match v {
        Value::Int(i, _) => Ok(*i),
        Value::Uint(u, _) => Ok(*u as i64),
        _ => Err(err(format!("{name}: want an integer"))),
    }
}

pub fn call(recv: &Value, ty: &str, name: &str, args: &[Value]) -> GoResult {
    let so = recv.downcast::<SObj>();
    let nil_deref = || err("runtime error: invalid memory address or nil pointer dereference");
    match (ty, name) {
        ("template.T", "Method0") => {
            check_args(name, args, 0, false)?;
            Ok(s("M0"))
        }
        ("template.T", "Method1") => {
            check_args(name, args, 1, false)?;
            Ok(int(want_int(&args[0], name)?))
        }
        ("template.T", "Method2") => {
            check_args(name, args, 2, false)?;
            let a = want_int(&args[0], name)?;
            let b = str_of(&args[1]).ok_or_else(|| err("Method2: want string"))?;
            Ok(s(go_fmt::sprintf("Method2: %d %s", &[int(a), s(b)])))
        }
        ("template.T", "Method3") => {
            check_args(name, args, 1, false)?;
            Ok(s(go_fmt::sprintf("Method3: %v", &args[..1])))
        }
        ("template.T", "Copy") => {
            check_args(name, args, 0, false)?;
            let o = so.ok_or_else(nil_deref)?;
            Ok(obj(SObj::new("template.T", true, o.fields.clone())))
        }
        ("template.T", "MAdd") => {
            check_args(name, args, 2, false)?;
            let a = want_int(&args[0], name)?;
            let b = args[1].as_list().ok_or_else(|| err("MAdd: want []int"))?;
            let mut out = Vec::new();
            for x in &b.items {
                out.push(int(want_int(x, name)? + a));
            }
            Ok(Value::list(SliceType::Int, out))
        }
        ("template.T", "MyError") => {
            check_args(name, args, 1, false)?;
            match args[0] {
                Value::Bool(true) => Err(err("my error")),
                Value::Bool(false) => Ok(Value::Bool(false)),
                _ => Err(err("MyError: want bool")),
            }
        }
        ("template.T", "GetU") => {
            check_args(name, args, 0, false)?;
            let o = so.ok_or_else(nil_deref)?;
            Ok(o.get("U").cloned().unwrap_or(Value::Invalid))
        }
        ("template.S", "Method0") => {
            check_args(name, args, 0, false)?;
            Ok(s("M0"))
        }
        ("template.U", "TrueFalse") => {
            check_args(name, args, 1, false)?;
            so.ok_or_else(nil_deref)?;
            match args[0] {
                Value::Bool(true) => Ok(s("true")),
                Value::Bool(false) => Ok(s("")),
                _ => Err(err("TrueFalse: want bool")),
            }
        }
        ("template.V", "String") => {
            check_args(name, args, 0, false)?;
            Ok(match so {
                None => s("nilV"),
                Some(o) => s(format!("<{}>", o.i("j"))),
            })
        }
        ("template.W", "Error") => {
            check_args(name, args, 0, false)?;
            Ok(match so {
                None => s("nilW"),
                Some(o) => s(format!("[{}]", o.i("k"))),
            })
        }
        ("bytes.Buffer" | "strings.Builder", "String") => {
            check_args(name, args, 0, false)?;
            Ok(s(so
                .map(|o| o.s("buf"))
                .unwrap_or_else(|| "<nil>".to_string())))
        }
        _ => Err(err(format!("{ty}.{name} not modelled"))),
    }
}

// ---------------------------------------------------------------------------
// Functions (Go: testExecute's funcs).

fn f(g: impl Fn(&[Value]) -> GoResult + Send + Sync + 'static) -> gotemplate::text::Func {
    Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| g(args))
}

fn want_str(v: &Value, name: &str) -> go_value::Result<Vec<u8>> {
    str_of(v).ok_or_else(|| err(format!("{name}: want string")))
}

pub fn gt_funcs() -> FuncMap {
    let mut m = FuncMap::new();
    m.insert(
        "add".into(),
        f(|a| {
            let mut sum = 0i64;
            for x in a {
                sum += want_int(x, "add")?;
            }
            Ok(int(sum))
        }),
    );
    m.insert(
        "count".into(),
        f(|_| Err(err("channels are not in the value model"))),
    );
    m.insert(
        "dddArg".into(),
        f(|a| {
            check_args("dddArg", a, 2, true)?;
            let mut strs = Vec::new();
            for x in &a[1..] {
                strs.push(s(want_str(x, "dddArg")?));
            }
            Ok(s(go_fmt::sprintln(&[
                a[0].clone(),
                Value::list(SliceType::String, strs),
            ])))
        }),
    );
    m.insert("die".into(), f(|_| Err(err("die"))));
    m.insert(
        "echo".into(),
        f(|a| {
            check_args("echo", a, 1, false)?;
            Ok(if a[0].is_invalid() {
                nil_any()
            } else {
                a[0].clone()
            })
        }),
    );
    m.insert(
        "makemap".into(),
        f(|a| {
            if a.len() % 2 != 0 {
                return Err(err("bad makemap"));
            }
            let mut mm = Map::new(MapType::StringString);
            for kv in a.chunks(2) {
                mm.insert(
                    want_str(&kv[0], "makemap")?,
                    s(want_str(&kv[1], "makemap")?),
                );
            }
            Ok(Value::map(mm))
        }),
    );
    m.insert(
        "mapOfThree".into(),
        f(|a| {
            check_args("mapOfThree", a, 0, false)?;
            Ok(named_map("map[string]int", &[("three", int(3))]))
        }),
    );
    m.insert(
        "oneArg".into(),
        f(|a| {
            check_args("oneArg", a, 1, false)?;
            let mut out = b"oneArg=".to_vec();
            out.extend(want_str(&a[0], "oneArg")?);
            Ok(s(out))
        }),
    );
    m.insert(
        "returnInt".into(),
        f(|a| {
            check_args("returnInt", a, 0, false)?;
            Ok(int(7))
        }),
    );
    m.insert(
        "stringer".into(),
        f(|a| {
            check_args("stringer", a, 1, false)?;
            match &a[0] {
                Value::Object(o) => o
                    .go_string()
                    .map(Value::String)
                    .ok_or_else(|| err("stringer: want a Stringer")),
                _ => Err(err("stringer: want a Stringer")),
            }
        }),
    );
    m.insert(
        "twoArgs".into(),
        f(|a| {
            check_args("twoArgs", a, 2, false)?;
            let mut out = b"twoArgs=".to_vec();
            out.extend(want_str(&a[0], "twoArgs")?);
            out.extend(want_str(&a[1], "twoArgs")?);
            Ok(s(out))
        }),
    );
    m.insert(
        "typeOf".into(),
        f(|a| {
            check_args("typeOf", a, 1, false)?;
            Ok(s(go_fmt::sprintf("%T", &a[..1])))
        }),
    );
    m.insert(
        "valueString".into(),
        f(|a| {
            check_args("valueString", a, 1, false)?;
            want_str(&a[0], "valueString")?;
            Ok(s("value is ignored"))
        }),
    );
    m.insert(
        "vfunc".into(),
        f(|a| {
            check_args("vfunc", a, 2, false)?;
            Ok(s("vfunc"))
        }),
    );
    m.insert(
        "zeroArgs".into(),
        f(|a| {
            check_args("zeroArgs", a, 0, false)?;
            Ok(s("zeroArgs"))
        }),
    );
    m
}

/// Go: TestExecutePanicDuringCall's funcs.
pub fn panic_funcs() -> FuncMap {
    let mut m = FuncMap::new();
    m.insert("doPanic".into(), f(|_| Err(err("custom panic string"))));
    m
}
