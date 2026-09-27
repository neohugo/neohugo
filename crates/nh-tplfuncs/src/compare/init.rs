//! Port of `tpl/compare/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/compare` init — namespace `compare` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Default", &["default"]),
    ("Eq", &["eq"]),
    ("Ge", &["ge"]),
    ("Gt", &["gt"]),
    ("Le", &["le"]),
    ("Lt", &["lt"]),
    ("Ne", &["ne"]),
    ("Conditional", &["cond"]),
];

// Go: tpl/compare/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("compare", Arc::new(super::compare::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/compare/init.go (93 lines; 1/1 funcs executed)
// EX L26-93: init()
// ---------------------------------------------------------------------------
