//! Go's `texttemplate/exec_test.go` tables (execTests, cmpTests,
//! TestInterfaceValues, TestEvalFieldErrors, TestExecutePanicDuringCall,
//! TestFunctionCheckDuringCall, TestDelims, TestExecError,
//! TestExecuteError, TestTree, TestIssue31810, TestFinalForPrintf,
//! TestExecuteGivesExecError), run through the Rust engine and compared
//! with the forked Go engine's results recorded in
//! `tests/fixtures/text/exectests.txt` (the Go data is mirrored by
//! `common_text/gotests.rs`).
//!
//! The fork's results equal the expectations of Go's tables (the `expect`
//! lines); the oracle records the fork's actual output, which is what the
//! Rust engine must reproduce.
//!
//! Regenerate:
//!
//! ```text
//! tools/go-oracle/gotemplate/sync-fork.sh
//! GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exectests crates/gotemplate/tests/fixtures/text
//! ```

mod common_text;

use std::collections::BTreeMap;
use std::sync::Arc;

use common_text::gotests::{gt_data, gt_funcs, named_map, panic_funcs, v};
use common_text::model::{ReflectHelper, int};
use common_text::*;
use go_value::{HostCtx, IntKind, SliceType, Value};
use gotemplate::Error;
use gotemplate::text::{Func, FuncMap, Template, js_escape_string};

struct Case<'a> {
    index: &'a str,
    group: String,
    name: String,
    data: String,
    left: String,
    right: String,
    exec: String,
    src: Vec<u8>,
    /// The fork's result lines (`perr`, or `out` and `err`).
    want: Vec<&'a str>,
}

fn parse_case<'a>(rec: &Record<'a>) -> Case<'a> {
    let mut c = Case {
        index: rec.header[0],
        group: unquote_str(rec.header[1]),
        name: unquote_str(rec.header[2]),
        data: String::new(),
        left: String::new(),
        right: String::new(),
        exec: String::new(),
        src: Vec::new(),
        want: Vec::new(),
    };
    for line in &rec.lines {
        let (key, rest) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "data" => c.data = unquote_str(rest),
            "opts" => {
                let f = fields(rest);
                c.left = unquote_str(f[0]);
                c.right = unquote_str(f[1]);
                c.exec = unquote_str(f[2]);
            }
            "src" => c.src = unquote(rest),
            "expect" => {}
            _ => c.want.push(line),
        }
    }
    c
}

/// Runs one case the way the oracle does; `Err` when its data is not
/// expressible in the value model.
fn run(c: &Case<'_>) -> Result<Vec<String>, &'static str> {
    let data = gt_data(&c.data)?;
    let t = Template::new(&c.name);
    match c.group.as_str() {
        "execTests" => {
            t.funcs(&gt_funcs());
        }
        "TestExecutePanicDuringCall" => {
            t.funcs(&panic_funcs());
        }
        _ => {}
    }
    t.delims(&c.left, &c.right);
    if let Err(e) = t.parse(&c.src) {
        return Ok(vec![format!("perr {}", q(e.to_string()))]);
    }
    let mut out = Vec::new();
    let res = if c.exec.is_empty() {
        t.execute_with_helper(&(), &ReflectHelper, &mut out, &data)
    } else {
        match t.lookup(&c.exec) {
            Some(tmpl) => tmpl.execute_with_helper(&(), &ReflectHelper, &mut out, &data),
            None => Err(Error::Other(format!(
                "template: no template {} associated with template {}",
                q(&c.exec),
                q(t.name())
            ))),
        }
    };
    let mut lines = vec![format!("out {}", q(&out))];
    if let Err(e) = res {
        lines.push(format!(
            "err {} {}",
            q(e.to_string()),
            matches!(e, Error::Exec(_))
        ));
    }
    Ok(lines)
}

/// Go test cases whose result the value model cannot reproduce, by
/// `group/name`, with the reason (see PORTING.md).
fn deviation(group: &str, name: &str, src: &str) -> Option<&'static str> {
    // Complex numbers are not in the value model: a complex constant is an
    // execution error (`complex constant 1i is not supported`), and T has
    // no ComplexZero field.
    if has_complex_literal(src) || src.contains(".ComplexZero") {
        return Some("complex");
    }
    Some(match (group, name) {
        // Objects expose exported fields only (Go: `unexported is an
        // unexported field of struct type *template.T`).
        ("execTests", ".unexported") => "unexported-field",
        // Value-model maps have string keys (Go: `map[int]int` rejects
        // `.one`; `index .MI32S 2` converts 2 to the int32 key).
        (
            "execTests",
            "map .WRONG type" | "map MI64S" | "map MI32S" | "map MUI64S" | "map MI8S" | "map MUI8S",
        ) => "non-string-map-keys",
        // Pointers to basic types are the pointed-to value (`PI *int`).
        ("execTests", ".NilOKFunc not nil") => "pointer-to-basic",
        // Slices have cap == len (`SICap: make([]int, 5, 10)`).
        ("execTests", "len(s) < indexes < cap(s)" | "indexes > cap(s)") => "slice-cap",
        // Channels are not in the value model (`count` returns a chan).
        ("execTests", "range count" | "range nil count") => "channels",
        // C7: functions receive their arguments as `any` and check their
        // own types, so Go's `wrong type for value; expected string; got
        // int` (reported at the argument) is the function's own error.
        ("execTests", "bug8a" | "bug8b" | "bug11" | "bug15" | "bug16f" | "bug16h") => {
            "host-typed-params"
        }
        // Go formats the copy `reflect.Value.Interface()` makes, which is
        // not addressable, so `V`'s pointer-receiver String is not used:
        // `template.V: {0}`; model objects decide their Stringer by their
        // own addressability (`<0>`, as `{{.V1}}` prints in Go).
        ("cmpTests", _) if src.contains("eq .Map .V1") => "addressable-copy-stringer",
        // A typed nil pointer does not know its struct's fields: `nil
        // pointer evaluating *template.T.MissingField` instead of `can't
        // evaluate field MissingField in type *template.T`.
        ("TestEvalFieldErrors", _) if src == "{{.MissingField}}" => "nil-pointer-unknown-field",
        _ => return None,
    })
}

