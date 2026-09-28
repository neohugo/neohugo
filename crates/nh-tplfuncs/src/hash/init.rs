//! Port of `tpl/hash/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).
//!
//! Go has no `tpl/hash/init.go`: the namespace registers itself in `hash.go`'s `init()`.

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/hash` init — namespace `hash` and its aliases (`FNV32a` has none).
pub const ALIASES: &[(&str, &[&str])] = &[("XxHash", &["xxhash"])];

// Go: tpl/hash/hash.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "hash",
        Arc::new(super::hash::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/hash/init.go (not found in signature dump; the init is in hash.go)
// ---------------------------------------------------------------------------
