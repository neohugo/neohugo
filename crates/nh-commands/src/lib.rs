//! `nh-commands`: neohugo commands/* + main.go (bin `neohugo`): flags -> config, build, version/env/config commands, static copy (spf13/fsync).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod commandeer;
pub mod commands;
pub mod helpers;
pub mod hugobuilder;
pub mod config;
pub mod env;
pub mod fsync;
