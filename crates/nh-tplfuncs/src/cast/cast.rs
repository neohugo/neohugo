//! Port of `tpl/cast/cast.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `cast.Namespace` (template value `*cast.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/cast:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/cast:ToFloat
    pub fn to_float(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/cast:ToInt
    pub fn to_int(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/cast:ToString
    pub fn to_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "ToFloat" => |n, ctx, a| n.to_float(ctx, a),
    "ToInt" => |n, ctx, a| n.to_int(ctx, a),
    "ToString" => |n, ctx, a| n.to_string(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*cast.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/cast/cast.go (62 lines; 1/5 funcs executed)
//   types: Namespace
// EX L24-26: New() *Namespace
//    L32-35: (ns *Namespace) ToInt(v any) (int, error)
//    L38-40: (ns *Namespace) ToString(v any) (string, error)
//    L43-46: (ns *Namespace) ToFloat(v any) (float64, error)
//    L48-62: convertTemplateToString(v any) any
// ---------------------------------------------------------------------------
