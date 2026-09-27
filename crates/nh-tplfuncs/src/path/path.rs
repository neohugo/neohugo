//! Port of `tpl/path/path.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `path.Ext` on a resource uses its String() (= Name()).

/// Go: `path.Namespace` (template value `*path.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/path:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/path:Base
    pub fn base(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:BaseName
    pub fn base_name(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:Clean
    pub fn clean(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:Dir
    pub fn dir(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:Ext
    pub fn ext(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:Join
    pub fn join(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/path:Split
    pub fn split(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Base" => |n, ctx, a| n.base(ctx, a),
    "BaseName" => |n, ctx, a| n.base_name(ctx, a),
    "Clean" => |n, ctx, a| n.clean(ctx, a),
    "Dir" => |n, ctx, a| n.dir(ctx, a),
    "Ext" => |n, ctx, a| n.ext(ctx, a),
    "Join" => |n, ctx, a| n.join(ctx, a),
    "Split" => |n, ctx, a| n.split(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*path.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/path/path.go (162 lines; 2/8 funcs executed)
//   types: Namespace
// EX L28-32: New(deps *deps.Deps) *Namespace
// EX L45-52: (ns *Namespace) Ext(path any) (string, error)
//    L63-70: (ns *Namespace) Dir(path any) (string, error)
//    L78-85: (ns *Namespace) Base(path any) (string, error)
//    L93-100: (ns *Namespace) BaseName(path any) (string, error)
//    L109-118: (ns *Namespace) Split(path any) (paths.DirFile, error)
//    L126-151: (ns *Namespace) Join(elements ...any) (string, error)
//    L155-162: (ns *Namespace) Clean(path any) (string, error)
// ---------------------------------------------------------------------------
