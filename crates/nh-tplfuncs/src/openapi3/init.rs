//! Port of `tpl/openapi/openapi3/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/openapi3` init — namespace `openapi3` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[];

// Go: tpl/openapi3/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "openapi3",
        Arc::new(super::openapi3::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/openapi/openapi3/init.go (43 lines; 1/1 funcs executed)
// OK L25-43: init()
// ---------------------------------------------------------------------------
