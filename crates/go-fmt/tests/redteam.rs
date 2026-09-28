//! Regression tests of the third (red-team) pass: the vectors of
//! `tools/go-oracle/go-fmt/redteam.go` checked in by `oracle -mode vectors`
//! (see PORTING.md, "Red-team pass (third)"), plus the two bugs it fixed:
//! deep values overflowed the Rust stack, and named basic types
//! (`time.Month`, `hstring.HTML`, ... via `Object::underlying`) were
//! printed as empty structs.

mod common;

use std::sync::Arc;

use common::*;
use go_value::{HostCtx, IntKind, Kind, Object, Result, SliceType, Value};

fn formats() -> Vec<Vec<u8>> {
    read_fixture("formats.txt").lines().map(unquote).collect()
}

/// Random operands nested up to 6 levels (nil-receiver hooks, named
/// slice/map types with String/Error methods, named basic types, times in
/// fixed zones with arbitrary names) with formats of 1-6 pieces: flags in
/// any order and repeated, zero-padded widths, star and indexed star
/// widths/precisions, malformed indexes, parsenum overflow, truncated UTF-8
/// verbs; Sprint/Sprintln operand lists; Errorf with %w.
#[test]
fn redteam_cases() {
    let (n, failures) = run_cases(&read_fixture("redteam_cases.txt"));
    assert!(n > 2500, "too few red-team cases");
    eprintln!("redteam cases: {n}, {} differ", failures.len());
    assert_no_failures("redteam cases", &failures);
}

