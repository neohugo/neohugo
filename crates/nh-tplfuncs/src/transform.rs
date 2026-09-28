//! Module `transform`.
//!
//! Owner: Wave B task T19 (tplfuncs-host) — parent module file (only `pub mod` lines).

pub mod init;
pub mod remarshal;
// The module path mirrors Go's `tpl/transform/transform.go`.
#[allow(clippy::module_inception)]
pub mod transform;
pub mod unmarshal;
