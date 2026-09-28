//! Port of `tpl/inflect/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/inflect` init — namespace `inflect` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Humanize", &["humanize"]),
    ("Pluralize", &["pluralize"]),
    ("Singularize", &["singularize"]),
];

// Go: tpl/inflect/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "inflect",
        Arc::new(super::inflect::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/inflect/init.go (62 lines; 1/1 funcs executed)
// OK L25-62: init()
// ---------------------------------------------------------------------------
