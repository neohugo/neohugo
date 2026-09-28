//! Port of `tpl/hugo/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// `hugo` returns `neohugo.HugoInfo` (`hugo.Environment` is a struct field).
pub const ALIASES: &[(&str, &[&str])] = &[];

// Go: tpl/hugo/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/hugo/init.go (44 lines; 1/1 funcs executed)
// EX L26-44: init()
// ---------------------------------------------------------------------------
