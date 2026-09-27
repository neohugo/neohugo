//! Port of `tpl/encoding/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/encoding` init — namespace `encoding` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Base64Decode", &["base64Decode"]),
    ("Base64Encode", &["base64Encode"]),
    ("Jsonify", &["jsonify"]),
];

// Go: tpl/encoding/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("encoding", Arc::new(super::encoding::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/encoding/init.go (61 lines; 1/1 funcs executed)
// EX L25-61: init()
// ---------------------------------------------------------------------------
