//! Port of `tpl/templates/templates.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `templates.Namespace` (template value `*templates.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/templates:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/templates:Current
    pub fn current(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/templates:Defer
    pub fn defer(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/templates:DoDefer
    pub fn do_defer(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/templates:Exists
    pub fn exists(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Current" => |n, ctx, a| n.current(ctx, a),
    "Defer" => |n, ctx, a| n.defer(ctx, a),
    "DoDefer" => |n, ctx, a| n.do_defer(ctx, a),
    "Exists" => |n, ctx, a| n.exists(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*templates.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/templates/templates.go (109 lines; 1/5 funcs executed)
//   types: Namespace, DeferOpts
// EX L30-36: New(deps *deps.Deps) *Namespace
//    L46-48: (ns *Namespace) Exists(name string) bool
//    L51-60: (ns *Namespace) Defer(args ...any) (bool, error)
//    L75-104: (ns *Namespace) DoDefer(ctx context.Context, id string, optsv any) string
//    L107-109: (ns *Namespace) Current(ctx context.Context) *tpl.CurrentTemplateInfo
// ---------------------------------------------------------------------------
