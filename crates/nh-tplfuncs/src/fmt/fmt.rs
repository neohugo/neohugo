//! Port of `tpl/fmt/fmt.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: Exact Go `fmt` semantics (go-fmt crate): `%!s(<nil>)`, Sprint spacing rule, `%q` of Stringers; `warnf`/`errorf` log (errorf makes the build fail).

/// Go: `fmt.Namespace` (template value `*fmt.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/fmt:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/fmt:Errorf
    pub fn errorf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Erroridf
    pub fn erroridf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Errormf
    pub fn errormf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Print
    pub fn print(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Printf
    pub fn printf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Println
    pub fn println(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Warnf
    pub fn warnf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Warnidf
    pub fn warnidf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/fmt:Warnmf
    pub fn warnmf(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Errorf" => |n, ctx, a| n.errorf(ctx, a),
    "Erroridf" => |n, ctx, a| n.erroridf(ctx, a),
    "Errormf" => |n, ctx, a| n.errormf(ctx, a),
    "Print" => |n, ctx, a| n.print(ctx, a),
    "Printf" => |n, ctx, a| n.printf(ctx, a),
    "Println" => |n, ctx, a| n.println(ctx, a),
    "Warnf" => |n, ctx, a| n.warnf(ctx, a),
    "Warnidf" => |n, ctx, a| n.warnidf(ctx, a),
    "Warnmf" => |n, ctx, a| n.warnmf(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*fmt.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/fmt/fmt.go (117 lines; 3/11 funcs executed)
//   types: Namespace
// EX L28-39: New(d *deps.Deps) *Namespace
// EX L47-49: (ns *Namespace) Print(args ...any) string
// EX L52-54: (ns *Namespace) Printf(format string, args ...any) string
//    L57-59: (ns *Namespace) Println(args ...any) string
//    L63-66: (ns *Namespace) Errorf(format string, args ...any) string
//    L71-74: (ns *Namespace) Erroridf(id, format string, args ...any) string
//    L78-81: (ns *Namespace) Warnf(format string, args ...any) string
//    L86-89: (ns *Namespace) Warnidf(id, format string, args ...any) string
//    L92-94: (ns *Namespace) Warnmf(m any, format string, args ...any) string
//    L97-99: (ns *Namespace) Errormf(m any, format string, args ...any) string
//    L101-117: (ns *Namespace) logmf(l logg.LevelLogger, m any, format string, args ...any) string
// ---------------------------------------------------------------------------
