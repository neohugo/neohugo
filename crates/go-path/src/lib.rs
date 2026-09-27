//! Port of Go's `path` and `path/filepath` (unix) packages, go1.27.1.
//!
//! * [`path`] — slash-separated paths (`path.Clean`, `Join`, `Split`, `Ext`,
//!   `Base`, `Dir`, `IsAbs`, `Match`).
//! * [`filepath`] — OS paths for unix (`filepath.Clean`, `Join`, `Split`,
//!   `Ext`, `Base`, `Dir`, `IsAbs`, `IsLocal`, `Rel`, `Match`, `SplitList`,
//!   `ToSlash`, `FromSlash`, `VolumeName`).
//!
//! Go strings are bytes: every function has a `*_bytes` form over `&[u8]`
//! and a `&str` form. See `PORTING.md`.

// Lints that fight a faithful line-by-line port of the Go control flow.
#![allow(clippy::collapsible_match)]

pub mod filepath;
pub mod path;
mod utf8;

/// Go `path.ErrBadPattern` / `filepath.ErrBadPattern`: "syntax error in pattern".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadPattern;

impl std::fmt::Display for BadPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("syntax error in pattern")
    }
}

impl std::error::Error for BadPattern {}
