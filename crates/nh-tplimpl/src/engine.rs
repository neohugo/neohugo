//! Module `engine`.
//!
//! NEW: facade over the gotemplate crate (text/html template namespaces, parse trees, exec, escaper)
//!
//! Owner: Wave B task T13 (tplimpl).


//! The single adaptation point between the Hugo template store and the Wave A `gotemplate` crate
//! (Go text/template + html/template as forked in tpl/internal/go_templates, go1.24 semantics).
//!
//! CONTRACT: `crates/GOTEMPLATE_CONTRACT.md` is the pinned host contract between gotemplate and
//! the Hugo layer (it extends template-engine spec §16). The Wave A gotemplate task implements it
//! as acceptance criteria; T13 reviews the gotemplate API against it before building on it. The
//! trait below mirrors the contract's `ExecHelper` clause (C1) exactly: when gotemplate lands, T13
//! replaces this definition with a re-export of `gotemplate`'s trait (same methods) and replaces
//! the placeholder template types with re-exports/thin wrappers. Nothing outside nh-tplimpl
//! (except nh-tpl's `strip_html` and nh-i18n's message templates) touches gotemplate.
//!
//! Semantics that live INSIDE the engine (they cannot be patched here later) — see the contract:
//! truthiness through `ExecHelper::is_true` (C5); methods before fields and map keys for every
//! receiver kind (C2); `and`/`or` return the deciding operand (C6); Go `evalArg`/`validateType`
//! /ideal-constant rules and nil conversions (C7); `printValue` `<no value>` vs `<nil>` (C8);
//! html escapers unwrap `Object::printable_value` (C9); the `mainsections` pointer-identity
//! special case (C10, implemented by the helper, requires the engine to keep `Arc` identity);
//! one host context passed unchanged to every call, and re-entrant nested execution (C4).

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_common::Result;

/// A template function (Go: any func in the FuncMap). Receives the template context as `HostCtx`
/// (Go injects `context.Context` when the first param is one; Rust always passes it). Arguments
/// arrive converted per contract C7 (nil interfaces -> `Invalid`, ideal constants -> int/float64).
pub type TplFunc = Arc<dyn Fn(HostCtx<'_>, &[Value]) -> go_value::Result<Value> + Send + Sync>;

/// Go: `map[string]any` func map (Hugo namespaces + aliases). Built by nh-tplfuncs `tplimplinit`.
pub type FuncMap = BTreeMap<String, TplFunc>;

/// Go: `texttemplate.ExecHelper` (texttemplate/hugo_template.go:45-51) as implemented by
/// `tplimpl.templateExecHelper` (template_funcs.go), plus Hugo's `isTrue`. Contract clause C1.
/// The engine calls these for every function lookup, method call, map lookup and truth test,
/// with the SAME host context it was given (C4). No method may assume it is the only execution
/// in flight: helpers are called re-entrantly from nested executions.
pub trait ExecHelper: Send + Sync {
    /// Go: `Init(ctx, tmpl)` — once per `execute_with_context`. Hugo uses it for dependency
    /// tracking only (watch mode): a no-op for a one-shot build.
    fn init(&self, _ctx: HostCtx<'_>, _template_name: &str) {}
    /// Go: `GetFunc(ctx, tmpl, name)` — Hugo funcs first, then html escaper funcs, then text
    /// builtins (the engine falls back to its builtins when this returns `None`).
    fn get_func(&self, ctx: HostCtx<'_>, name: &str) -> Option<TplFunc>;
    /// Go: `GetMethod(...).IsValid()` — method-set membership for ANY receiver: `Object`, named
    /// `List`/`Map` (NamedTypeRegistry), `Time`, `Safe`, typed nils with nil-safe methods. The
    /// engine asks this BEFORE looking at struct fields or map keys (methods win).
    fn has_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool;
    /// Calls the method (after the engine evaluated the arguments). Includes the `mainsections`
    /// special case: receiver `Value::Map(m)` with `Arc::ptr_eq(m, site_params)` and name
    /// EqualFold "mainsections" -> `site.MainSections()` (contract C10).
    fn call_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str, args: &[Value]) -> go_value::Result<Value>;
    /// Go: `GetMapValue` — `maps.Params`: lower-cased key, nil value -> `None` (= missing);
    /// other maps (and `Kind::Map` objects): exact key.
    fn get_map_value(&self, ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value>;
    /// Go: `OnCalled(ctx, tmpl, name, args, result)` — watch-mode bookkeeping; no-op here.
    fn on_called(&self, _ctx: HostCtx<'_>, _name: &str, _args: &[Value], _result: &Value) {}
    /// Hugo's `isTrue` = `hreflect.IsTruthfulValue`: `IsZero()` first (`Object::is_zero`, zero
    /// `time.Time`, `maps.Params` that is empty or holds only `_merge`, nil receivers of types
    /// with `IsZero`), then Go kind rules. Used by if/with/else-with/and/or/not (contract C5).
    fn is_true(&self, v: &Value) -> bool;
}

/// Placeholder for gotemplate's text template (Wave B: `gotemplate::text::Template`).
#[derive(Clone)]
pub struct TextTemplate {
    pub(crate) inner: Arc<()>,
}

/// Placeholder for gotemplate's html template (Wave B: `gotemplate::html::Template`).
#[derive(Clone)]
pub struct HtmlTemplate {
    pub(crate) inner: Arc<()>,
}

/// Go: `tpl.Template` — either engine.
#[derive(Clone)]
pub enum Template {
    Text(TextTemplate),
    Html(HtmlTemplate),
}

impl Template {
    pub fn name(&self) -> String {
        todo!()
    }

    /// Go: `Prepare()` — html: escape (once) and return the rewritten text template.
    pub fn prepare(&self) -> Result<TextTemplate> {
        todo!()
    }
}

/// Go: `texttemplate.Executer` — executes a prepared template with a helper. MUST be re-entrant
/// (contract C4): a func or method called during an execution may execute other templates (or
/// the same one) on the same thread before returning — partials, render hooks through
/// `.Content`, `resources.ExecuteAsTemplate`, `markdownify`. Templates are immutable and shared
/// after `prepare`; all execution state is per call; no lock is held while calling out.
pub trait Executer: Send + Sync {
    fn execute_with_context(&self, ctx: HostCtx<'_>, t: &Template, w: &mut Vec<u8>, data: &Value) -> Result<()>;
}

/// Go: `htmltemplate.StripTags` (re-exported for nh-tpl).
pub fn strip_tags(s: &[u8]) -> Vec<u8> {
    todo!("gotemplate html::strip_tags")
}
