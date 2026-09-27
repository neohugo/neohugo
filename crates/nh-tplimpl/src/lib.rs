//! `nh-tplimpl`: neohugo tpl/tplimpl/** (template store, layout lookup, baseof, AST transforms, exec helper) + embedded templates; engine.rs is the single adaptation point to the Wave A gotemplate crate.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod engine;
pub mod templatestore;
pub mod templates;
pub mod templatetransform;
pub mod templatedescriptor;
pub mod template_funcs;
pub mod legacy;
pub mod template_info;
pub mod category;
pub mod embedded;