/// Whether `src` has a complex constant such as `1i`, `1.5i` or `1+2i`.
fn has_complex_literal(src: &str) -> bool {
    let b = src.as_bytes();
    (1..b.len()).any(|i| {
        b[i] == b'i'
            && b[i - 1].is_ascii_digit()
            && b.get(i + 1).is_none_or(|c| !c.is_ascii_alphanumeric())
    })
}

#[test]
fn exec_go_tables() {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(exec_go_tables_body)
        .unwrap()
        .join()
        .unwrap();
}

fn exec_go_tables_body() {
    let text = read_fixture("exectests.txt");
    let recs = records(&text);
    assert!(recs.len() > 500, "fixture has {} cases", recs.len());
    let mut passed = 0;
    let mut addr = 0;
    let mut bad = 0;
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut deviations: BTreeMap<&'static str, usize> = BTreeMap::new();
    for rec in &recs {
        let c = parse_case(rec);
        if c.want == ["addr"] {
            addr += 1;
            continue;
        }
        let got = match run(&c) {
            Ok(got) => got,
            Err(why) => {
                *skipped.entry(why).or_default() += 1;
                continue;
            }
        };
        let Some(d) = first_diff(&c.want, &got) else {
            passed += 1;
            continue;
        };
        let src = String::from_utf8_lossy(&c.src);
        if let Some(why) = deviation(&c.group, &c.name, &src) {
            *deviations.entry(why).or_default() += 1;
            continue;
        }
        bad += 1;
        eprintln!(
            "case {} {}/{}: src {}\n  {d}",
            c.index,
            c.group,
            c.name,
            q(&c.src)
        );
    }
    eprintln!(
        "{} Go exec_test.go cases: {passed} match, {addr} skipped (Go pointer addresses), \
         unmodelled data {skipped:?}, known deviations {deviations:?}, {bad} differ",
        recs.len()
    );
    assert_eq!(
        bad,
        0,
        "{bad} of {} exec_test.go cases differ from Go",
        recs.len()
    );
}

// ---------------------------------------------------------------------------
// exec_test.go's tests that are not tables.

