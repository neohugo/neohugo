//! Module `glob`.
//!
//! Owner: Wave B task T02 (common-paths-text) — parent module file (only `pub mod` lines).

pub mod filename_filter;
// The module path mirrors Go's `hugofs/glob/glob.go`.
#[allow(clippy::module_inception)]
pub mod glob;
pub mod gobwas;
