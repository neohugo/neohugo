//! Hand ports of the small tests in $GOROOT/src/fmt/fmt_test.go (go1.27.1)
//! that are not table-driven, plus the printing rules documented in the
//! template-engine research spec (§8.2), with the expected outputs taken
//! from Go.

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{
    GoString, HostCtx, IntKind, Kind, List, Map, MapType, Object, Result, SliceType, Value,
};

fn s(v: &[u8]) -> String {
    String::from_utf8(v.to_vec()).unwrap()
}

fn int(i: i64) -> Value {
    Value::int(i)
}

fn st(x: &str) -> Value {
    Value::string(x)
}

// Go: fmt_test.go:TestFmtInterface
#[test]
fn fmt_interface() {
    assert_eq!(s(&go_fmt::sprintf("%s", &[st("abc")])), "abc");
}

// Go: fmt_test.go:TestBlank
#[test]
fn blank() {
    let got = go_fmt::sprint(&[st("<"), int(1), st(">:"), int(1), int(2), int(3), st("!")]);
    assert_eq!(s(&got), "<1>:1 2 3!");
}

// Go: fmt_test.go:TestBlankln
#[test]
fn blankln() {
    let got = go_fmt::sprintln(&[st("<"), int(1), st(">:"), int(1), int(2), int(3), st("!")]);
    assert_eq!(s(&got), "< 1 >: 1 2 3 !\n");
}

/// Go: `fmt_test.T` of TestStructPrinter (unexported fields a, b, c; the
/// value model exposes them through `struct_fields`).
struct T {
    ptr: bool,
}

impl Object for T {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr {
            "*fmt_test.T"
        } else {
            "fmt_test.T"
        })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("a"), st("abc")),
            (Cow::Borrowed("b"), st("def")),
            (Cow::Borrowed("c"), int(123)),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: fmt_test.go:TestStructPrinter
