//! Port of `tpl/templates/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/templates` init — namespace `templates` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[("DoDefer", &["doDefer"])];

// Go: tpl/templates/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "templates",
        Arc::new(super::templates::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/templates/init.go (56 lines; 1/1 funcs executed)
// OK L25-56: init()
// ---------------------------------------------------------------------------
