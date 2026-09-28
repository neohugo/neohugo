//! Module `strings`.
//!
//! Owner: Wave B task T19 (tplfuncs-host) — parent module file (only `pub mod` lines).

pub mod diff;
pub mod init;
pub mod regexp;
// The module path mirrors Go's `tpl/strings/strings.go`.
#[allow(clippy::module_inception)]
pub mod strings;
pub mod truncate;
