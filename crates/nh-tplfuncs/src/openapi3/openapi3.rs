//! Port of `tpl/openapi/openapi3/openapi3.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `openapi3.Namespace` (template value `*openapi3.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/openapi3:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/openapi3:Unmarshal
    pub fn unmarshal(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Unmarshal" => |n, ctx, a| n.unmarshal(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*openapi3.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/openapi/openapi3/openapi3.go (100 lines; 1/3 funcs executed)
//   types: Namespace, OpenAPIDocument
// EX L33-38: New(deps *deps.Deps) *Namespace
//    L52-54: (o *OpenAPIDocument) GetIdentityGroup() identity.Identity
//    L57-100: (ns *Namespace) Unmarshal(r resource.UnmarshableResource) (*OpenAPIDocument, error)
// ---------------------------------------------------------------------------
