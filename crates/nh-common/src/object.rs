//! Module `object`.
//!
//! NEW: go_methods! macro, argument helpers, NamedTypeRegistry (replaces reflect method lookup)
//!
//! Owner: Wave B task T01 (common-values).

//!
//! Go templates reach Hugo objects through reflection (`reflect.Value.MethodByName`, struct fields,
//! map keys). The Rust port replaces that with explicit method tables:
//!
//! * Host types that Go passes as pointers/structs (Page, Site, Resource, Pager, Scratch, Language,
//!   OutputFormat, namespaces, hook contexts, ...) implement [`go_value::Object`]. Their exported
//!   method set is declared once with [`go_methods!`](crate::go_methods) and wired into the `Object`
//!   impl with [`object_basics!`](crate::object_basics).
//! * Methods declared on *named slice/map types* that are represented as `Value::List` /
//!   `Value::Map` (e.g. `page.Pages.Reverse`, `resource.Resources.Get`, `maps.Params.IsZero`,
//!   `page.Taxonomy.Alphabetical`) are registered in a [`NamedTypeRegistry`] keyed by the Go type
//!   string stored in [`go_value::SliceType::Named`] / [`go_value::MapType::Named`]
//!   (`MapType::Params` is `"maps.Params"`). The template exec helper (nh-tplimpl
//!   `template_funcs`) consults it (Go: `hreflect.GetMethodByName` on the named type).
//! * Go interface assertions on host values (`compare.Eqer`, `types.Zeroer`, `fmt.Stringer`,
//!   `resource.Resource`, ...) become either an `Object` hook (`is_zero`, `go_string`, ...) or a
//!   `has_method("Eq")` + `call_method` pair: this mirrors Go, where the interface check *is* a
//!   method-set check. Interfaces that Go code asserts on values of *other* crates' types are
//!   declared with [`crate::hreflect::register_interface`].
//! * Named basic types (`hstring.HTML`, `maps.ParamsMergeStrategy`, `time.Month`, ...) are objects
//!   that implement [`go_value::Object::underlying`], so `fmt`, `cast`, truthiness and hashing see
//!   their `reflect.Kind`.
//!
//! Method arguments arrive exactly as the template engine evaluated them (Go `evalArg` for
//! `any`/`...any` parameters: ideal constants become `int`/`float64`, contract C3/C7). Methods with
//! typed Go parameters convert with the [`args`] helpers, which apply Go's `validateType`
//! (assignability) with Go's error texts. The engine cannot tell an ideal constant from a variable,
//! so the helpers for `int64`, `float64` and named string parameters also accept what a literal
//! evaluates to (`int`, `string`); see each helper.
//!
//! # Stability
//!
//! Every public item of this module is a contract for all nh-* crates (HUGO_LAYER.md §1 rule 3):
//! `GoResult`, `go_methods!`, `object_basics!`, `args::*`, `bad_results_error`, `NamedMethods`,
//! `NamedTypeRegistry`, `ToValue`, `string_slice`. The reflect-type registries that go with it
//! (`register_named_elem`, `register_interface`) are in [`crate::hreflect`].

use std::borrow::Cow;
use std::collections::HashMap;

pub use go_value::{HostCtx, Object, Value};

/// Result type of template-visible methods and functions.
pub type GoResult<T> = go_value::Result<T>;