fn must_parse(name: &str, src: &str) -> Template {
    let t = Template::new(name);
    t.parse(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    t
}

fn exec_str(t: &Template, data: &Value) -> Result<String, Error> {
    let mut out = Vec::new();
    t.execute_with_helper(&(), &ReflectHelper, &mut out, data)?;
    Ok(String::from_utf8(out).unwrap())
}

fn func(f: impl Fn(&[Value]) -> go_value::Result<Value> + Send + Sync + 'static) -> Func {
    Arc::new(move |_ctx: HostCtx<'_>, args: &[Value]| f(args))
}

fn zero() -> Value {
    Value::Int(0, IntKind::Int)
}

/// Go: TestExecError_CustomError — the function's error is reachable
/// (`errors.As`): here `ExecError::cause`.
#[test]
fn exec_error_custom_error() {
    let t = Template::new("top");
    let mut fm = FuncMap::new();
    fm.insert(
        "err".into(),
        func(|_| Err(go_value::Error::new("custom error"))),
    );
    t.funcs(&fm);
    t.parse("{{ err }}").unwrap();
    let err = exec_str(&t, &Value::Invalid).unwrap_err();
    let Error::Exec(e) = &err else {
        panic!("not an ExecError: {err}")
    };
    assert_eq!(e.cause.as_ref().map(|c| c.message()), Some("custom error"));
    assert_eq!(
        err.to_string(),
        "template: top:1:3: executing \"top\" at <err>: error calling err: custom error"
    );
}

/// Go: TestJSEscaping.
#[test]
fn js_escaping() {
    for (input, want) in [
        ("a", "a"),
        ("'foo", "\\'foo"),
        ("Go \"jump\" \\", "Go \\\"jump\\\" \\\\"),
        (
            "Yukihiro says \"今日は世界\"",
            "Yukihiro says \\\"今日は世界\\\"",
        ),
        ("unprintable \u{FFFE}", "unprintable \\uFFFE"),
        ("<html>", "\\u003Chtml\\u003E"),
        ("no = in attributes", "no \\u003D in attributes"),
        (
            "&#x27; does not become HTML entity",
            "\\u0026#x27; does not become HTML entity",
        ),
    ] {
        let got = js_escape_string(input.as_bytes());
        assert_eq!(String::from_utf8_lossy(&got), want, "JS escaping {input:?}");
    }
}

/// Go: TestExecuteOnNewTemplate (issues 3872 and 11379): no panics.
#[test]
fn execute_on_new_template() {
    let _ = Template::new("Name").templates();
    Template::new("").parse("").unwrap();
    Template::new("").new_associated("abc").parse("").unwrap();
    assert!(
        Template::new("")
            .execute(&mut Vec::new(), &Value::Invalid)
            .is_err()
    );
    assert!(
        Template::new("")
            .execute_template(&mut Vec::new(), "XXX", &Value::Invalid)
            .is_err()
    );
}

/// Go: TestMessageForExecuteEmpty.
#[test]
fn message_for_execute_empty() {
    let tmpl = Template::new("empty");
    let want = "template: empty: \"empty\" is an incomplete or empty template";
    let err = tmpl.execute(&mut Vec::new(), &zero()).unwrap_err();
    assert_eq!(err.to_string(), want);
    // Add a non-empty template to check that the error is helpful.
    let tests = must_parse(
        "",
        r#"{{define "one"}}one{{end}}{{define "two"}}two{{end}}"#,
    );
    tmpl.add_parse_tree("secondary", tests.tree().expect("tree"))
        .unwrap();
    let err = tmpl.execute(&mut Vec::new(), &zero()).unwrap_err();
    assert_eq!(err.to_string(), want);
    // Make sure we can execute the secondary.
    tmpl.execute_template(&mut Vec::new(), "secondary", &zero())
        .unwrap();
}

/// Go: TestMissingMapKey.
#[test]
fn missing_map_key() {
    let data = named_map("map[string]int", &[("x", int(99))]);
    let tmpl = must_parse("t1", "{{.x}} {{.y}}");
    // By default, just get "<no value>".
    assert_eq!(exec_str(&tmpl, &data).unwrap(), "99 <no value>");
    // Same if we set the option explicitly to the default.
    tmpl.set_option(&["missingkey=default"]);
    assert_eq!(exec_str(&tmpl, &data).unwrap(), "99 <no value>");
    // Next we ask for a zero value.
    tmpl.set_option(&["missingkey=zero"]);
    assert_eq!(exec_str(&tmpl, &data).unwrap(), "99 0");
    // Now we ask for an error.
    tmpl.set_option(&["missingkey=error"]);
    assert_eq!(
        exec_str(&tmpl, &data).unwrap_err().to_string(),
        "template: t1:1:9: executing \"t1\" at <.y>: map has no entry for key \"y\""
    );
    // Same option, but now a nil interface: ask for an error.
    assert_eq!(
        exec_str(&tmpl, &Value::Invalid).unwrap_err().to_string(),
        "template: t1:1:2: executing \"t1\" at <.x>: nil data; no entry for key \"x\""
    );
}

/// Go: TestUnterminatedStringError — the error refers to the line of the
/// opening quote.
#[test]
fn unterminated_string_error() {
    let err = Template::new("X")
        .parse("hello\n\n{{`unterminated\n\n\n\n}}\n some more\n\n")
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("X:3: unterminated raw quoted string"),
        "unexpected error: {err}"
    );
}

/// A writer that always fails (Go: `ErrorWriter`).
struct ErrorWriter;

impl std::io::Write for ErrorWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("always be failing"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Go: TestExecuteGivesExecError.
#[test]
fn execute_gives_exec_error() {
    // First, a non-execution error shouldn't be an ExecError.
    let tmpl = must_parse("X", "hello");
    let err = tmpl.execute(&mut ErrorWriter, &zero()).unwrap_err();
    assert!(
        !matches!(err, Error::Exec(_)),
        "write error is an ExecError: {err:?}"
    );
    assert_eq!(err.to_string(), "always be failing");
    // This one should be an ExecError.
    let tmpl = must_parse("X", "hello, {{.X.Y}}");
    let err = tmpl.execute(&mut std::io::sink(), &zero()).unwrap_err();
    assert!(matches!(err, Error::Exec(_)), "not an ExecError: {err:?}");
    assert!(err.to_string().contains("field X in type int"), "{err}");
}

/// Go: TestGoodFuncNames and TestBadFuncNames (`Funcs` panics on a bad
/// name).
#[test]
fn func_names() {
    let f = func(|_| Ok(Value::Invalid));
    for name in ["_", "a", "a1", "a1", "Ӵ"] {
        let mut fm = FuncMap::new();
        fm.insert(name.into(), f.clone());
        Template::new("X").funcs(&fm);
    }
    for name in ["", "2", "a-b"] {
        let mut fm = FuncMap::new();
        fm.insert(name.into(), f.clone());
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Template::new("X").funcs(&fm);
        }));
        let Err(payload) = res else {
            panic!("{name:?} succeeded incorrectly as function name")
        };
        let msg = payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            msg,
            format!("function name {} is not a valid identifier", q(name))
        );
    }
}

