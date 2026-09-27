//! Port of `tpl/time/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/time` init — namespace `time` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Format", &["dateFormat"]),
    ("Now", &["now"]),
    ("Duration", &["duration"]),
];

// Go: tpl/time/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("time", Arc::new(super::time::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/time/init.go (96 lines; 1/1 funcs executed)
// EX L27-96: init()
// ---------------------------------------------------------------------------
