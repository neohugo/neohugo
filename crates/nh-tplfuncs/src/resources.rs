//! Module `resources`.
//!
//! Owner: Wave B task T15 (resource-factories) — parent module file (only `pub mod` lines).

pub mod init;
// The module path mirrors Go's `tpl/resources/resources.go`.
#[allow(clippy::module_inception)]
pub mod resources;
