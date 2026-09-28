//! `nh-helpers`: neohugo helpers/*, source/*, cache/filecache, cache/httpcache (+ gohugoio/httpcache + net/http response-dump subset).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

pub mod cache;
pub mod content;
pub mod emoji;
pub mod general;
pub mod path;
pub mod pathspec;
pub mod processing_stats;
pub mod source;
mod tablewriter;
pub mod url;
