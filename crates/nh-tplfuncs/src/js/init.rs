//! Port of `tpl/js/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/js` init — namespace `js` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[("Babel", &["babel"])];

// Go: tpl/js/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "js",
        Arc::new(super::js::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/js/init.go (46 lines; 1/1 funcs executed)
// EX L25-46: init()
// ---------------------------------------------------------------------------
