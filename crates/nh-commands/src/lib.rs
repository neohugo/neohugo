//! `nh-commands`: neohugo commands/* + main.go (bin `neohugo-rs`): flags -> config, build, version/env/config commands, static copy (spf13/fsync).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Allowed for a faithful port (README rule 11): Go signatures and nested types.
#![allow(
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod cobra;
pub mod commandeer;
pub mod commands;
pub mod config;
pub mod env;
pub mod fsync;
pub mod funcmap;
pub mod helpers;
pub mod hugobuilder;
pub mod pflag;
