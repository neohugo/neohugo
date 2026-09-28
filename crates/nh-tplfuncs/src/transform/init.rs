//! Port of `tpl/transform/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/transform` init — namespace `transform` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Emojify", &["emojify"]),
    ("Highlight", &["highlight"]),
    ("HTMLEscape", &["htmlEscape"]),
    ("HTMLUnescape", &["htmlUnescape"]),
    ("Markdownify", &["markdownify"]),
    ("Plainify", &["plainify"]),
    ("Unmarshal", &["unmarshal"]),
];

// Go: tpl/transform/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "transform",
        Arc::new(super::transform::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/transform/init.go (129 lines; 1/1 funcs executed)
// OK L25-129: init()
// ---------------------------------------------------------------------------
