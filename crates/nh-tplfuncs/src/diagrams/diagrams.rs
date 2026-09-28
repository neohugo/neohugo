//! Port of `tpl/diagrams/diagrams.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! `SVGDiagram` is the interface of `Goat`'s result (see `diagrams::goat`, a stub).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `diagrams.Namespace` (template value `*diagrams.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    /// Go: `&Namespace{d: d}` (the namespace `init`).
    // Go: tpl/diagrams/init.go:init
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Goat creates a new SVG diagram from input v (see `diagrams::goat`).
    // Go: tpl/diagrams/goat.go:Goat
    pub fn goat(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        super::goat::goat(ctx, args)
    }
}

nh_common::go_methods!(Namespace {
    "Goat" => |n, ctx, a| n.goat(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*diagrams.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/diagrams/diagrams.go (33 lines; 0/0 funcs executed)
//   types: SVGDiagram
// ---------------------------------------------------------------------------
