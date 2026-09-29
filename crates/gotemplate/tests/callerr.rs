//! Which call errors get the `error calling X: ` prefix, against
//! `tools/go-oracle/gotemplate/callerr` (fixture `callerr.tsv`, Go 1.27.1 `text/template`).
//!
//! The host methods and functions receive every argument as `any` (contract C3) and make
//! Go's pre-call checks themselves, marking those errors with `go_value::Error::eval_call`:
//! the argument count (at the call) and the argument type (at the argument). The engine
//! reports them like Go's `evalCall`, without the prefix; an error the callee returns keeps
//! it, even when its text looks like an argument-count error.

use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use go_value::{EvalCallError, HostCtx, IntKind, Kind, Object, Result, Value};
use gotemplate::text::{Func, Template};

fn arity(a: &[Value], n: usize, name: &str) -> Result<()> {
    if a.len() != n {
        return Err(go_value::Error::eval_call(
            format!("wrong number of args for {name}: want {n} got {}", a.len()),
            EvalCallError::Call,
        ));
    }
    Ok(())
}

fn at_least(a: &[Value], n: usize, name: &str) -> Result<()> {
    if a.len() < n {
        return Err(go_value::Error::eval_call(
            format!(
                "wrong number of args for {name}: want at least {n} got {}",
                a.len()
            ),
            EvalCallError::Call,
        ));
    }
    Ok(())
}

fn string(a: &[Value], i: usize) -> Result<String> {
    match &a[i] {
        Value::String(s) => Ok(s.to_str_lossy().into_owned()),
        v => Err(go_value::Error::eval_call(
            format!(
                "wrong type for value; expected string; got {}",
                v.go_type_name()
            ),
            EvalCallError::Arg(i),
        )),
    }
}

fn int(a: &[Value], i: usize) -> Result<i64> {
    match &a[i] {
        Value::Int(n, IntKind::Int) => Ok(*n),
        v => Err(go_value::Error::eval_call(
            format!(
                "wrong type for value; expected int; got {}",
                v.go_type_name()
            ),
            EvalCallError::Arg(i),
        )),
    }
}

/// Go: the oracle's `T`.
struct T;

impl Object for T {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.T")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(
            name,
            "M0" | "M1" | "M2" | "V" | "Err" | "ErrLike" | "Bad" | "Arg1Err"
        )
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, a: &[Value]) -> Option<Result<Value>> {
        Some((|| match name {
            "M0" => arity(a, 0, name).map(|_| Value::string("m0")),
            "M1" => {
                arity(a, 1, name)?;
                Ok(Value::string(format!("m1:{}", string(a, 0)?)))
            }
            "M2" => {
                arity(a, 2, name)?;
                Ok(Value::int(int(a, 0)? + int(a, 1)?))
            }
            "V" => {
                at_least(a, 1, name)?;
                for i in 0..a.len() {
                    string(a, i)?;
                }
                Ok(Value::int(a.len() as i64 - 1))
            }
            "Err" => {
                arity(a, 0, name)?;
                Err(go_value::Error::new("boom"))
            }
            "ErrLike" => {
                arity(a, 0, name)?;
                Err(go_value::Error::new(
                    "wrong number of args for ErrLike: want 9 got 9",
                ))
            }
            "Bad" => {
                arity(a, 0, name)?;
                Err(go_value::Error::eval_call(
                    "can't call method/function \"Bad\" with 2 results",
                    EvalCallError::Call,
                ))
            }
            "Arg1Err" => {
                arity(a, 1, name)?;
                Err(go_value::Error::new(format!("bad {}", string(a, 0)?)))
            }
            _ => unreachable!(),
        })())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn funcs() -> HashMap<String, Func> {
    let mut m: HashMap<String, Func> = HashMap::new();
    m.insert(
        "f0".into(),
        Arc::new(|_c: HostCtx<'_>, a: &[Value]| {
            arity(a, 0, "f0")?;
            Ok(Value::string("f0"))
        }),
    );
    m.insert(
        "f1".into(),
        Arc::new(|_c: HostCtx<'_>, a: &[Value]| {
            arity(a, 1, "f1")?;
            Ok(Value::string(string(a, 0)?))
        }),
    );
    m.insert(
        "fv".into(),
        Arc::new(|_c: HostCtx<'_>, a: &[Value]| {
            at_least(a, 1, "fv")?;
            for i in 0..a.len() {
                string(a, i)?;
            }
            Ok(Value::int(a.len() as i64 - 1))
        }),
    );
    m.insert(
        "ferr".into(),
        Arc::new(|_c: HostCtx<'_>, a: &[Value]| {
            arity(a, 0, "ferr")?;
            Err(go_value::Error::new("ferr failed"))
        }),
    );
    m
}

#[test]
fn call_errors_match_go() {
    let fx = include_str!("fixtures/callerr.tsv");
    let data = Value::object(T);
    let mut bad = Vec::new();
    let mut n = 0;
    for line in fx.lines() {
        let mut parts = line.split('\t');
        let (src, out, err) = (
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
        );
        n += 1;
        let t = Template::new("t");
        t.funcs(&funcs());
        t.parse(src).unwrap();
        let mut w = Vec::new();
        let got_err = match t.execute(&mut w, &data) {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
        let got_out = String::from_utf8_lossy(&w).into_owned();
        if got_out != out || got_err != err {
            bad.push(format!(
                "{src}\n  got  {got_out:?} {got_err:?}\n  want {out:?} {err:?}"
            ));
        }
    }
    assert!(n > 30);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
