//! Go: tpl/internal/go_templates/texttemplate/hugo_template.go — the Hugo
//! additions: `ExecHelper`, `Executer`, `Preparer`, `TryValue`/`TryError`,
//! and Hugo's truthiness (`hreflect.IsTruthfulValue`, which the fork uses
//! for `isTrue` everywhere).

use std::borrow::Cow;
use std::io::Write;
use std::sync::Arc;

use go_value::{HostCtx, Kind, MapType, Object, Value};

use crate::error::Error;

use super::exec::execute_template;
use super::funcs::Func;
use super::template::Template;

/// Go: `ExecHelper` (hugo_template.go:45-51) extended with Hugo's truthiness
/// — contract clause C1 (`crates/GOTEMPLATE_CONTRACT.md`).
///
/// The engine calls these for every function lookup, method call, map
/// lookup and truth test, with the same host context it was given. Calls
/// may be re-entrant (a function may execute templates).
pub trait ExecHelper: Send + Sync {
    /// Go: `Init(ctx, tmpl)` — once per `execute_with_context`.
    fn init(&self, _ctx: HostCtx<'_>, _template_name: &str) {}

    /// Go: `GetFunc(ctx, tmpl, name)` — consulted first for every function
    /// name; `None` falls back to the template's own funcs and the
    /// builtins (Go `findFunction`).
    fn get_func(&self, _ctx: HostCtx<'_>, _name: &str) -> Option<Func> {
        None
    }

    /// Go: `GetMethod(...)` validity — whether `receiver` has an exported
    /// method `name`. Asked for every receiver kind before fields and map
    /// keys (methods win). The default consults `Object::has_method`.
    fn has_method(&self, _ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        match receiver {
            Value::Object(o) => o.has_method(name),
            _ => false,
        }
    }

    /// Calls the method found by `has_method` with the evaluated arguments.
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        receiver: &Value,
        name: &str,
        args: &[Value],
    ) -> go_value::Result<Value> {
        match receiver {
            Value::Object(o) => o
                .call_method(ctx, name, args)
                .unwrap_or_else(|| Err(go_value::Error::new(format!("method {name} not found")))),
            _ => Err(go_value::Error::new(format!("method {name} not found"))),
        }
    }

    /// Go: `GetMapValue(ctx, tmpl, receiver, key)`. `None` is Go's invalid
    /// result (missing key; the `missingkey` option applies);
    /// `Some(Value::Invalid)` is a present key holding a nil interface.
    /// The default is Go's `receiver.MapIndex(key)`.
    fn get_map_value(&self, _ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        let key = key.as_go_string()?;
        match receiver {
            Value::Map(m) => m.get(key).cloned(),
            Value::Object(o) if o.kind() == Kind::Map => o.map_get(key),
            _ => None,
        }
    }

