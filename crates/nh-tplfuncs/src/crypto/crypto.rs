//! Port of `tpl/crypto/crypto.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `crypto.Namespace` (template value `*crypto.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/crypto:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/crypto:FNV32a
    pub fn fnv32a(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/crypto:HMAC
    pub fn hmac(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/crypto:MD5
    pub fn md5(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/crypto:SHA1
    pub fn sha1(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/crypto:SHA256
    pub fn sha256(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "FNV32a" => |n, ctx, a| n.fnv32a(ctx, a),
    "HMAC" => |n, ctx, a| n.hmac(ctx, a),
    "MD5" => |n, ctx, a| n.md5(ctx, a),
    "SHA1" => |n, ctx, a| n.sha1(ctx, a),
    "SHA256" => |n, ctx, a| n.sha256(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*crypto.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/crypto/crypto.go (139 lines; 1/6 funcs executed)
//   types: Namespace
// EX L33-35: New() *Namespace
//    L41-49: (ns *Namespace) MD5(v any) (string, error)
//    L52-60: (ns *Namespace) SHA1(v any) (string, error)
//    L63-71: (ns *Namespace) SHA256(v any) (string, error)
//    L75-84: (ns *Namespace) FNV32a(v any) (int, error)
//    L87-139: (ns *Namespace) HMAC(h any, k any, m any, e ...any) (string, error)
// ---------------------------------------------------------------------------
