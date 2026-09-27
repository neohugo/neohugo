//! Port of `tpl/safe/safe.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `safeHTML` etc. wrap `cast.ToString` in `Value::Safe(kind, ..)`; `try` wraps the result in a TryValue object.

/// Go: `safe.Namespace` (template value `*safe.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/safe:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/safe:CSS
    pub fn css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/safe:HTML
    pub fn html(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/safe:HTMLAttr
    pub fn html_attr(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/safe:JS
    pub fn js(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/safe:JSStr
    pub fn js_str(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/safe:URL
    pub fn url(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
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
// EX L25-27: New() *Namespace
// EX L33-36: (ns *Namespace) CSS(s any) (template.CSS, error)
// EX L39-42: (ns *Namespace) HTML(s any) (template.HTML, error)
//    L45-48: (ns *Namespace) HTMLAttr(s any) (template.HTMLAttr, error)
// EX L51-54: (ns *Namespace) JS(s any) (template.JS, error)
//    L57-60: (ns *Namespace) JSStr(s any) (template.JSStr, error)
// EX L63-66: (ns *Namespace) URL(s any) (template.URL, error)
// ---------------------------------------------------------------------------
