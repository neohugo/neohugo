//! Port of `tpl/reflect/reflect.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

/// Go: `reflect.Namespace` (template value `*reflect.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/reflect:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/reflect:IsMap
    pub fn is_map(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "IsMap")?;
        // Go: tpl/reflect/reflect.go:IsMap
        Ok(Value::Bool(nh_common::hreflect::is_map(&a[0])))
    }

    // Go: tpl/reflect:IsSlice
    pub fn is_slice(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "IsSlice")?;
        // Go: tpl/reflect/reflect.go:IsSlice
        Ok(Value::Bool(nh_common::hreflect::is_slice(&a[0])))
    }
}

nh_common::go_methods!(Namespace {
    "IsMap" => |n, ctx, a| n.is_map(ctx, a),
    "IsSlice" => |n, ctx, a| n.is_slice(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*reflect.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/reflect/reflect.go (36 lines; 2/3 funcs executed)
//   types: Namespace
// OK L21-23: New() *Namespace
// OK L29-31: (ns *Namespace) IsMap(v any) bool
// OK L34-36: (ns *Namespace) IsSlice(v any) bool
// ---------------------------------------------------------------------------
