//! Port of `tpl/strings/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/strings` init — namespace `strings` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Chomp", &["chomp"]),
    ("CountRunes", &["countrunes"]),
    ("CountWords", &["countwords"]),
    ("FindRE", &["findRE"]),
    ("FindRESubmatch", &["findRESubmatch"]),
    ("HasPrefix", &["hasPrefix"]),
    ("HasSuffix", &["hasSuffix"]),
    ("ToLower", &["lower"]),
    ("Replace", &["replace"]),
    ("ReplaceRE", &["replaceRE"]),
    ("SliceString", &["slicestr"]),
    ("Split", &["split"]),
    ("Substr", &["substr"]),
    ("Trim", &["trim"]),
    ("Title", &["title"]),
    ("Truncate", &["truncate"]),
    ("ToUpper", &["upper"]),
];

// Go: tpl/strings/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("strings", Arc::new(super::strings::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/init.go (249 lines; 1/1 funcs executed)
// EX L25-249: init()
// ---------------------------------------------------------------------------
