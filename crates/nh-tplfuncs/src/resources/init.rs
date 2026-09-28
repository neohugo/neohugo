//! Port of `tpl/resources/init.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/resources` init — namespace `resources` and its aliases.
pub const ALIASES: &[(&str, &[&str])] =
    &[("Fingerprint", &["fingerprint"]), ("Minify", &["minify"])];

// Go: tpl/resources/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "resources",
        Arc::new(super::resources::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/resources/init.go (82 lines; 1/1 funcs executed)
// EX L27-82: init()
// ---------------------------------------------------------------------------
