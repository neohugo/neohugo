//! Port of `tpl/math/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/math` init — namespace `math` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Add", &["add"]),
    ("Div", &["div"]),
    ("Mod", &["mod"]),
    ("ModBool", &["modBool"]),
    ("Mul", &["mul"]),
    ("Pow", &["pow"]),
    ("Sub", &["sub"]),
];

// Go: tpl/math/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "math",
        Arc::new(super::math::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/math/init.go (227 lines; 1/1 funcs executed)
// OK L25-227: init()
// ---------------------------------------------------------------------------
