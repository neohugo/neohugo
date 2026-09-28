//! `nh-publisher`: neohugo publisher/* (DestinationPublisher, transformer chain order, htmlElementsCollector) + x/net/html subset.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port of x/net/html: Go's `switch` statements with explicit
// `return true` in every case, its long `case a.X, a.Y, ...` atom lists, the "No-op." branches of
// `unescapeEntity`, the tokenizer's `goto` labels (all `scriptData...`) and `Tokenizer.Next`.
#![allow(
    clippy::needless_return,
    clippy::match_like_matches_macro,
    clippy::if_same_then_else,
    clippy::enum_variant_names,
    clippy::should_implement_trait
)]

pub mod html_elements_collector;
pub mod publisher;
pub mod xnethtml;
