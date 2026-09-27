//! `nh-helpers`: neohugo helpers/*, source/*, cache/filecache, cache/httpcache (+ gohugoio/httpcache + net/http response-dump subset).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod content;
pub mod general;
pub mod path;
pub mod pathspec;
pub mod processing_stats;
pub mod url;
pub mod emoji;
pub mod source;
pub mod cache;
