//! Port of `tpl/collections/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/collections` init — namespace `collections` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("After", &["after"]),
    ("Apply", &["apply"]),
    ("Complement", &["complement"]),
    ("SymDiff", &["symdiff"]),
    ("Delimit", &["delimit"]),
    ("Dictionary", &["dict"]),
    ("First", &["first"]),
    ("KeyVals", &["keyVals"]),
    ("In", &["in"]),
    ("Index", &["index"]),
    ("Intersect", &["intersect"]),
    ("IsSet", &["isSet", "isset"]),
    ("Last", &["last"]),
    ("Querify", &["querify"]),
    ("Shuffle", &["shuffle"]),
    ("Slice", &["slice"]),
    ("Sort", &["sort"]),
    ("Union", &["union"]),
    ("Where", &["where"]),
    ("Append", &["append"]),
    ("Group", &["group"]),
    ("Seq", &["seq"]),
    ("NewScratch", &["newScratch"]),
    ("Uniq", &["uniq"]),
    ("Merge", &["merge"]),
];

// Go: tpl/collections/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("collections", Arc::new(super::collections::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/init.go (209 lines; 1/1 funcs executed)
// EX L25-209: init()
// ---------------------------------------------------------------------------
