//! Port of `tpl/js/js.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `js.Namespace` (template value `*js.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/js:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/js:Babel
    pub fn babel(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/js:Batch
    pub fn batch(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/js:Build
    pub fn build(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Babel" => |n, ctx, a| n.babel(ctx, a),
    "Batch" => |n, ctx, a| n.batch(ctx, a),
    "Build" => |n, ctx, a| n.build(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*js.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/js/js.go (117 lines; 2/4 funcs executed)
//   types: Namespace
// EX L32-43: New(d *deps.Deps) (*Namespace, error)
// EX L55-78: (ns *Namespace) Build(args ...any) (resource.Resource, error)
//    L84-96: (ns *Namespace) Batch(id string) (js.Batcher, error)
//    L99-117: (ns *Namespace) Babel(args ...any) (resource.Resource, error)
// ---------------------------------------------------------------------------
