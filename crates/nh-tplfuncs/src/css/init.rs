//! Port of `tpl/css/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/css` init — namespace `css` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[("Sass", &["toCSS"]), ("PostCSS", &["postCSS"])];

// Go: tpl/css/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object(
        "css",
        Arc::new(super::css::Namespace::new(d.clone())),
        ALIASES,
    )
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/css/init.go (not found in signature dump; Go registers the namespace from `init()` in css.go, see `css::css`)
// ---------------------------------------------------------------------------
