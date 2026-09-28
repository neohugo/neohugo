//! `nh-transform`: neohugo transform/{chain,urlreplacers} and minifiers/* (tdewolff registry by media type).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port: Go's `minifiers` package holds `minifiers.go`, so the
// module is `minifiers::minifiers`.
#![allow(clippy::module_inception)]

pub mod chain;
pub mod livereloadinject;
pub mod metainject;
pub mod minifiers;
pub mod urlreplacers;