/// Declares the exported Go method set of a host type.
///
/// ```ignore
/// nh_common::go_methods!(Scratch {
///     "Get" => |s, _ctx, a| Ok(s.get(&args::string(a, 0)?)),
///     "Set" => |s, _ctx, a| Ok(Value::String(s.set(&args::string(a, 0)?, args::get(a, 1)?))),
/// });
/// nh_common::go_methods!(SitemapConfig {}); // a type without exported methods
/// ```
///
/// Generates, in an inherent `impl`:
/// - `GO_METHODS: &[&str]` — the exported method names (exact case, table order);
/// - `go_has_method(name) -> bool` — Go `reflect.Type.MethodByName(name)` membership;
/// - `go_call_method(&self, ctx, name, args) -> Option<GoResult<Value>>` — `None` when the type
///   has no such method (the engine then tries fields and map keys, Go `evalField`).
///
/// Each body is a non-capturing closure `fn(&Self, HostCtx, &[Value]) -> GoResult<Value>`. Check
/// the argument count with [`args::exactly`]/[`args::at_least`] and convert typed parameters with
/// the other [`args`] helpers. Methods that take `context.Context` first in Go receive the `ctx`;
/// templates call them without that argument.
#[macro_export]
macro_rules! go_methods {
    ($ty:ty {}) => {
        impl $ty {
            /// Exported Go method names of this type (none).
            pub const GO_METHODS: &'static [&'static str] = &[];

            /// Go: `reflect.Type.MethodByName(name)` for this type (no methods).
            pub fn go_has_method(_name: &str) -> bool {
                false
            }

            /// This type has no exported methods: always `None`.
            pub fn go_call_method(
                &self,
                _ctx: $crate::object::HostCtx<'_>,
                _name: &str,
                _args: &[$crate::object::Value],
            ) -> ::core::option::Option<$crate::object::GoResult<$crate::object::Value>> {
                ::core::option::Option::None
            }
        }
    };
    ($ty:ty { $( $name:literal => $f:expr ),+ $(,)? }) => {
        impl $ty {
            /// Exported Go method names of this type (exact case).
            pub const GO_METHODS: &'static [&'static str] = &[$($name),+];

            /// Go: `reflect.Type.MethodByName(name)` for this type.
            pub fn go_has_method(name: &str) -> bool {
                matches!(name, $($name)|+)
            }

            /// Calls the Go method `name`; `None` if the type has no such method.
            pub fn go_call_method(
                &self,
                ctx: $crate::object::HostCtx<'_>,
                name: &str,
                args: &[$crate::object::Value],
            ) -> ::core::option::Option<$crate::object::GoResult<$crate::object::Value>> {
                match name {
                    $(
                        $name => {
                            let f: fn(&Self, $crate::object::HostCtx<'_>, &[$crate::object::Value])
                                -> $crate::object::GoResult<$crate::object::Value> = $f;
                            ::core::option::Option::Some(f(self, ctx, args))
                        }
                    )+
                    _ => ::core::option::Option::None,
                }
            }
        }
    };
}

/// Expands, inside an `impl go_value::Object for T` block, to `type_name`, `has_method`,
/// `call_method` and `as_any`, delegating to the table generated by [`go_methods!`].
/// Everything else (`kind`, `field`, `is_zero`, `go_string`, `underlying`, ...) is written next to
/// it as needed.
///
/// ```ignore
/// impl Object for Scratch {
///     nh_common::object_basics!("*maps.Scratch");
/// }
/// ```
///
/// The type string is Go's `%T` spelling (`*page.Pager`, `media.Type`, `hstring.HTML`).
#[macro_export]
macro_rules! object_basics {
    ($go_type:expr) => {
        fn type_name(&self) -> ::std::borrow::Cow<'_, str> {
            ::std::borrow::Cow::Borrowed($go_type)
        }
        fn has_method(&self, name: &str) -> bool {
            Self::go_has_method(name)
        }
        fn call_method(
            &self,
            ctx: $crate::object::HostCtx<'_>,
            name: &str,
            args: &[$crate::object::Value],
        ) -> ::core::option::Option<$crate::object::GoResult<$crate::object::Value>> {
            self.go_call_method(ctx, name, args)
        }
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
    };
}

/// Argument helpers for method/function bodies: Go `evalCall`'s argument-count check and
/// `validateType` (text/template exec.go) for typed parameters, with Go's error texts.
///
/// The template engine evaluates every argument as for an `any` parameter (contract C3/C7):
/// ideal integer constants arrive as `int`, float constants as `float64`, string constants as
/// `string`, `nil` as `Invalid`, and a nil non-empty interface as `Invalid`. A Go `string`, `bool`
/// or `int` parameter is therefore checked exactly. For `int64`, `float64` and named string types
/// Go would have converted a *literal* (`evalInteger`/`evalFloat`/`evalString`), so those helpers
/// also accept `int` resp. `string` (a variable of that type is accepted too, where Go errors;
/// this only matters for templates that fail in Go).
pub mod args {
    use super::*;
    use go_value::{FloatKind, GoString, IntKind};

    // Go: text/template/exec.go:evalCall (argument count)
    /// Errors unless exactly `n` arguments were passed
    /// (Go: `wrong number of args for %s: want %d got %d`).
    pub fn exactly(args: &[Value], n: usize, name: &str) -> GoResult<()> {
        if args.len() != n {
            return Err(go_value::Error::new(format!(
                "wrong number of args for {name}: want {n} got {}",
                args.len()
            )));
        }
        Ok(())
    }