#[test]
fn struct_printer() {
    let tests = [
        ("%v", "{abc def 123}"),
        ("%+v", "{a:abc b:def c:123}"),
        ("%#v", r#"fmt_test.T{a:"abc", b:"def", c:123}"#),
    ];
    for (f, want) in tests {
        let out = go_fmt::sprintf(f, &[Value::object(T { ptr: false })]);
        assert_eq!(s(&out), want, "Sprintf({f:?}, s)");
        // The same but with a pointer.
        let out = go_fmt::sprintf(f, &[Value::object(T { ptr: true })]);
        assert_eq!(s(&out), format!("&{want}"), "Sprintf({f:?}, &s)");
    }
}

// Go: fmt_test.go:TestSlicePrinter (without the pointer-to-slice case,
// which the value model has no representation for).
#[test]
fn slice_printer() {
    assert_eq!(
        s(&go_fmt::sprint(&[Value::list(SliceType::Int, vec![])])),
        "[]"
    );
    assert_eq!(
        s(&go_fmt::sprint(&[Value::list(
            SliceType::Int,
            vec![int(1), int(2), int(3)]
        )])),
        "[1 2 3]"
    );
}

// Go: fmt_test.go:TestEmptyMap
#[test]
fn empty_map() {
    let nil_map = Value::TypedNil(Arc::from("map[string]int"));
    assert_eq!(s(&go_fmt::sprint(&[nil_map])), "map[]");
    let m = Value::map(Map::new(MapType::Named(Arc::from("map[string]int"))));
    assert_eq!(s(&go_fmt::sprint(&[m])), "map[]");
}

// Go: fmt_test.go:TestMapPrinter (string keys; Go prints them sorted).
#[test]
fn map_printer() {
    let mut m = Map::new(MapType::Named(Arc::from("map[string]string")));
    m.insert("3", st("three"));
    m.insert("1", st("one"));
    m.insert("2", st("two"));
    let v = Value::map(m);
    assert_eq!(
        s(&go_fmt::sprintf("%v", std::slice::from_ref(&v))),
        "map[1:one 2:two 3:three]"
    );
    assert_eq!(s(&go_fmt::sprint(&[v])), "map[1:one 2:two 3:three]");
}

/// Go: `fmt_test.B{}` of TestNilDoesNotBecomeTyped (an empty struct).
struct B;

impl Object for B {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("fmt_test.B")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: fmt_test.go:TestNilDoesNotBecomeTyped
#[test]
fn nil_does_not_become_typed() {
    let a = Value::TypedNil(Arc::from("*fmt_test.A"));
    let got = go_fmt::sprintf(
        "%s %s %s %s %s",
        &[
            Value::Invalid,
            a,
            Value::Invalid,
            Value::object(B),
            Value::Invalid,
        ],
    );
    assert_eq!(
        s(&got),
        "%!s(<nil>) %!s(*fmt_test.A=<nil>) %!s(<nil>) {} %!s(<nil>)"
    );
}

const APPEND_RESULT: &str = "hello world, 23";
const HELLO: &str = "hello ";

// Go: fmt_test.go:TestAppendf
#[test]
fn appendf() {
    let mut b = HELLO.as_bytes().to_vec();
    go_fmt::appendf(&mut b, "world, %d", &[int(23)]);
    assert_eq!(s(&b), APPEND_RESULT);
}

// Go: fmt_test.go:TestAppend
#[test]
fn append() {
    let mut b = HELLO.as_bytes().to_vec();
    go_fmt::append(&mut b, &[st("world"), st(", "), int(23)]);
    assert_eq!(s(&b), APPEND_RESULT);
}

// Go: fmt_test.go:TestAppendln
#[test]
fn appendln() {
    let mut b = HELLO.as_bytes().to_vec();
    go_fmt::appendln(&mut b, &[st("world,"), int(23)]);
    assert_eq!(s(&b), format!("{APPEND_RESULT}\n"));
}

#[test]
fn fprint_family() {
    let mut w: Vec<u8> = Vec::new();
    assert_eq!(
        go_fmt::fprintf(&mut w, "%d-%s", &[int(1), st("x")]).unwrap(),
        3
    );
    assert_eq!(go_fmt::fprint(&mut w, &[int(1), int(2)]).unwrap(), 3);
    assert_eq!(go_fmt::fprintln(&mut w, &[st("a")]).unwrap(), 2);
    assert_eq!(s(&w), "1-x1 2a\n");
}

// Go: fmt_test.go:startests entries with huge star widths (the oracle
// skips outputs over 2000 bytes).
#[test]
fn huge_star_width() {
    let out = go_fmt::sprintf("%*d", &[int(1_000_000), int(7)]);
    assert_eq!(out.len(), 1_000_000);
    assert!(out[..999_999].iter().all(|&c| c == b' '));
    assert_eq!(out[999_999], b'7');
    assert_eq!(
        s(&go_fmt::sprintf("%*d", &[int(1_000_001), int(7)])),
        "%!(BADWIDTH)7"
    );
    let out = go_fmt::sprintf("%.*d", &[int(1_000_000), int(-7)]);
    assert_eq!(out.len(), 1_000_001);
    assert_eq!(out[0], b'-');
}

// The template-engine spec §8.2 printing rules (probe evidence from Go).
#[test]
fn hugo_printing_rules() {
    let f64v = Value::float64;
    let cases: Vec<(Value, &str)> = vec![
        (f64v(1.0), "1"),
        (f64v(4.5), "4.5"),
        (f64v(123456.0), "123456"),
        (f64v(1234567.0), "1.234567e+06"),
        (f64v(1e6), "1e+06"),
        (f64v(1.5e10), "1.5e+10"),
        (f64v(1e20), "1e+20"),
        (f64v(1e21), "1e+21"),
        (f64v(0.0001), "0.0001"),
        (f64v(0.00001), "1e-05"),
        (f64v(1e-7), "1e-07"),
        (f64v(-0.0), "-0"),
        (f64v(2.675), "2.675"),
        (Value::Invalid, "<nil>"),
        (
            Value::list(
                SliceType::Uint8,
                vec![Value::Uint(1, go_value::UintKind::Uint8)],
            ),
            "[1]",
        ),
        (Value::string_list(["a", "b", "c"]), "[a b c]"),
        (
            Value::any_list(vec![int(1), st("two"), f64v(3.5), Value::Bool(true)]),
            "[1 two 3.5 true]",
        ),
        (Value::any_list(vec![]), "[]"),
        (
            Value::any_list(vec![int(1), Value::Invalid, st("x")]),
            "[1 <nil> x]",
        ),
    ];
    for (v, want) in cases {
        assert_eq!(s(&go_fmt::sprint(std::slice::from_ref(&v))), want, "{v:?}");
    }
    assert_eq!(
        s(&go_fmt::sprint(&[st("a"), int(1), int(2), st("b")])),
        "a1 2b"
    );
    assert_eq!(s(&go_fmt::sprint(&[int(1), int(2)])), "1 2");

    let mut m = Map::new(MapType::StringAny);
    m.insert("b", int(2));
    m.insert("a", int(1));
    m.insert("c", Value::list(SliceType::Int, vec![int(1), int(2)]));
    assert_eq!(s(&go_fmt::sprint(&[Value::map(m)])), "map[a:1 b:2 c:[1 2]]");
    let mut m = Map::new(MapType::StringAny);
    m.insert("a", Value::Invalid);
    assert_eq!(s(&go_fmt::sprint(&[Value::map(m)])), "map[a:<nil>]");

    assert_eq!(s(&go_fmt::sprintf("%d", &[st("x")])), "%!d(string=x)");
    assert_eq!(s(&go_fmt::sprintf("%s", &[Value::Invalid])), "%!s(<nil>)");
    assert_eq!(s(&go_fmt::sprintf("%s", &[])), "%!s(MISSING)");
    assert_eq!(
        s(&go_fmt::sprintf("%s", &[st("a"), st("b")])),
        "a%!(EXTRA string=b)"
    );
    assert_eq!(
        s(&go_fmt::sprintf("%q", &[Value::string_list(["a", "b"])])),
        r#"["a" "b"]"#
    );
    assert_eq!(s(&go_fmt::sprintf("%x", &[st("hi")])), "6869");
    assert_eq!(
        s(&go_fmt::sprintf(
            "%%%02x",
            &[Value::Uint(7, go_value::UintKind::Uint8)]
        )),
        "%07"
    );
    assert_eq!(
        s(&go_fmt::sprintf("%T", &[Value::html("x")])),
        "template.HTML"
    );
    assert_eq!(
        s(&go_fmt::sprintf(
            "%T",
            &[Value::map(Map::new(MapType::StringAny))]
        )),
        "map[string]interface {}"
    );
    assert_eq!(s(&go_fmt::sprintf("%T", &[Value::int64(3)])), "int64");
}

// Named nil types: classification used for Value::TypedNil.
#[test]
fn typed_nil_kinds() {
    use go_fmt::NilKind::*;
    for (t, k) in [
        ("*page.Pager", Ptr),
        ("[]string", Slice),
        ("map[string]interface {}", Map),
        ("maps.Params", Map),
        ("page.Pages", Slice),
        ("resource.Resources", Slice),
        ("func()", Func),
        ("chan int", Chan),
        ("page.Page", Interface),
        ("error", Interface),
    ] {
        assert_eq!(go_fmt::typed_nil_kind(t), k, "{t}");
    }
    // A nil interface-typed value is a nil `any`.
    let v = Value::TypedNil(Arc::from("page.Page"));
    assert_eq!(
        s(&go_fmt::sprintf("%v|%T|%d", &[v.clone(), v.clone(), v])),
        "<nil>|<nil>|%!d(<nil>)"
    );
    let p = Value::TypedNil(Arc::from("*page.Pager"));
    assert_eq!(
        s(&go_fmt::sprintf(
            "%v|%T|%d|%p|%#v|%s",
            &[p.clone(), p.clone(), p.clone(), p.clone(), p.clone(), p]
        )),
        "<nil>|*page.Pager|0|0x0|(*page.Pager)(nil)|%!s(*page.Pager=<nil>)"
    );
    // page.Pages has a value-receiver String method (Go: "Pages(0)" for a
    // nil Pages), but no GoString.
    let ps = Value::TypedNil(Arc::from("page.Pages"));
    assert_eq!(
        s(&go_fmt::sprintf("%v|%#v|%d", &[ps.clone(), ps.clone(), ps])),
        "Pages(0)|page.Pages(nil)|[]"
    );
}

/// A host object with a Stringer, used through nested containers.
struct MediaType;

impl Object for MediaType {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("media.Type")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn go_string(&self) -> Option<GoString> {
        Some("text/html".into())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Type"), st("text/html")),
            (Cow::Borrowed("MainType"), st("text")),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn stringer_objects() {
    let m = Value::object(MediaType);
    assert_eq!(
        s(&go_fmt::sprintf("%q", std::slice::from_ref(&m))),
        r#""text/html""#
    );
    assert_eq!(
        s(&go_fmt::sprintf(
            "%v|%s|%x",
            &[m.clone(), m.clone(), m.clone()]
        )),
        "text/html|text/html|746578742f68746d6c"
    );
    assert_eq!(
        s(&go_fmt::sprintf("%d", std::slice::from_ref(&m))),
        "{%!d(string=text/html) %!d(string=text)}"
    );
    assert_eq!(
        s(&go_fmt::sprintf("%#v", std::slice::from_ref(&m))),
        r#"media.Type{Type:"text/html", MainType:"text"}"#
    );
    let l = Value::List(Arc::new(List::new(
        SliceType::Named(Arc::from("media.Types")),
        vec![m.clone(), m],
    )));
    assert_eq!(s(&go_fmt::sprint(&[l])), "[text/html text/html]");
}

#[test]
fn ints_of_every_kind() {
    let cases: Vec<(Value, &str, &str)> = vec![
        (Value::Int(-1, IntKind::Int8), "%x", "-1"),
        (Value::Int(-128, IntKind::Int8), "%d", "-128"),
        (Value::Uint(255, go_value::UintKind::Uint8), "%#v", "0xff"),
        (
            Value::Uint(u64::MAX, go_value::UintKind::Uint64),
            "%d",
            "18446744073709551615",
        ),
        (
            Value::Int(i64::MIN, IntKind::Int64),
            "%d",
            "-9223372036854775808",
        ),
        (
            Value::Int(0x263a, IntKind::Int32),
            "%c|%q|%U|%#U",
            "☺|'☺'|U+263A|U+263A '☺'",
        ),
    ];
    for (v, f, want) in cases {
        let n = f.matches('%').count();
        let args = vec![v; n];
        assert_eq!(s(&go_fmt::sprintf(f, &args)), want, "{f}");
    }
}

#[test]
fn errorf_wrapping() {
    struct E;
    impl Object for E {
        fn type_name(&self) -> Cow<'_, str> {
            Cow::Borrowed("*errors.errorString")
        }
        fn has_method(&self, _: &str) -> bool {
            false
        }
        fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
            None
        }
        fn go_error(&self) -> Option<String> {
            Some("boom".into())
        }
        fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
            Some(vec![(Cow::Borrowed("s"), Value::string("boom"))])
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    let e = Value::object(E);
    let (msg, wrapped) = go_fmt::errorf("x: %w", std::slice::from_ref(&e));
    assert_eq!(s(&msg), "x: boom");
    assert_eq!(wrapped, vec![0]);
    // Sprintf does not accept %w.
    assert_eq!(
        s(&go_fmt::sprintf("x: %w", &[e])),
        "x: %!w(*errors.errorString=&{boom})"
    );
}

/// An error value for the ported TestErrorf table: `ptr` selects
/// `*errors.errorString` (errors.New) or the value-receiver `errString`.
struct ErrV {
    ptr: bool,
    msg: &'static str,
}

impl Object for ErrV {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr {
            "*errors.errorString"
        } else {
            "fmt_test.errString"
        })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn go_error(&self) -> Option<String> {
        Some(self.msg.to_string())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: errors_test.go:TestErrorf (the text and the wrapped operands; a
// `wantUnwrap`/`wantSplit` operand is given by its index).
#[test]
fn errorf_table() {
    let wrapped = || {
        Value::object(ErrV {
            ptr: true,
            msg: "inner error",
        })
    };
    let es = |m: &'static str| Value::object(ErrV { ptr: false, msg: m });
    let cases: Vec<(&str, Vec<Value>, &str, Vec<usize>)> = vec![
        ("%w", vec![wrapped()], "inner error", vec![0]),
        (
            "added context: %w",
            vec![wrapped()],
            "added context: inner error",
            vec![0],
        ),
        (
            "%w with added context",
            vec![wrapped()],
            "inner error with added context",
            vec![0],
        ),
        (
            "%s %w %v",
            vec![st("prefix"), wrapped(), st("suffix")],
            "prefix inner error suffix",
            vec![1],
        ),
        (
            "%[2]s: %[1]w",
            vec![wrapped(), st("positional verb")],
            "positional verb: inner error",
            vec![0],
        ),
        ("%v", vec![wrapped()], "inner error", vec![]),
        (
            "added context: %v",
            vec![wrapped()],
            "added context: inner error",
            vec![],
        ),
        (
            "%v with added context",
            vec![wrapped()],
            "inner error with added context",
            vec![],
        ),
        (
            "%w is not an error",
            vec![st("not-an-error")],
            "%!w(string=not-an-error) is not an error",
            vec![],
        ),
        ("no verbs", vec![], "no verbs", vec![]),
        (
            "no verbs with extra arg",
            vec![st("extra")],
            "no verbs with extra arg%!(EXTRA string=extra)",
            vec![],
        ),
        (
            "too many verbs: %w %v",
            vec![],
            "too many verbs: %!w(MISSING) %!v(MISSING)",
            vec![],
        ),
        (
            "wrapped two errors: %w %w",
            vec![es("1"), es("2")],
            "wrapped two errors: 1 2",
            vec![0, 1],
        ),
        (
            "wrapped three errors: %w %w %w",
            vec![es("1"), es("2"), es("3")],
            "wrapped three errors: 1 2 3",
            vec![0, 1, 2],
        ),
        (
            "wrapped nil error: %w %w %w",
            vec![es("1"), Value::Invalid, es("2")],
            "wrapped nil error: 1 %!w(<nil>) 2",
            vec![0, 2],
        ),
        (
            "wrapped one non-error: %w %w %w",
            vec![es("1"), st("not-an-error"), es("3")],
            "wrapped one non-error: 1 %!w(string=not-an-error) 3",
            vec![0, 2],
        ),
        (
            "wrapped errors out of order: %[3]w %[2]w %[1]w",
            vec![es("1"), es("2"), es("3")],
            "wrapped errors out of order: 3 2 1",
            vec![0, 1, 2],
        ),
        (
            "wrapped several times: %[1]w %[1]w %[2]w %[1]w",
            vec![es("1"), es("2")],
            "wrapped several times: 1 1 2 1",
            vec![0, 1],
        ),
        ("%w", vec![Value::Invalid], "%!w(<nil>)", vec![]),
    ];
    for (format, args, want_text, want_wrapped) in cases {
        let (msg, w) = go_fmt::errorf(format, &args);
        assert_eq!(s(&msg), want_text, "{format}");
        assert_eq!(w, want_wrapped, "{format}");
    }
}

/// A named basic type with a value-receiver String method (Go:
/// stringer_test.go's TI, TU8, TF32, TB, TS ...): the value model has no
/// named basic types, so it is a struct-kind object whose String result is
/// computed with this crate's Sprintf, as the Go methods do.
struct NamedBasic {
    name: &'static str,
    format: &'static str,
    v: Value,
}

impl Object for NamedBasic {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name)
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn go_string(&self) -> Option<GoString> {
        Some(go_fmt::sprintf(self.format, std::slice::from_ref(&self.v)).into())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: stringer_test.go:TestStringer
#[test]
fn stringer_table() {
    use go_value::{FloatKind, UintKind};
    let nb = |name, format, v| Value::object(NamedBasic { name, format, v });
    let got = go_fmt::sprintf(
        "%v %v %v %v %v",
        &[
            nb("fmt_test.TI", "I: %d", Value::Int(0, IntKind::Int)),
            nb("fmt_test.TI8", "I8: %d", Value::Int(1, IntKind::Int8)),
            nb("fmt_test.TI16", "I16: %d", Value::Int(2, IntKind::Int16)),
            nb("fmt_test.TI32", "I32: %d", Value::Int(3, IntKind::Int32)),
            nb("fmt_test.TI64", "I64: %d", Value::Int(4, IntKind::Int64)),
        ],
    );
    assert_eq!(s(&got), "I: 0 I8: 1 I16: 2 I32: 3 I64: 4");
    let got = go_fmt::sprintf(
        "%v %v %v %v %v %v",
        &[
            nb("fmt_test.TU", "U: %d", Value::Uint(5, UintKind::Uint)),
            nb("fmt_test.TU8", "U8: %d", Value::Uint(6, UintKind::Uint8)),
            nb("fmt_test.TU16", "U16: %d", Value::Uint(7, UintKind::Uint16)),
            nb("fmt_test.TU32", "U32: %d", Value::Uint(8, UintKind::Uint32)),
            nb("fmt_test.TU64", "U64: %d", Value::Uint(9, UintKind::Uint64)),
            nb("fmt_test.TUI", "UI: %d", Value::Uint(10, UintKind::Uintptr)),
        ],
    );
    assert_eq!(s(&got), "U: 5 U8: 6 U16: 7 U32: 8 U64: 9 UI: 10");
    let got = go_fmt::sprintf(
        "%v %v %v",
        &[
            nb("fmt_test.TF", "F: %f", Value::Float(1.0, FloatKind::F64)),
            nb(
                "fmt_test.TF32",
                "F32: %f",
                Value::Float(2.0, FloatKind::F32),
            ),
            nb(
                "fmt_test.TF64",
                "F64: %f",
                Value::Float(3.0, FloatKind::F64),
            ),
        ],
    );
    assert_eq!(s(&got), "F: 1.000000 F32: 2.000000 F64: 3.000000");
    let got = go_fmt::sprintf(
        "%v %v",
        &[
            nb("fmt_test.TB", "B: %t", Value::Bool(true)),
            nb("fmt_test.TS", "S: %q", st("x")),
        ],
    );
    assert_eq!(s(&got), "B: true S: \"x\"");
}

/// Named collection types with a String method and nil `*time.Location`
/// (Go outputs; see also fixtures/model_cases.txt).
#[test]
fn named_type_methods() {
    let pages = Value::list(
        SliceType::Named(Arc::from("page.Pages")),
        vec![Value::Invalid, Value::Invalid],
    );
    assert_eq!(
        s(&go_fmt::sprintf(
            "%v|%s|%d|%#v|%T",
            &[
                pages.clone(),
                pages.clone(),
                pages.clone(),
                pages.clone(),
                pages.clone()
            ]
        )),
        "Pages(2)|Pages(2)|[<nil> <nil>]|page.Pages{page.Page(nil), page.Page(nil)}|page.Pages"
    );
    assert_eq!(
        s(&go_fmt::sprint(&[Value::any_list(vec![pages])])),
        "[Pages(2)]"
    );
    let loc = Value::TypedNil(Arc::from("*time.Location"));
    assert_eq!(
        s(&go_fmt::sprintf(
            "%v|%s|%d|%#v",
            &[loc.clone(), loc.clone(), loc.clone(), loc]
        )),
        "UTC|UTC|0|(*time.Location)(nil)"
    );
    // time.Time's loc field is unexported: no String call there.
    let t = Value::Time(go_value::Time::from_unix(0, 0, None));
    assert_eq!(s(&go_fmt::sprintf("%d", &[t])), "{0 62135596800 0}");
    // Registered by a host crate: an error type and a nil-panicking pointer.
    fn my_err(_: &Value) -> Option<Vec<u8>> {
        Some(b"my error".to_vec())
    }
    fn panics(_: &Value) -> Option<Vec<u8>> {
        None
    }
    go_fmt::register_named_method("test.ErrList", go_fmt::NamedMethod::Error(my_err));
    go_fmt::register_named_method("*test.File", go_fmt::NamedMethod::String(panics));
    let el = Value::list(SliceType::Named(Arc::from("test.ErrList")), vec![]);
    let (msg, wrapped) = go_fmt::errorf("e: %w", std::slice::from_ref(&el));
    assert_eq!((s(&msg).as_str(), wrapped), ("e: my error", vec![0]));
    let f = Value::TypedNil(Arc::from("*test.File"));
    assert_eq!(
        s(&go_fmt::sprintf("%s|%8v|%d", &[f.clone(), f.clone(), f])),
        "<nil>|<nil>|0"
    );
}
