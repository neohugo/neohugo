//! Port of `tpl/reflect/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/reflect` init — namespace `reflect` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[];

// Go: tpl/reflect/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "reflect",
        Arc::new(super::reflect::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/reflect/init.go (53 lines; 1/1 funcs executed)
// OK L26-53: init()
// ---------------------------------------------------------------------------
