//! Module `tplapi::values`.
//!
//! NEW: helpers shared by the template method tables of `tplapi` (argument counts as Go's
//! `evalCall` reports them, Go's nil results as template values).
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go's text/template (`evalCall`, Hugo's fork) counts the injected `context.Context` as an
//! argument: `{{ .Content 1 }}` fails with `wrong number of args for Content: want 1 got 2`.
//! A method whose result count Go rejects (`goodFunc`: one result, or two with an `error`) is
//! still in the method set (it shadows fields and map keys) but calling it is an error, checked
//! after the argument count.

use std::sync::Arc;

use go_value::{SliceType, Value};
use nh_common::object::GoResult;
use nh_page::page::{PageRef, Pages, pages_to_value};

/// A Go-arity check for a method of `num_in` parameters (the `context.Context` included when
/// `ctx` is set; templates never pass it).
pub fn arity(args: &[Value], num_in: usize, ctx: bool, name: &str) -> GoResult<()> {
    let got = args.len() + usize::from(ctx);
    if got != num_in {
        return Err(go_value::Error::new(format!(
            "wrong number of args for {name}: want {num_in} got {got}"
        )));
    }
    Ok(())
}

/// The check for a variadic method of `num_in` parameters (the last one variadic, the context
/// included when `ctx` is set). Go's message reports the arguments without the context.
pub fn arity_variadic(args: &[Value], num_in: usize, ctx: bool, name: &str) -> GoResult<()> {
    let got = args.len() + usize::from(ctx);
    if got < num_in - 1 {
        return Err(go_value::Error::new(format!(
            "wrong number of args for {name}: want at least {} got {}",
            num_in - 1,
            args.len()
        )));
    }
    Ok(())
}

/// A method with a result count templates cannot call (Go's `goodFunc`): the argument count is
/// checked first, then the call fails.
pub fn bad_results(
    args: &[Value],
    num_in: usize,
    variadic: bool,
    ctx: bool,
    name: &str,
    num_out: usize,
) -> GoResult<Value> {
    if variadic {
        arity_variadic(args, num_in, ctx, name)?;
    } else {
        arity(args, num_in, ctx, name)?;
    }
    Err(nh_common::object::bad_results_error(name, num_out))
}

/// The explicit error of a method the port does not support (README rule 5).
pub fn unsupported(type_name: &str, name: &str) -> go_value::Error {
    go_value::Error::new(format!("neohugo-rs: ({type_name}).{name} is not supported"))
}

/// Go's nil of a named type (`page.Page`, `page.Pages`, `[]string`, ...).
pub fn nil_of(t: &str) -> Value {
    Value::TypedNil(Arc::from(t))
}

/// A `page.Page` result: the page, or Go's nil interface.
pub fn page_or_nil(p: Option<PageRef>) -> Value {
    match p {
        Some(p) => p.to_value(),
        None => nil_of("page.Page"),
    }
}

/// A `page.Pages` result with Go's nil (`None`).
pub fn pages_opt_value(p: Option<Pages>) -> Value {
    match p {
        Some(p) => pages_to_value(&p),
        None => nil_of("page.Pages"),
    }
}

/// A `page.Pages` result built by appending to a nil slice (nil when empty).
pub fn pages_nil_if_empty(p: Pages) -> Value {
    if p.is_empty() {
        nil_of("page.Pages")
    } else {
        pages_to_value(&p)
    }
}

/// A `[]string` result (`None` is Go's nil).
pub fn strings_opt_value(v: Option<Vec<String>>) -> Value {
    match v {
        Some(v) => Value::list(
            SliceType::String,
            v.into_iter().map(|s| Value::string(s.as_str())).collect(),
        ),
        None => nil_of("[]string"),
    }
}

/// A `[]string` result from a Rust list that cannot tell nil from empty: nil when empty (the
/// front matter lists are nil when not set).
pub fn strings_nil_if_empty(v: &[String]) -> Value {
    if v.is_empty() {
        nil_of("[]string")
    } else {
        strings_opt_value(Some(v.to_vec()))
    }
}

/// An `error`-returning result into the template result (Go `(v, err)`).
pub fn res<T>(r: nh_common::Result<T>, f: impl FnOnce(T) -> Value) -> GoResult<Value> {
    r.map(f).map_err(go_value::Error::from)
}