/// formats.txt (every verb x flag set x width x precision) over random
/// nested operands.
#[test]
fn redteam_matrix() {
    let (hashed, failures) = check_matrix(&formats(), &read_fixture("redteam_matrix.txt"));
    eprintln!("redteam matrix: {hashed} outputs compared");
    assert!(hashed > 1_000_000, "matrix too small");
    assert!(
        failures.is_empty(),
        "{} operands differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Go: redteam.go `flagPermFormats(maxLen)`: every flag string of length
/// 0..=maxLen over "+-# 0" (every order, repeats) x 25 verbs x widths
/// {"", 7} x precisions {"", .3}.
fn flag_perm_formats(max_len: usize) -> Vec<Vec<u8>> {
    let mut flags: Vec<String> = vec![String::new()];
    let mut level: Vec<String> = vec![String::new()];
    for _ in 0..max_len {
        let next: Vec<String> = level
            .iter()
            .flat_map(|p| "+-# 0".chars().map(move |c| format!("{p}{c}")))
            .collect();
        flags.extend(next.iter().cloned());
        level = next;
    }
    let verbs = [
        "v", "d", "s", "q", "x", "X", "t", "b", "o", "O", "c", "U", "e", "E", "f", "F", "g", "G",
        "T", "p", "%", "w", "z", "!", "\u{e9}",
    ];
    let mut fs = Vec::new();
    for v in verbs {
        for fl in &flags {
            for wd in ["", "7"] {
                for p in ["", ".3"] {
                    fs.push(format!("%{fl}{wd}{p}{v}").into_bytes());
                }
            }
        }
    }
    fs
}

/// Flags in every order and repeated, over values.txt plus the red-team
/// hook, named-type and named-basic-type operands.
#[test]
fn flag_permutations() {
    let formats = flag_perm_formats(3);
    assert_eq!(formats.len(), 15_600);
    let (hashed, failures) = check_matrix(&formats, &read_fixture("flagperm_matrix.txt"));
    eprintln!("flag permutations: {hashed} outputs compared");
    assert!(hashed > 4_000_000, "matrix too small");
    assert!(
        failures.is_empty(),
        "{} operands differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Bug 1: values nested 10000-100000 levels deep (Go prints them; its
/// goroutine stacks grow) overflowed the Rust stack. `printValue`'s
/// recursion now runs under `stack::guard`. Checked on a 2 MiB thread
/// against Go's output length and FNV-1a hash (deep.txt).
#[test]
fn deep_nesting_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let fixture = read_fixture("deep.txt");
            let mut failures = Vec::new();
            let mut current: Option<(String, Value)> = None;
            for line in fixture.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                assert_eq!(parts.len(), 4, "bad deep line {line:?}");
                if current.as_ref().is_none_or(|(s, _)| s != parts[0]) {
                    // Dropping a 100000-deep value recurses in drop glue:
                    // leak it instead.
                    if let Some((_, v)) = current.take() {
                        std::mem::forget(v);
                    }
                    current = Some((parts[0].to_string(), spec_value(parts[0])));
                }
                let v = &current.as_ref().unwrap().1;
                let out = go_fmt::sprintf(unquote(parts[1]), std::slice::from_ref(v));
                let mut h = Fnv64::new();
                h.write(&out);
                let want_len: usize = parts[2].parse().unwrap();
                let want = u64::from_str_radix(parts[3], 16).unwrap();
                if out.len() != want_len || h.0 != want {
                    failures.push(format!(
                        "{} {}: len {} want {want_len}",
                        parts[0],
                        parts[1],
                        out.len()
                    ));
                }
            }
            if let Some((_, v)) = current.take() {
                std::mem::forget(v);
            }
            assert!(
                failures.is_empty(),
                "{} differ:\n{}",
                failures.len(),
                failures.join("\n")
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

/// A deep value built from host objects whose methods are consulted on
/// the helper threads.
#[test]
fn deep_stringers_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let mut v = Value::object(NamedBasic {
                ty: "time.Month".to_string(),
                under: Value::Int(9, IntKind::Int),
            });
            for _ in 0..50_000 {
                v = Value::any_list(vec![v]);
            }
            let out = go_fmt::sprintf("%v|%d", &[v.clone(), v.clone()]);
            let mut want = "[".repeat(50_000);
            want.push_str("September");
            want.push_str(&"]".repeat(50_000));
            want.push('|');
            want.push_str(&"[".repeat(50_000));
            want.push('9');
            want.push_str(&"]".repeat(50_000));
            assert!(out == want.as_bytes(), "deep Stringer list differs");
            std::mem::forget(v);
        })
        .unwrap()
        .join()
        .unwrap();
}

fn month(m: i64) -> Value {
    Value::object(NamedBasic {
        ty: "time.Month".to_string(),
        under: Value::Int(m, IntKind::Int),
    })
}

fn named(ty: &str, under: Value) -> Value {
    Value::object(NamedBasic {
        ty: ty.to_string(),
        under,
    })
}

/// `main.NErr`: a named string type with an Error method.
struct NErr(&'static str);

impl Object for NErr {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        "main.NErr".into()
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::string(self.0))
    }
    fn go_error(&self) -> Option<String> {
        Some(format!("NE:{}", self.0))
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<Result<Value>> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Bug 2: a named basic type (Go `type Month int`, `type HTML string`)
/// was a struct object without fields: `%02d` of a `time.Month` printed
/// `{}`, `Sprint` put spaces around a named string, `*` rejected a named
/// integer. `Object::underlying` now gives fmt its reflect kind. The
/// expected strings are Go 1.27.1 output.
#[test]
fn named_basic_types() {
    let s =
        |format: &str, args: &[Value]| String::from_utf8(go_fmt::sprintf(format, args)).unwrap();
    let dur = |d: i64| named("time.Duration", Value::Int(d, IntKind::Int64));
    let hstr = |x: &str| named("main.HStr", Value::string(x));
    let nstr = |x: &str| named("main.NStr", Value::string(x));
    assert_eq!(s("%02d", &[month(9)]), "09");
    assert_eq!(s("%v", &[month(9)]), "September");
    assert_eq!(s("%x", &[month(9)]), "53657074656d626572");
    assert_eq!(s("%#v", &[month(9)]), "9");
    assert_eq!(s("%t", &[month(9)]), "%!t(time.Month=9)");
    assert_eq!(s("%p", &[month(9)]), "%!p(time.Month=9)");
    assert_eq!(s("%T", &[month(9)]), "time.Month");
    assert_eq!(
        s("%d", &[named("time.Weekday", Value::Int(3, IntKind::Int))]),
        "3"
    );
    assert_eq!(s("%v", &[dur(90_000_000_000)]), "1m30s");
    assert_eq!(s("%d", &[dur(90_000_000_000)]), "90000000000");
    assert_eq!(
        s("%.1f", &[dur(1_500_000_000)]),
        "%!f(time.Duration=1500000000)"
    );
    assert_eq!(s("%*d", &[month(5), Value::int(3)]), "    3");
    let months = Value::list(
        SliceType::Named(Arc::from("[]time.Month")),
        vec![month(1), month(12)],
    );
    assert_eq!(s("%d", std::slice::from_ref(&months)), "[1 12]");
    assert_eq!(s("%v", std::slice::from_ref(&months)), "[January December]");
    assert_eq!(
        s("%#v", std::slice::from_ref(&months)),
        "[]time.Month{1, 12}"
    );
    let sprint = |args: &[Value]| String::from_utf8(go_fmt::sprint(args)).unwrap();
    assert_eq!(
        sprint(&[
            hstr("a"),
            Value::int(1),
            hstr("b"),
            Value::int(2),
            Value::int(3)
        ]),
        "a1b2 3"
    );
    assert_eq!(
        sprint(&[nstr("a"), Value::int(1), Value::Bool(true)]),
        "a1 true"
    );
    assert_eq!(s("%d", &[hstr("x")]), "%!d(main.HStr=x)");
    assert_eq!(s("%#v", &[hstr("x")]), "\"x\"");
    assert_eq!(s("%5.1s", &[nstr("xyz")]), "    x");
    assert_eq!(s("%x", &[nstr("hi")]), "6869");
    assert_eq!(s("x", &[month(2)]), "x%!(EXTRA time.Month=February)");
    let nerr = Value::object(NErr("e"));
    assert_eq!(s("%w", std::slice::from_ref(&nerr)), "%!w(main.NErr=e)");
    assert_eq!(s("%d", std::slice::from_ref(&nerr)), "%!d(main.NErr=e)");
    let (msg, wrapped) = go_fmt::errorf("%w", std::slice::from_ref(&nerr));
    assert_eq!((msg, wrapped), (b"NE:e".to_vec(), vec![0]));
}
