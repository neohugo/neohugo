//! Port of `tpl/site/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// `site` returns the current `page.Site` (deps.site).
pub const ALIASES: &[(&str, &[&str])] = &[];

// Go: tpl/site/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/site/init.go (42 lines; 1/1 funcs executed)
// EX L28-42: init()
// ---------------------------------------------------------------------------
