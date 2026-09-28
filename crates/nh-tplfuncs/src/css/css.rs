//! Port of `tpl/css/css.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `css.Namespace` (template value `*css.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/css:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/css:PostCSS
    pub fn post_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/css:Quoted
    pub fn quoted(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/css:Sass
    pub fn sass(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/css:TailwindCSS
    pub fn tailwind_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/css:Unquoted
    pub fn unquoted(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "PostCSS" => |n, ctx, a| n.post_css(ctx, a),
    "Quoted" => |n, ctx, a| n.quoted(ctx, a),
    "Sass" => |n, ctx, a| n.sass(ctx, a),
    "TailwindCSS" => |n, ctx, a| n.tailwind_css(ctx, a),
    "Unquoted" => |n, ctx, a| n.unquoted(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*css.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/css/css.go (192 lines; 3/7 funcs executed)
//   types: Namespace
//    L43-46: (ns *Namespace) Quoted(v any) css.QuotedString
//    L49-52: (ns *Namespace) Unquoted(v any) css.UnquotedString
// EX L55-66: (ns *Namespace) PostCSS(args ...any) (resource.Resource, error)
//    L69-80: (ns *Namespace) TailwindCSS(args ...any) (resource.Resource, error)
// EX L83-144: (ns *Namespace) Sass(args ...any) (resource.Resource, error)
// EX L146-179: init()
//    L181-192: (ns *Namespace) getscssClientDartSass() (*dartsass.Client, error)
// ---------------------------------------------------------------------------