    // Go: text/template/exec.go:evalCall (variadic argument count)
    /// For a variadic method with `n` fixed parameters: errors when fewer were passed
    /// (Go: `wrong number of args for %s: want at least %d got %d`).
    pub fn at_least(args: &[Value], n: usize, name: &str) -> GoResult<()> {
        if args.len() < n {
            return Err(go_value::Error::new(format!(
                "wrong number of args for {name}: want at least {n} got {}",
                args.len()
            )));
        }
        Ok(())
    }

    fn missing(i: usize) -> go_value::Error {
        go_value::Error::new(format!("missing argument {i}"))
    }

    /// Go: `validateType` failure text (`wrong type for value; expected %s; got %s`).
    pub fn wrong_type(expected: &str, got: &Value) -> go_value::Error {
        go_value::Error::new(format!(
            "wrong type for value; expected {expected}; got {}",
            got.go_type_name()
        ))
    }

    /// Go: `validateType` of a nil value for a parameter that cannot be nil
    /// (`invalid value; expected %s`).
    pub fn invalid_value(expected: &str) -> go_value::Error {
        go_value::Error::new(format!("invalid value; expected {expected}"))
    }

    /// The i-th argument of an `any` parameter (cloned); error if missing.
    pub fn get(args: &[Value], i: usize) -> GoResult<Value> {
        args.get(i).cloned().ok_or_else(|| missing(i))
    }

    /// The i-th argument if present (an optional variadic element).
    pub fn opt(args: &[Value], i: usize) -> Option<&Value> {
        args.get(i)
    }

