//! Integration tests of `neohugo-funcs` (the crate's single test binary, REWRITE_PLAN.md §2.2).

#[cfg(feature = "runtime")]
mod support;

#[cfg(feature = "runtime")]
mod determinism;
#[cfg(feature = "math")]
mod math;
#[cfg(feature = "runtime")]
mod oracle;
#[cfg(feature = "runtime")]
mod registration;
#[cfg(feature = "runtime")]
mod snapshots;
