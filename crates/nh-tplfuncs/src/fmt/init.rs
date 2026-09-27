//! Port of `tpl/fmt/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/fmt` init — namespace `fmt` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Print", &["print"]),
    ("Println", &["println"]),
    ("Printf", &["printf"]),
    ("Errorf", &["errorf"]),
    ("Erroridf", &["erroridf"]),
    ("Warnidf", &["warnidf"]),
    ("Warnf", &["warnf"]),
];

// Go: tpl/fmt/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("fmt", Arc::new(super::fmt::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/fmt/init.go (86 lines; 1/1 funcs executed)
// EX L25-86: init()
// ---------------------------------------------------------------------------
