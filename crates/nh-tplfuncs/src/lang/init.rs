//! Port of `tpl/lang/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/lang` init — namespace `lang` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[("Translate", &["i18n", "T"])];

// Go: tpl/lang/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "lang",
        Arc::new(super::lang::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/lang/init.go (84 lines; 1/1 funcs executed)
// OK L26-84: init()
// ---------------------------------------------------------------------------