    /// Go: `OnCalled(ctx, tmpl, name, args, result)`.
    fn on_called(&self, _ctx: HostCtx<'_>, _name: &str, _args: &[Value], _result: &Value) {}

    /// Hugo's `isTrue` (hugo_template.go:434): `hreflect.IsTruthfulValue`.
    fn is_true(&self, v: &Value) -> bool {
        is_truthful_value(v)
    }
}

/// The helper used by plain [`Template::execute`]: Go's behaviour without
/// a helper (reflection-only lookups, the fork's truthiness).
pub struct DefaultHelper;

impl ExecHelper for DefaultHelper {}

// Go: common/hreflect/helpers.go:IsTruthfulValue
/// Returns whether the given value has a meaningful truth value. This is
/// based on template.IsTrue in Go's stdlib, but also considers IsZero and
/// any interface value will be unwrapped before it's considered for
/// truthfulness.
pub fn is_truthful_value(val: &Value) -> bool {
    match val {
        // Something like var x interface{}, never set. It's a form of nil.
        Value::Invalid => false,
        // Nil pointers, funcs, chans and interfaces are false; nil slices
        // and maps have length 0. (A nil maps.Params is IsZero.)
        Value::TypedNil(_) => false,
        Value::Bool(b) => *b,
        Value::Int(i, _) => *i != 0,
        Value::Uint(u, _) => *u != 0,
        Value::Float(f, _) => *f != 0.0,
        Value::String(s) | Value::Safe(_, s) => !s.is_empty(),
        // time.Time implements types.Zeroer.
        Value::Time(t) => !t.is_zero(),
        Value::List(l) => !l.items.is_empty(),
        Value::Map(m) => {
            if m.ty == MapType::Params {
                // maps.Params implements types.Zeroer.
                !params_is_zero(m)
            } else {
                !m.entries.is_empty()
            }
        }
        Value::Object(o) => {
            if let Some(z) = o.is_zero() {
                return !z;
            }
            match o.kind() {
                Kind::Map => !o.map_keys().is_empty(),
                Kind::Slice => o.list().is_some_and(|l| !l.is_empty()),
                // Non-nil pointers, funcs and interfaces; struct values are always true.
                Kind::Ptr | Kind::Func | Kind::Interface | Kind::Struct => true,
            }
        }
    }
}

// Go: common/maps/params.go:(Params).IsZero
/// Params is zero when empty or when its only key is the merge strategy key.
fn params_is_zero(m: &go_value::Map) -> bool {
    if m.entries.is_empty() {
        return true;
    }
    if m.entries.len() > 1 {
        return false;
    }
    m.entries
        .keys()
        .next()
        .is_some_and(|k| k.as_bytes() == b"_merge")
}

/// Go: `Preparer` — prepares the template before execution (html/template
/// escapes; text/template returns itself).
pub trait Preparer {
    fn prepare(&self) -> Result<Template, Error>;
    /// The template's name (Go: passed to `ExecHelper.Init` as `tmpl`).
    fn preparer_name(&self) -> String;
}

// Go: hugo_template.go:(*Template).Prepare
impl Preparer for Template {
    fn prepare(&self) -> Result<Template, Error> {
        Ok(self.clone())
    }
    fn preparer_name(&self) -> String {
        self.name().to_string()
    }
}

/// Go: `Executer` — executes a given template with a helper.
#[derive(Clone)]
pub struct Executer {
    helper: Arc<dyn ExecHelper>,
}

impl Executer {
    // Go: hugo_template.go:NewExecuter
    pub fn new(helper: Arc<dyn ExecHelper>) -> Executer {
        Executer { helper }
    }

    pub fn helper(&self) -> &Arc<dyn ExecHelper> {
        &self.helper
    }

    // Go: hugo_template.go:(*executer).ExecuteWithContext
    /// Prepares `p`, then executes it with `data` as dot, passing `ctx`
    /// unchanged to every helper call and every function/method call.
    /// Re-entrant: functions called during execution may execute templates.
    pub fn execute_with_context(
        &self,
        ctx: HostCtx<'_>,
        p: &dyn Preparer,
        wr: &mut dyn Write,
        data: &Value,
    ) -> Result<(), Error> {
        let tmpl = p.prepare()?;
        self.helper.init(ctx, &p.preparer_name());
        execute_template(&tmpl, ctx, &*self.helper, wr, data)
    }
}

impl Template {
    // Go: exec.go:(*Template).Execute
    /// Applies a parsed template to the specified data object, and writes
    /// the output to wr. If an error occurs executing the template or
    /// writing its output, execution stops, but partial results may already
    /// have been written to the output writer. A template may be executed
    /// safely in parallel, although if parallel executions share a Writer
    /// the output may be interleaved.
    pub fn execute(&self, wr: &mut dyn Write, data: &Value) -> Result<(), Error> {
        execute_template(self, &(), &DefaultHelper, wr, data)
    }

