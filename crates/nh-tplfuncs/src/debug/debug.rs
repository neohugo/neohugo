//! Port of `tpl/debug/debug.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `debug.Namespace` (template value `*debug.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/debug:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/debug:Dump
    pub fn dump(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/debug:Timer
    pub fn timer(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/debug:VisualizeSpaces
    pub fn visualize_spaces(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Dump" => |n, ctx, a| n.dump(ctx, a),
    "Timer" => |n, ctx, a| n.timer(ctx, a),
    "VisualizeSpaces" => |n, ctx, a| n.visualize_spaces(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*debug.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/debug/debug.go (185 lines; 1/9 funcs executed)
//   types: Namespace, nopTimerImpl, Timer, timer
// EX L32-92: New(d *deps.Deps) *Namespace
//    L109-115: (ns *Namespace) Dump(val any) string
//    L118-121: (ns *Namespace) VisualizeSpaces(val any) string
//    L123-132: (ns *Namespace) Timer(name string) Timer
//    L138-140: (nopTimerImpl) Stop() string
//    L156-162: (t *timer) Stop() string
//    L165-169: (ns *Namespace) TestDeprecationInfo(item, alternative string) string
//    L172-177: (ns *Namespace) TestDeprecationWarn(item, alternative string) string
//    L180-185: (ns *Namespace) TestDeprecationErr(item, alternative string) string
// ---------------------------------------------------------------------------
