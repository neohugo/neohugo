//! `nh-config`: neohugo config (base), config/{security,privacy,services}, common/hexec, common/neohugo.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod common_config;
pub mod config_loader;
pub mod config_provider;
pub mod default_config_provider;
pub mod env;
pub mod namespace;
pub mod decode;
pub mod security;
pub mod privacy;
pub mod services;
pub mod hexec;
pub mod neohugo;