    // Go: text/template/exec.go:validateType (typ = string)
    /// A Go `string` parameter: only a `string` value is assignable (not `template.HTML`, not
    /// `hstring.HTML`, not a number).
    pub fn string(args: &[Value], i: usize) -> GoResult<GoString> {
        match args.get(i) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(Value::Invalid) => Err(invalid_value("string")),
            Some(v) => Err(wrong_type("string", v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:validateType / evalString (typ = a named string type)
    /// A parameter of a named string type `go_type` (e.g. `maps.ParamsMergeStrategy`): a value of
    /// that type (an object whose `type_name` is `go_type` with a string `underlying`), or a
    /// `string` (what a string literal evaluates to).
    pub fn named_string(args: &[Value], i: usize, go_type: &str) -> GoResult<GoString> {
        match args.get(i) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(Value::Invalid) => Err(invalid_value(go_type)),
            Some(v @ Value::Object(o)) => {
                if o.type_name() == go_type
                    && let Some(Value::String(s)) = o.underlying()
                {
                    return Ok(s);
                }
                Err(wrong_type(go_type, v))
            }
            Some(v) => Err(wrong_type(go_type, v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:validateType (typ = bool)
    /// A Go `bool` parameter.
    pub fn bool(args: &[Value], i: usize) -> GoResult<bool> {
        match args.get(i) {
            Some(Value::Bool(b)) => Ok(*b),
            Some(Value::Invalid) => Err(invalid_value("bool")),
            Some(v) => Err(wrong_type("bool", v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:validateType (typ = int)
    /// A Go `int` parameter: only an `int` value (what integer literals evaluate to).
    pub fn int(args: &[Value], i: usize) -> GoResult<i64> {
        match args.get(i) {
            Some(Value::Int(n, IntKind::Int)) => Ok(*n),
            Some(Value::Invalid) => Err(invalid_value("int")),
            Some(v) => Err(wrong_type("int", v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:validateType / evalInteger (typ = int64)
    /// A Go `int64` parameter: an `int64`, or an `int` (an integer literal).
    pub fn int64(args: &[Value], i: usize) -> GoResult<i64> {
        match args.get(i) {
            Some(Value::Int(n, IntKind::Int64 | IntKind::Int)) => Ok(*n),
            Some(Value::Invalid) => Err(invalid_value("int64")),
            Some(v) => Err(wrong_type("int64", v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:validateType / evalFloat (typ = float64)
    /// A Go `float64` parameter: a `float64`, or an `int` (an integer literal, which Go's
    /// `evalFloat` accepts because it is representable as a float).
    pub fn float64(args: &[Value], i: usize) -> GoResult<f64> {
        match args.get(i) {
            Some(Value::Float(f, FloatKind::F64)) => Ok(*f),
            Some(Value::Int(n, IntKind::Int)) => Ok(*n as f64),
            Some(Value::Invalid) => Err(invalid_value("float64")),
            Some(v) => Err(wrong_type("float64", v)),
            None => Err(missing(i)),
        }
    }

    // Go: text/template/exec.go:evalCall (variadic `...string`)
    /// The variadic `...string` tail `args[from..]`, each element checked as a `string`.
    pub fn strings(args: &[Value], from: usize) -> GoResult<Vec<GoString>> {
        (from..args.len()).map(|i| string(args, i)).collect()
    }

    /// Variadic tail `args[from..]` (Go `...any`).
    pub fn rest(args: &[Value], from: usize) -> &[Value] {
        if from >= args.len() {
            &[]
        } else {
            &args[from..]
        }
    }
}

/// Go text/template `evalCall`'s check that a method or function has one result, or two with
/// the second of type `error` (`goodFunc`). Methods that Go declares with another result count
/// are in the method set (so they shadow map keys and fields) but calling them from a template is
/// this error.
// Go: text/template/exec.go:evalCall (goodFunc)
pub fn bad_results_error(name: &str, num_out: usize) -> go_value::Error {
    go_value::Error::new(format!(
        "can't call method/function {name:?} with {num_out} results"
    ))
}

/// A method dispatcher for a named Go type that is represented as `Value::List` or `Value::Map`.
#[derive(Clone, Copy)]
pub struct NamedMethods {
    /// Go: method-set membership (`reflect.Type.MethodByName`).
    pub has_method: fn(name: &str) -> bool,
    /// Calls method `name` on the receiver value; `None` if no such method.
    pub call:
        fn(ctx: HostCtx<'_>, recv: &Value, name: &str, args: &[Value]) -> Option<GoResult<Value>>,
}

/// Registry of methods declared on named slice/map Go types, keyed by the Go type string
/// (`"page.Pages"`, `"resource.Resources"`, `"maps.Params"`, `"page.Taxonomy"`, ...).
///
/// Built once per build by nh-hugolib (`tplapi::named_types::registry()`), from the
/// `*_METHODS` constants each owning crate exports, and handed to the template store.
#[derive(Default, Clone)]
pub struct NamedTypeRegistry {
    map: HashMap<&'static str, NamedMethods>,
}

impl NamedTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or replaces) the methods of the named type `go_type`.
    pub fn register(&mut self, go_type: &'static str, methods: NamedMethods) {
        self.map.insert(go_type, methods);
    }

    pub fn lookup(&self, go_type: &str) -> Option<&NamedMethods> {
        self.map.get(go_type)
    }

    /// Go type string of a `Value::List`/`Value::Map` receiver, if it is a named type
    /// (`MapType::Params` is `maps.Params`).
    pub fn named_type_of(recv: &Value) -> Option<Cow<'_, str>> {
        match recv {
            Value::List(l) => match &l.ty {
                go_value::SliceType::Named(n) => Some(Cow::Borrowed(n)),
                _ => None,
            },
            Value::Map(m) => match &m.ty {
                go_value::MapType::Named(n) => Some(Cow::Borrowed(n)),
                go_value::MapType::Params => Some(Cow::Borrowed("maps.Params")),
                _ => None,
            },
            _ => None,
        }
    }

    /// Method lookup on a named slice/map receiver (the template engine's `GetMethod` for
    /// non-Object values). `None` when the receiver has no such method.
    pub fn call(
        &self,
        ctx: HostCtx<'_>,
        recv: &Value,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        let tn = Self::named_type_of(recv)?;
        let m = self.lookup(&tn)?;
        if !(m.has_method)(name) {
            return None;
        }
        (m.call)(ctx, recv, name, args)
    }

    /// Whether the named slice/map receiver has the method `name`.
    pub fn has_method(&self, recv: &Value, name: &str) -> bool {
        Self::named_type_of(recv)
            .and_then(|tn| self.lookup(&tn).copied())
            .map(|m| (m.has_method)(name))
            .unwrap_or(false)
    }
}

/// Conversion of a Rust host value into a template value.
pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl ToValue for Value {
    fn to_value(&self) -> Value {
        self.clone()
    }
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::string(self)
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::string(self.as_str())
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}

/// A Go `[]string` value from Rust strings.
pub fn string_slice<S: AsRef<str>>(items: &[S]) -> Value {
    Value::string_list(items.iter().map(|s| go_value::GoString::from(s.as_ref())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use go_value::{GoString, IntKind};
    use std::borrow::Cow;

    struct Probe(i64);

    crate::go_methods!(Probe {
        "N" => |p, _c, a| { args::exactly(a, 0, "N")?; Ok(Value::int(p.0)) },
        "Add" => |p, _c, a| { args::exactly(a, 1, "Add")?; Ok(Value::int(p.0 + args::int(a, 0)?)) },
        "Join" => |_p, _c, a| Ok(Value::String(GoString::from(
            args::strings(a, 0)?.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",")
        ))),
    });

    impl Object for Probe {
        crate::object_basics!("*test.Probe");
    }

    struct Empty;
    crate::go_methods!(Empty {});
    impl Object for Empty {
        crate::object_basics!("test.Empty");
        fn kind(&self) -> go_value::Kind {
            go_value::Kind::Struct
        }
    }

    #[test]
    fn method_tables() {
        let v = Value::object(Probe(40));
        let o = v.as_object().unwrap();
        assert_eq!(o.type_name(), Cow::Borrowed("*test.Probe"));
        assert!(o.has_method("Add"));
        assert!(!o.has_method("add"));
        assert_eq!(Probe::GO_METHODS, &["N", "Add", "Join"]);
        let got = o
            .call_method(&(), "Add", &[Value::int(2)])
            .unwrap()
            .unwrap();
        assert_eq!(got, Value::int(42));
        assert!(o.call_method(&(), "Nope", &[]).is_none());
        let err = o
            .call_method(&(), "N", &[Value::int(1)])
            .unwrap()
            .unwrap_err();
        assert_eq!(err.message(), "wrong number of args for N: want 0 got 1");
        let err = o
            .call_method(&(), "Add", &[Value::int64(1)])
            .unwrap()
            .unwrap_err();
        assert_eq!(
            err.message(),
            "wrong type for value; expected int; got int64"
        );
        let err = o
            .call_method(&(), "Add", &[Value::Invalid])
            .unwrap()
            .unwrap_err();
        assert_eq!(err.message(), "invalid value; expected int");
        let got = o
            .call_method(&(), "Join", &[Value::string("a"), Value::string("b")])
            .unwrap()
            .unwrap();
        assert_eq!(got, Value::string("a,b"));
        let err = o
            .call_method(&(), "Join", &[Value::html("a")])
            .unwrap()
            .unwrap_err();
        assert_eq!(
            err.message(),
            "wrong type for value; expected string; got template.HTML"
        );

        let e = Value::object(Empty);
        let o = e.as_object().unwrap();
        assert!(!o.has_method("X"));
        assert!(Empty::GO_METHODS.is_empty());
        assert!(o.call_method(&(), "X", &[]).is_none());
    }

    #[test]
    fn arg_helpers() {
        let a = [
            Value::int(3),
            Value::float64(1.5),
            Value::Int(7, IntKind::Int64),
            Value::Bool(true),
        ];
        assert_eq!(args::float64(&a, 0).unwrap(), 3.0);
        assert_eq!(args::float64(&a, 1).unwrap(), 1.5);
        assert_eq!(args::int64(&a, 2).unwrap(), 7);
        assert_eq!(args::int64(&a, 0).unwrap(), 3);
        assert!(args::int(&a, 2).is_err());
        assert!(args::bool(&a, 3).unwrap());
        assert!(args::get(&a, 9).is_err());
        assert_eq!(args::rest(&a, 2).len(), 2);
        assert!(args::rest(&a, 9).is_empty());
        assert!(args::at_least(&a, 5, "F").is_err());
        assert!(args::at_least(&a, 4, "F").is_ok());
        assert_eq!(
            args::named_string(&[Value::string("deep")], 0, "maps.ParamsMergeStrategy").unwrap(),
            "deep"
        );
        assert_eq!(
            bad_results_error("GetMergeStrategy", 2).message(),
            "can't call method/function \"GetMergeStrategy\" with 2 results"
        );
    }
}