    // Go: exec.go:(*Template).ExecuteTemplate
    /// Applies the template associated with t that has the given name to
    /// the specified data object and writes the output to wr.
    pub fn execute_template(
        &self,
        wr: &mut dyn Write,
        name: &str,
        data: &Value,
    ) -> Result<(), Error> {
        let Some(tmpl) = self.lookup(name) else {
            return Err(Error::Other(format!(
                "template: no template {} associated with template {}",
                go_strconv::quote(name),
                go_strconv::quote(self.name())
            )));
        };
        tmpl.execute(wr, data)
    }

    /// Executes with a helper and host context (Go: `ExecuteWithContext`
    /// on a text template, which `Prepare`s to itself).
    pub fn execute_with_helper(
        &self,
        ctx: HostCtx<'_>,
        helper: &dyn ExecHelper,
        wr: &mut dyn Write,
        data: &Value,
    ) -> Result<(), Error> {
        helper.init(ctx, self.name());
        execute_template(self, ctx, helper, wr, data)
    }
}

/// Go: `TryValue` — what gets returned when using the "try" keyword.
pub struct TryValue {
    /// Value is the value returned by the function or method wrapped with
    /// "try". This will always be nil if Err is set.
    pub value: Value,
    /// Err is the error returned by the function or method wrapped with
    /// "try". This will always be nil if Value is set.
    pub err: Option<Arc<TryError>>,
}

impl TryValue {
    fn err_value(&self) -> Value {
        match &self.err {
            Some(e) => Value::Object(e.clone()),
            None => {
                register_nil_try_error();
                Value::TypedNil(Arc::from("*template.TryError"))
            }
        }
    }
}

/// A nil `*TryError` still has the `Error` method (pointer receiver), which
/// dereferences it: fmt's `handleMethods` calls it for `%v %s %q %x %X` and
/// `catchPanic` prints `<nil>` (fmt/print.go), e.g. `printf "%s" .Err`
/// prints `<nil>`, not `%!s(*template.TryError=<nil>)`. Declared to go-fmt
/// as a method that panics on the nil receiver.
fn register_nil_try_error() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        go_fmt::register_named_method(
            "*template.TryError",
            // Go: hugo_template.go:(*TryError).Error — e.Err.Error() on a nil e.
            go_fmt::NamedMethod::Error(|_| None),
        )
    });
}

