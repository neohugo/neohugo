//! Port of `tpl/safe/safe.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{HostCtx, Object, SafeKind, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: `safeHTML` etc. wrap `cast.ToString` in `Value::Safe(kind, ..)`; `try` wraps the result in a TryValue object.

/// Go: `safe.Namespace` (template value `*safe.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

/// `template.X(cast.ToStringE(s))`: Go returns the typed value AND the error; the template
/// engine only sees the error.
fn safe(kind: SafeKind, a: &[Value], name: &str) -> GoResult<Value> {
    args::exactly(a, 1, name)?;
    let ss = caste::to_string_e(&a[0])?;
    Ok(Value::Safe(kind, ss))
}

impl Namespace {
    // Go: tpl/safe:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/safe:CSS
    pub fn css(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:CSS
        safe(SafeKind::Css, a, "CSS")
    }

    // Go: tpl/safe:HTML
    pub fn html(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:HTML
        safe(SafeKind::Html, a, "HTML")
    }

    // Go: tpl/safe:HTMLAttr
    pub fn html_attr(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:HTMLAttr
        safe(SafeKind::HtmlAttr, a, "HTMLAttr")
    }

    // Go: tpl/safe:JS
    pub fn js(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:JS
        safe(SafeKind::Js, a, "JS")
    }

    // Go: tpl/safe:JSStr
    pub fn js_str(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:JSStr
        safe(SafeKind::JsStr, a, "JSStr")
    }

    // Go: tpl/safe:URL
    pub fn url(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/safe/safe.go:URL
        safe(SafeKind::Url, a, "URL")
    }
}

nh_common::go_methods!(Namespace {
    "CSS" => |n, ctx, a| n.css(ctx, a),
    "HTML" => |n, ctx, a| n.html(ctx, a),
    "HTMLAttr" => |n, ctx, a| n.html_attr(ctx, a),
    "JS" => |n, ctx, a| n.js(ctx, a),
    "JSStr" => |n, ctx, a| n.js_str(ctx, a),
    "URL" => |n, ctx, a| n.url(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*safe.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/safe/safe.go (66 lines; 5/7 funcs executed)
//   types: Namespace
// OK L25-27: New() *Namespace
// OK L33-36: (ns *Namespace) CSS(s any) (template.CSS, error)
// OK L39-42: (ns *Namespace) HTML(s any) (template.HTML, error)
// OK L45-48: (ns *Namespace) HTMLAttr(s any) (template.HTMLAttr, error)
// OK L51-54: (ns *Namespace) JS(s any) (template.JS, error)
// OK L57-60: (ns *Namespace) JSStr(s any) (template.JSStr, error)
// OK L63-66: (ns *Namespace) URL(s any) (template.URL, error)
// ---------------------------------------------------------------------------
