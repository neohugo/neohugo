//! Go `regexp` for the patterns Hugo's config compiles: the security whitelists
//! (`security.exec.allow`, `osEnv`, `funcs.getenv`, `http.*`), the build cache busters and the
//! server redirects (`fromRe`).
//!
//! The port of Go's `regexp` (and `regexp/syntax`) lives in `nh_common::goregexp`, shared by
//! every Hugo-layer crate; this module re-exports it so the config callers keep their paths.

pub use nh_common::goregexp::*;
