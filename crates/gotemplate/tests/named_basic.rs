//! Named basic types (Go `type HTML string`, `type Month int`) are hosted as
//! objects whose `Object::underlying` is the basic value. Go's reflection
//! switches on their Kind, so `if`/`with`/`not`/`and`, `len` and the
//! builtin comparisons must treat them as the underlying string or int.
//!
//! Expected values: Go 1.27.1 `text/template` with `type HTML string` and
//! `type Month int` as data, with the fork's (go1.24.0) comparison error
//! text, which has no type names.

use std::any::Any;
use std::borrow::Cow;

use go_value::{HostCtx, Kind, Object, Result, Value};
use gotemplate::text::Template;

struct Named {
    type_name: &'static str,
    v: Value,
}

impl Object for Named {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.type_name)
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
    fn underlying(&self) -> Option<Value> {
        Some(self.v.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn named(type_name: &'static str, v: Value) -> Value {
    Value::object(Named { type_name, v })
}

fn run(src: &str, data: &Value) -> String {
    let t = Template::new("t");
    t.parse(src).unwrap();
    let mut out = Vec::new();
    match t.execute(&mut out, data) {
        Ok(()) => String::from_utf8_lossy(&out).into_owned(),
        Err(e) => format!("ERR: {e}"),
    }
}

const SRCS: [&str; 10] = [
    "{{if .}}y{{else}}n{{end}}",
    "{{len .}}",
    "{{eq . \"x\"}}",
    "{{eq . \"\" \"x\"}}",
    "{{lt . \"y\"}}",
    "{{ne . \"x\"}}",
    "{{eq . 1}}",
    "{{and . 1}}",
    "{{not .}}",
    "{{with .}}w{{end}}",
];

fn check(data: &Value, want: [&str; 10]) {
    for (src, want) in SRCS.iter().zip(want) {
        let got = run(src, data);
        if let Some(msg) = want.strip_prefix("ERR:") {
            assert!(
                got.starts_with("ERR: ") && got.ends_with(msg),
                "{src}: got {got:?}, want error ending in {msg:?}"
            );
        } else {
            assert_eq!(got, want, "{src}");
        }
    }
}

#[test]
fn named_string() {
    const BAD: &str = "ERR:error calling eq: incompatible types for comparison";
    check(
        &named("main.HTML", Value::string("")),
        [
            "n", "0", "false", "true", "true", "true", BAD, "", "true", "",
        ],
    );
    check(
        &named("main.HTML", Value::string("x")),
        [
            "y", "1", "true", "true", "true", "false", BAD, "1", "false", "w",
        ],
    );
}

#[test]
fn named_int() {
    const LEN: &str = "ERR:error calling len: len of type main.Month";
    const EQ: &str = "ERR:error calling eq: incompatible types for comparison";
    const LT: &str = "ERR:error calling lt: incompatible types for comparison";
    const NE: &str = "ERR:error calling ne: incompatible types for comparison";
    check(
        &named("main.Month", Value::int(0)),
        ["n", LEN, EQ, EQ, LT, NE, "false", "0", "true", ""],
    );
    check(
        &named("main.Month", Value::int(3)),
        ["y", LEN, EQ, EQ, LT, NE, "false", "1", "false", "w"],
    );
}