impl Object for TryValue {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("template.TryValue")
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
        match name {
            "Value" => Some(self.value.clone()),
            "Err" => Some(self.err_value()),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Value"), self.value.clone()),
            (Cow::Borrowed("Err"), self.err_value()),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `TryError` — wraps an error with a cause.
pub struct TryError {
    pub err: Error,
    /// The `Err` and `Cause` field values, created once so that they keep
    /// their identity (Go: the same error values on every access).
    fields: std::sync::OnceLock<(Value, Value)>,
}

impl TryError {
    // Go: hugo_template.go:newErrorWithCause
    pub(crate) fn new(err: Error) -> TryError {
        TryError {
            err,
            fields: std::sync::OnceLock::new(),
        }
    }

    fn fields(&self) -> &(Value, Value) {
        self.fields.get_or_init(|| {
            let err = Value::object(match &self.err {
                Error::Exec(e) => ErrorValue::exec_error(e),
                other => ErrorValue::plain(ERROR_STRING, other.to_string()),
            });
            // Go: herrors.Cause(err), the innermost error of the Unwrap
            // chain (a host error is a message, PORTING deviation 9).
            let cause = Value::object(ErrorValue::plain(ERROR_STRING, self.err.cause_message()));
            (err, cause)
        })
    }
}

/// A Go `error` value visible to templates (the `Err`/`Cause` fields of
/// `TryError` and what they wrap): `template.ExecError` (a struct with
/// the fields `Name` and `Err` and the methods `Error` and `Unwrap`),
/// `*fmt.wrapError` (`fmt.Errorf` with `%w`: `Error`, `Unwrap`) or
/// `*errors.errorString` (`Error`).
pub struct ErrorValue {
    pub type_name: String,
    pub message: String,
    /// Go `ExecError.Name` (for `template.ExecError`).
    pub name: Option<String>,
    /// The wrapped error: `ExecError.Err`, or what `(*fmt.wrapError).Unwrap`
    /// returns.
    pub inner: Option<Arc<ErrorValue>>,
}

const EXEC_ERROR: &str = "template.ExecError";
const WRAP_ERROR: &str = "*fmt.wrapError";
const ERROR_STRING: &str = "*errors.errorString";

impl ErrorValue {
    fn plain(type_name: &str, message: String) -> ErrorValue {
        ErrorValue {
            type_name: type_name.to_string(),
            message,
            name: None,
            inner: None,
        }
    }

    // Go: exec.go:(*state).errorf builds `ExecError{Name, Err:
    // fmt.Errorf(format, args...)}`; the format wraps (`%w`) the error of a
    // function or method call ("error calling %s: %w").
    fn exec_error(e: &crate::error::ExecError) -> ErrorValue {
        let err = match &e.cause {
            Some(cause) => ErrorValue {
                inner: Some(Arc::new(ErrorValue::plain(
                    ERROR_STRING,
                    cause.message().to_string(),
                ))),
                ..ErrorValue::plain(WRAP_ERROR, e.message.clone())
            },
            None => ErrorValue::plain(ERROR_STRING, e.message.clone()),
        };
        ErrorValue {
            name: Some(e.name.clone()),
            inner: Some(Arc::new(err)),
            ..ErrorValue::plain(EXEC_ERROR, e.message.clone())
        }
    }

    fn inner_value(&self) -> Value {
        match &self.inner {
            Some(e) => Value::Object(e.clone()),
            None => Value::TypedNil(Arc::from("error")),
        }
    }
}

impl Object for ErrorValue {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.type_name)
    }
    fn kind(&self) -> Kind {
        if self.type_name == EXEC_ERROR {
            Kind::Struct
        } else {
            Kind::Ptr
        }
    }
    fn has_method(&self, name: &str) -> bool {
        match name {
            "Error" => true,
            "Unwrap" => self.type_name == EXEC_ERROR || self.type_name == WRAP_ERROR,
            _ => false,
        }
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if !self.has_method(name) {
            return None;
        }
        Some(Ok(match name {
            // Go: (ExecError).Error, (*wrapError).Error, (*errorString).Error
            "Error" => Value::string(self.message.as_str()),
            // Go: (ExecError).Unwrap, (*wrapError).Unwrap
            _ => self.inner_value(),
        }))
    }
    fn field(&self, name: &str) -> Option<Value> {
        if self.type_name != EXEC_ERROR {
            return None;
        }
        match name {
            "Name" => Some(Value::string(self.name.clone().unwrap_or_default())),
            "Err" => Some(self.inner_value()),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        (self.type_name == EXEC_ERROR).then(|| {
            vec![
                (
                    Cow::Borrowed("Name"),
                    Value::string(self.name.clone().unwrap_or_default()),
                ),
                (Cow::Borrowed("Err"), self.inner_value()),
            ]
        })
    }
    fn go_error(&self) -> Option<String> {
        Some(self.message.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Object for TryError {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*template.TryError")
    }
    fn kind(&self) -> Kind {
        Kind::Ptr
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Error" | "Unwrap")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            // Go: (*TryError).Error
            "Error" => Some(Ok(Value::string(self.err.to_string()))),
            // Go: (*TryError).Unwrap
            "Unwrap" => Some(Ok(self.field("Err").unwrap_or(Value::Invalid))),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Err" => Some(self.fields().0.clone()),
            "Cause" => Some(self.fields().1.clone()),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (
                Cow::Borrowed("Err"),
                self.field("Err").unwrap_or(Value::Invalid),
            ),
            (
                Cow::Borrowed("Cause"),
                self.field("Cause").unwrap_or(Value::Invalid),
            ),
        ])
    }
    fn go_error(&self) -> Option<String> {
        Some(self.err.to_string())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
