//! Module `cache::httpcache`.
//!
//! Owner: Wave B task T08 (helpers-source-cache) — parent module file (only `pub mod` lines).

pub mod http;
// The module layout mirrors the Go package path (cache/httpcache/httpcache.go).
#[allow(clippy::module_inception)]
pub mod httpcache;
pub mod transport;
