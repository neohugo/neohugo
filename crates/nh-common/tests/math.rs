//! `common/math.DoArithmetic`: the Go test table (common/math/math_test.go) and the differential
//! test against `tools/go-oracle/nh-common/math` (every pair of operand kinds × every operator,
//! from an arm64 oracle build).

mod support;

use go_value::{FloatKind, IntKind, UintKind, Value};
use nh_common::math::do_arithmetic;
use serde_json::json;
use support::*;

fn i(n: i64) -> Value {
    Value::int(n)
}
fn u(n: u64) -> Value {
    Value::Uint(n, UintKind::Uint)
}
fn f(x: f64) -> Value {
    Value::float64(x)
}
fn s(x: &str) -> Value {
    Value::string(x)
}

// Go: common/math/math_test.go:TestDoArithmetic
#[test]
fn go_test_do_arithmetic() {
    let i64v = |n: i64| Value::Int(n, IntKind::Int64);
    let u64v = |n: u64| Value::Uint(n, UintKind::Uint64);
    let f64v = |x: f64| Value::Float(x, FloatKind::F64);
    let table: Vec<(Value, Value, char, Option<Value>)> = vec![
        (i(3), i(2), '+', Some(i64v(5))),
        (i(0), i(0), '+', Some(i64v(0))),
        (i(3), i(2), '-', Some(i64v(1))),
        (i(3), i(2), '*', Some(i64v(6))),
        (i(3), i(2), '/', Some(i64v(1))),
        (f(3.0), i(2), '+', Some(f64v(5.0))),
        (f(0.0), i(0), '+', Some(f64v(0.0))),
        (f(3.0), i(2), '-', Some(f64v(1.0))),
        (f(3.0), i(2), '*', Some(f64v(6.0))),
        (f(3.0), i(2), '/', Some(f64v(1.5))),
        (i(3), f(2.0), '+', Some(f64v(5.0))),
        (i(3), f(2.0), '-', Some(f64v(1.0))),
        (i(3), f(2.0), '*', Some(f64v(6.0))),
        (i(3), f(2.0), '/', Some(f64v(1.5))),
        (f(3.0), f(2.0), '+', Some(f64v(5.0))),
        (f(0.0), f(0.0), '+', Some(f64v(0.0))),
        (f(3.0), f(2.0), '-', Some(f64v(1.0))),
        (f(3.0), f(2.0), '*', Some(f64v(6.0))),
        (f(3.0), f(2.0), '/', Some(f64v(1.5))),
        (u(3), u(2), '+', Some(u64v(5))),
        (u(0), u(0), '+', Some(u64v(0))),
        (u(3), u(2), '-', Some(u64v(1))),
        (u(3), u(2), '*', Some(u64v(6))),
        (u(3), u(2), '/', Some(u64v(1))),
        (u(3), i(2), '+', Some(u64v(5))),
        (u(0), i(0), '+', Some(u64v(0))),
        (u(3), i(2), '-', Some(u64v(1))),
        (u(3), i(2), '*', Some(u64v(6))),
        (u(3), i(2), '/', Some(u64v(1))),
        (i(3), u(2), '+', Some(u64v(5))),
        (i(0), u(0), '+', Some(u64v(0))),
        (i(3), u(2), '-', Some(u64v(1))),
        (i(3), u(2), '*', Some(u64v(6))),
        (i(3), u(2), '/', Some(u64v(1))),
        (u(3), i(-2), '+', Some(i64v(1))),
        (u(3), i(-2), '-', Some(i64v(5))),
        (u(3), i(-2), '*', Some(i64v(-6))),
        (u(3), i(-2), '/', Some(i64v(-1))),
        (i(-3), u(2), '+', Some(i64v(-1))),
        (i(-3), u(2), '-', Some(i64v(-5))),
        (i(-3), u(2), '*', Some(i64v(-6))),
        (i(-3), u(2), '/', Some(i64v(-1))),
        (u(3), f(2.0), '+', Some(f64v(5.0))),
        (u(0), f(0.0), '+', Some(f64v(0.0))),
        (u(3), f(2.0), '-', Some(f64v(1.0))),
        (u(3), f(2.0), '*', Some(f64v(6.0))),
        (u(3), f(2.0), '/', Some(f64v(1.5))),
        (f(3.0), u(2), '+', Some(f64v(5.0))),
        (f(0.0), u(0), '+', Some(f64v(0.0))),
        (f(3.0), u(2), '-', Some(f64v(1.0))),
        (f(3.0), u(2), '*', Some(f64v(6.0))),
        (f(3.0), u(2), '/', Some(f64v(1.5))),
        (s("foo"), s("bar"), '+', Some(s("foobar"))),
        (i(3), i(0), '/', None),
        (f(3.0), i(0), '/', None),
        (i(3), f(0.0), '/', None),
        (u(3), u(0), '/', None),
        (i(3), u(0), '/', None),
        (i(-3), u(0), '/', None),
        (u(3), i(0), '/', None),
        (f(3.0), u(0), '/', None),
        (u(3), f(0.0), '/', None),
        (i(3), s("foo"), '+', None),
        (f(3.0), s("foo"), '+', None),
        (u(3), s("foo"), '+', None),
        (s("foo"), i(3), '+', None),
        (s("foo"), s("bar"), '-', None),
        (i(3), i(2), '%', None),
    ];
    for (a, b, op, expect) in table {
        let got = do_arithmetic(&a, &b, op);
        match expect {
            None => assert!(got.is_err(), "{a:?} {op} {b:?}: {got:?}"),
            Some(e) => assert_eq!(got.unwrap(), e, "{a:?} {op} {b:?}"),
        }
    }
}

#[test]
fn oracle() {
    init();
    let fx = fixture("math/math.json.gz");
    assert_eq!(
        fx["goarch"], "arm64",
        "the fixture must come from an arm64 oracle build"
    );
    let operands: Vec<Value> = fx["operands"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode)
        .collect();
    let ops: Vec<char> = fx["ops"].as_str().unwrap().chars().collect();
    let mut n = 0;
    let mut bad = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let a = &operands[c["a"].as_u64().unwrap() as usize];
        let b = &operands[c["b"].as_u64().unwrap() as usize];
        for (op, want) in ops.iter().zip(c["r"].as_array().unwrap()) {
            n += 1;
            let got = match do_arithmetic(a, b, *op) {
                Ok(v) => json!({"ok": encode(&v)}),
                Err(e) => json!({"err": e.message()}),
            };
            if !same_result(want, &got) {
                bad.push(format!("{a:?} {op} {b:?}: want {want} got {got}"));
            }
        }
    }
    eprintln!("math: {n} checks, {} mismatches", bad.len());
    for b in bad.iter().take(20) {
        eprintln!("{b}");
    }
    assert!(n > 15000, "too few cases: {n}");
    assert!(bad.is_empty());
}
