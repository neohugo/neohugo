//! Module `types`.
//!
//! Owner: Wave B task T01 (common-values) — parent module file (only `pub mod` lines).

pub mod convert;
pub mod css;
pub mod hstring;
// The module path mirrors Go's `common/types/types.go`.
#[allow(clippy::module_inception)]
pub mod types;
