//! `nh-tplimpl`: neohugo tpl/tplimpl/** (template store, layout lookup, baseof, AST transforms, exec helper) + embedded templates; engine.rs is the single adaptation point to the Wave A gotemplate crate.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port: Go functions with many parameters (insertTemplate2) and
// Go's nested map types.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod category;
pub mod embedded;
pub mod engine;
pub mod legacy;
pub mod template_funcs;
pub mod template_info;
pub mod templatedescriptor;
pub mod templates;
pub mod templatestore;
pub mod templatetransform;