/// Go: TestBlock.
#[test]
fn block() {
    let tmpl = must_parse("outer", r#"a({{block "inner" .}}bar({{.}})baz{{end}})b"#);
    let tmpl2 = tmpl.clone_ns().unwrap();
    tmpl2
        .parse(r#"{{define "inner"}}foo({{.}})bar{{end}}"#)
        .unwrap();
    assert_eq!(
        exec_str(&tmpl, &Value::string("hello")).unwrap(),
        "a(bar(hello)baz)b"
    );
    assert_eq!(
        exec_str(&tmpl2, &Value::string("goodbye")).unwrap(),
        "a(foo(goodbye)bar)b"
    );
}

/// Go: TestAddrOfIndex (issue 14916): slice elements are addressable, so
/// the pointer-receiver `String` of `V` is found by range and index.
#[test]
fn addr_of_index() {
    let data = Value::list(
        SliceType::Named(Arc::from("[]template.V")),
        vec![v(1, false, true)],
    );
    for text in [
        "{{range .}}{{.String}}{{end}}",
        "{{with index . 0}}{{.String}}{{end}}",
    ] {
        let tmpl = must_parse("tmpl", text);
        assert_eq!(exec_str(&tmpl, &data).unwrap(), "<1>", "{text}");
    }
}

/// Go: TestIssue39807 — AddParseTree and Execute from many threads.
#[test]
fn issue_39807() {
    let foo = must_parse("foo", r#"{{ template "bar" . }}"#);
    let bar = must_parse("bar", "bar");
    std::thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                for _ in 0..10 {
                    foo.add_parse_tree(bar.name(), bar.tree().unwrap()).unwrap();
                    let mut out = Vec::new();
                    foo.execute(&mut out, &Value::Invalid).unwrap();
                    assert_eq!(out, b"bar");
                }
            });
        }
    });
}
