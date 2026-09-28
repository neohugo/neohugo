//! Port of `tpl/cast/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/cast` init — namespace `cast` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("ToInt", &["int"]),
    ("ToString", &["string"]),
    ("ToFloat", &["float"]),
];

// Go: tpl/cast/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "cast",
        Arc::new(super::cast::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/cast/init.go (59 lines; 1/1 funcs executed)
// OK L25-59: init()
// ---------------------------------------------------------------------------
