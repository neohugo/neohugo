//! Port of `tpl/data/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/data` init — namespace `data` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[("GetCSV", &["getCSV"]), ("GetJSON", &["getJSON"])];

// Go: tpl/data/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "data",
        Arc::new(super::data::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/data/init.go (47 lines; 1/1 funcs executed)
// OK L25-47: init()
// ---------------------------------------------------------------------------
