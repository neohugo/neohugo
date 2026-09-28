//! Module `collections`.
//!
//! Owner: Wave B task T18 (tplfuncs-data) — parent module file (only `pub mod` lines).

pub mod append;
pub mod apply;
// The module path mirrors Go's `tpl/collections/collections.go`.
#[allow(clippy::module_inception)]
pub mod collections;
pub mod complement;
pub mod index;
pub mod init;
pub mod merge;
pub mod querify;
pub mod reflect_helpers;
pub mod sort;
pub mod symdiff;
pub mod where_;
