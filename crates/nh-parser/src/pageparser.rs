//! Module `pageparser`.
//!
//! Owner: Wave B task T03 (parser-langs) — parent module file (only `pub mod` lines).

pub mod item;
pub mod pagelexer;
pub mod pagelexer_intro;
pub mod pagelexer_shortcode;
#[allow(clippy::module_inception)]
// the skeleton's module path (Go: parser/pageparser/pageparser.go)
pub mod pageparser;
