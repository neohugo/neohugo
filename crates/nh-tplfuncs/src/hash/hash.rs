//! Port of `tpl/hash/hash.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `hash.Namespace` (template value `*hash.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/hash:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/hash:FNV32a
    pub fn fnv32a(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/hash:XxHash
    pub fn xx_hash(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "FNV32a" => |n, ctx, a| n.fnv32a(ctx, a),
    "XxHash" => |n, ctx, a| n.xx_hash(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*hash.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/hash/hash.go (85 lines; 2/4 funcs executed)
//   types: Namespace
// EX L28-30: New() *Namespace
//    L36-44: (ns *Namespace) FNV32a(v any) (int, error)
//    L47-54: (ns *Namespace) XxHash(v any) (string, error)
// EX L58-85: init()
// ---------------------------------------------------------------------------
