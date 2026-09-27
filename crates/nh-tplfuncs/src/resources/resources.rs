//! Port of `tpl/resources/resources.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `Get` (nil for missing), `GetRemote` (getresource file cache), `Concat` (first caller wins per target path), `ExecuteAsTemplate`, `Fingerprint`, `Minify`, `PostProcess`, `ToCSS`/`PostCSS` (delegating to tpl/css).

/// Go: `resources.Namespace` (template value `*resources.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/resources:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/resources:Babel
    pub fn babel(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:ByType
    pub fn by_type(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Concat
    pub fn concat(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Copy
    pub fn copy(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:ExecuteAsTemplate
    pub fn execute_as_template(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Fingerprint
    pub fn fingerprint(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:FromString
    pub fn from_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Get
    pub fn get(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:GetMatch
    pub fn get_match(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:GetRemote
    pub fn get_remote(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Match
    pub fn match_(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:Minify
    pub fn minify(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:PostCSS
    pub fn post_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:PostProcess
    pub fn post_process(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/resources:ToCSS
    pub fn to_css(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Babel" => |n, ctx, a| n.babel(ctx, a),
    "ByType" => |n, ctx, a| n.by_type(ctx, a),
    "Concat" => |n, ctx, a| n.concat(ctx, a),
    "Copy" => |n, ctx, a| n.copy(ctx, a),
    "ExecuteAsTemplate" => |n, ctx, a| n.execute_as_template(ctx, a),
    "Fingerprint" => |n, ctx, a| n.fingerprint(ctx, a),
    "FromString" => |n, ctx, a| n.from_string(ctx, a),
    "Get" => |n, ctx, a| n.get(ctx, a),
    "GetMatch" => |n, ctx, a| n.get_match(ctx, a),
    "GetRemote" => |n, ctx, a| n.get_remote(ctx, a),
    "Match" => |n, ctx, a| n.match_(ctx, a),
    "Minify" => |n, ctx, a| n.minify(ctx, a),
    "PostCSS" => |n, ctx, a| n.post_css(ctx, a),
    "PostProcess" => |n, ctx, a| n.post_process(ctx, a),
    "ToCSS" => |n, ctx, a| n.to_css(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*resources.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/resources/resources.go (330 lines; 8/16 funcs executed)
//   types: Namespace
// EX L44-62: New(deps *deps.Deps) (*Namespace, error)
//    L83-89: (ns *Namespace) Copy(s any, r resource.Resource) (resource.Resource, error)
// EX L93-109: (ns *Namespace) Get(filename any) resource.Resource
// EX L118-151: (ns *Namespace) GetRemote(args ...any) (resource.Resource, error)
//    L158-170: (ns *Namespace) GetMatch(pattern any) resource.Resource
//    L173-175: (ns *Namespace) ByType(typ any) resource.Resources
//    L193-205: (ns *Namespace) Match(pattern any) resource.Resources
// EX L209-231: (ns *Namespace) Concat(targetPathIn any, r any) (resource.Resource, error)
//    L234-245: (ns *Namespace) FromString(targetPathIn, contentIn any) (resource.Resource, error)
// EX L249-265: (ns *Namespace) ExecuteAsTemplate(ctx context.Context, args ...any) (resource.Resource, error)
// EX L269-296: (ns *Namespace) Fingerprint(args ...any) (resource.Resource, error)
// EX L300-302: (ns *Namespace) Minify(r resources.ResourceTransformer) (resource.Resource, error)
//    L308-311: (ns *Namespace) ToCSS(args ...any) (resource.Resource, error)
//    L315-318: (ns *Namespace) PostCSS(args ...any) (resource.Resource, error)
// EX L321-323: (ns *Namespace) PostProcess(r resource.Resource) (postpub.PostPublishedResource, error)
//    L327-330: (ns *Namespace) Babel(args ...any) (resource.Resource, error)
// ---------------------------------------------------------------------------
