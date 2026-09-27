//! Go: tpl/internal/go_templates/htmltemplate/content.go, Hugo's `indirect`
//! override (htmltemplate/hugo_template.go) and `evalArgs` (escape.go).
//!
//! # Pointers in the value model
//!
//! Go's `reflect` dereferencing (`v.Elem()`) of a `*T` host value is
//! emulated on `Value::Object`s of `Kind::Ptr`: the pointee is a
//! [`Pointee`] wrapper of `Kind::Struct` that prints and marshals like the
//! struct (`{a b}` instead of `&{a b}` in `fmt`, the same fields in
//! `encoding/json`). Its method set follows the go-fmt convention for the
//! pointee of a `Kind::Ptr` object (go-fmt PORTING.md, deviation 6): the
//! methods of a `Kind::Ptr` object are pointer-receiver methods, so the
//! pointee has no `String`, `Error`, `MarshalJSON` or `MarshalText`. The one
//! exception is Hugo's `PrintableValue`, which the pointee keeps (both Hugo
//! implementations, `hstring.HTML` and `page.Summary`, have value
//! receivers, and it is only consulted here, after `doIndirect`).
//!
//! A typed nil pointer (`Value::TypedNil` of a pointer type) stops every
//! dereferencing loop (`v.IsNil()`). A `Value::TypedNil` of an interface
//! type cannot exist inside a Go `any` (it is a nil `any`), so it is treated
//! like `Value::Invalid` (`arg == nil`).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, NilKind, Object, SafeKind, Value, typed_nil_kind};

/// Go: `contentType`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum ContentType {
    /// Go: `contentTypePlain`.
    Plain,
    /// Go: `contentTypeCSS`.
    Css,
    /// Go: `contentTypeHTML`.
    Html,
    /// Go: `contentTypeHTMLAttr`.
    HtmlAttr,
    /// Go: `contentTypeJS`.
    Js,
    /// Go: `contentTypeJSStr`.
    JsStr,
    /// Go: `contentTypeURL`.
    Url,
    /// Go: `contentTypeSrcset`.
    Srcset,
    /// Go: `contentTypeUnsafe` — used in attr.go for values that affect how
    /// embedded content and network messages are formed, vetted, or
    /// interpreted; or which credentials network messages carry.
    Unsafe,
}

/// Go: `arg == nil` for an `any` holding `v`.
pub(crate) fn is_nil_any(v: &Value) -> bool {
    match v {
        Value::Invalid => true,
        Value::TypedNil(t) => typed_nil_kind(t) == NilKind::Interface,
        _ => false,
    }
}

/// The pointee of a `Kind::Ptr` host object (Go: `reflect.Value.Elem()` of
/// a `*T`, then `Interface()`). See the module documentation.
pub(crate) struct Pointee(pub(crate) Arc<dyn Object>);

