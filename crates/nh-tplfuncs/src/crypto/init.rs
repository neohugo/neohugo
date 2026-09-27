//! Port of `tpl/crypto/init.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).


use std::sync::Arc;

use nh_deps::deps::Deps;

use crate::internal::registry::TemplateFuncsNamespace;

/// Go: `tpl/crypto` init — namespace `crypto` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("MD5", &["md5"]),
    ("SHA1", &["sha1"]),
    ("SHA256", &["sha256"]),
    ("HMAC", &["hmac"]),
];

// Go: tpl/crypto/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    TemplateFuncsNamespace::from_object("crypto", Arc::new(super::crypto::Namespace::new(d.clone())), ALIASES)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/crypto/init.go (67 lines; 1/1 funcs executed)
// EX L25-67: init()
// ---------------------------------------------------------------------------
