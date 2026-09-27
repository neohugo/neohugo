//! Port of `tpl/compare/compare.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `eq` normalises ints to int64, floats to float64 and uses `Eqer` (`has_method("Eq")`) — `eq 1 1.0` is false; `ge/gt/le/lt` via `compareGetWithCollator` (numeric strings as numbers, time as Unix seconds); `default` rules; `cond`.

/// Go: `compare.Namespace` (template value `*compare.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/compare:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/compare:Conditional
    pub fn conditional(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Default
    pub fn default(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Eq
    pub fn eq(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Ge
    pub fn ge(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Gt
    pub fn gt(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Le
    pub fn le(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Lt
    pub fn lt(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:LtCollate
    pub fn lt_collate(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/compare:Ne
    pub fn ne(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Conditional" => |n, ctx, a| n.conditional(ctx, a),
    "Default" => |n, ctx, a| n.default(ctx, a),
    "Eq" => |n, ctx, a| n.eq(ctx, a),
    "Ge" => |n, ctx, a| n.ge(ctx, a),
    "Gt" => |n, ctx, a| n.gt(ctx, a),
    "Le" => |n, ctx, a| n.le(ctx, a),
    "Lt" => |n, ctx, a| n.lt(ctx, a),
    "LtCollate" => |n, ctx, a| n.lt_collate(ctx, a),
    "Ne" => |n, ctx, a| n.ne(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*compare.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/compare/compare.go (388 lines; 13/15 funcs executed)
//   types: Namespace
// EX L33-35: New(loc *time.Location, caseInsensitive bool) *Namespace
// EX L48-96: (*Namespace) Default(defaultv any, givenv ...any) (any, error)
// EX L99-156: (n *Namespace) Eq(first any, others ...any) bool
// EX L159-167: (n *Namespace) Ne(first any, others ...any) bool
// EX L170-179: (n *Namespace) Ge(first any, others ...any) bool
// EX L182-191: (n *Namespace) Gt(first any, others ...any) bool
// EX L194-203: (n *Namespace) Le(first any, others ...any) bool
// EX L208-217: (n *Namespace) LtCollate(collator *langs.Collator, first any, others ...any) bool
// EX L220-222: (n *Namespace) Lt(first any, others ...any) bool
// EX L224-229: (n *Namespace) checkComparisonArgCount(min int, others ...any) bool
//    L234-239: (n *Namespace) Conditional(cond any, v1, v2 any) any
// EX L241-243: (ns *Namespace) compareGet(a any, b any) (float64, float64)
//    L245-253: (ns *Namespace) compareTwoUints(a uint64, b uint64) (float64, float64)
// EX L255-380: (ns *Namespace) compareGetWithCollator(collator *langs.Collator, a any, b any) (float64, float64)
// EX L382-388: (ns *Namespace) toTimeUnix(v reflect.Value) int64
// ---------------------------------------------------------------------------
