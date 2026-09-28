//! Port of `tpl/partials/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/partials` init — namespace `partials` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Include", &["partial"]),
    ("IncludeCached", &["partialCached"]),
];

// Go: tpl/partials/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "partials",
        Arc::new(super::partials::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/partials/init.go (57 lines; 1/1 funcs executed)
// EX L25-57: init()
// ---------------------------------------------------------------------------
