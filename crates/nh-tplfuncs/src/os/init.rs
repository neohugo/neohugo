//! Port of `tpl/os/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/os` init — namespace `os` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Getenv", &["getenv"]),
    ("ReadDir", &["readDir"]),
    ("ReadFile", &["readFile"]),
    ("FileExists", &["fileExists"]),
];

// Go: tpl/os/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("os", Arc::new(super::os::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/os/init.go (64 lines; 1/1 funcs executed)
// EX L25-64: init()
// ---------------------------------------------------------------------------
