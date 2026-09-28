//! `nh-allconfig`: neohugo config/allconfig (load, decode all sections, compile, per-language configs), hugolib/segments, deploy/deployconfig stub.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(
    unused,
    dead_code,
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod allconfig;
pub mod alldecoders;
pub mod configlanguage;
pub mod deployconfig;
pub mod json;
pub mod load;
pub mod sections;
pub mod segments;
