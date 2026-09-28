//! Module `collections`.
//!
//! Owner: Wave B task T01 (common-values) — parent module file (only `pub mod` lines).

pub mod append;
// The module path mirrors Go's `common/collections/collections.go`.
#[allow(clippy::module_inception)]
pub mod collections;
pub mod order;
pub mod slice;
pub mod stack;
