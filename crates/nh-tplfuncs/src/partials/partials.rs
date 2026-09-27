//! Port of `tpl/partials/partials.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

// Parity notes: `partial NAME DATA`: TemplateStore.LookupPartial (lower-cased, html by default), returns `template.HTML` (text partials: string) or the `return` value; executes with a child TplContext (level+1).

/// Go: `partials.Namespace` (template value `*partials.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/partials:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/partials:Include
    pub fn include(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/partials:IncludeCached
    pub fn include_cached(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "Include" => |n, ctx, a| n.include(ctx, a),
    "IncludeCached" => |n, ctx, a| n.include_cached(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*partials.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/partials/partials.go (251 lines; 5/9 funcs executed)
//   types: partialCacheKey, includeResult, partialCache, Namespace, contextWrapper
//    L51-56: (k partialCacheKey) Key() string
//    L63-67: (p *partialCache) clear()
// EX L70-87: New(deps *deps.Deps) *Namespace
//    L102-105: (c *contextWrapper) Set(in any) string
// EX L112-123: (ns *Namespace) Include(ctx context.Context, name string, contextList ...any) (any, error)
// EX L125-131: (ns *Namespace) include(ctx context.Context, name string, dataList ...any) includeResult
// EX L133-144: (ns *Namespace) lookup(name string) (*tplimpl.TemplInfo, error)
// EX L148-192: (ns *Namespace) doInclude(ctx context.Context, templ *tplimpl.TemplInfo, dataList ...any) includeResult
//    L196-251: (ns *Namespace) IncludeCached(ctx context.Context, name string, context any, variants ...any) (any, error)
// ---------------------------------------------------------------------------
