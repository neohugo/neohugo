//! Port of `tpl/os/os.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `os.Namespace` (template value `*os.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/os:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/os:FileExists
    pub fn file_exists(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/os:Getenv
    pub fn getenv(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/os:ReadDir
    pub fn read_dir(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/os:ReadFile
    pub fn read_file(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/os:Stat
    pub fn stat(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "FileExists" => |n, ctx, a| n.file_exists(ctx, a),
    "Getenv" => |n, ctx, a| n.getenv(ctx, a),
    "ReadDir" => |n, ctx, a| n.read_dir(ctx, a),
    "ReadFile" => |n, ctx, a| n.read_file(ctx, a),
    "Stat" => |n, ctx, a| n.stat(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*os.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/os/os.go (163 lines; 1/7 funcs executed)
//   types: Namespace
// EX L32-52: New(d *deps.Deps) *Namespace
//    L63-74: (ns *Namespace) Getenv(key any) (string, error)
//    L78-90: readFile(fs afero.Fs, filename string) (string, error)
//    L95-110: (ns *Namespace) ReadFile(i any) (string, error)
//    L113-125: (ns *Namespace) ReadDir(i any) ([]_os.FileInfo, error)
//    L128-144: (ns *Namespace) FileExists(i any) (bool, error)
//    L147-163: (ns *Namespace) Stat(i any) (_os.FileInfo, error)
// ---------------------------------------------------------------------------