impl Object for Pointee {
    fn type_name(&self) -> Cow<'_, str> {
        let t = self.0.type_name();
        match t {
            Cow::Borrowed(s) => Cow::Borrowed(s.strip_prefix('*').unwrap_or(s)),
            Cow::Owned(s) => Cow::Owned(s.strip_prefix('*').unwrap_or(&s).to_string()),
        }
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
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.0.field(name)
    }
    fn printable_value(&self) -> Option<Value> {
        self.0.printable_value()
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        self.0.struct_fields()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go `v.Elem().Interface()` of a non-nil `*T` host object.
fn elem(o: &Arc<dyn Object>) -> Value {
    Value::Object(Arc::new(Pointee(o.clone())))
}

// Go: content.go:doIndirect
/// indirect returns the value, after dereferencing as many times
/// as necessary to reach the base type (or nil).
pub(crate) fn do_indirect(a: &Value) -> Value {
    if is_nil_any(a) {
        return Value::Invalid;
    }
    match a {
        // Avoid creating a reflect.Value if it's not a pointer.
        Value::Object(o) if o.kind() == Kind::Ptr => elem(o),
        _ => a.clone(),
    }
}

// Go: content.go:indirectToStringerOrError
/// indirectToStringerOrError returns the value, after dereferencing as many
/// times as necessary to reach the base type (or nil) or an implementation
/// of fmt.Stringer or error.
pub(crate) fn indirect_to_stringer_or_error(a: &Value) -> Value {
    if is_nil_any(a) {
        return Value::Invalid;
    }
    match a {
        Value::Object(o)
            if o.kind() == Kind::Ptr && o.go_string().is_none() && o.go_error().is_none() =>
        {
            elem(o)
        }
        _ => a.clone(),
    }
}

// Go: htmltemplate/hugo_template.go:indirect
/// Hugo's override of `indirect`: `doIndirect`, then a
/// `types.PrintableValueProvider` is replaced by its `PrintableValue()`.
pub(crate) fn indirect(a: &Value) -> Value {
    let in_ = do_indirect(a);

    // We have a special Result type that we want to unwrap when printed.
    if let Value::Object(o) = &in_
        && let Some(pv) = o.printable_value()
    {
        return pv;
    }

    in_
}

// Go: content.go:stringify
/// stringify converts its arguments to a string and the type of the content.
/// All pointers are dereferenced, as in the text/template package.
pub(crate) fn stringify(args: &[Value]) -> (Vec<u8>, ContentType) {
    if args.len() == 1 {
        match indirect(&args[0]) {
            Value::String(s) => return (s.to_vec(), ContentType::Plain),
            Value::Safe(kind, s) => {
                let t = match kind {
                    SafeKind::Css => ContentType::Css,
                    SafeKind::Html => ContentType::Html,
                    SafeKind::HtmlAttr => ContentType::HtmlAttr,
                    SafeKind::Js => ContentType::Js,
                    SafeKind::JsStr => ContentType::JsStr,
                    SafeKind::Url => ContentType::Url,
                    SafeKind::Srcset => ContentType::Srcset,
                };
                return (s.to_vec(), t);
            }
            _ => {}
        }
    }
    let mut out: Vec<Value> = Vec::with_capacity(args.len());
    for arg in args {
        // We skip untyped nil arguments for backward compatibility.
        // Without this they would be output as <nil>, escaped.
        // See issue 25875.
        if is_nil_any(arg) {
            continue;
        }

        out.push(indirect_to_stringer_or_error(arg));
    }
    (go_fmt::sprint(&out), ContentType::Plain)
}

// Go: escape.go:evalArgs
/// evalArgs formats the list of arguments into a string. It is equivalent
/// to fmt.Sprint(args...), except that it dereferences all pointers.
pub(crate) fn eval_args(args: &[Value]) -> Vec<u8> {
    // Optimization for simple common case of a single string argument.
    if args.len() == 1
        && let Value::String(s) = &args[0]
    {
        return s.to_vec();
    }
    let args: Vec<Value> = args.iter().map(indirect_to_stringer_or_error).collect();
    go_fmt::sprint(&args)
}

/// Go: `_, ok := a.(json.Marshaler)`.
pub(crate) fn is_json_marshaler(a: &Value) -> bool {
    match a {
        // time.Time has a MarshalJSON method.
        Value::Time(_) => true,
        Value::Object(o) => o.marshal_json().is_some(),
        _ => false,
    }
}

// Go: js.go:indirectToJSONMarshaler
/// indirectToJSONMarshaler returns the value, after dereferencing as many
/// times as necessary to reach the base type (or nil) or an implementation
/// of json.Marshal.
pub(crate) fn indirect_to_json_marshaler(a: &Value) -> Value {
    // text/template now supports passing untyped nil as a func call
    // argument, so we must support it. Otherwise we'd panic below, as one
    // cannot call the Type or Interface methods on an invalid
    // reflect.Value. See golang.org/issue/18716.
    if is_nil_any(a) {
        return Value::Invalid;
    }

    match a {
        Value::Object(o) if o.kind() == Kind::Ptr && !is_json_marshaler(a) => elem(o),
        _ => a.clone(),
    }
}

/// Go: `t, ok := a.(fmt.Stringer)` then `t.String()`.
///
/// Host objects report `String` through `Object::go_string`. Values the
/// model carries only by type name (named slices/maps and typed nils) have
/// their `String` methods in go-fmt's named-method registry. A registered
/// method that panics (`None`, a nil receiver) is treated as absent.
pub(crate) fn stringer_string(a: &Value) -> Option<GoString> {
    match a {
        Value::Object(o) => o.go_string(),
        Value::List(_) | Value::Map(_) | Value::TypedNil(_) => match go_fmt::named_method(a) {
            Some(go_fmt::NamedMethod::String(f)) => f(a).map(GoString::from),
            _ => None,
        },
        _ => None,
    }
}
